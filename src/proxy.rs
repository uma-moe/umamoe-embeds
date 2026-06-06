use axum::{
    body::{Body, Bytes},
    http::{header, HeaderMap, Method, Response, StatusCode, Uri},
};
use reqwest::Client;
use tracing::warn;

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
