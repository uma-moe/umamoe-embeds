use std::{
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value};
use tokio::task;

use crate::embed::{embed_class_list, EmbedMetadata, EmbedMetric};

mod activity;
mod affinity;
mod club;
mod clubs;
mod database;
mod home;
mod inheritance;
mod lineage_planner;
mod overview;
mod page;
mod profile;
mod rankings;
mod statistics;
mod tierlist;
mod timeline;
mod tools;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 630;
const DEFAULT_CHROMIUM_STARTUP_TIMEOUT_SECONDS: u64 = 45;
const DEFAULT_CHROMIUM_RENDER_TIMEOUT_SECONDS: u64 = 15;
const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 12 * 1024 * 1024;
static RENDER_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct HtmlRenderer {
    inner: Arc<HtmlRendererInner>,
}

struct HtmlRendererInner {
    chromium: Mutex<Option<ChromiumProcess>>,
}

impl HtmlRenderer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(HtmlRendererInner {
                chromium: Mutex::new(None),
            }),
        }
    }

    pub fn warm_up(&self) {
        let renderer = self.clone();
        task::spawn_blocking(move || {
            if let Err(error) = renderer.ensure_chromium() {
                tracing::warn!(%error, "failed to warm Chromium renderer");
            }
        });
    }

    pub async fn render_png(&self, meta: &EmbedMetadata) -> Result<Vec<u8>> {
        let renderer = self.clone();
        let meta = meta.clone();
        task::spawn_blocking(move || renderer.render_png_sync(&meta))
            .await
            .context("html card render task failed")?
    }

    fn ensure_chromium(&self) -> Result<()> {
        let mut guard = self
            .inner
            .chromium
            .lock()
            .map_err(|_| anyhow!("Chromium renderer lock is poisoned"))?;
        ensure_chromium_process(&mut guard).map(|_| ())
    }

    fn render_png_sync(&self, meta: &EmbedMetadata) -> Result<Vec<u8>> {
        let files = TempRenderFiles::new()?;

        fs::write(&files.html_path, render_card_html(meta)).with_context(|| {
            format!(
                "failed to write temporary embed html to {}",
                files.html_path.display()
            )
        })?;

        let url = file_url(&files.html_path);
        let mut guard = self
            .inner
            .chromium
            .lock()
            .map_err(|_| anyhow!("Chromium renderer lock is poisoned"))?;

        let first_error = match capture_with_persistent_chromium(&mut guard, &url) {
            Ok(bytes) => return Ok(bytes),
            Err(error) => error,
        };

        tracing::warn!(%first_error, "persistent Chromium render failed; restarting renderer");
        stop_chromium_process(&mut guard);

        match capture_with_persistent_chromium(&mut guard, &url) {
            Ok(bytes) => Ok(bytes),
            Err(error) => {
                tracing::warn!(
                    %error,
                    "persistent Chromium retry failed; using Chromium CLI renderer"
                );
                render_png_with_chromium_cli(meta)
                    .context("Chromium CLI fallback failed after persistent renderer failed")
            }
        }
    }
}

impl Default for HtmlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HtmlRendererInner {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.chromium.lock() {
            stop_chromium_process(&mut guard);
        }
    }
}

struct ChromiumProcess {
    child: Child,
    port: u16,
    profile_dir: PathBuf,
    cache_dir: PathBuf,
}

fn capture_with_persistent_chromium(
    guard: &mut Option<ChromiumProcess>,
    url: &str,
) -> Result<Vec<u8>> {
    let chromium = ensure_chromium_process(guard)?;
    chromium.capture(url)
}

fn ensure_chromium_process(guard: &mut Option<ChromiumProcess>) -> Result<&mut ChromiumProcess> {
    let needs_start = match guard.as_mut() {
        Some(chromium) => chromium.child.try_wait()?.is_some(),
        None => true,
    };

    if needs_start {
        stop_chromium_process(guard);
        *guard = Some(ChromiumProcess::start()?);
    }

    guard
        .as_mut()
        .ok_or_else(|| anyhow!("Chromium renderer is unavailable"))
}

fn stop_chromium_process(guard: &mut Option<ChromiumProcess>) {
    if let Some(mut chromium) = guard.take() {
        let _ = chromium.child.kill();
        let _ = chromium.child.wait();
        let _ = fs::remove_dir_all(&chromium.profile_dir);
        let _ = fs::remove_dir_all(&chromium.cache_dir);
    }
}

impl ChromiumProcess {
    fn start() -> Result<Self> {
        let chromium = chromium_binary();
        let id = unique_render_id();
        let base = env::temp_dir();
        let profile_dir = base.join(format!("umamoe-embed-chrome-profile-{id}"));
        let cache_dir = base.join(format!("umamoe-embed-chrome-cache-{id}"));
        fs::create_dir_all(&profile_dir).with_context(|| {
            format!(
                "failed to create persistent Chromium profile at {}",
                profile_dir.display()
            )
        })?;
        fs::create_dir_all(&cache_dir).with_context(|| {
            format!(
                "failed to create persistent Chromium cache at {}",
                cache_dir.display()
            )
        })?;

        let port = chromium_debug_port()?;
        let window_size_arg = format!("--window-size={WIDTH},{HEIGHT}");
        let profile_arg = format!("--user-data-dir={}", profile_dir.display());
        let cache_arg = format!("--disk-cache-dir={}", cache_dir.display());
        let crash_dumps_arg = format!("--crash-dumps-dir={}", cache_dir.display());
        let remote_debugging_arg = format!("--remote-debugging-port={port}");

        let child = Command::new(&chromium)
            .args([
                "--headless=new",
                "--disable-background-networking",
                "--disable-breakpad",
                "--disable-client-side-phishing-detection",
                "--disable-component-update",
                "--disable-crash-reporter",
                "--disable-default-apps",
                "--disable-gpu",
                "--disable-dev-shm-usage",
                "--disable-domain-reliability",
                "--disable-extensions",
                "--disable-features=Translate,MediaRouter,OptimizationHints,AutofillServerCommunication",
                "--disable-hang-monitor",
                "--disable-namespace-sandbox",
                "--disable-popup-blocking",
                "--disable-prompt-on-repost",
                "--disable-renderer-backgrounding",
                "--disable-seccomp-filter-sandbox",
                "--disable-setuid-sandbox",
                "--disable-sync",
                "--hide-scrollbars",
                "--metrics-recording-only",
                "--mute-audio",
                "--no-default-browser-check",
                "--no-first-run",
                "--no-sandbox",
                "--no-zygote",
                "--password-store=basic",
                "--remote-debugging-address=127.0.0.1",
                "--run-all-compositor-stages-before-draw",
                "--use-mock-keychain",
                "--force-device-scale-factor=1",
                &window_size_arg,
                &profile_arg,
                &cache_arg,
                &crash_dumps_arg,
                &remote_debugging_arg,
                "about:blank",
            ])
            .env("HOME", &profile_dir)
            .env("XDG_CACHE_HOME", &cache_dir)
            .env("XDG_CONFIG_HOME", &profile_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to start Chromium binary `{chromium}`"))?;

        let mut process = Self {
            child,
            port,
            profile_dir,
            cache_dir,
        };
        process.wait_until_ready()?;
        Ok(process)
    }

    fn wait_until_ready(&mut self) -> Result<()> {
        let startup_timeout = chromium_startup_timeout();
        let started_at = SystemTime::now();
        loop {
            if let Some(status) = self.child.try_wait()? {
                return Err(anyhow!(
                    "Chromium exited during startup with status {status}"
                ));
            }

            if http_request(self.port, "GET", "/json/version").is_ok() {
                return Ok(());
            }

            if started_at.elapsed().unwrap_or_default() > startup_timeout {
                return Err(anyhow!(
                    "Chromium did not expose DevTools on 127.0.0.1:{} within {:?}",
                    self.port,
                    startup_timeout
                ));
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn capture(&mut self, url: &str) -> Result<Vec<u8>> {
        let target_path = format!("/json/new?{}", urlencoding::encode(url));
        let body = http_request(self.port, "PUT", &target_path)
            .or_else(|_| http_request(self.port, "GET", &target_path))
            .context("failed to create Chromium screenshot target")?;
        let target: Value = serde_json::from_str(&body).context("invalid Chromium target JSON")?;
        let target_id = target
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Chromium target JSON did not contain id"))?;
        let websocket_url = target
            .get("webSocketDebuggerUrl")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Chromium target JSON did not contain webSocketDebuggerUrl"))?;

        let result = (|| {
            let mut socket = DevToolsSocket::connect(websocket_url)?;
            socket.command("Page.enable", json!({}))?;
            socket.command(
                "Emulation.setDeviceMetricsOverride",
                json!({
                    "width": WIDTH,
                    "height": HEIGHT,
                    "deviceScaleFactor": 1,
                    "mobile": false,
                }),
            )?;
            socket.command("Page.navigate", json!({ "url": url }))?;
            socket.wait_for_event("Page.domContentEventFired")?;
            let _ = socket.command(
                "Runtime.evaluate",
                json!({
                    "expression": "new Promise(resolve => { const finish = () => requestAnimationFrame(() => requestAnimationFrame(() => resolve(true))); if (document.fonts && document.fonts.ready) { document.fonts.ready.then(finish, finish); } else { finish(); } setTimeout(() => resolve(true), 1500); })",
                    "awaitPromise": true,
                    "returnByValue": true,
                }),
            );
            let response = socket.command(
                "Page.captureScreenshot",
                json!({
                    "format": "png",
                    "fromSurface": true,
                    "captureBeyondViewport": false,
                    "clip": {
                        "x": 0,
                        "y": 0,
                        "width": WIDTH,
                        "height": HEIGHT,
                        "scale": 1,
                    },
                }),
            )?;
            let encoded = response
                .get("data")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Chromium screenshot response did not contain data"))?;
            let bytes = BASE64
                .decode(encoded)
                .context("Chromium screenshot was not valid base64")?;
            if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err(anyhow!("Chromium screenshot did not produce a PNG"));
            }
            Ok(bytes)
        })();

        let _ = http_request(self.port, "GET", &format!("/json/close/{target_id}"));
        result
    }
}

struct DevToolsSocket {
    stream: TcpStream,
    next_id: u64,
    pending_events: Vec<Value>,
}

impl DevToolsSocket {
    fn connect(websocket_url: &str) -> Result<Self> {
        let url = url::Url::parse(websocket_url)
            .with_context(|| format!("invalid Chromium websocket URL `{websocket_url}`"))?;
        let host = url
            .host_str()
            .ok_or_else(|| anyhow!("Chromium websocket URL did not include host"))?;
        let port = url
            .port_or_known_default()
            .ok_or_else(|| anyhow!("Chromium websocket URL did not include port"))?;
        let path = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_string(),
        };
        let mut stream = TcpStream::connect((host, port))
            .with_context(|| format!("failed to connect to Chromium websocket at {host}:{port}"))?;
        let render_timeout = chromium_render_timeout();
        stream.set_read_timeout(Some(render_timeout))?;
        stream.set_write_timeout(Some(render_timeout))?;

        let key = websocket_key();
        let request = format!(
            "GET {path} HTTP/1.1\r\n\
             Host: {host}:{port}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {key}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             \r\n"
        );
        stream.write_all(request.as_bytes())?;
        let response = read_http_headers(&mut stream)?;
        if !response.starts_with("HTTP/1.1 101") && !response.starts_with("HTTP/1.0 101") {
            return Err(anyhow!(
                "Chromium websocket handshake failed: {}",
                response.lines().next().unwrap_or_default()
            ));
        }

        Ok(Self {
            stream,
            next_id: 0,
            pending_events: Vec::new(),
        })
    }

    fn command(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        self.send_text(&json!({ "id": id, "method": method, "params": params }).to_string())?;

        loop {
            let message = self.read_text()?;
            let value: Value = serde_json::from_str(&message).context("invalid DevTools JSON")?;
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                self.pending_events.push(value);
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(anyhow!("DevTools command {method} failed: {error}"));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    fn wait_for_event(&mut self, method: &str) -> Result<()> {
        if let Some(index) = self
            .pending_events
            .iter()
            .position(|value| value.get("method").and_then(Value::as_str) == Some(method))
        {
            self.pending_events.swap_remove(index);
            return Ok(());
        }

        loop {
            let message = self.read_text()?;
            let value: Value = serde_json::from_str(&message).context("invalid DevTools JSON")?;
            if value.get("method").and_then(Value::as_str) == Some(method) {
                return Ok(());
            }
        }
    }

    fn send_text(&mut self, message: &str) -> Result<()> {
        write_websocket_frame(&mut self.stream, 0x1, message.as_bytes())
    }

    fn read_text(&mut self) -> Result<String> {
        read_websocket_text(&mut self.stream)
    }
}

fn render_png_with_chromium_cli(meta: &EmbedMetadata) -> Result<Vec<u8>> {
    let chromium = chromium_binary();
    let files = TempRenderFiles::new()?;

    fs::write(&files.html_path, render_card_html(meta)).with_context(|| {
        format!(
            "failed to write temporary embed html to {}",
            files.html_path.display()
        )
    })?;
    fs::create_dir_all(&files.profile_dir).with_context(|| {
        format!(
            "failed to create temporary Chromium profile at {}",
            files.profile_dir.display()
        )
    })?;
    fs::create_dir_all(&files.cache_dir).with_context(|| {
        format!(
            "failed to create temporary Chromium cache at {}",
            files.cache_dir.display()
        )
    })?;

    let screenshot_arg = format!("--screenshot={}", files.png_path.display());
    let window_size_arg = format!("--window-size={WIDTH},{HEIGHT}");
    let profile_arg = format!("--user-data-dir={}", files.profile_dir.display());
    let crash_dumps_arg = format!("--crash-dumps-dir={}", files.cache_dir.display());
    let url = file_url(&files.html_path);

    let output = Command::new(&chromium)
        .args([
            "--headless=new",
            "--disable-breakpad",
            "--disable-crash-reporter",
            "--disable-gpu",
            "--disable-dev-shm-usage",
            "--disable-setuid-sandbox",
            "--hide-scrollbars",
            "--mute-audio",
            "--no-first-run",
            "--no-sandbox",
            "--run-all-compositor-stages-before-draw",
            "--force-device-scale-factor=1",
            &window_size_arg,
            &profile_arg,
            &crash_dumps_arg,
            &screenshot_arg,
            &url,
        ])
        .env("HOME", &files.profile_dir)
        .env("XDG_CACHE_HOME", &files.cache_dir)
        .env("XDG_CONFIG_HOME", &files.profile_dir)
        .output()
        .with_context(|| format!("failed to run Chromium binary `{chromium}`"))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Chromium screenshot failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let bytes = fs::read(&files.png_path).with_context(|| {
        format!(
            "failed to read Chromium screenshot from {}",
            files.png_path.display()
        )
    })?;

    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(anyhow!("Chromium screenshot did not produce a PNG"));
    }

    Ok(bytes)
}

pub(crate) fn render_card_html(meta: &EmbedMetadata) -> String {
    if home::renders_full_card(meta) {
        return home::render_card_html(meta);
    }
    if profile::renders_full_card(meta) {
        return profile::render_card_html(meta);
    }
    if clubs::renders_full_card(meta) {
        return clubs::render_card_html(meta);
    }
    if club::renders_full_card(meta) {
        return club::render_card_html(meta);
    }
    if database::renders_full_card(meta) {
        return database::render_card_html(meta);
    }
    if rankings::renders_full_card(meta) {
        return rankings::render_card_html(meta);
    }
    if activity::renders_full_card(meta) {
        return activity::render_card_html(meta);
    }
    if timeline::renders_full_card(meta) {
        return timeline::render_card_html(meta);
    }
    if tierlist::renders_full_card(meta) {
        return tierlist::render_card_html(meta);
    }
    if tools::renders_full_card(meta) {
        return tools::render_card_html(meta);
    }
    if statistics::renders_full_card(meta) {
        return statistics::render_card_html(meta);
    }
    if lineage_planner::renders_full_card(meta) {
        return lineage_planner::render_card_html(meta);
    }

    let accent = accent_for_kind(&meta.kind_label);
    let identity = render_identity(meta);
    let stats = render_stats(meta);
    let body = render_body(meta);
    let url = html_escape(&compact_url(&meta.canonical_url));
    let footer = render_footer(meta, &url);
    let brand_logo_url = html_escape(&brand_logo_url(meta));
    let class_list = embed_class_list(meta);
    let view_class = overview::card_view_class(meta);

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=1200, initial-scale=1">
  <style>
    :root {{
      --accent: {accent};
      --accent-soft: {accent_soft};
      --accent-faint: {accent_faint};
      --bg-primary: #0a0a0a;
      --bg-secondary: #121212;
      --bg-tertiary: #1e1e1e;
      --surface-1: rgba(255, 255, 255, 0.018);
      --surface-2: rgba(255, 255, 255, 0.045);
      --surface-3: rgba(255, 255, 255, 0.075);
      --surface-4: rgba(255, 255, 255, 0.12);
      --text-primary: #ffffff;
      --text-secondary: rgba(255, 255, 255, 0.7);
      --text-muted: rgba(255, 255, 255, 0.5);
      --text-disabled: rgba(255, 255, 255, 0.3);
      --accent-primary: #64b5f6;
      --accent-secondary: #81c784;
      --accent-warning: #ffb74d;
      --color-pink: #e91e63;
      --color-green: #4caf50;
      --color-orange: #ff9800;
      --border-subtle: rgba(255, 255, 255, 0.06);
      --border-primary: rgba(255, 255, 255, 0.12);
      --border-secondary: rgba(255, 255, 255, 0.2);
      --border-strong: rgba(255, 255, 255, 0.3);
      --radius-xs: 4px;
      --radius-sm: 6px;
      --radius-md: 8px;
      --radius-lg: 12px;
      color-scheme: dark;
      font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: var(--bg-secondary);
      color: var(--text-primary);
    }}

    * {{
      box-sizing: border-box;
    }}

    html,
    body {{
      width: 1200px;
      height: 630px;
      margin: 0;
      overflow: hidden;
      background: var(--bg-secondary);
    }}

    body {{
      display: block;
    }}

    .embed-page {{
      position: relative;
      width: 1200px;
      height: 630px;
      padding: 14px 18px 14px;
      display: grid;
      grid-template-rows: 52px minmax(0, 1fr);
      align-content: start;
      gap: 8px;
      background:
        radial-gradient(circle at 17% 3%, rgba(100, 181, 246, 0.055), transparent 360px),
        radial-gradient(circle at 82% 10%, rgba(129, 199, 132, 0.045), transparent 340px),
        var(--bg-secondary);
    }}

    .embed-header {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 24px;
      min-width: 0;
    }}

    .embed-identity,
    .embed-brand {{
      display: flex;
      align-items: center;
      min-width: 0;
    }}

    .embed-identity {{
      flex-direction: column;
      align-items: flex-start;
      justify-content: center;
      gap: 5px;
      flex: 1 1 auto;
    }}

    .embed-user-name {{
      max-width: 570px;
      overflow: hidden;
      color: #76cfff;
      font-size: 38px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .embed-user-id {{
      display: block;
      max-width: 640px;
      overflow: hidden;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 820;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
      text-transform: uppercase;
    }}

    .embed-result-rank {{
      display: inline-block;
      align-items: center;
      color: #90caf9;
      font-size: 13px;
      font-weight: 800;
      line-height: 1;
      white-space: nowrap;
      text-transform: uppercase;
    }}

    .embed-result-rank::before {{
      content: " / ";
      color: rgba(255, 255, 255, 0.22);
      font-weight: 900;
    }}

    .embed-brand {{
      gap: 16px;
      flex: 0 0 auto;
      justify-content: flex-end;
    }}

    .brand-logo {{
      width: 64px;
      height: 64px;
      flex-shrink: 0;
      object-fit: contain;
      filter: drop-shadow(0 0 18px rgba(100, 181, 246, 0.26));
    }}

    .brand-text {{
      font-size: 40px;
      font-weight: 900;
      line-height: 1;
      white-space: nowrap;
      color: #ffffff;
      text-shadow: 0 4px 18px rgba(0, 0, 0, 0.4);
    }}

    .kind {{
      display: inline-flex;
      align-items: center;
      height: 32px;
      padding: 0 12px;
      border: 1px solid var(--accent-soft);
      border-radius: var(--radius-sm);
      background: var(--accent-faint);
      color: var(--accent);
      font-size: 12px;
      font-weight: 800;
      line-height: 1;
      text-transform: uppercase;
      letter-spacing: 0;
      white-space: nowrap;
    }}

    .record-card {{
      position: relative;
      display: flex;
      flex-direction: column;
      gap: 0;
      height: 100%;
      min-height: 0;
      padding: 13px 18px 10px;
      border: 1px solid var(--border-primary);
      border-radius: var(--radius-lg);
      background: var(--surface-2);
      box-shadow: 0 18px 42px rgba(0, 0, 0, 0.24);
      overflow: hidden;
    }}

    .record-card::before {{
      content: "";
      position: absolute;
      inset: 0 0 auto;
      height: 3px;
      background: linear-gradient(90deg, var(--accent), #81c784 64%, rgba(255, 255, 255, 0));
      opacity: 0.9;
    }}

    .record-header {{
      display: flex;
      flex-direction: column;
      gap: 10px;
      min-width: 0;
    }}

    .record-context {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 18px;
      width: 100%;
      min-width: 0;
    }}

    .context-copy {{
      display: grid;
      gap: 4px;
      min-width: 0;
      flex: 1 1 auto;
    }}

    .context-title {{
      display: block;
      overflow: hidden;
      color: var(--text-primary);
      font-size: 18px;
      font-weight: 750;
      line-height: 1.2;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .context-description {{
      display: block;
      overflow: hidden;
      color: var(--text-secondary);
      font-size: 13px;
      line-height: 1.3;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    p {{
      display: -webkit-box;
      margin: 0;
      overflow: hidden;
      color: var(--text-secondary);
      font-size: 15px;
      line-height: 1.34;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 2;
    }}

    .record-header-stats {{
      display: flex;
      align-items: center;
      flex-wrap: wrap;
      gap: 12px 28px;
      width: 100%;
      min-height: 54px;
      padding: 6px 12px;
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: var(--radius-md);
      background: rgba(0, 0, 0, 0.18);
    }}

    .main-stats {{
      display: flex;
      align-items: center;
      gap: 24px;
      min-width: 0;
      flex-wrap: wrap;
    }}

    .stat-pill {{
      display: inline-flex;
      flex-direction: column;
      align-items: flex-start;
      justify-content: center;
      gap: 4px;
      min-width: 0;
      line-height: 1;
    }}

    .stat-number {{
      max-width: 160px;
      overflow: hidden;
      color: var(--accent-primary);
      font-size: 23px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .stat-label {{
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 750;
      line-height: 1;
      text-transform: uppercase;
      letter-spacing: 0;
    }}

    .affinity-stat .stat-number {{ color: var(--color-pink); }}
    .wins-stat .stat-number {{ color: var(--color-green); }}
    .white-stat .stat-number {{ color: var(--color-orange); }}
    .score-stat .stat-number {{ color: var(--accent-primary); }}
    .neutral-stat .stat-number {{ color: var(--text-primary); }}

    .rank-score-section {{
      display: flex;
      align-items: center;
      gap: 14px;
      margin-left: auto;
      padding-left: 18px;
      border-left: 1px solid rgba(255, 255, 255, 0.07);
    }}

    .rank-image-wrap {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 48px;
      height: 48px;
      flex: 1 1 auto;
    }}

    .rank-image {{
      display: block;
      max-width: 48px;
      max-height: 48px;
      object-fit: contain;
      filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.35));
    }}

    .inheritance-body {{
      display: grid;
      grid-template-columns: 218px minmax(0, 1fr);
      gap: 18px;
      align-items: stretch;
      min-height: 0;
      margin-top: 9px;
      padding-top: 9px;
      border-top: 1px solid var(--border-primary);
      flex: 1 1 auto;
      overflow: hidden;
    }}

    .lineage-panel,
    .summary-panel {{
      min-width: 0;
      padding: 12px;
      border: 1px solid rgba(255, 255, 255, 0.055);
      border-radius: var(--radius-md);
      background: rgba(255, 255, 255, 0.018);
    }}

    .lineage-panel {{
      display: flex;
      align-items: center;
      justify-content: center;
    }}

    .lineage-frame {{
      display: flex;
      flex-direction: column;
      align-items: center;
      width: 100%;
    }}

    .main-character,
    .parent-with-badges {{
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 3px;
      min-width: 0;
    }}

    .parent-characters {{
      display: flex;
      justify-content: center;
      gap: 28px;
      width: 100%;
    }}

    .lineage-bracket {{
      width: 136px;
      height: 20px;
      margin: 2px 0;
    }}

    .lineage-bracket path {{
      fill: none;
      stroke: #5a6470;
      stroke-width: 2;
      stroke-linecap: square;
    }}

    .portrait-wrapper {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      border-radius: 50%;
      border: 2px solid transparent;
    }}

    .portrait-main {{
      width: 58px;
      height: 58px;
      border-color: var(--border-strong);
      background: linear-gradient(145deg, rgba(100, 181, 246, 0.24), rgba(255, 255, 255, 0.04));
    }}

    .portrait-gp {{
      width: 44px;
      height: 44px;
      border-color: var(--border-secondary);
      background: linear-gradient(145deg, rgba(206, 147, 216, 0.18), rgba(255, 255, 255, 0.035));
    }}

    .portrait-label {{
      color: var(--text-primary);
      font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      font-size: 13px;
      font-weight: 800;
      line-height: 1;
    }}

    .portrait-gp .portrait-label {{
      font-size: 11px;
    }}

    .affinity-badge {{
      display: inline-flex;
      align-items: center;
      gap: 3px;
      max-width: 84px;
      padding: 2px 6px;
      border: 1px solid rgba(100, 181, 246, 0.25);
      border-radius: 10px;
      background: rgba(100, 181, 246, 0.12);
      color: var(--accent-primary);
      font-size: 10px;
      font-weight: 800;
      line-height: 1;
      white-space: nowrap;
    }}

    .affinity-badge.gp {{
      border-color: rgba(206, 147, 216, 0.25);
      background: rgba(206, 147, 216, 0.12);
      color: #ce93d8;
    }}

    .affinity-badge.gp-left {{
      border-color: rgba(233, 30, 99, 0.28);
      background: rgba(233, 30, 99, 0.12);
      color: #ff78b2;
    }}

    .affinity-badge.gp-right {{
      border-color: rgba(156, 39, 176, 0.3);
      background: rgba(156, 39, 176, 0.13);
      color: #ce93d8;
    }}

    .node-role-label {{
      color: var(--text-disabled);
      font-size: 9px;
      font-weight: 800;
      line-height: 1;
      text-transform: uppercase;
      letter-spacing: 0;
    }}

    .node-role-main {{
      color: var(--accent-primary);
      opacity: 0.95;
    }}

    .spark-arrays {{
      display: flex;
      flex-direction: column;
      justify-content: center;
      gap: 8px;
      min-width: 0;
      overflow: hidden;
    }}

    .spark-row {{
      display: flex;
      align-items: stretch;
      gap: 10px;
      min-width: 0;
    }}

    .spark-type-indicator {{
      width: 4px;
      min-height: 100%;
      border-radius: 999px;
      flex-shrink: 0;
      opacity: 0.95;
    }}

    .spark-type-indicator.blue {{ background: #2196f3; }}
    .spark-type-indicator.pink {{ background: #e91e63; }}
    .spark-type-indicator.green {{ background: #4caf50; }}
    .spark-type-indicator.white {{ background: #bdbdbd; }}

    .spark-list {{
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 7px;
      min-width: 0;
    }}

    .spark-item {{
      display: inline-flex;
      align-items: center;
      gap: 4px;
      max-width: 300px;
      padding: 4px 9px;
      border: 1px solid;
      border-radius: var(--radius-md);
      color: var(--text-secondary);
      font-size: 13px;
      font-weight: 500;
      line-height: 1;
      white-space: nowrap;
    }}

    .spark-name {{
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
    }}

    .spark-pct {{
      min-width: 0;
      max-width: 115px;
      overflow: hidden;
      padding: 2px 5px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: var(--radius-xs);
      background: rgba(0, 0, 0, 0.22);
      color: rgba(255, 255, 255, 0.78);
      font-family: inherit;
      font-size: 0.85em;
      font-weight: 700;
      line-height: 1;
      text-overflow: ellipsis;
      font-variant-numeric: tabular-nums;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      align-self: center;
    }}

    .blue-spark {{
      border-color: rgba(33, 150, 243, 0.48);
      background: rgba(33, 150, 243, 0.11);
      color: var(--accent-primary);
    }}

    .pink-spark {{
      border-color: rgba(233, 30, 99, 0.48);
      background: rgba(233, 30, 99, 0.11);
      color: #f06292;
    }}

    .green-spark {{
      border-color: rgba(76, 175, 80, 0.48);
      background: rgba(76, 175, 80, 0.11);
      color: var(--accent-secondary);
    }}

    .white-spark {{
      border-color: rgba(158, 158, 158, 0.46);
      background: rgba(158, 158, 158, 0.1);
      color: #cfcfcf;
    }}

    .inheritance-body {{
      grid-template-columns: 236px minmax(0, 1fr);
      gap: 14px;
      margin-top: 8px;
      padding-top: 8px;
    }}

    .character-images {{
      display: flex;
      flex-direction: column;
      align-items: stretch;
      justify-content: flex-start;
      gap: 14px;
      min-width: 0;
      min-height: 0;
      padding: 6px 10px;
      border: 0;
      border-radius: 0;
      background: transparent;
      align-self: flex-start;
    }}

    .character-images .lineage-frame {{
      width: 100%;
      min-height: 186px;
      justify-content: flex-start;
    }}

    .character-images .lineage-bracket {{
      width: 168px;
      height: 21px;
      margin: 1px 0 2px;
    }}

    .character-images .parent-characters {{
      justify-content: space-between;
      gap: 0;
      padding: 0 16px;
    }}

    .character-images .main-character,
    .character-images .parent-with-badges {{
      gap: 4px;
    }}

    .character-image {{
      display: block;
      width: 100%;
      height: 100%;
      border-radius: 50%;
      object-fit: cover;
      object-position: top;
    }}

    .portrait-wrapper {{
      overflow: hidden;
      border: 2px solid rgba(255, 255, 255, 0.22);
      background: rgba(255, 255, 255, 0.045);
      box-shadow:
        inset 0 0 0 1px rgba(0, 0, 0, 0.3),
        0 6px 14px rgba(0, 0, 0, 0.28);
    }}

    .portrait-main {{
      width: 82px;
      height: 82px;
      padding: 2px;
      border-color: rgba(74, 168, 255, 0.58);
    }}

    .portrait-gp {{
      width: 62px;
      height: 62px;
      padding: 2px;
      border-color: rgba(206, 147, 216, 0.5);
    }}

    .portrait-left {{
      border-color: rgba(255, 120, 178, 0.55);
    }}

    .portrait-right {{
      border-color: rgba(206, 147, 216, 0.55);
    }}

    .parent-affinity-badges {{
      display: flex;
      justify-content: center;
      min-height: 17px;
    }}

    .heart-icon {{
      color: currentColor;
      font-size: 10px;
      line-height: 1;
    }}

    .node-role-label {{
      display: block;
      color: var(--text-disabled);
      font-size: 9px;
      font-weight: 800;
      line-height: 1;
      text-transform: uppercase;
      letter-spacing: 0;
    }}

    .node-role-main {{
      color: var(--accent-primary);
      opacity: 0.95;
    }}

    .support-card-section {{
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 12px;
      min-height: 76px;
      padding: 9px 12px;
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: var(--radius-md);
      background: rgba(255, 255, 255, 0.025);
    }}

    .support-card-image {{
      display: block;
      width: 64px;
      height: 64px;
      flex: 0 0 auto;
      border-radius: 7px;
      object-fit: cover;
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
    }}

    .card-limit-break {{
      display: flex;
      flex-direction: row;
      gap: 3px;
      align-items: center;
      justify-content: center;
    }}

    .limit-break-icon {{
      display: block;
      width: 22px;
      height: 22px;
      flex: 0 0 auto;
    }}

    .limit-break-icon path {{
      fill: rgba(100, 181, 246, 0.58);
    }}

    .limit-break-icon.filled path {{
      fill: #2196f3;
    }}

    .spark-arrays {{
      justify-content: flex-start;
      padding: 2px 0 0;
    }}

    .spark-container {{
      display: flex;
      flex-direction: column;
      gap: 8px;
      min-width: 0;
      overflow: hidden;
    }}

    .spark-list {{
      gap: 6px;
    }}

    .spark-item {{
      max-width: 310px;
      padding: 4px 8px;
      gap: 4px;
      font-size: 13px;
    }}

    .spark-level {{
      color: currentColor;
      font-weight: 600;
      font-variant-numeric: tabular-nums;
    }}

    .spark-star {{
      display: block;
      width: 13px;
      height: 13px;
      flex: 0 0 13px;
      color: currentColor;
      opacity: 1;
    }}

    .spark-star path {{
      fill: currentColor;
    }}

    .spark-item.matched-filter {{
      border-color: #ffd666 !important;
      background: linear-gradient(45deg, rgba(255, 214, 102, 0.1), rgba(255, 214, 102, 0.035));
      box-shadow: inset 0 0 0 1px rgba(255, 214, 102, 0.28);
    }}

    .parent-source {{
      display: inline-flex;
      align-items: center;
      gap: 1px;
      margin-left: 0;
      color: #ff9100;
      line-height: 1;
    }}

    .parent-icon {{
      display: block;
      width: 14px;
      height: 14px;
      color: currentColor;
      flex: 0 0 auto;
      opacity: 0.9;
    }}

    .parent-icon path {{
      fill: currentColor;
    }}

    .parent-contribution {{
      display: inline-flex;
      align-items: center;
      gap: 1px;
      color: currentColor;
      font-size: 12px;
      font-weight: 700;
      line-height: 1;
      opacity: 0.9;
    }}

    .parent-contribution::before {{
      content: "(";
      opacity: 0.85;
    }}

    .parent-contribution::after {{
      content: ")";
      opacity: 0.85;
    }}

    .parent-contribution .spark-star {{
      width: 13px;
      height: 13px;
      flex-basis: 13px;
    }}

    .spark-name {{
      max-width: 185px;
    }}

    .spark-pct {{
      max-width: 72px;
      padding: 2px 5px;
    }}

    .overflow-spark {{
      opacity: 0.78;
    }}

    .summary-panel {{
      display: flex;
      flex-direction: column;
      justify-content: center;
      gap: 10px;
    }}

    .summary-label {{
      color: var(--text-muted);
      font-size: 11px;
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0;
    }}

    .summary-title {{
      margin: 0;
      color: var(--text-primary);
      font-size: 22px;
      font-weight: 800;
      line-height: 1.15;
    }}

    .summary-text {{
      margin: 0;
      color: var(--text-secondary);
      font-size: 14px;
      line-height: 1.45;
      -webkit-line-clamp: 4;
    }}

    .overview-body {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) 388px;
      gap: 18px;
      align-items: stretch;
      min-height: 0;
      margin-top: 9px;
      padding-top: 12px;
      border-top: 1px solid var(--border-primary);
      flex: 1 1 auto;
      overflow: hidden;
    }}

    .overview-copy {{
      display: flex;
      flex-direction: column;
      justify-content: center;
      gap: 10px;
      min-width: 0;
    }}

    .overview-label {{
      width: fit-content;
      max-width: 260px;
      padding: 4px 8px;
      border: 1px solid var(--accent-soft);
      border-radius: var(--radius-xs);
      background: var(--accent-faint);
      color: var(--accent);
      font-size: 12px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }}

    .overview-title {{
      display: block;
      max-width: 100%;
      color: var(--text-primary);
      font-size: 36px;
      font-weight: 900;
      line-height: 1.05;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .overview-text {{
      max-width: 620px;
      margin: 0;
      color: var(--text-secondary);
      font-size: 16px;
      line-height: 1.45;
    }}

    .overview-metrics {{
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-top: 2px;
    }}

    .overview-metric {{
      display: grid;
      gap: 3px;
      min-width: 92px;
      max-width: 150px;
      padding: 8px 10px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.035);
    }}

    .overview-metric-value {{
      color: var(--text-primary);
      font-size: 15px;
      font-weight: 800;
      line-height: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .overview-metric-label {{
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 800;
      line-height: 1;
      text-transform: uppercase;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .overview-visual {{
      min-width: 0;
      min-height: 0;
      display: flex;
      align-items: stretch;
    }}

    .visual-panel {{
      position: relative;
      width: 100%;
      min-height: 312px;
      max-height: 330px;
      overflow: hidden;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-md);
      background:
        linear-gradient(135deg, var(--accent-faint), rgba(255, 255, 255, 0.018)),
        rgba(255, 255, 255, 0.025);
    }}

    .hub-visual {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 9px;
      padding: 14px;
    }}

    .hub-cell {{
      display: grid;
      align-content: center;
      gap: 5px;
      min-width: 0;
      padding: 12px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.045);
    }}

    .hub-title {{
      color: var(--text-primary);
      font-size: 17px;
      font-weight: 850;
      line-height: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .hub-subtitle {{
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 700;
      line-height: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .profile-visual {{
      display: grid;
      grid-template-rows: 88px auto 76px 44px;
      gap: 10px;
      padding: 16px;
    }}

    .profile-avatar {{
      display: flex;
      align-items: center;
      justify-content: center;
      justify-self: center;
      width: 78px;
      height: 78px;
      border: 2px solid rgba(255, 64, 129, 0.5);
      border-radius: 50%;
      background: rgba(255, 64, 129, 0.12);
      color: #ff80ab;
      font-size: 18px;
      font-weight: 900;
    }}

    .profile-lines {{
      display: grid;
      gap: 4px;
      text-align: center;
      min-width: 0;
    }}

    .profile-lines strong,
    .profile-lines span {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .profile-lines strong {{ font-size: 19px; }}
    .profile-lines span {{ color: var(--text-muted); font-size: 12px; font-weight: 700; }}

    .profile-stat-strip {{
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 8px;
    }}

    .profile-stat-strip span {{
      display: grid;
      align-content: center;
      gap: 4px;
      min-width: 0;
      padding: 8px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.04);
      text-align: center;
    }}

    .profile-stat-strip b,
    .profile-stat-strip small {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .profile-stat-strip b {{ font-size: 13px; }}
    .profile-stat-strip small {{ color: var(--text-muted); font-size: 10px; font-weight: 800; text-transform: uppercase; }}

    .profile-tabs {{
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 6px;
    }}

    .profile-tabs span {{
      display: flex;
      align-items: center;
      justify-content: center;
      min-width: 0;
      border: 1px solid rgba(255, 64, 129, 0.2);
      border-radius: var(--radius-xs);
      background: rgba(255, 64, 129, 0.08);
      color: #ff80ab;
      font-size: 11px;
      font-weight: 800;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .leaderboard-visual,
    .club-detail-visual {{
      display: flex;
      flex-direction: column;
      gap: 9px;
      padding: 16px;
    }}

    .visual-kicker {{
      color: var(--accent);
      font-size: 12px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .database-visual {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr) auto;
      gap: 14px;
      padding: 16px;
    }}

    .database-search-rail {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
    }}

    .database-search-rail span {{
      min-width: 0;
      padding: 8px 9px;
      border: 1px solid rgba(100, 181, 246, 0.22);
      border-radius: var(--radius-sm);
      background: rgba(100, 181, 246, 0.075);
      color: #90caf9;
      font-size: 11px;
      font-weight: 850;
      line-height: 1;
      text-align: center;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .database-tree-preview {{
      display: grid;
      grid-template-rows: 78px 56px 70px;
      justify-items: center;
      align-content: center;
      min-width: 0;
      min-height: 0;
      padding: 4px 0;
    }}

    .database-node {{
      display: grid;
      place-items: center;
      width: 70px;
      height: 70px;
      border: 2px solid rgba(100, 181, 246, 0.58);
      border-radius: 50%;
      background: rgba(100, 181, 246, 0.09);
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }}

    .database-node-parent {{
      width: 58px;
      height: 58px;
      border-color: rgba(206, 147, 216, 0.54);
      background: rgba(206, 147, 216, 0.09);
      font-size: 12px;
    }}

    .database-tree-lines {{
      width: 180px;
      height: 56px;
    }}

    .database-tree-lines path {{
      fill: none;
      stroke: rgba(205, 214, 224, 0.52);
      stroke-width: 2;
      stroke-linecap: square;
    }}

    .database-parent-row {{
      display: flex;
      justify-content: space-between;
      width: 190px;
    }}

    .database-factor-grid {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }}

    .database-factor {{
      display: grid;
      grid-template-columns: 20px 20px minmax(0, 1fr);
      align-items: center;
      gap: 6px;
      min-width: 0;
      padding: 7px 8px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: var(--radius-sm);
      background: rgba(0, 0, 0, 0.16);
      color: var(--text-secondary);
      font-size: 12px;
      font-weight: 760;
      line-height: 1;
    }}

    .database-factor b,
    .database-factor span,
    .database-factor strong {{
      display: inline-flex;
      align-items: center;
      min-width: 0;
      line-height: 1;
      white-space: nowrap;
    }}

    .database-factor b {{
      justify-content: flex-end;
      color: var(--text-primary);
      font-weight: 900;
      font-variant-numeric: tabular-nums;
    }}

    .database-factor span {{
      justify-content: center;
      height: 18px;
      border-radius: 4px;
      font-size: 10px;
      font-weight: 900;
    }}

    .database-factor strong {{
      overflow: hidden;
      text-overflow: ellipsis;
      font-weight: 800;
    }}

    .database-factor-b span {{
      background: rgba(33, 150, 243, 0.18);
      color: #64b5f6;
    }}

    .database-factor-p span {{
      background: rgba(233, 30, 99, 0.18);
      color: #f06292;
    }}

    .database-factor-g span {{
      background: rgba(76, 175, 80, 0.18);
      color: #81c784;
    }}

    .database-factor-w span {{
      background: rgba(189, 189, 189, 0.16);
      color: #d0d0d0;
    }}

    .leader-row {{
      display: grid;
      grid-template-columns: 54px minmax(0, 1fr) 86px;
      align-items: center;
      gap: 10px;
      padding: 10px 12px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.04);
    }}

    .leader-rank {{
      color: #ffd666;
      font-size: 19px;
      font-weight: 900;
      line-height: 1;
    }}

    .leader-name,
    .leader-row strong {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .leader-name {{ color: var(--text-primary); font-size: 15px; font-weight: 800; }}
    .leader-row strong {{ color: var(--text-secondary); font-size: 12px; text-align: right; }}

    .club-rank-token {{
      width: fit-content;
      padding: 7px 10px;
      border: 1px solid rgba(255, 183, 77, 0.3);
      border-radius: var(--radius-sm);
      background: rgba(255, 183, 77, 0.1);
      color: #ffb74d;
      font-size: 22px;
      font-weight: 900;
      line-height: 1;
    }}

    .club-name-block {{
      display: grid;
      gap: 4px;
      min-width: 0;
    }}

    .club-name-block strong,
    .club-name-block span {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .club-name-block strong {{ font-size: 22px; }}
    .club-name-block span {{ color: var(--text-muted); font-size: 12px; font-weight: 750; }}

    .club-member-table {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) 70px 86px;
      gap: 8px 10px;
      padding-top: 9px;
      border-top: 1px solid var(--border-subtle);
      align-items: center;
    }}

    .club-member-table span {{
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .club-member-table b,
    .club-member-table em,
    .club-member-table strong {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-style: normal;
      font-size: 13px;
    }}

    .club-member-table em {{ color: var(--text-muted); }}
    .club-member-table strong {{ color: #81c784; text-align: right; }}

    .activity-visual {{
      display: grid;
      gap: 10px;
      padding: 16px;
      align-content: center;
    }}

    .activity-row {{
      display: grid;
      grid-template-columns: 16px minmax(0, 1fr) 66px;
      align-items: center;
      gap: 10px;
      padding: 9px 10px;
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.04);
    }}

    .activity-row span {{
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: #81c784;
      box-shadow: 0 0 10px rgba(129, 199, 132, 0.35);
    }}

    .activity-row b,
    .activity-row strong {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 13px;
    }}

    .activity-row strong {{ color: #81c784; text-align: right; }}

    .timeline-visual {{
      padding: 18px 16px;
    }}

    .timeline-line {{
      position: absolute;
      top: 50%;
      left: 28px;
      right: 28px;
      height: 3px;
      border-radius: 999px;
      background: linear-gradient(90deg, #81c784, #64b5f6, #ffb74d);
    }}

    .timeline-node {{
      position: absolute;
      display: grid;
      gap: 4px;
      width: 106px;
      padding: 9px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(18, 18, 18, 0.94);
    }}

    .timeline-node::before {{
      content: "";
      position: absolute;
      left: 14px;
      width: 12px;
      height: 12px;
      border: 2px solid var(--accent);
      border-radius: 50%;
      background: var(--bg-secondary);
    }}

    .node-a {{ left: 24px; top: 62px; }}
    .node-b {{ left: 138px; bottom: 56px; }}
    .node-c {{ right: 24px; top: 74px; }}
    .node-a::before, .node-c::before {{ bottom: -38px; }}
    .node-b::before {{ top: -36px; }}

    .timeline-node span {{ color: var(--accent); font-size: 11px; font-weight: 850; text-transform: uppercase; }}
    .timeline-node b {{ color: var(--text-primary); font-size: 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}

    .tierlist-visual {{
      display: grid;
      gap: 10px;
      padding: 16px;
      align-content: center;
    }}

    .tier-row-mini {{
      display: grid;
      grid-template-columns: 52px minmax(0, 1fr);
      gap: 10px;
      align-items: center;
    }}

    .tier-row-mini > span {{
      display: flex;
      align-items: center;
      justify-content: center;
      height: 45px;
      border-radius: var(--radius-sm);
      background: #ffd666;
      color: #171717;
      font-size: 22px;
      font-weight: 950;
    }}

    .tier-row-mini:nth-child(2) > span {{ background: #81c784; }}
    .tier-row-mini:nth-child(3) > span {{ background: #64b5f6; }}

    .tier-row-mini div {{
      display: flex;
      gap: 7px;
      min-width: 0;
    }}

    .tier-card {{
      width: 38px;
      height: 45px;
      flex: 0 0 38px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-xs);
      background:
        linear-gradient(150deg, rgba(255, 255, 255, 0.14), rgba(255, 255, 255, 0.02)),
        rgba(255, 255, 255, 0.08);
    }}

    .statistics-visual {{
      display: grid;
      grid-template-rows: minmax(0, 1fr) 46px;
      gap: 12px;
      padding: 16px;
    }}

    .chart-bars {{
      display: flex;
      align-items: end;
      gap: 13px;
      min-height: 0;
      padding: 12px 12px 0;
      border-left: 1px solid var(--border-subtle);
      border-bottom: 1px solid var(--border-subtle);
    }}

    .chart-bars span {{
      width: 42px;
      border-radius: 5px 5px 0 0;
      background: linear-gradient(180deg, #64b5f6, rgba(100, 181, 246, 0.35));
    }}

    .stat-icons {{
      display: grid;
      grid-template-columns: repeat(5, 1fr);
      gap: 6px;
    }}

    .stat-icons b {{
      display: flex;
      align-items: center;
      justify-content: center;
      min-width: 0;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-xs);
      color: var(--text-secondary);
      font-size: 11px;
    }}

    .planner-visual {{
      position: relative;
      padding: 16px;
    }}

    .planner-node {{
      position: absolute;
      display: flex;
      align-items: center;
      justify-content: center;
      width: 92px;
      height: 44px;
      border: 1px solid rgba(100, 181, 246, 0.32);
      border-radius: var(--radius-sm);
      background: rgba(100, 181, 246, 0.1);
      color: var(--text-primary);
      font-size: 12px;
      font-weight: 850;
    }}

    .planner-node.target {{ left: 148px; top: 22px; }}
    .planner-node.parent-a {{ left: 78px; top: 112px; }}
    .planner-node.parent-b {{ right: 78px; top: 112px; }}
    .planner-node.gp {{ width: 62px; height: 36px; top: 224px; color: var(--text-secondary); }}
    .planner-node.gp-a {{ left: 30px; }}
    .planner-node.gp-b {{ left: 112px; }}
    .planner-node.gp-c {{ right: 112px; }}
    .planner-node.gp-d {{ right: 30px; }}

    .planner-branch {{
      position: absolute;
      inset: 72px 62px 78px;
      border-top: 2px solid rgba(100, 181, 246, 0.36);
      border-left: 2px solid rgba(100, 181, 246, 0.24);
      border-right: 2px solid rgba(100, 181, 246, 0.24);
      opacity: 0.9;
    }}

    .page-visual {{
      display: grid;
      gap: 10px;
      align-content: center;
      padding: 16px;
    }}

    .page-visual-row {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) 120px;
      gap: 10px;
      align-items: center;
      padding: 12px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.04);
    }}

    .page-visual-row span,
    .page-visual-row strong {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-size: 13px;
    }}

    .page-visual-row span {{ color: var(--text-muted); }}
    .page-visual-row strong {{ color: var(--text-primary); text-align: right; }}

    .home-visual {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr) 54px;
      gap: 12px;
      padding: 16px;
      background:
        radial-gradient(circle at 18% 0%, rgba(100, 181, 246, 0.16), transparent 190px),
        radial-gradient(circle at 86% 12%, rgba(129, 199, 132, 0.12), transparent 210px),
        linear-gradient(135deg, rgba(255, 255, 255, 0.045), rgba(255, 255, 255, 0.012));
    }}

    .home-hero-mini {{
      display: grid;
      gap: 5px;
      padding: 12px 14px;
      border: 1px solid rgba(100, 181, 246, 0.2);
      border-radius: var(--radius-sm);
      background: rgba(10, 10, 10, 0.42);
    }}

    .home-hero-mini strong {{
      color: #ffffff;
      font-size: 32px;
      font-weight: 900;
      line-height: 0.95;
    }}

    .home-hero-mini span,
    .home-quick-link small,
    .home-stats-mini small {{
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 750;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .home-quick-grid {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
      min-height: 0;
    }}

    .home-quick-link {{
      display: grid;
      align-content: center;
      gap: 4px;
      min-width: 0;
      padding: 8px 9px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.035);
    }}

    .home-quick-link b,
    .home-stats-mini b {{
      font-size: 12px;
      font-weight: 850;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .ql-database b {{ color: #64b5f6; }}
    .ql-clubs b {{ color: #81c784; }}
    .ql-rankings b {{ color: #ffb74d; }}
    .ql-tierlist b {{ color: #ffd666; }}
    .ql-timeline b {{ color: #ba68c8; }}
    .ql-tools b {{ color: #ce93d8; }}

    .home-stats-mini {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
    }}

    .home-stats-mini span {{
      display: grid;
      align-content: center;
      gap: 3px;
      min-width: 0;
      padding: 8px 10px;
      border: 1px solid rgba(129, 199, 132, 0.18);
      border-radius: var(--radius-xs);
      background: rgba(129, 199, 132, 0.06);
    }}

    .profile-visual {{
      grid-template-rows: 112px 70px 44px;
      gap: 0;
      padding: 0;
      background:
        radial-gradient(circle at 18% 0%, rgba(100, 181, 246, 0.18), transparent 210px),
        radial-gradient(circle at 90% 4%, rgba(129, 199, 132, 0.12), transparent 220px),
        rgba(255, 255, 255, 0.025);
    }}

    .profile-hero-strip {{
      display: flex;
      align-items: center;
      gap: 14px;
      min-width: 0;
      padding: 18px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.07);
      background: linear-gradient(135deg, rgba(100, 181, 246, 0.16), rgba(129, 199, 132, 0.09));
    }}

    .profile-avatar {{
      justify-self: auto;
      width: 64px;
      height: 64px;
      flex: 0 0 64px;
      border-color: rgba(255, 255, 255, 0.12);
      background: rgba(255, 255, 255, 0.06);
      box-shadow: none;
    }}

    .profile-avatar span {{
      color: rgba(255, 255, 255, 0.48);
      font-size: 19px;
    }}

    .profile-lines {{
      align-content: center;
      justify-items: start;
      min-width: 0;
      text-align: left;
    }}

    .profile-lines strong {{
      font-size: 25px;
      color: #ffffff;
    }}

    .profile-lines span {{
      font-size: 12px;
      color: rgba(255, 255, 255, 0.58);
    }}

    .profile-stat-strip {{
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
      padding: 10px 14px;
    }}

    .profile-stat-strip span {{
      border-color: rgba(100, 181, 246, 0.14);
      background: rgba(100, 181, 246, 0.055);
    }}

    .profile-stat-strip b {{
      color: #81c784;
    }}

    .profile-tabs {{
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 7px;
      padding: 0 14px 14px;
    }}

    .profile-tabs span {{
      border-color: rgba(100, 181, 246, 0.18);
      background: rgba(100, 181, 246, 0.06);
      color: #90caf9;
    }}

    .leaderboard-visual,
    .club-detail-visual {{
      background:
        radial-gradient(circle at 85% 0%, var(--accent-faint), transparent 190px),
        rgba(255, 255, 255, 0.025);
    }}

    .tools-visual {{
      display: grid;
      grid-template-rows: 96px minmax(0, 1fr);
      gap: 12px;
      padding: 16px;
      background:
        radial-gradient(circle at 14% 0%, rgba(206, 147, 216, 0.18), transparent 210px),
        radial-gradient(circle at 88% 18%, rgba(100, 181, 246, 0.12), transparent 220px),
        rgba(255, 255, 255, 0.025);
    }}

    .tools-hero-mini {{
      display: grid;
      align-content: center;
      gap: 5px;
      min-width: 0;
      padding: 14px;
      border: 1px solid rgba(206, 147, 216, 0.2);
      border-radius: var(--radius-sm);
      background: rgba(206, 147, 216, 0.07);
    }}

    .tools-hero-mini span {{
      color: #ce93d8;
      font-size: 12px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .tools-hero-mini strong {{
      color: #ffffff;
      font-size: 26px;
      font-weight: 900;
      line-height: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .tools-grid-mini {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 9px;
      min-height: 0;
    }}

    .tool-card-mini {{
      display: grid;
      align-content: center;
      gap: 5px;
      min-width: 0;
      padding: 11px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.035);
    }}

    .tool-card-mini b,
    .tool-card-mini small {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .tool-card-mini b {{ font-size: 14px; font-weight: 850; }}
    .tool-card-mini small {{ color: var(--text-muted); font-size: 11px; font-weight: 700; }}
    .tool-statistics b {{ color: #64b5f6; }}
    .tool-lineage b {{ color: #ce93d8; }}
    .tool-stamina b {{ color: #81c784; }}
    .tool-simulator b {{ color: #ffb74d; }}

    .leader-row.leader-top-1 {{
      border-color: rgba(255, 215, 0, 0.25);
      background: linear-gradient(90deg, rgba(255, 215, 0, 0.12), rgba(255, 255, 255, 0.035));
    }}

    .leader-row.leader-top-2 {{
      border-color: rgba(192, 192, 192, 0.22);
      background: linear-gradient(90deg, rgba(192, 192, 192, 0.09), rgba(255, 255, 255, 0.035));
    }}

    .leader-row.leader-top-3 {{
      border-color: rgba(205, 127, 50, 0.22);
      background: linear-gradient(90deg, rgba(205, 127, 50, 0.09), rgba(255, 255, 255, 0.035));
    }}

    .club-header-row {{
      display: grid;
      grid-template-columns: 62px minmax(0, 1fr);
      gap: 12px;
      align-items: center;
      padding: 12px;
      border: 1px solid rgba(129, 199, 132, 0.18);
      border-radius: var(--radius-sm);
      background: linear-gradient(135deg, rgba(100, 181, 246, 0.09), rgba(129, 199, 132, 0.07));
    }}

    .club-name-block,
    .club-rank-token {{
      min-width: 0;
    }}

    .club-rank-token {{
      display: flex;
      align-items: center;
      justify-content: center;
      height: 50px;
      border: 1px solid rgba(255, 183, 77, 0.24);
      border-radius: var(--radius-xs);
      background: rgba(255, 183, 77, 0.08);
      color: #ffcc80;
      font-size: 16px;
      font-weight: 900;
    }}

    .club-name-block,
    .club-title-lines {{
      display: grid;
      gap: 4px;
      min-width: 0;
    }}

    .club-name-block strong,
    .club-name-block span,
    .club-title-lines strong,
    .club-title-lines span {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .club-name-block strong,
    .club-title-lines strong {{ font-size: 18px; }}
    .club-name-block span,
    .club-title-lines span {{ color: var(--text-muted); font-size: 12px; font-weight: 750; }}

    .activity-visual {{
      align-content: stretch;
      grid-template-rows: 72px repeat(4, 44px);
      background:
        radial-gradient(circle at 18% 0%, rgba(255, 183, 77, 0.14), transparent 190px),
        rgba(255, 255, 255, 0.025);
    }}

    .activity-summary-row {{
      display: grid;
      grid-template-columns: 58px minmax(0, 1fr);
      gap: 12px;
      align-items: center;
      padding: 10px;
      border: 1px solid rgba(255, 183, 77, 0.2);
      border-radius: var(--radius-sm);
      background: rgba(255, 183, 77, 0.07);
    }}

    .activity-score {{
      display: flex;
      align-items: center;
      justify-content: center;
      height: 50px;
      border-radius: var(--radius-xs);
      background: rgba(255, 183, 77, 0.12);
      color: #ffcc80;
      font-size: 22px;
      font-weight: 900;
    }}

    .activity-summary-row div {{
      display: grid;
      gap: 4px;
      min-width: 0;
    }}

    .activity-summary-row b,
    .activity-summary-row small {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .activity-summary-row b {{ font-size: 16px; }}
    .activity-summary-row small {{ color: var(--text-muted); font-size: 11px; font-weight: 700; }}

    .activity-row.severity-high span {{ background: #e57373; }}
    .activity-row.severity-watch span {{ background: #ffb74d; }}
    .activity-row.severity-low span {{ background: #81c784; }}
    .activity-row.severity-info span {{ background: #64b5f6; }}

    .timeline-visual {{
      background:
        radial-gradient(circle at 18% 0%, rgba(129, 199, 132, 0.15), transparent 210px),
        radial-gradient(circle at 86% 12%, rgba(100, 181, 246, 0.13), transparent 210px),
        rgba(255, 255, 255, 0.025);
    }}

    .timeline-event-card {{
      position: absolute;
      display: grid;
      gap: 5px;
      width: 134px;
      min-width: 0;
      padding: 11px;
      border: 1px solid var(--border-subtle);
      border-radius: var(--radius-sm);
      background: rgba(18, 18, 18, 0.9);
      box-shadow: 0 12px 28px rgba(0, 0, 0, 0.22);
    }}

    .timeline-event-card span,
    .timeline-event-card b,
    .timeline-event-card small {{
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .timeline-event-card span {{
      color: var(--accent);
      font-size: 10px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .timeline-event-card b {{ font-size: 14px; }}
    .timeline-event-card small {{ color: var(--text-muted); font-size: 11px; font-weight: 700; }}
    .event-character {{ border-color: rgba(129, 199, 132, 0.22); }}
    .event-support {{ border-color: rgba(100, 181, 246, 0.24); }}
    .event-story {{ border-color: rgba(255, 183, 77, 0.24); }}

    .tierlist-visual {{
      grid-template-rows: 30px repeat(3, 54px);
      background:
        radial-gradient(circle at 22% 0%, rgba(255, 214, 102, 0.16), transparent 190px),
        radial-gradient(circle at 92% 12%, rgba(233, 30, 99, 0.09), transparent 210px),
        rgba(255, 255, 255, 0.025);
    }}

    .tierlist-tabs-mini {{
      display: flex;
      gap: 7px;
      min-width: 0;
    }}

    .tierlist-tabs-mini span {{
      display: flex;
      align-items: center;
      justify-content: center;
      min-width: 0;
      flex: 1 1 0;
      border: 1px solid rgba(255, 214, 102, 0.18);
      border-radius: var(--radius-xs);
      background: rgba(255, 214, 102, 0.055);
      color: #ffe082;
      font-size: 11px;
      font-weight: 850;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .tier-row-mini:nth-child(2) > span {{ background: #e57373; }}
    .tier-row-mini:nth-child(3) > span {{ background: #ffd666; color: #0a0a0a; }}
    .tier-row-mini:nth-child(4) > span {{ background: #81c784; color: #0a0a0a; }}

    .statistics-visual {{
      grid-template-rows: minmax(0, 1fr) 46px;
      background:
        radial-gradient(circle at 16% 0%, rgba(100, 181, 246, 0.15), transparent 200px),
        rgba(255, 255, 255, 0.025);
    }}

    .chart-card-mini {{
      display: grid;
      grid-template-rows: 24px minmax(0, 1fr);
      gap: 8px;
      min-height: 0;
      padding: 10px;
      border: 1px solid rgba(100, 181, 246, 0.14);
      border-radius: var(--radius-sm);
      background: rgba(255, 255, 255, 0.035);
    }}

    .chart-title-mini {{
      color: #90caf9;
      font-size: 12px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .planner-node {{
      flex-direction: column;
      gap: 3px;
      line-height: 1;
    }}

    .planner-node b,
    .planner-node small {{
      max-width: 78px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .planner-node b {{
      color: var(--text-primary);
      font-size: 12px;
    }}

    .planner-node small {{
      color: #90caf9;
      font-size: 10px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .record-footer {{
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 8px;
      margin-top: auto;
      padding-top: 6px;
      border-top: 1px solid rgba(255, 255, 255, 0.04);
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 650;
      line-height: 1;
    }}

    .url {{
      max-width: 620px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .verified-meta {{
      display: inline-flex;
      align-items: center;
      gap: 6px;
      color: var(--text-muted);
      white-space: nowrap;
    }}

    .verified-meta::before {{
      content: "";
      width: 7px;
      height: 7px;
      border-radius: 50%;
      background: var(--accent-secondary);
      box-shadow: 0 0 9px rgba(129, 199, 132, 0.48);
    }}

    .verified-text {{
      color: var(--accent-secondary);
      font-size: 11px;
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0;
    }}

    .footer-dot {{
      color: var(--text-disabled);
    }}
  </style>
</head>
<body class="embed-card-page {class_list} {view_class}">
  <main class="embed-page {class_list} {view_class}">
    <header class="embed-header">
      {identity}
      <div class="embed-brand"><img class="brand-logo" src="{brand_logo_url}" alt=""><span class="brand-text">uma.moe</span></div>
    </header>

    <article class="record-card {class_list} {view_class}">
      <header class="record-header">
        {stats}
      </header>

      {body}

      {footer}
    </article>
  </main>
</body>
</html>
"#,
        accent = accent.hex,
        accent_soft = accent.soft,
        accent_faint = accent.faint,
        identity = identity,
        stats = stats,
        body = body,
        footer = footer,
        brand_logo_url = brand_logo_url,
        class_list = class_list,
        view_class = view_class,
    )
}

fn render_identity(meta: &EmbedMetadata) -> String {
    if let Some(database) = &meta.database {
        let primary = display_trainer_name(&database.trainer_name);
        let secondary = format!(
            "#{} / {}",
            format_trainer_id_display(&database.trainer_id),
            database.query_label
        );
        let result_rank = render_result_rank(database.result_total);

        return format!(
            r#"<div class="embed-identity"><span class="embed-user-name">{}</span><span class="embed-user-id">{}{}</span></div>"#,
            html_escape(&truncate_chars(&primary, 48)),
            html_escape(&truncate_chars(&secondary, 72)),
            result_rank,
        );
    }

    if is_database_like(meta) {
        let primary = display_title(&meta.title);
        let secondary = "Inheritance search / parents, factors, support cards".to_string();

        return format!(
            r#"<div class="embed-identity"><span class="embed-user-name">{}</span><span class="embed-user-id">{}</span></div>"#,
            html_escape(&truncate_chars(&primary, 48)),
            html_escape(&secondary),
        );
    }

    let primary = metric_value(&meta.metrics, &["Trainer", "Leader"])
        .unwrap_or_else(|| meta.kind_label.clone());
    let secondary = metric_value(&meta.metrics, &["Trainer ID", "Club ID"])
        .or_else(|| extract_parenthesized_id(&meta.description))
        .or_else(|| metric_value(&meta.metrics, &["Record"]))
        .unwrap_or_else(|| compact_url(&meta.canonical_url));

    format!(
        r#"<div class="embed-identity"><span class="embed-user-name">{}</span><span class="embed-user-id">{}</span></div>"#,
        html_escape(&truncate_chars(&primary, 48)),
        html_escape(&truncate_chars(&secondary, 32)),
    )
}

fn render_result_rank(total: i64) -> String {
    if total <= 0 {
        return String::new();
    }

    format!(
        r#"<span class="embed-result-rank">Result 1 of {}</span>"#,
        html_escape(&format_result_total(total))
    )
}

fn format_result_total(total: i64) -> String {
    if total > 10_000 {
        "10k+".to_string()
    } else {
        format_number_grouped(total, ',')
    }
}

fn render_stats(meta: &EmbedMetadata) -> String {
    if let Some(database) = &meta.database {
        return database::render_stats(database);
    }

    if is_database_like(meta) && metric_value(&meta.metrics, &["Results"]).is_none() {
        return String::new();
    }

    let metrics = &meta.metrics;
    let preferred = [
        "Affinity",
        "G1 Wins",
        "White",
        "Rank",
        "Score",
        "Fans",
        "Fan Rank",
        "Followers",
        "Members",
        "Points",
        "Results",
    ];
    let mut selected = Vec::new();

    for label in preferred {
        if let Some(metric) = metrics
            .iter()
            .find(|metric| metric.label.eq_ignore_ascii_case(label) && should_show_stat(metric))
        {
            selected.push(metric);
        }

        if selected.len() >= 5 {
            break;
        }
    }

    for metric in metrics.iter().filter(|metric| should_show_stat(metric)) {
        if selected.len() >= 5 {
            break;
        }

        if !selected
            .iter()
            .any(|existing| existing.label.eq_ignore_ascii_case(&metric.label))
        {
            selected.push(metric);
        }
    }

    let mut items = selected
        .into_iter()
        .take(5)
        .map(render_stat_pill)
        .collect::<Vec<_>>();

    if items.is_empty() {
        items = metrics.iter().take(4).map(render_stat_pill).collect();
    }

    if items.is_empty() {
        return String::new();
    }

    format!(
        r#"<div class="record-header-stats"><div class="main-stats">{}</div></div>"#,
        items.join("")
    )
}

fn render_stat_pill(metric: &EmbedMetric) -> String {
    format!(
        r#"<span class="stat-pill {}"><span class="stat-number">{}</span><span class="stat-label">{}</span></span>"#,
        stat_class(&metric.label),
        html_escape(&truncate_chars(&metric.value, 16)),
        html_escape(&truncate_chars(&metric.label, 18)),
    )
}

fn render_body(meta: &EmbedMetadata) -> String {
    if let Some(database) = &meta.database {
        database::render_body(database)
    } else {
        overview::render_body(meta)
    }
}

fn render_footer(meta: &EmbedMetadata, url: &str) -> String {
    if let Some(database) = &meta.database {
        let date = database
            .last_updated
            .as_deref()
            .map(format_embed_date)
            .unwrap_or_else(|| "recently".to_string());
        let record_id = database
            .record_id
            .map(|record_id| record_id.to_string())
            .unwrap_or_default();
        let followers = database
            .follower_num
            .map(|followers| followers.to_string())
            .unwrap_or_default();

        return format!(
            r#"<footer class="record-footer" data-query="{}" data-record-id="{}" data-followers="{}">
        <span class="verified-meta"><span class="verified-text">Verified</span></span>
        <span class="footer-dot">&middot;</span>
        <span class="url">{}</span>
      </footer>"#,
            html_escape(&database.query_label),
            html_escape(&record_id),
            html_escape(&followers),
            html_escape(&date),
        );
    }

    format!(
        r#"<footer class="record-footer">
        <span class="verified-meta"><span class="verified-text">Preview</span></span>
        <span class="footer-dot">&middot;</span>
        <span class="url">{url}</span>
      </footer>"#
    )
}

fn should_show_stat(metric: &EmbedMetric) -> bool {
    let label = metric.label.to_ascii_lowercase();

    !matches!(
        label.as_str(),
        "trainer"
            | "leader"
            | "search"
            | "site"
            | "view"
            | "use"
            | "data"
            | "focus"
            | "tools"
            | "record"
    )
}

fn stat_class(label: &str) -> &'static str {
    let label = label.to_ascii_lowercase();

    if label.contains("affinity") {
        "affinity-stat"
    } else if label.contains("win") {
        "wins-stat"
    } else if label.contains("white") {
        "white-stat"
    } else if label.contains("rank")
        || label.contains("score")
        || label.contains("result")
        || label.contains("fan")
        || label.contains("point")
    {
        "score-stat"
    } else {
        "neutral-stat"
    }
}

fn is_database_like(meta: &EmbedMetadata) -> bool {
    meta.kind_label.eq_ignore_ascii_case("database")
        || meta.metrics.iter().any(|metric| {
            matches!(
                metric.label.as_str(),
                "Affinity" | "G1 Wins" | "White" | "Record"
            )
        })
}

fn metric_value(metrics: &[EmbedMetric], labels: &[&str]) -> Option<String> {
    labels.iter().find_map(|label| {
        metrics
            .iter()
            .find(|metric| metric.label.eq_ignore_ascii_case(label))
            .map(|metric| metric.value.clone())
    })
}

fn display_title(title: &str) -> String {
    title
        .strip_suffix(" | uma.moe")
        .unwrap_or(title)
        .trim()
        .to_string()
}

fn extract_parenthesized_id(value: &str) -> Option<String> {
    let start = value.find('(')?;
    let after_start = &value[start + 1..];
    let end = after_start.find(')')?;
    let candidate = after_start[..end].trim();

    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

fn short_id(value: &str) -> String {
    let trimmed = value.trim();

    if trimmed.len() <= 4 {
        return trimmed.to_ascii_uppercase();
    }

    trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn asset_url(base_url: &str, path: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');

    if base_url.is_empty() {
        format!("/{path}")
    } else {
        format!("{base_url}/{path}")
    }
}

const BRAND_LOGO_DATA_URL: &str = concat!(
    "data:image/svg+xml;base64,",
    "PHN2ZyB3aWR0aD0iNDkzIiBoZWlnaHQ9IjQzMiIgdmlld0JveD0iMCAwIDQ5MyA0MzIiIGZpbGw9Im5vbmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+CjxwYXRoIGQ9Ik0xNSAxMTVDMTQuOTI1NCA5MS43MzM5IDE1LjM3MzggNzUuMDMwOSAxOS4zMjgzIDQ2LjVDMjMuNDk4OCA0NS4zNjUyIDI4LjEwOTYgNDQuNTk0MyAzMCA0Ni41QzMxLjg5MDQgNDguNDA1NyA1OSA3MyA3OC41IDExNy41QzgwLjMzNTIgMTE3LjgwMyA4MS40ODMyIDExNy44NTcgODQgMTE3LjVWNjUuNUwxMzcuNjE5IDAuOTk5OTk5QzE0MC41MzUgMC42Mjc0ODEgMTQ1LjYyMiAzLjQ0OTEzIDE0NS42MTkgNS41QzE0NS42MTggNi4wNjU2OCAxNDQuNjE5IDU5IDE0NC42MTkgNTlDMTQxLjExOSA2OS41IDEzNi42MTkgODcgMTM3LjYxOSA4Ni41QzEzOC42MTkgODYgMTY0LjcgNzguNjUxOSAxNzkuNjE5IDc2TDE4MC4xMTkgODFDOTUuMTE5IDEyMC41IDk4LjExOSAyNTcuNSAyMTYuNjE5IDI2OS41QzIxNi42MTkgMjY5LjUgMjAyLjYxOSAyOTUgMTQ0LjExOSAyODZDMTQwLjYzNCAyOTQuNDMzIDEzOS4wMDggMjk5LjMxNSAxMzcuMTE5IDMwOC41SDEzMy42MTlDMTMwLjE5NiAyOTkuODgyIDEyOS4wNTIgMjk2LjUxMiAxMjAuMTE5IDI3Ny41QzExNy42MDggMjc2LjQxNiAxMTIuMjA0IDI3OC4zNTQgMTEyLjExOSAyNzguNUMxMTIuMDM1IDI3OC42NDMgMTA2LjU5NyAyOTAuNzU5IDEwNi42MTkgMjkzQzEwNi42NDUgMjk1LjYxNCAxMTAuMTg1IDMwNC4xMzYgMTEwLjExOSAzMDYuNUMxMTAuNjYgMzA5Ljg0NiAxMTAuNjY5IDMxMy43NTcgMTEwLjExOSAzMjQuNUMxMDUuNjgzIDMyOS40NSAxMDQuMjgxIDMzMC45MTQgMTAwLjYxOSAzMzEuNUM5My43MTUzIDMyOS4xMDUgOTAuMDQxNSAzMjQuMDgzIDg0LjExOSAzMDEuNUg4MC42MTlMODAuNTk4MiAzMDEuMzI5Qzc5LjU1MjMgMjkyLjc4NCA3OC45NTk1IDI4Ny45NDEgOTAuNjE5IDI3Ny41QzcwLjAzNzIgMjcyLjU3MiA2MS4yNTk3IDI2OS4wMjUgNTUuMTE5IDI2MEM1My42MjkgMjMwLjY5NyA1My45OTU3IDIxNy4yNzMgNjEuMTE5IDIwOS41QzY5LjkwMjUgMjM4LjU2NyA3OS4wNjczIDI1MS45NDkgMTAzLjExOSAyNzAuNUgxMTcuMTE5Qzg4LjA0MDcgMjQ3Ljc3OCA3NC4yMTg2IDIzMS44MDcgNjguNjE5IDE3OC41QzQ4LjYxOSAyMDQgNDAuMTE5IDI0MSA1MC4xMTkgMjc0QzYwLjExOSAzMDcgNTkuOTQ0NiAzMDUuNDIgODMuMTE5IDM0NC41TDY4LjExOSAzNjAuNUM2Ny41MjYyIDM2NS4wMzEgNjcuNjAyNyAzNjcuMzU0IDY5LjYxOSAzNzAuNUM4Ni41ODEgMzg1LjE2OCA5Ni4yMzcxIDM5My40NzkgMTE3LjExOSA0MTAuNUMxNDMuMTk4IDM4Ni40OTUgMTU4LjEwOCAzNzEuMTM5IDE4NS4xMTkgMzQxQzE5Mi4yNjcgMzQ5LjkxOSAxOTguOTU2IDM1My4wNTMgMjE2LjExOSAzNTVDMjI1LjQ5NiAzNTEuNTMzIDIzMS45OSAzNDguMTc4IDI0NC42MTkgMzQxQzI1Ny4xNDkgMzMxLjQ1NiAyNjQuNTI2IDMyNC41MjcgMjgwLjYxOSAyOTlDMjkwLjM5IDI4MS4wMjMgMjkzLjQ4NCAyNjcuNTE3IDI5Ni42MTkgMjQwQzI5Ni45MTIgMjE4LjA5IDI5NC42MTkgMjA5LjUgMjgzLjYxOSAxOTFDMjY4LjExOSAxNjcgMjQ5LjYxOSAxNTggMjQyLjExOSAxNTMuNUMyNDIuMTE5IDE1My41IDIyMS4xMTkgMTQwLjUgMTgyLjYxOSAxMzZDMTc5LjQxNiAxMzUuNjI2IDE0Ny4wOTEgMTM2LjcyNyAxNDQuNSAxMzVDMTQyLjUgMTMzLjY2NyAxNDIgMTI3LjUgMTQ0IDEyMy41QzE0NiAxMTkuNSAxNjYuNSA5MCAxODYuNjE5IDkwQzIwMC41IDkwIDIyNS41IDEwMS41IDIzOS4xMTkgMTExQzI1NCAxMjEgMjY0LjA1MSAxMzEuMzU2IDI3OC42MTkgMTUwTDI3OS4xMyAxNTAuNzM2QzI4Ny44OTEgMTYzLjM2MiAyOTIuMjE5IDE2OS41OTkgMzAzLjExOSAxOTVDMzA3LjAyOSAyMTEuMTc0IDMwOS41NTcgMjIyLjcwMSAzMTAuNjE5IDI0MEMzMTAuMDMzIDI2MS4zMzcgMzA5LjY2MyAyNjkuNSAzMDQuNjE5IDMwM0MzMDIuNTg2IDMxNi41IDI5MSAzMzcgMjc4LjYxOSAzNTNDMjY2LjIzOCAzNjkgMjIxLjg0IDQxMC4wNzIgMjIwLjExOSA0MTEuNUMyMTguMzk4IDQxMi45MjggMjEzLjYxOSA0MTMuNjYxIDIxMi42MTkgNDEzLjVDMjExLjYxOSA0MTMuMzM5IDIwOS42OTQgNDEzLjIzNyAyMDkuMTE5IDQxMUMyMDkuNzgzIDQwNC40MTggMjEwLjAyNCA0MDAuNDc0IDIwNi42MTkgMzg2QzIwNC42MDMgMzgwLjk1NiAyMDIuMTk2IDM3OC43IDE5OC4xMTkgMzc2LjVDMTk1LjQxMSAzNzUuNTYzIDE5My45OTQgMzc1LjYwMiAxOTEuNjE5IDM3Ni41TDE5MC42MjIgMzc3LjUzNUMxNjQuNzkzIDQwNC4zNDkgMTQ1LjE3NCA0MjQuNzE3IDEzNC4xMTkgNDMwLjVDMTMwIDQzMiAxMjQuNSA0MzEuNSAxMjEgNDI5LjVDMTE3LjUgNDI3LjUgNTEuNSAzNTkuOTM4IDQ5LjYxOSAzNTUuNUM0Ny43MzgxIDM1MS4wNjIgNDcuNSAzNDYuNSA0OS4xMTkgMzQ1LjVDNDkuMTE5IDM0NS41IDU5LjYxOSAzNDEgNjIuMTE5IDMzN0M2NC42MTkgMzMzIDYxLjQzNzYgMzI1LjYwNCA1OC4xMTkgMzIxLjVDMzMuNjE5IDMyMS41IC0yLjMzMjE5IDI4MC41IDAuMTE5MDE4IDI0N0MyLjU3MDIzIDIxMy41IDIxLjM4NTYgMTc2LjkzOSA0NC41IDE1NS41QzM0Ljg3MDIgMTQxLjQxNSAyNy4xODE2IDEzMS45NDEgMTUgMTE1WiIgZmlsbD0id2hpdGUiLz4KPHBhdGggZD0iTTMyNS41IDIzMEMzMTcuNzI3IDExNy41IDI2MiA5OCAyMTEgNzQuNUMyMTEgNzQuNSAyMjMgNjIuNTAwMSAyODMuNSA1OS41QzM0NCA1Ni41IDM4OS42NzkgODUuNjM5IDM5MiAxNDAuNUMzOTQuMzIxIDE5NS4zNjEgMzk0IDI2NyAzOTQgMjk1QzM5NCAzMjMgNDA5LjUgMzUzIDQ5MyAzODMuNUM0OTMgMzgzLjUgNDkxLjYyOSAzODcuMzQ2IDQ4OSAzODlDMzk2LjUgMzg0IDMzMy4yNzMgMzQyLjUgMzI1LjUgMjMwWiIgZmlsbD0id2hpdGUiLz4KPC9zdmc+Cg==",
);

pub(super) fn chart_js() -> &'static str {
    include_str!("html_card/chart.umd.min.js")
}

pub(super) fn js_string_array(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string())
}

pub(super) fn js_number_array(values: &[f64]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string())
}

pub(super) fn parse_display_number(value: &str) -> Option<f64> {
    let trimmed = value
        .trim()
        .trim_start_matches('+')
        .trim_start_matches('#')
        .trim_end_matches('%')
        .replace(',', "");
    if trimmed.is_empty() {
        return None;
    }

    let (number, multiplier) = match trimmed.chars().last().map(|ch| ch.to_ascii_uppercase()) {
        Some('K') => (&trimmed[..trimmed.len() - 1], 1_000.0),
        Some('M') => (&trimmed[..trimmed.len() - 1], 1_000_000.0),
        Some('B') => (&trimmed[..trimmed.len() - 1], 1_000_000_000.0),
        _ => (trimmed.as_str(), 1.0),
    };

    number
        .trim()
        .parse::<f64>()
        .ok()
        .map(|value| value * multiplier)
}

fn brand_corner_css() -> &'static str {
    r#"
    .embed-brand-corner {
      display: inline-flex;
      align-items: center;
      justify-content: end;
      gap: 13px;
      min-width: 0;
      height: 70px;
      justify-self: end;
      transform: translateY(-8px);
      color: var(--text-primary);
      white-space: nowrap;
    }

    .embed-brand-mark {
      position: relative;
      display: grid;
      place-items: center;
      width: 68px;
      height: 68px;
      flex-shrink: 0;
      overflow: hidden;
      background: linear-gradient(45deg, #64b5f6, #81c784 54%, #ffb74d);
      -webkit-mask: var(--embed-brand-logo) center / contain no-repeat;
      mask: var(--embed-brand-logo) center / contain no-repeat;
    }

    .embed-brand-url {
      overflow: hidden;
      background: linear-gradient(45deg, #64b5f6, #81c784 54%, #ffb74d);
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      -webkit-text-fill-color: transparent;
      font-size: 32px;
      font-weight: 800;
      line-height: 1.05;
      text-overflow: ellipsis;
    }
"#
}

fn render_brand_corner() -> String {
    format!(
        r#"<div class="embed-brand-corner"><span class="embed-brand-mark" style="--embed-brand-logo:url({logo})"></span><span class="embed-brand-url">uma.moe</span></div>"#,
        logo = html_escape(BRAND_LOGO_DATA_URL)
    )
}

fn brand_logo_url(_meta: &EmbedMetadata) -> String {
    BRAND_LOGO_DATA_URL.to_string()
}

fn format_trainer_id_display(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }

    if !trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return trimmed.to_string();
    }

    let mut groups = Vec::new();
    let mut end = trimmed.len();
    while end > 0 {
        let start = end.saturating_sub(3);
        groups.push(&trimmed[start..end]);
        end = start;
    }
    groups.reverse();
    groups.join(" ")
}

fn display_trainer_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "Unknown Trainer".to_string();
    }

    if let Some((name, club)) = trainer_name_parts(trimmed) {
        let name = name.trim();
        let club = club.trim();
        if !name.is_empty() && !club.is_empty() {
            return format!("{club} | {name}");
        }
    }

    trimmed.to_string()
}

fn trainer_name_parts(value: &str) -> Option<(&str, &str)> {
    value
        .split_once('@')
        .or_else(|| value.split_once('\u{ff20}'))
}

fn format_number_grouped(value: i64, separator: char) -> String {
    let mut chars: Vec<char> = value.abs().to_string().chars().rev().collect();
    let mut formatted = String::new();

    for (index, ch) in chars.drain(..).enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(separator);
        }
        formatted.push(ch);
    }

    let formatted: String = formatted.chars().rev().collect();
    if value < 0 {
        format!("-{formatted}")
    } else {
        formatted
    }
}

fn format_decimal(value: f64, suffix: &str) -> String {
    let value = if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    };
    format!("{value:.2}{suffix}").replace('.', ",")
}

fn format_embed_date(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 10 && trimmed.as_bytes().get(4) == Some(&b'-') {
        return trimmed[..10].to_string();
    }

    truncate_chars(trimmed, 28)
}

fn chromium_binary() -> String {
    if let Ok(path) = env::var("UMAMOE_EMBEDS_CHROMIUM") {
        let path = path.trim();
        if !path.is_empty() {
            return path.to_string();
        }
    }

    for candidate in [
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
    ] {
        if Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }

    #[cfg(windows)]
    {
        let mut candidates = Vec::new();

        if let Ok(path) = env::var("ProgramFiles") {
            let root = PathBuf::from(path);
            candidates.push(root.join("Google\\Chrome\\Application\\chrome.exe"));
            candidates.push(root.join("Microsoft\\Edge\\Application\\msedge.exe"));
        }

        if let Ok(path) = env::var("ProgramFiles(x86)") {
            let root = PathBuf::from(path);
            candidates.push(root.join("Google\\Chrome\\Application\\chrome.exe"));
            candidates.push(root.join("Microsoft\\Edge\\Application\\msedge.exe"));
        }

        if let Ok(path) = env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(path).join("Google\\Chrome\\Application\\chrome.exe"));
        }

        for candidate in candidates {
            if candidate.exists() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }

    "chromium".to_string()
}

fn chromium_debug_port() -> Result<u16> {
    if let Ok(value) = env::var("UMAMOE_EMBEDS_CHROMIUM_DEBUG_PORT") {
        let value = value.trim();
        if !value.is_empty() {
            return value
                .parse::<u16>()
                .context("UMAMOE_EMBEDS_CHROMIUM_DEBUG_PORT must be a TCP port");
        }
    }

    let listener =
        TcpListener::bind(("127.0.0.1", 0)).context("failed to reserve a Chromium debug port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

fn chromium_startup_timeout() -> Duration {
    duration_from_env(
        "UMAMOE_EMBEDS_CHROMIUM_STARTUP_TIMEOUT_SECONDS",
        DEFAULT_CHROMIUM_STARTUP_TIMEOUT_SECONDS,
    )
}

fn chromium_render_timeout() -> Duration {
    duration_from_env(
        "UMAMOE_EMBEDS_CHROMIUM_RENDER_TIMEOUT_SECONDS",
        DEFAULT_CHROMIUM_RENDER_TIMEOUT_SECONDS,
    )
}

fn duration_from_env(name: &str, default_seconds: u64) -> Duration {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(default_seconds))
}

fn http_request(port: u16, method: &str, path: &str) -> Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .with_context(|| format!("failed to connect to Chromium DevTools on port {port}"))?;
    let render_timeout = chromium_render_timeout();
    stream.set_read_timeout(Some(render_timeout))?;
    stream.set_write_timeout(Some(render_timeout))?;

    let request = format!(
        "{method} {path} HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Connection: close\r\n\
         Content-Length: 0\r\n\
         \r\n"
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response = String::from_utf8_lossy(&response);
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow!("invalid Chromium DevTools HTTP response"))?;
    let status = headers.lines().next().unwrap_or_default();
    if !(status.starts_with("HTTP/1.1 2") || status.starts_with("HTTP/1.0 2")) {
        return Err(anyhow!("Chromium DevTools request failed: {status}"));
    }

    Ok(body.to_string())
}

fn read_http_headers(stream: &mut TcpStream) -> Result<String> {
    let mut response = Vec::new();
    let mut byte = [0_u8; 1];

    while !response.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte)?;
        response.push(byte[0]);
        if response.len() > 16 * 1024 {
            return Err(anyhow!(
                "Chromium websocket response headers were too large"
            ));
        }
    }

    String::from_utf8(response).context("Chromium websocket response headers were not UTF-8")
}

fn websocket_key() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = RENDER_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&(millis as u64).to_be_bytes());
    bytes[8..].copy_from_slice(&counter.to_be_bytes());
    BASE64.encode(bytes)
}

fn write_websocket_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<()> {
    let mut header = Vec::with_capacity(14);
    header.push(0x80 | (opcode & 0x0f));

    if payload.len() <= 125 {
        header.push(0x80 | payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        header.push(0x80 | 126);
        header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        header.push(0x80 | 127);
        header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }

    let mask = websocket_mask();
    header.extend_from_slice(&mask);
    stream.write_all(&header)?;

    let masked: Vec<u8> = payload
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ mask[index % mask.len()])
        .collect();
    stream.write_all(&masked)?;
    Ok(())
}

fn websocket_mask() -> [u8; 4] {
    let counter = RENDER_COUNTER.fetch_add(1, Ordering::Relaxed) as u32;
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or_default();
    (counter ^ millis).to_be_bytes()
}

fn read_websocket_text(stream: &mut TcpStream) -> Result<String> {
    let mut message = Vec::new();

    loop {
        let mut header = [0_u8; 2];
        stream.read_exact(&mut header)?;
        let fin = header[0] & 0x80 != 0;
        let opcode = header[0] & 0x0f;
        let masked = header[1] & 0x80 != 0;
        let mut length = (header[1] & 0x7f) as u64;

        if length == 126 {
            let mut bytes = [0_u8; 2];
            stream.read_exact(&mut bytes)?;
            length = u16::from_be_bytes(bytes) as u64;
        } else if length == 127 {
            let mut bytes = [0_u8; 8];
            stream.read_exact(&mut bytes)?;
            length = u64::from_be_bytes(bytes);
        }

        if length as usize > MAX_WEBSOCKET_MESSAGE_BYTES {
            return Err(anyhow!("Chromium websocket frame was too large"));
        }

        let mut mask = [0_u8; 4];
        if masked {
            stream.read_exact(&mut mask)?;
        }

        let mut payload = vec![0_u8; length as usize];
        stream.read_exact(&mut payload)?;
        if masked {
            for (index, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[index % mask.len()];
            }
        }

        match opcode {
            0x0 | 0x1 => {
                message.extend_from_slice(&payload);
                if message.len() > MAX_WEBSOCKET_MESSAGE_BYTES {
                    return Err(anyhow!("Chromium websocket message was too large"));
                }
                if fin {
                    return String::from_utf8(message)
                        .context("Chromium websocket text was not UTF-8");
                }
            }
            0x8 => return Err(anyhow!("Chromium websocket closed")),
            0x9 => {
                write_websocket_frame(stream, 0xA, &payload)?;
            }
            0xA => {}
            _ => {}
        }
    }
}

fn file_url(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        format!("file:///{path}")
    } else {
        format!("file://{path}")
    }
}

fn compact_url(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string()
}

fn canonical_path(url: &str) -> String {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path_start = after_scheme.find('/').unwrap_or(after_scheme.len());
    let path = &after_scheme[path_start..];
    let path = path
        .split(['?', '#'])
        .next()
        .filter(|path| !path.is_empty())
        .unwrap_or("/");

    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    truncated.push_str("...");
    truncated
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn unique_render_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let counter = RENDER_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}-{counter}", std::process::id(), millis)
}

struct TempRenderFiles {
    html_path: PathBuf,
    png_path: PathBuf,
    profile_dir: PathBuf,
    cache_dir: PathBuf,
}

impl TempRenderFiles {
    fn new() -> Result<Self> {
        let id = unique_render_id();
        let base = env::temp_dir();

        Ok(Self {
            html_path: base.join(format!("umamoe-embed-{id}.html")),
            png_path: base.join(format!("umamoe-embed-{id}.png")),
            profile_dir: base.join(format!("umamoe-embed-chrome-{id}")),
            cache_dir: base.join(format!("umamoe-embed-cache-{id}")),
        })
    }
}

impl Drop for TempRenderFiles {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.html_path);
        let _ = fs::remove_file(&self.png_path);
        let _ = fs::remove_dir_all(&self.profile_dir);
        let _ = fs::remove_dir_all(&self.cache_dir);
    }
}

struct Accent {
    hex: &'static str,
    soft: &'static str,
    faint: &'static str,
}

fn accent_for_kind(kind: &str) -> Accent {
    match kind.to_ascii_lowercase().as_str() {
        "home" | "uma.moe" => Accent {
            hex: "#64b5f6",
            soft: "rgba(100, 181, 246, 0.22)",
            faint: "rgba(100, 181, 246, 0.13)",
        },
        "club" | "clubs" => Accent {
            hex: "#81c784",
            soft: "rgba(129, 199, 132, 0.22)",
            faint: "rgba(129, 199, 132, 0.13)",
        },
        "profile" | "veterans" | "career menu" | "achievements" | "titles" => Accent {
            hex: "#64b5f6",
            soft: "rgba(100, 181, 246, 0.22)",
            faint: "rgba(100, 181, 246, 0.13)",
        },
        "rankings" | "activity" => Accent {
            hex: "#ffb74d",
            soft: "rgba(255, 183, 77, 0.22)",
            faint: "rgba(255, 183, 77, 0.13)",
        },
        "timeline" => Accent {
            hex: "#81c784",
            soft: "rgba(129, 199, 132, 0.20)",
            faint: "rgba(129, 199, 132, 0.13)",
        },
        "tierlist" => Accent {
            hex: "#ffd666",
            soft: "rgba(255, 214, 102, 0.20)",
            faint: "rgba(255, 214, 102, 0.13)",
        },
        "tools" => Accent {
            hex: "#ce93d8",
            soft: "rgba(206, 147, 216, 0.20)",
            faint: "rgba(206, 147, 216, 0.13)",
        },
        "database" | "statistics" | "lineage planner" => Accent {
            hex: "#4aa8ff",
            soft: "rgba(74, 168, 255, 0.22)",
            faint: "rgba(74, 168, 255, 0.13)",
        },
        _ => Accent {
            hex: "#4aa8ff",
            soft: "rgba(74, 168, 255, 0.22)",
            faint: "rgba(74, 168, 255, 0.13)",
        },
    }
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    use crate::embed::{DatabaseEmbedDetails, EmbedMetadata, EmbedMetric, ResourceCatalog};

    use super::*;

    fn overview_meta(kind_label: &str, canonical_url: &str) -> EmbedMetadata {
        EmbedMetadata {
            title: format!("{kind_label} | uma.moe"),
            description: format!("{kind_label} preview card."),
            canonical_url: canonical_url.to_string(),
            image_url: "https://uma.moe/__embeds/images/page/home.png".to_string(),
            image_alt: format!("{kind_label} image"),
            kind_label: kind_label.to_string(),
            metrics: vec![
                EmbedMetric {
                    label: "Rank".to_string(),
                    value: "#42".to_string(),
                },
                EmbedMetric {
                    label: "Fans".to_string(),
                    value: "1.2M".to_string(),
                },
            ],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        }
    }

    #[test]
    fn renders_self_contained_card_html() {
        let meta = EmbedMetadata {
            title: "Test Club | uma.moe".to_string(),
            description: "A generated preview card for a club link.".to_string(),
            canonical_url: "https://uma.moe/circles/772781438".to_string(),
            image_url: "https://uma.moe/__embeds/images/circle/772781438.png".to_string(),
            image_alt: "Test image".to_string(),
            kind_label: "Club".to_string(),
            metrics: vec![EmbedMetric {
                label: "Rank".to_string(),
                value: "#42".to_string(),
            }],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        };

        let html = render_card_html(&meta);
        assert!(html.contains(
            r#"<body class="embed-card-page embed-kind-club embed-type-club embed-route-club card-view-club">"#
        ));
        assert!(html.contains(r#"<main class="club-card embed-kind-club embed-type-club embed-route-club card-view-club">"#));
        assert!(html.contains("Club Information"));
        assert!(html.contains("Member Gains"));
        assert!(!html.contains("Club snapshot"));
        assert!(!html.contains("Recruitment"));
        assert!(!html.contains("record-card"));
        assert!(html.contains("Test Club"));
        assert!(html.contains("#42"));
    }

    #[test]
    fn renders_home_as_page_specific_card() {
        let html = render_card_html(&overview_meta("Home", "https://uma.moe/"));

        assert!(html.contains("home-card"));
        assert!(html.contains("embed-brand-corner"));
        assert!(html.contains("quick-links"));
        assert!(html.contains("stats-strip"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_clubs_as_page_specific_card() {
        let html = render_card_html(&overview_meta("Clubs", "https://uma.moe/circles"));

        assert!(html.contains("clubs-card"));
        assert!(html.contains("embed-brand-corner"));
        assert!(!html.contains("summary-strip"));
        assert!(html.contains("gains-row"));
        assert!(html.contains("Yesterday:"));
        assert!(html.contains("club-row rank-1"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_rankings_as_page_specific_card() {
        let html = render_card_html(&overview_meta("Rankings", "https://uma.moe/rankings"));

        assert!(html.contains("rankings-card"));
        assert!(html.contains("Trainer Rankings"));
        assert!(html.contains("embed-brand-corner"));
        assert!(html.contains("leader-head"));
        assert!(html.contains("lb-row top-1"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_rankings_query_state_labels() {
        let meta = EmbedMetadata {
            title: "Trainer Rankings | uma.moe".to_string(),
            description: "Global trainer fan rankings monthly, all-time, and recent gains."
                .to_string(),
            canonical_url: "https://uma.moe/rankings?tab=gains&sortBy=gain_7d".to_string(),
            image_url: "https://uma.moe/__embeds/images/page/rankings.png?tab=gains&sortBy=gain_7d"
                .to_string(),
            image_alt: "rankings".to_string(),
            kind_label: "Rankings".to_string(),
            metrics: vec![
                EmbedMetric {
                    label: "Tab".to_string(),
                    value: "Gains".to_string(),
                },
                EmbedMetric {
                    label: "Period".to_string(),
                    value: "7-Day Gain".to_string(),
                },
                EmbedMetric {
                    label: "Primary Label".to_string(),
                    value: "7d".to_string(),
                },
                EmbedMetric {
                    label: "Secondary Label".to_string(),
                    value: "3d".to_string(),
                },
                EmbedMetric {
                    label: "Tertiary Label".to_string(),
                    value: "30d".to_string(),
                },
            ],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        };
        let html = render_card_html(&meta);

        assert!(html.contains(
            r#"<div class="leader-head"><span>Rank</span><span>Trainer</span><span>Club</span><span>7d</span><span>3d</span><span>30d</span></div>"#
        ));
        assert!(html.contains("7-Day Gain"));
    }

    #[test]
    fn renders_activity_as_page_specific_card() {
        let html = render_card_html(&overview_meta("Activity", "https://uma.moe/activity"));

        assert!(html.contains("activity-card"));
        assert!(html.contains("Top 100 Club Activity Reports"));
        assert!(html.contains("embed-brand-corner"));
        assert!(html.contains("Limited snapshot reconstruction"));
        assert!(html.contains("Not proof of botting"));
        assert!(html.contains("activity-row score-high"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_activity_detail_as_page_specific_card() {
        let meta = EmbedMetadata {
            title: "Test Trainer Activity Report | uma.moe".to_string(),
            description: "Snapshot-based activity report.".to_string(),
            canonical_url: "https://uma.moe/activity/123456789".to_string(),
            image_url: "https://uma.moe/__embeds/images/activity/123456789.png".to_string(),
            image_alt: "activity detail".to_string(),
            kind_label: "Activity".to_string(),
            metrics: vec![
                EmbedMetric {
                    label: "Report Mode".to_string(),
                    value: "Detail".to_string(),
                },
                EmbedMetric {
                    label: "Trainer".to_string(),
                    value: "Test Trainer".to_string(),
                },
                EmbedMetric {
                    label: "Score".to_string(),
                    value: "87".to_string(),
                },
                EmbedMetric {
                    label: "Score Class".to_string(),
                    value: "score-high".to_string(),
                },
                EmbedMetric {
                    label: "Heatmap Pattern".to_string(),
                    value: "1234".repeat(42),
                },
            ],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        };
        let html = render_card_html(&meta);

        assert!(html.contains("detail-card"));
        assert!(html.contains("embed-brand-corner"));
        assert!(html.contains("Weekly activity heatmap"));
        assert!(html.contains("Daily fan gain"));
        assert!(html.contains("Test Trainer"));
        assert!(!html.contains("activity-list"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_timeline_as_page_specific_card() {
        let html = render_card_html(&overview_meta("Timeline", "https://uma.moe/timeline"));

        assert!(html.contains("timeline-card"));
        assert!(html.contains("embed-brand-corner"));
        assert!(html.contains("event-stage"));
        assert!(html.contains("inset: 0;"));
        assert!(html.contains("left: -2px;"));
        assert!(html.contains("background: transparent;"));
        assert!(html.contains("Taiki Shuttle + 1 more"));
        assert!(html.contains("Mejiro Dober"));
        assert!(html.contains("event-participant"));
        assert!(html.contains("images/character_stand/chara_stand_101002.webp"));
        assert!(html.contains("images/character_stand/chara_stand_105902.webp"));
        assert!(html.contains("images/support_card/half/support_card_s_30102.webp"));
        assert!(html.contains("images/character/banner/2022_30098.webp"));
        assert!(html.contains("Seek, Solve, Summer Walk!"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_tierlist_as_page_specific_card() {
        let html = render_card_html(&overview_meta("Tierlist", "https://uma.moe/tierlist"));

        assert!(html.contains("tierlist-card"));
        assert!(html.contains("Support Card Tierlist"));
        assert!(html.contains("embed-brand-corner"));
        assert!(html.contains("Kitasan Black"));
        assert!(html.contains("LB4 support cards"));
        assert!(html.contains("Zenno Rob Roy"));
        assert!(html.contains("Mejiro Dober"));
        assert!(html.contains("support_card_s_30065.webp"));
        assert!(!html.contains("Reference tierlist"));
        assert!(!html.contains("Scatter chart"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_tools_as_page_specific_card() {
        let html = render_card_html(&overview_meta("Tools", "https://uma.moe/tools"));

        assert!(html.contains("tools-card"));
        assert!(html.contains("Tools &amp; Calculators"));
        assert!(html.contains("Team Stadium Statistics"));
        assert!(html.contains("Stamina Calculator"));
        assert!(html.contains("Race Simulator"));
        assert!(html.contains("Lineage Planner"));
        assert!(html.contains("WIP"));
        assert!(html.contains("Coming soon"));
        assert!(html.contains("embed-brand-corner"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_statistics_as_page_specific_card() {
        let mut meta = overview_meta("Statistics", "https://uma.moe/tools/statistics");
        meta.metrics.extend([
            EmbedMetric {
                label: "Asset Base".to_string(),
                value: "https://uma.moe/assets".to_string(),
            },
            EmbedMetric {
                label: "Uma 1".to_string(),
                value: "Sakura Bakushin O".to_string(),
            },
            EmbedMetric {
                label: "Uma Value 1".to_string(),
                value: "7.14%".to_string(),
            },
            EmbedMetric {
                label: "Uma Id 1".to_string(),
                value: "104101".to_string(),
            },
            EmbedMetric {
                label: "Uma Detail 1".to_string(),
                value: "trained umas".to_string(),
            },
            EmbedMetric {
                label: "Uma Count 1".to_string(),
                value: "1.7M runs".to_string(),
            },
            EmbedMetric {
                label: "Support 1".to_string(),
                value: "Kitasan Black".to_string(),
            },
            EmbedMetric {
                label: "Support Value 1".to_string(),
                value: "64.21%".to_string(),
            },
            EmbedMetric {
                label: "Support Id 1".to_string(),
                value: "30028".to_string(),
            },
            EmbedMetric {
                label: "Support Detail 1".to_string(),
                value: "Speed support".to_string(),
            },
            EmbedMetric {
                label: "Support Count 1".to_string(),
                value: "15.3M".to_string(),
            },
            EmbedMetric {
                label: "Deck 1".to_string(),
                value: "4 SPD / 2 STA".to_string(),
            },
            EmbedMetric {
                label: "Deck Value 1".to_string(),
                value: "3.71%".to_string(),
            },
            EmbedMetric {
                label: "Deck Count 1".to_string(),
                value: "881.6K".to_string(),
            },
            EmbedMetric {
                label: "Deck 2".to_string(),
                value: "1 FRD / 1 GRP".to_string(),
            },
            EmbedMetric {
                label: "Deck Value 2".to_string(),
                value: "2.00%".to_string(),
            },
            EmbedMetric {
                label: "Deck Count 2".to_string(),
                value: "400K".to_string(),
            },
            EmbedMetric {
                label: "Scenario URA".to_string(),
                value: "51.0%".to_string(),
            },
            EmbedMetric {
                label: "Scenario Aoharu".to_string(),
                value: "26.1%".to_string(),
            },
            EmbedMetric {
                label: "Scenario MANT".to_string(),
                value: "22.9%".to_string(),
            },
        ]);
        let html = render_card_html(&meta);

        assert!(html.contains("statistics-card"));
        assert!(html.contains("Team Stadium Statistics"));
        assert!(html.contains("Team Class Split"));
        assert!(html.contains("Popular Deck Builds"));
        assert!(html.contains("deck-chip speed"));
        assert!(html.contains("images/icon/stats/speed.webp"));
        assert!(html.contains("images/icon/stats/friend.webp"));
        assert!(html.contains("images/icon/stats/group.webp"));
        assert!(html.contains("scenarioChart"));
        assert!(html.contains("statistics-rank-thumb") || html.contains("rank-thumb"));
        assert!(html.contains("images/character_stand/chara_stand_104101.webp"));
        assert!(html.contains("images/support_card/half/support_card_s_30028.webp"));
        assert!(html.contains("Speed support"));
        assert!(html.contains("embed-brand-corner"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_lineage_planner_as_page_specific_card() {
        let html = render_card_html(&overview_meta(
            "Lineage Planner",
            "https://uma.moe/tools/lineage-planner",
        ));

        assert!(html.contains("lineage-card"));
        assert!(html.contains("Inheritance tree"));
        assert!(html.contains("Spark Odds"));
        assert!(html.contains("combined"));
        assert!(html.contains(r#"<span class="affinity-badge">+72</span>"#));
        assert!(!html.contains(r#"<span class="node-role">P1</span>"#));
        assert!(html.contains("embed-brand-corner"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_lineage_planner_shared_tree_from_url_state() {
        let state = r#"{"v":1,"n":[{"p":0,"c":101401},{"p":1,"c":100101,"s":[103,3202],"r":[42]},{"p":3,"c":100301,"s":[203],"r":[42]}]}"#;
        let encoded = URL_SAFE_NO_PAD.encode(state);
        let html = render_card_html(&overview_meta(
            "Lineage Planner",
            &format!("https://uma.moe/tools/lineage-planner?tree={encoded}"),
        ));

        assert!(html.contains("Shared planner tree / decoded from URL"));
        assert!(html.contains("images/character_stand/chara_stand_100101.webp"));
        assert!(!html.contains("Uma #100101"));
        assert!(!html.contains("node-name"));
        assert!(html.contains("Speed"));
        assert!(html.contains("Mile"));
        assert!(html.contains("Nodes / sparks"));
        assert!(html.contains("Race affinity"));
    }

    #[test]
    fn renders_profile_as_page_specific_card() {
        let meta = EmbedMetadata {
            title: "Test Trainer | uma.moe".to_string(),
            description: "Profile page for Test Trainer. Club: Test Club. Total fans: 1,200,000."
                .to_string(),
            canonical_url: "https://uma.moe/profile/123456789".to_string(),
            image_url: "https://uma.moe/__embeds/images/profile/123456789.png".to_string(),
            image_alt: "profile preview".to_string(),
            kind_label: "Profile".to_string(),
            metrics: vec![
                EmbedMetric {
                    label: "Trainer".to_string(),
                    value: "Test Trainer".to_string(),
                },
                EmbedMetric {
                    label: "Trainer ID".to_string(),
                    value: "123456789".to_string(),
                },
                EmbedMetric {
                    label: "Leader Chara Dress".to_string(),
                    value: "#101301".to_string(),
                },
                EmbedMetric {
                    label: "Fans".to_string(),
                    value: "1.2M".to_string(),
                },
                EmbedMetric {
                    label: "7d Gain".to_string(),
                    value: "+42.0K".to_string(),
                },
                EmbedMetric {
                    label: "Club".to_string(),
                    value: "Test Club".to_string(),
                },
                EmbedMetric {
                    label: "Club Members".to_string(),
                    value: "29".to_string(),
                },
                EmbedMetric {
                    label: "Club Monthly Rank".to_string(),
                    value: "#1601".to_string(),
                },
                EmbedMetric {
                    label: "Club Fans".to_string(),
                    value: "130.7M".to_string(),
                },
                EmbedMetric {
                    label: "Club Tier".to_string(),
                    value: "B".to_string(),
                },
                EmbedMetric {
                    label: "Club Tier Id".to_string(),
                    value: "7".to_string(),
                },
                EmbedMetric {
                    label: "Spark Sums".to_string(),
                    value: "B7 P8 G4 W13".to_string(),
                },
                EmbedMetric {
                    label: "Stadium".to_string(),
                    value: "15 Umas".to_string(),
                },
                EmbedMetric {
                    label: "Team".to_string(),
                    value: "140.6K".to_string(),
                },
                EmbedMetric {
                    label: "Asset Base".to_string(),
                    value: "https://uma.moe/assets".to_string(),
                },
                EmbedMetric {
                    label: "Stadium Member 1 Character".to_string(),
                    value: "#100101".to_string(),
                },
                EmbedMetric {
                    label: "Stadium Member 1 Distance".to_string(),
                    value: "1".to_string(),
                },
                EmbedMetric {
                    label: "Stadium Member 1 Score".to_string(),
                    value: "11.1K".to_string(),
                },
                EmbedMetric {
                    label: "Stadium Member 1 Running Style".to_string(),
                    value: "2".to_string(),
                },
            ],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        };
        let html = render_card_html(&meta);

        assert!(html.contains("profile-card"));
        assert!(html.contains("images/character_stand/chara_stand_101301.webp"));
        assert!(html.contains(r#"<div class="profile-avatar-large"><img"#));
        assert!(!html.contains("data-fallback="));
        assert!(html.contains("Fan History"));
        assert!(html.contains("Current Circle"));
        assert!(html.contains("Inheritance"));
        assert!(html.contains("inheritance-head-totals"));
        assert!(
            html.contains(r#"inheritance-total inheritance-total-blue"><span>B</span><b>7</b>"#)
        );
        assert!(
            html.contains(r#"inheritance-total inheritance-total-pink"><span>P</span><b>8</b>"#)
        );
        assert!(
            html.contains(r#"inheritance-total inheritance-total-green"><span>G</span><b>4</b>"#)
        );
        assert!(
            html.contains(r#"inheritance-total inheritance-total-white"><span>W</span><b>13</b>"#)
        );
        assert!(html.contains("profile-borrow-display"));
        assert!(html.contains("inheritance-body"));
        assert!(html.contains("Team Stadium"));
        assert!(html.contains("circle-identity"));
        assert!(html.contains("circle-stat-row"));
        assert!(html.contains("profile-circle-rank-emblem"));
        assert!(html.contains("29/30"));
        assert!(html.contains("130.7M"));
        assert!(html.contains("rolling-stat-grid"));
        assert!(html.contains("historic-stat-grid"));
        assert!(html.contains("stadium-team-stack"));
        assert!(html.contains("stadium-distance-column"));
        assert!(html.contains("stadium-character-icon"));
        assert!(html.contains("stadium-rank-badge"));
        assert!(html.contains("stadium-runstyle-badge"));
        assert!(html.contains("images/character_stand/chara_stand_100101.webp"));
        assert!(html.contains("images/icon/ranks/"));
        assert!(html.contains("images/icon/common/utx_ico_runstyle_01.png"));
        assert!(!html.contains(r#"<span class="visibility-pill">Public</span>"#));
        assert!(html.contains("embed-brand-corner"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains("overview-body"));
    }

    #[test]
    fn renders_hidden_profile_fallback() {
        let meta = EmbedMetadata {
            title: "Hidden Trainer | uma.moe".to_string(),
            description: "Profile page for Hidden Trainer.".to_string(),
            canonical_url: "https://uma.moe/profile/987654321".to_string(),
            image_url: "https://uma.moe/__embeds/images/profile/987654321.png".to_string(),
            image_alt: "profile preview".to_string(),
            kind_label: "Profile".to_string(),
            metrics: vec![
                EmbedMetric {
                    label: "Trainer".to_string(),
                    value: "Hidden Trainer".to_string(),
                },
                EmbedMetric {
                    label: "Trainer ID".to_string(),
                    value: "987654321".to_string(),
                },
                EmbedMetric {
                    label: "Visibility".to_string(),
                    value: "Hidden".to_string(),
                },
            ],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        };
        let html = render_card_html(&meta);

        assert!(html.contains("profile-hidden-state"));
        assert!(html.contains("Profile Hidden"));
        assert!(html.contains("987654321"));
        assert!(!html.contains(r#"<section class="overview-card-grid""#));
        assert!(!html.contains(r#"<span class="visibility-pill">Public</span>"#));
    }

    #[test]
    fn renders_distinct_overview_card_views() {
        let cases = [(
            "uma.moe",
            "https://uma.moe/unknown",
            "card-view-page",
            "page-visual",
        )];

        for (kind_label, canonical_url, view_class, visual_class) in cases {
            let html = render_card_html(&overview_meta(kind_label, canonical_url));
            assert!(
                html.contains(view_class),
                "{kind_label} should render {view_class}"
            );
            assert!(
                html.contains(visual_class),
                "{kind_label} should render {visual_class}"
            );
            assert!(
                html.contains("overview-body"),
                "{kind_label} should use the overview body"
            );
        }
    }

    #[test]
    fn renders_database_default_as_full_card() {
        let html = render_card_html(&overview_meta("Database", "https://uma.moe/database"));

        assert!(html.contains(r#"<main class="database-card embed-kind-database embed-type-database embed-route-database card-view-database">"#));
        assert!(html.contains("database-default-content"));
        assert!(html.contains("database-default-dashboard"));
        assert!(html.contains("database-tool-board"));
        assert!(html.contains("database-mode-card"));
        assert!(html.contains("database-lineage-preview"));
        assert!(html.contains("embed-brand-corner"));
        assert!(!html.contains("overview-body"));
        assert!(!html.contains("record-card"));
        assert!(!html.contains(r#"<section class="inheritance-body""#));
    }

    #[test]
    fn renders_matched_database_spark_borders() {
        let meta = EmbedMetadata {
            title: "UUC | FishPineApl | uma.moe".to_string(),
            description: "Database preview".to_string(),
            canonical_url: "https://uma.moe/database?blue_sparks=103&white_sparks=2012701"
                .to_string(),
            image_url: "https://uma.moe/__embeds/images/database/query.png".to_string(),
            image_alt: "Database preview".to_string(),
            kind_label: "Database".to_string(),
            metrics: vec![],
            database: Some(DatabaseEmbedDetails {
                asset_base_url: "https://uma.moe/assets".to_string(),
                resources: ResourceCatalog::default(),
                query_label: "test".to_string(),
                result_total: 1,
                matched_factor_ids: vec![10, 201270],
                matched_main_factor_ids: vec![10],
                trainer_name: "UUC".to_string(),
                trainer_id: "540903147493".to_string(),
                record_id: Some(1),
                main_parent_id: Some(106801),
                parent_left_id: Some(101401),
                parent_right_id: Some(101502),
                parent_rank: Some(17012),
                parent_rarity: Some(10),
                affinity_score: Some(42),
                left_affinity_score: Some(8),
                right_affinity_score: Some(19),
                win_count: Some(14),
                white_count: Some(13),
                follower_num: None,
                support_card_id: Some(30036),
                limit_break_count: Some(1),
                last_updated: Some("2026-05-08".to_string()),
                blue_sparks: vec![103],
                pink_sparks: vec![],
                green_sparks: vec![],
                white_sparks: vec![2012701],
                main_blue_factors: Some(103),
                main_pink_factors: None,
                main_green_factors: None,
                main_white_factors: vec![2012701],
                left_blue_factors: None,
                left_pink_factors: None,
                left_green_factors: None,
                left_white_factors: vec![],
                right_blue_factors: None,
                right_pink_factors: None,
                right_green_factors: None,
                right_white_factors: vec![],
                main_win_saddles: vec![10, 20],
                left_win_saddles: vec![10],
                right_win_saddles: vec![20],
            }),
            tierlist: None,
            resources: ResourceCatalog::default(),
        };

        let html = render_card_html(&meta);
        assert!(html.contains(
            r#"<body class="embed-card-page embed-kind-database embed-type-database embed-route-database card-view-database">"#
        ));
        assert!(html.contains(r#"<main class="database-card embed-kind-database embed-type-database embed-route-database card-view-database">"#));
        assert!(html.contains(r#"<article class="database-result-card">"#));
        assert!(html.contains("embed-brand-corner"));
        assert!(html.contains(".spark-item.matched-filter"));
        assert!(html.contains("spark-item blue-spark matched-filter from-main-parent"));
        assert!(html.contains("spark-item white-spark matched-filter from-main-parent"));
        assert!(html.contains("parent-source"));
        assert!(html.contains("Factor 10"));
        assert!(html.contains("Factor 201270"));
    }
}
