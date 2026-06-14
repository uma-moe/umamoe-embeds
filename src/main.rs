mod bots;
mod config;
mod embed;
mod html_card;
mod image_card;
mod proxy;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, Method, Response, StatusCode, Uri},
    routing::{any, get},
    Router,
};
use reqwest::Client;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    bots::{has_debug_query, should_render_embed},
    config::Config,
    embed::{metadata_for_image, metadata_for_path, render_embed_html},
    proxy::{plain_response, proxy_request},
};

struct AppState {
    config: Config,
    client: Client,
    html_renderer: html_card::HtmlRenderer,
    image_cache: ImageCache,
}

struct ImageCache {
    entries: Mutex<HashMap<String, CachedImage>>,
    max_entries: usize,
    render_permits: Arc<Semaphore>,
}

#[derive(Clone)]
struct CachedImage {
    bytes: Bytes,
    stored_at: Instant,
}

struct RenderedImage {
    bytes: Vec<u8>,
    cacheable: bool,
}

enum CacheLookup {
    Fresh(Bytes),
    Stale(Bytes),
    Missing,
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

    let html_renderer = html_card::HtmlRenderer::new();
    html_renderer.warm_up();

    let image_cache = ImageCache::new(
        config.image_cache_max_entries,
        config.render_max_concurrency,
    );

    let state = Arc::new(AppState {
        config,
        client,
        html_renderer,
        image_cache,
    });

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
        if let Some(meta) =
            metadata_for_path(&state.client, &state.config, uri.path(), uri.query()).await
        {
            let redirect_humans = !has_debug_query(&uri, &state.config.debug_query_key);
            let html = render_embed_html(&meta, redirect_humans);
            return embed_html_response(method, html);
        }
    }

    proxy_request(&state.client, &state.config, method, uri, headers, body).await
}

async fn image_handler(
    State(state): State<Arc<AppState>>,
    Path((kind, id)): Path<(String, String)>,
    uri: Uri,
) -> Response<Body> {
    let cache_key = image_cache_key(&uri);
    match state.image_cache.get(
        &cache_key,
        state.config.image_cache_max_age,
        state.config.image_cache_stale_while_revalidate,
    ) {
        CacheLookup::Fresh(bytes) => {
            return image_response(&state.config, Method::GET, Some(bytes));
        }
        CacheLookup::Stale(bytes) => {
            if let Some(permit) = state.image_cache.try_acquire_render() {
                let state = state.clone();
                let cache_key = cache_key.clone();
                let kind = kind.clone();
                let id = id.clone();
                let query = uri.query().map(str::to_string);
                tokio::spawn(async move {
                    let _permit = permit;
                    refresh_image_cache(state, cache_key, kind, id, query).await;
                });
            }

            return image_response(&state.config, Method::GET, Some(bytes));
        }
        CacheLookup::Missing => {}
    }

    let render_permit = state.image_cache.try_acquire_render();
    let Some(meta) =
        metadata_for_image(&state.client, &state.config, &kind, &id, uri.query()).await
    else {
        return plain_response(StatusCode::NOT_FOUND, "Unknown embed image.");
    };

    let bytes = match render_permit {
        Some(permit) => {
            let _permit = permit;
            render_image_bytes(&state, &meta).await
        }
        None => {
            warn!(%cache_key, "embed image renderer is busy; using fallback image renderer");
            image_card::render_png(&meta).map(|bytes| RenderedImage {
                bytes,
                cacheable: false,
            })
        }
    };

    match bytes {
        Ok(rendered) => {
            let bytes = Bytes::from(rendered.bytes);
            if rendered.cacheable {
                state.image_cache.insert(cache_key, bytes.clone());
            }
            image_response(&state.config, Method::GET, Some(bytes))
        }
        Err(error) => {
            warn!(%error, "failed to render embed image");
            plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render embed image.",
            )
        }
    }
}

async fn image_head_handler(State(state): State<Arc<AppState>>) -> Response<Body> {
    image_response(&state.config, Method::HEAD, None)
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

fn image_response(config: &Config, method: Method, bytes: Option<Bytes>) -> Response<Body> {
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from(bytes.unwrap_or_default())
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, image_cache_control(config))
        .body(body)
        .expect("image response is valid")
}

async fn refresh_image_cache(
    state: Arc<AppState>,
    cache_key: String,
    kind: String,
    id: String,
    query: Option<String>,
) {
    let Some(meta) =
        metadata_for_image(&state.client, &state.config, &kind, &id, query.as_deref()).await
    else {
        return;
    };

    match render_image_bytes(&state, &meta).await {
        Ok(rendered) if rendered.cacheable => state
            .image_cache
            .insert(cache_key, Bytes::from(rendered.bytes)),
        Ok(_) => warn!(%cache_key, "skipping cache refresh for fallback embed image"),
        Err(error) => warn!(%error, "failed to refresh stale embed image cache entry"),
    }
}

async fn render_image_bytes(
    state: &AppState,
    meta: &embed::EmbedMetadata,
) -> anyhow::Result<RenderedImage> {
    match state.html_renderer.render_png(meta).await {
        Ok(bytes) => Ok(RenderedImage {
            bytes,
            cacheable: true,
        }),
        Err(error) => {
            warn!(%error, "failed to render html embed image; falling back to rust image renderer");
            image_card::render_png(meta).map(|bytes| RenderedImage {
                bytes,
                cacheable: false,
            })
        }
    }
}

fn image_cache_key(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|path_and_query| path_and_query.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn image_cache_control(config: &Config) -> String {
    format!(
        "public, max-age={}, stale-while-revalidate={}",
        config.image_cache_max_age.as_secs(),
        config.image_cache_stale_while_revalidate.as_secs()
    )
}

impl ImageCache {
    fn new(max_entries: usize, render_max_concurrency: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_entries,
            render_permits: Arc::new(Semaphore::new(render_max_concurrency)),
        }
    }

    fn get(&self, key: &str, fresh_for: Duration, stale_for: Duration) -> CacheLookup {
        let Ok(entries) = self.entries.lock() else {
            return CacheLookup::Missing;
        };
        let Some(entry) = entries.get(key) else {
            return CacheLookup::Missing;
        };

        let age = entry.stored_at.elapsed();
        if age <= fresh_for {
            CacheLookup::Fresh(entry.bytes.clone())
        } else if age <= fresh_for + stale_for {
            CacheLookup::Stale(entry.bytes.clone())
        } else {
            CacheLookup::Missing
        }
    }

    fn insert(&self, key: String, bytes: Bytes) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };

        if entries.len() >= self.max_entries && !entries.contains_key(&key) {
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.stored_at)
                .map(|(key, _)| key.clone())
            {
                entries.remove(&oldest_key);
            }
        }

        entries.insert(
            key,
            CachedImage {
                bytes,
                stored_at: Instant::now(),
            },
        );
    }

    fn try_acquire_render(&self) -> Option<OwnedSemaphorePermit> {
        self.render_permits.clone().try_acquire_owned().ok()
    }
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
