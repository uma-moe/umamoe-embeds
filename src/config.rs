use std::{env, net::SocketAddr, time::Duration};

use anyhow::{Context, Result};

const DEFAULT_BOT_TOKENS: &[&str] = &[
    "Discordbot",
    "Twitterbot",
    "Slackbot",
    "facebookexternalhit",
    "Facebot",
    "LinkedInBot",
    "WhatsApp",
    "TelegramBot",
    "SkypeUriPreview",
    "Pinterestbot",
    "redditbot",
    "Tumblr",
    "Viber",
    "Line",
    "Embedly",
    "Iframely",
    "vkShare",
    "Mastodon",
    "Misskey",
    "Bluesky",
];
const DEFAULT_RESOURCES_BASE_URL: &str = "http://umamoe-resources:3204/resources";
const DEFAULT_ASSET_BASE_URL: &str = "https://uma.moe/assets";
const DEFAULT_API_BASE_URL: &str = "http://umamoe-backend:3201";
const DEFAULT_SEARCH_BASE_URL: &str = "http://umamoe-search:3202";

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub public_base_url: String,
    pub frontend_origin: String,
    pub asset_base_url: String,
    pub api_base_url: String,
    pub search_base_url: String,
    pub resources_base_url: String,
    pub resources_api_token: Option<String>,
    pub bot_user_agent_tokens: Vec<String>,
    pub debug_query_key: String,
    pub image_cache_max_age: Duration,
    pub image_cache_stale_while_revalidate: Duration,
    pub image_cache_max_entries: usize,
    pub render_max_concurrency: usize,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind_addr = env::var("UMAMOE_EMBEDS_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse::<SocketAddr>()
            .context("UMAMOE_EMBEDS_BIND must be a socket address, for example 0.0.0.0:8080")?;

        let public_base_url = normalize_base_url(
            env::var("UMAMOE_PUBLIC_BASE_URL").unwrap_or_else(|_| "https://uma.moe".to_string()),
        );

        let frontend_origin = normalize_base_url(
            env::var("UMAMOE_FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://127.0.0.1:4200".to_string()),
        );

        let asset_base_url = normalize_base_url(
            env::var("UMAMOE_ASSET_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_ASSET_BASE_URL.to_string()),
        );

        let api_base_url = normalize_base_url(
            env::var("UMAMOE_API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string()),
        );

        let search_base_url = normalize_base_url(
            env::var("UMAMOE_SEARCH_BASE_URL")
                .unwrap_or_else(|_| DEFAULT_SEARCH_BASE_URL.to_string()),
        );

        let resources_base_url = normalize_base_url(
            env::var("UMAMOE_RESOURCES_BASE_URL")
                .or_else(|_| env::var("RESOURCE_BASE_URL"))
                .unwrap_or_else(|_| DEFAULT_RESOURCES_BASE_URL.to_string()),
        );

        let resources_api_token = env::var("UMAMOE_RESOURCES_API_TOKEN")
            .ok()
            .map(|token| token.trim().to_string())
            .filter(|token| !token.is_empty());

        let bot_user_agent_tokens = env::var("UMAMOE_EMBEDS_BOT_USER_AGENT_TOKENS")
            .ok()
            .map(|value| split_csv(&value))
            .filter(|tokens| !tokens.is_empty())
            .unwrap_or_else(|| {
                DEFAULT_BOT_TOKENS
                    .iter()
                    .map(|token| token.to_string())
                    .collect()
            });

        let debug_query_key =
            env::var("UMAMOE_EMBEDS_DEBUG_QUERY_KEY").unwrap_or_else(|_| "__embed".to_string());

        let image_cache_max_age =
            duration_from_env("UMAMOE_EMBEDS_IMAGE_CACHE_MAX_AGE_SECONDS", 300);
        let image_cache_stale_while_revalidate =
            duration_from_env("UMAMOE_EMBEDS_IMAGE_CACHE_STALE_SECONDS", 86_400);
        let image_cache_max_entries =
            usize_from_env("UMAMOE_EMBEDS_IMAGE_CACHE_MAX_ENTRIES", 256).max(1);
        let render_max_concurrency =
            usize_from_env("UMAMOE_EMBEDS_RENDER_MAX_CONCURRENCY", 1).max(1);

        Ok(Self {
            bind_addr,
            public_base_url,
            frontend_origin,
            asset_base_url,
            api_base_url,
            search_base_url,
            resources_base_url,
            resources_api_token,
            bot_user_agent_tokens,
            debug_query_key,
            image_cache_max_age,
            image_cache_stale_while_revalidate,
            image_cache_max_entries,
            render_max_concurrency,
        })
    }
}

fn normalize_base_url(value: String) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn duration_from_env(name: &str, default_seconds: u64) -> Duration {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(default_seconds))
}

fn usize_from_env(name: &str, default_value: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default_value)
}
