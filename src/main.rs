mod bots;
mod config;
mod embed;
mod image_card;
mod proxy;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, Method, Response, StatusCode, Uri},
    routing::{any, get},
    Router,
};
use reqwest::Client;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    bots::should_render_embed,
    config::Config,
    embed::{metadata_for_image, metadata_for_path, render_embed_html},
    image_card::render_png,
    proxy::{plain_response, proxy_request},
};

#[derive(Clone)]
struct AppState {
    config: Config,
    client: Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = Config::from_env()?;
    let bind_addr = config.bind_addr;
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("umamoe-embeds/0.1")
        .build()
        .context("failed to build HTTP client")?;

    let state = Arc::new(AppState { config, client });

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/__embeds/images/:kind/:id",
            get(image_handler).head(image_head_handler),
        )
        .fallback(any(page_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;

    info!(%bind_addr, "umamoe-embeds listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server failed")?;

    Ok(())
}

async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn page_handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    if (method == Method::GET || method == Method::HEAD)
        && should_render_embed(&headers, &uri, &state.config)
    {
        if let Some(meta) = metadata_for_path(&state.client, &state.config, uri.path()).await {
            let html = render_embed_html(&meta);
            return embed_html_response(method, html);
        }
    }

    proxy_request(&state.client, &state.config, method, uri, headers, body).await
}

async fn image_handler(
    State(state): State<Arc<AppState>>,
    Path((kind, id)): Path<(String, String)>,
) -> Response<Body> {
    let Some(meta) = metadata_for_image(&state.client, &state.config, &kind, &id).await else {
        return plain_response(StatusCode::NOT_FOUND, "Unknown embed image.");
    };

    match render_png(&meta) {
        Ok(bytes) => image_response(Method::GET, Some(bytes)),
        Err(error) => {
            warn!(%error, "failed to render embed image");
            plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render embed image.",
            )
        }
    }
}

async fn image_head_handler() -> Response<Body> {
    image_response(Method::HEAD, None)
}

fn embed_html_response(method: Method, html: String) -> Response<Body> {
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(html)
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::VARY, "User-Agent")
        .body(body)
        .expect("embed HTML response is valid")
}

fn image_response(method: Method, bytes: Option<Vec<u8>>) -> Response<Body> {
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(bytes.unwrap_or_default())
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(
            header::CACHE_CONTROL,
            "public, max-age=300, stale-while-revalidate=86400",
        )
        .body(body)
        .expect("image response is valid")
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("umamoe_embeds=info,tower_http=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        warn!(%error, "failed to listen for shutdown signal");
    }
}
