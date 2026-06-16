mod bots;
mod config;
mod embed;
mod html_card;
#[cfg(test)]
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
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tower_http::trace::TraceLayer;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    bots::{has_debug_query, should_render_embed},
    config::Config,
    embed::{metadata_for_image, metadata_for_path, render_embed_html, warm_static_caches},
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
    in_flight: Mutex<HashMap<String, watch::Sender<bool>>>,
    max_entries: usize,
    render_permits: Arc<Semaphore>,
}

#[derive(Clone)]
struct CachedImage {
    bytes: Bytes,
    stored_at: Instant,
}

enum CacheLookup {
    Fresh(Bytes),
    Stale(Bytes),
    Missing,
}

enum RenderClaim {
    Started,
    Waiting(watch::Receiver<bool>),
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

    let html_renderer = html_card::HtmlRenderer::new(config.render_max_concurrency);
    html_renderer.warm_up();
    warm_static_caches(&client, &config).await;

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
    if is_internal_embed_path(uri.path()) {
        return plain_response(StatusCode::NOT_FOUND, "Unknown embed path.");
    }

    if (method == Method::GET || method == Method::HEAD)
        && should_render_embed(&headers, &uri, &state.config)
    {
        let started_at = Instant::now();
        let user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        debug!(
            method = %method,
            path = uri.path(),
            query = uri.query().unwrap_or_default(),
            user_agent = %user_agent,
            "embed HTML request received"
        );
        if let Some(meta) =
            metadata_for_path(&state.client, &state.config, uri.path(), uri.query()).await
        {
            debug!(
                method = %method,
                path = uri.path(),
                query = uri.query().unwrap_or_default(),
                kind = %meta.kind_label,
                canonical_url = %meta.canonical_url,
                elapsed_ms = elapsed_ms(started_at),
                "embed HTML metadata resolved"
            );
            let redirect_humans = !has_debug_query(&uri, &state.config.debug_query_key);
            let html = render_embed_html(&meta, redirect_humans);
            return embed_html_response(method, html);
        }

        debug!(
            method = %method,
            path = uri.path(),
            query = uri.query().unwrap_or_default(),
            elapsed_ms = elapsed_ms(started_at),
            "embed HTML metadata did not resolve"
        );
    }

    proxy_request(&state.client, &state.config, method, uri, headers, body).await
}

async fn image_handler(
    State(state): State<Arc<AppState>>,
    Path((kind, id)): Path<(String, String)>,
    uri: Uri,
    headers: HeaderMap,
) -> Response<Body> {
    let request_started_at = Instant::now();
    let cache_key = image_cache_key(&uri);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    debug!(
        %cache_key,
        kind = %kind,
        id = %id,
        user_agent = %user_agent,
        "embed image request received"
    );
    match state.image_cache.get(
        &cache_key,
        state.config.image_cache_max_age,
        state.config.image_cache_stale_while_revalidate,
    ) {
        CacheLookup::Fresh(bytes) => {
            debug!(
                %cache_key,
                bytes = bytes.len(),
                elapsed_ms = elapsed_ms(request_started_at),
                "embed image cache hit"
            );
            return image_response(&state.config, Method::GET, Some(bytes));
        }
        CacheLookup::Stale(bytes) => {
            if let Some(permit) = state.image_cache.try_acquire_render() {
                if state.image_cache.try_claim_render(&cache_key) {
                    debug!(
                        %cache_key,
                        kind = %kind,
                        id = %id,
                        "stale embed image cache hit; queued background refresh"
                    );
                    let state = state.clone();
                    let cache_key = cache_key.clone();
                    let kind = kind.clone();
                    let id = id.clone();
                    let query = uri.query().map(str::to_string);
                    tokio::spawn(async move {
                        let _permit = permit;
                        refresh_image_cache(state.clone(), cache_key.clone(), kind, id, query)
                            .await;
                        state.image_cache.finish_render(&cache_key);
                    });
                } else {
                    debug!(
                        %cache_key,
                        "stale embed image cache hit; refresh already in flight"
                    );
                }
            } else {
                debug!(
                    %cache_key,
                    "stale embed image cache hit; refresh skipped because renderer permits are busy"
                );
            }

            debug!(
                %cache_key,
                bytes = bytes.len(),
                elapsed_ms = elapsed_ms(request_started_at),
                "served stale embed image cache entry"
            );
            return image_response(&state.config, Method::GET, Some(bytes));
        }
        CacheLookup::Missing => {
            debug!(%cache_key, kind = %kind, id = %id, "embed image cache miss");
        }
    }

    let metadata_started_at = Instant::now();
    let Some(meta) =
        metadata_for_image(&state.client, &state.config, &kind, &id, uri.query()).await
    else {
        debug!(
            %cache_key,
            kind = %kind,
            id = %id,
            elapsed_ms = elapsed_ms(metadata_started_at),
            "embed image metadata did not resolve"
        );
        return plain_response(StatusCode::NOT_FOUND, "Unknown embed image.");
    };
    debug!(
        %cache_key,
        kind = %kind,
        id = %id,
        meta_kind = %meta.kind_label,
        canonical_url = %meta.canonical_url,
        elapsed_ms = elapsed_ms(metadata_started_at),
        "embed image metadata resolved"
    );

    let bytes = match state.image_cache.claim_render(&cache_key) {
        RenderClaim::Started => {
            let permit_started_at = Instant::now();
            let _permit = state.image_cache.acquire_render().await;
            debug!(
                %cache_key,
                wait_ms = elapsed_ms(permit_started_at),
                "acquired embed image render permit"
            );
            let result = render_image_bytes(&state, &meta).await;
            state.image_cache.finish_render(&cache_key);
            result
        }
        RenderClaim::Waiting(mut receiver) => {
            let wait_started_at = Instant::now();
            debug!(%cache_key, "waiting for in-flight embed image render");
            if !*receiver.borrow() {
                let _ = receiver.changed().await;
            }
            debug!(
                %cache_key,
                wait_ms = elapsed_ms(wait_started_at),
                "in-flight embed image render finished"
            );

            match state.image_cache.get(
                &cache_key,
                state.config.image_cache_max_age,
                state.config.image_cache_stale_while_revalidate,
            ) {
                CacheLookup::Fresh(bytes) | CacheLookup::Stale(bytes) => {
                    return image_response(&state.config, Method::GET, Some(bytes));
                }
                CacheLookup::Missing => {
                    warn!(%cache_key, "in-flight embed image render completed without cache entry");
                    return plain_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Embed image is not ready yet.",
                    );
                }
            }
        }
    };

    match bytes {
        Ok(rendered) => {
            let bytes = Bytes::from(rendered);
            state.image_cache.insert(cache_key, bytes.clone());
            debug!(
                bytes = bytes.len(),
                elapsed_ms = elapsed_ms(request_started_at),
                "served newly rendered embed image"
            );
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
    let refresh_started_at = Instant::now();
    debug!(%cache_key, kind = %kind, id = %id, "started stale embed image cache refresh");
    let metadata_started_at = Instant::now();
    let Some(meta) =
        metadata_for_image(&state.client, &state.config, &kind, &id, query.as_deref()).await
    else {
        debug!(
            %cache_key,
            kind = %kind,
            id = %id,
            elapsed_ms = elapsed_ms(metadata_started_at),
            "stale embed image refresh metadata did not resolve"
        );
        return;
    };
    debug!(
        %cache_key,
        kind = %kind,
        id = %id,
        elapsed_ms = elapsed_ms(metadata_started_at),
        "stale embed image refresh metadata resolved"
    );

    match render_image_bytes(&state, &meta).await {
        Ok(bytes) => {
            let byte_count = bytes.len();
            state
                .image_cache
                .insert(cache_key.clone(), Bytes::from(bytes));
            debug!(
                %cache_key,
                bytes = byte_count,
                elapsed_ms = elapsed_ms(refresh_started_at),
                "finished stale embed image cache refresh"
            );
        }
        Err(error) => warn!(%error, "failed to refresh stale embed image cache entry"),
    }
}

async fn render_image_bytes(
    state: &AppState,
    meta: &embed::EmbedMetadata,
) -> anyhow::Result<Vec<u8>> {
    let started_at = Instant::now();
    let result = state.html_renderer.render_png(meta).await;
    match &result {
        Ok(bytes) => debug!(
            kind = %meta.kind_label,
            canonical_url = %meta.canonical_url,
            bytes = bytes.len(),
            elapsed_ms = elapsed_ms(started_at),
            "embed image renderer completed"
        ),
        Err(error) => debug!(
            kind = %meta.kind_label,
            canonical_url = %meta.canonical_url,
            %error,
            elapsed_ms = elapsed_ms(started_at),
            "embed image renderer failed"
        ),
    }

    result
}

fn elapsed_ms(started_at: Instant) -> u128 {
    started_at.elapsed().as_millis()
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

fn is_internal_embed_path(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path == "/__embeds" || path.starts_with("/__embeds/")
}

impl ImageCache {
    fn new(max_entries: usize, render_max_concurrency: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashMap::new()),
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

    async fn acquire_render(&self) -> OwnedSemaphorePermit {
        self.render_permits
            .clone()
            .acquire_owned()
            .await
            .expect("image render semaphore is not closed")
    }

    fn claim_render(&self, key: &str) -> RenderClaim {
        let Ok(mut in_flight) = self.in_flight.lock() else {
            return RenderClaim::Started;
        };

        if let Some(sender) = in_flight.get(key) {
            return RenderClaim::Waiting(sender.subscribe());
        }

        let (sender, _) = watch::channel(false);
        in_flight.insert(key.to_string(), sender);
        RenderClaim::Started
    }

    fn try_claim_render(&self, key: &str) -> bool {
        let Ok(mut in_flight) = self.in_flight.lock() else {
            return false;
        };

        if in_flight.contains_key(key) {
            return false;
        }

        let (sender, _) = watch::channel(false);
        in_flight.insert(key.to_string(), sender);
        true
    }

    fn finish_render(&self, key: &str) {
        let Ok(mut in_flight) = self.in_flight.lock() else {
            return;
        };

        if let Some(sender) = in_flight.remove(key) {
            let _ = sender.send(true);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> Arc<AppState> {
        let config = Config {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            public_base_url: "https://uma.moe".to_string(),
            frontend_origin: "https://uma.moe".to_string(),
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
        };

        Arc::new(AppState {
            client: Client::new(),
            html_renderer: html_card::HtmlRenderer::new(1),
            image_cache: ImageCache::new(256, 1),
            config,
        })
    }

    #[test]
    fn internal_embed_path_detection_covers_namespace() {
        assert!(is_internal_embed_path("/__embeds"));
        assert!(is_internal_embed_path("/__embeds/images/"));
        assert!(is_internal_embed_path("/__EMBEDS/images/circle/1.png"));
        assert!(!is_internal_embed_path("/circles/772781438"));
    }

    #[tokio::test]
    async fn malformed_internal_embed_path_returns_404_before_proxy() {
        let response = page_handler(
            State(test_state()),
            Method::GET,
            "/__embeds/images/".parse::<Uri>().unwrap(),
            HeaderMap::new(),
            Bytes::new(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
