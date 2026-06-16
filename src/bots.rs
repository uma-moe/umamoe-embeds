use axum::http::{header::USER_AGENT, HeaderMap, Uri};

use crate::config::Config;

pub fn should_render_embed(headers: &HeaderMap, uri: &Uri, config: &Config) -> bool {
    has_debug_query(uri, &config.debug_query_key) || is_embed_bot(headers, config)
}

fn is_embed_bot(headers: &HeaderMap, config: &Config) -> bool {
    let Some(user_agent) = headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };

    let user_agent = user_agent.to_ascii_lowercase();

    config
        .bot_user_agent_tokens
        .iter()
        .map(|token| token.to_ascii_lowercase())
        .any(|token| user_agent.contains(&token))
}

pub fn has_debug_query(uri: &Uri, debug_query_key: &str) -> bool {
    let Some(query) = uri.query() else {
        return false;
    };

    query.split('&').any(|part| {
        let key = part.split_once('=').map_or(part, |(key, _)| key);
        key == debug_query_key
    })
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, Uri};

    use super::*;

    fn config() -> Config {
        Config {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            public_base_url: "https://uma.moe".to_string(),
            frontend_origin: "http://127.0.0.1:4200".to_string(),
            asset_base_url: "https://uma.moe/assets".to_string(),
            api_base_url: "http://umamoe-backend:3201".to_string(),
            search_base_url: "http://umamoe-search:3202".to_string(),
            resources_base_url: "http://umamoe-resources:3204/resources".to_string(),
            resources_api_token: None,
            bot_user_agent_tokens: vec!["Discordbot".to_string(), "Slackbot".to_string()],
            debug_query_key: "__embed".to_string(),
            image_cache_bust: "test".to_string(),
            image_cache_max_age: std::time::Duration::from_secs(300),
            image_cache_stale_while_revalidate: std::time::Duration::from_secs(86_400),
            image_cache_max_entries: 256,
            render_max_concurrency: 1,
        }
    }

    #[test]
    fn detects_known_embed_bot() {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("Discordbot/2.0"));

        assert!(should_render_embed(
            &headers,
            &"/circles/123".parse::<Uri>().unwrap(),
            &config()
        ));
    }

    #[test]
    fn ignores_normal_browser() {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 Firefox/139"),
        );

        assert!(!should_render_embed(
            &headers,
            &"/circles/123".parse::<Uri>().unwrap(),
            &config()
        ));
    }

    #[test]
    fn debug_query_forces_embed() {
        let headers = HeaderMap::new();

        assert!(should_render_embed(
            &headers,
            &"/circles/123?__embed=1".parse::<Uri>().unwrap(),
            &config()
        ));
    }
}
