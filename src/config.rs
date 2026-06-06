use std::{env, net::SocketAddr};

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

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub public_base_url: String,
    pub frontend_origin: String,
    pub api_base_url: String,
    pub bot_user_agent_tokens: Vec<String>,
    pub debug_query_key: String,
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

        let api_base_url = normalize_base_url(
            env::var("UMAMOE_API_BASE_URL").unwrap_or_else(|_| public_base_url.clone()),
        );

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

        Ok(Self {
            bind_addr,
            public_base_url,
            frontend_origin,
            api_base_url,
            bot_user_agent_tokens,
            debug_query_key,
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
