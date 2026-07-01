use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, Method, Response, StatusCode, Uri},
};
use reqwest::Client;
use tracing::{debug, warn};
use url::Url;

use crate::config::Config;

const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

pub async fn proxy_request(
    client: &Client,
    config: &Config,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let path_and_query = uri.path_and_query().map_or("/", |value| value.as_str());
    let target_url = format!("{}{}", config.frontend_origin, path_and_query);
    if is_self_referential_frontend_proxy(config, &headers) {
        debug!(
            target_url = %target_url,
            frontend_origin = %config.frontend_origin,
            public_base_url = %config.public_base_url,
            "refusing self-referential frontend proxy request"
        );
        return plain_response(
            StatusCode::MISDIRECTED_REQUEST,
            "The uma.moe frontend proxy target is self-referential.",
        );
    }

    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET);

    let mut request = client.request(reqwest_method, target_url);
    for (name, value) in headers.iter() {
        if should_forward_request_header(name.as_str()) {
            request = request.header(name.as_str(), value.as_bytes());
        }
    }

    if let Some(host) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
    {
        request = request.header("x-forwarded-host", host);
    }

    let request = if method == Method::GET || method == Method::HEAD {
        request
    } else {
        request.body(body)
    };

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(%error, "frontend proxy request failed");
            return plain_response(
                StatusCode::BAD_GATEWAY,
                "The uma.moe frontend shell is currently unavailable.",
            );
        }
    };

    let status = response.status();
    let mut builder = Response::builder().status(status);

    for (name, value) in response.headers().iter() {
        if should_forward_response_header(name.as_str()) {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
    }

    if method == Method::HEAD {
        return builder.body(Body::empty()).unwrap_or_else(|_| {
            plain_response(StatusCode::BAD_GATEWAY, "Invalid upstream response")
        });
    }

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => {
            warn!(%error, "failed to read frontend proxy response body");
            return plain_response(StatusCode::BAD_GATEWAY, "Invalid upstream response body.");
        }
    };

    builder
        .body(Body::from(bytes))
        .unwrap_or_else(|_| plain_response(StatusCode::BAD_GATEWAY, "Invalid upstream response"))
}

pub fn plain_response(status: StatusCode, body: &'static str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(body))
        .expect("plain response is valid")
}

fn should_forward_request_header(name: &str) -> bool {
    let name = name.to_ascii_lowercase();

    !HOP_BY_HOP_HEADERS.contains(&name.as_str())
        && name != "host"
        && name != "content-length"
        && name != "accept-encoding"
}

fn should_forward_response_header(name: &str) -> bool {
    let name = name.to_ascii_lowercase();

    !HOP_BY_HOP_HEADERS.contains(&name.as_str())
        && name != "content-length"
        && name != "content-encoding"
}

fn is_self_referential_frontend_proxy(config: &Config, headers: &HeaderMap) -> bool {
    let Some(frontend_origin) = parse_origin(&config.frontend_origin) else {
        return false;
    };

    if parse_origin(&config.public_base_url).as_ref() == Some(&frontend_origin) {
        return true;
    }

    headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|host| host_header_matches_origin(host, &frontend_origin))
}

#[derive(Debug, PartialEq, Eq)]
struct Origin {
    scheme: String,
    host: String,
    port: u16,
}

fn parse_origin(value: &str) -> Option<Origin> {
    let url = Url::parse(value).ok()?;

    Some(Origin {
        scheme: url.scheme().to_ascii_lowercase(),
        host: normalize_host(url.host_str()?),
        port: url.port_or_known_default()?,
    })
}

fn host_header_matches_origin(host: &str, origin: &Origin) -> bool {
    let url = Url::parse(&format!("{}://{}", origin.scheme, host.trim())).ok();
    let Some(url) = url else {
        return false;
    };

    normalize_host(url.host_str().unwrap_or_default()) == origin.host
        && url.port_or_known_default() == Some(origin.port)
}

fn normalize_host(host: &str) -> String {
    host.trim_end_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::Duration};

    use axum::http::HeaderValue;

    use super::*;

    fn config(public_base_url: &str, frontend_origin: &str) -> Config {
        Config {
            bind_addr: "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
            public_base_url: public_base_url.to_string(),
            frontend_origin: frontend_origin.to_string(),
            asset_base_url: "https://uma.moe/assets".to_string(),
            api_base_url: "http://umamoe-backend:3201".to_string(),
            search_base_url: "http://umamoe-search:3202".to_string(),
            resources_base_url: "http://umamoe-resources:3204/resources".to_string(),
            resources_api_token: None,
            bot_user_agent_tokens: vec!["Discordbot".to_string()],
            debug_query_key: "__embed".to_string(),
            image_cache_bust: "test".to_string(),
            image_cache_max_age: Duration::from_secs(300),
            image_cache_stale_while_revalidate: Duration::from_secs(86_400),
            image_cache_max_entries: 256,
            render_max_concurrency: 1,
        }
    }

    #[test]
    fn detects_public_origin_as_self_referential() {
        let headers = HeaderMap::new();
        let config = config("https://uma.moe", "https://uma.moe");

        assert!(is_self_referential_frontend_proxy(&config, &headers));
    }

    #[test]
    fn detects_host_header_as_self_referential() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("uma.moe"));
        let config = config("https://beta.uma.moe", "https://uma.moe");

        assert!(is_self_referential_frontend_proxy(&config, &headers));
    }

    #[test]
    fn allows_internal_frontend_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("uma.moe"));
        let config = config("https://uma.moe", "http://umamoe-frontend-shell:80");

        assert!(!is_self_referential_frontend_proxy(&config, &headers));
    }

    #[tokio::test]
    async fn self_referential_proxy_returns_before_request() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("uma.moe"));
        let config = config("https://uma.moe", "https://uma.moe");

        let response = proxy_request(
            &Client::new(),
            &config,
            Method::GET,
            "/robots.txt".parse::<Uri>().unwrap(),
            headers,
            Bytes::new(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::MISDIRECTED_REQUEST);
    }
}
