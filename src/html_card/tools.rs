use crate::embed::{embed_class_list, EmbedMetadata};

use super::{asset_url, html_escape, metric_value, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-tools",
    render_visual,
};

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none() && super::canonical_path(&meta.canonical_url) == "/tools"
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let title_text = match super::display_title(&meta.title).as_str() {
        "Tools" => "Tools & Calculators".to_string(),
        title => title.to_string(),
    };
    let title = html_escape(&truncate_chars(&title_text, 44));
    let asset_base = metric_value(&meta.metrics, &["Asset Base"])
        .unwrap_or_else(|| "https://uma.moe/assets".to_string());
    let tools = render_tools_grid(&asset_base);
    let brand = super::render_brand_corner();
    let brand_css = super::brand_corner_css();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=1200, initial-scale=1">
  <title>{title}</title>
  <style>
    :root {{
      --bg-primary: #0a0a0a;
      --surface-1: rgba(255, 255, 255, 0.026);
      --surface-2: rgba(255, 255, 255, 0.052);
      --surface-3: rgba(255, 255, 255, 0.08);
      --border-subtle: rgba(255, 255, 255, 0.07);
      --border-primary: rgba(255, 255, 255, 0.13);
      --text-primary: #ffffff;
      --text-secondary: rgba(255, 255, 255, 0.72);
      --text-muted: rgba(255, 255, 255, 0.52);
      --text-disabled: rgba(255, 255, 255, 0.36);
      --accent-primary: #64b5f6;
      --accent-secondary: #81c784;
      --accent-warning: #ffb74d;
      --accent-pink: #f06292;
      --accent-purple: #ba68c8;
      --accent-teal: #26a69a;
      color-scheme: dark;
      font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: var(--bg-primary);
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
      background: var(--bg-primary);
    }}

    .tools-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 88px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 18% 14%, rgba(186, 104, 200, 0.12), transparent 330px),
        radial-gradient(circle at 80% 12%, rgba(100, 181, 246, 0.11), transparent 330px),
        radial-gradient(circle at 48% 90%, rgba(255, 183, 77, 0.07), transparent 360px),
        var(--bg-primary);
    }}

    .tools-card::before {{
      content: "";
      position: absolute;
      inset: 88px 0 0;
      background:
        linear-gradient(rgba(255, 255, 255, 0.022) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.016) 1px, transparent 1px);
      background-size: 64px 64px;
      opacity: 0.42;
      pointer-events: none;
    }}

    .tools-header {{
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 32px;
      align-items: center;
      min-width: 0;
      padding: 13px 48px 10px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(186, 104, 200, 0.08), rgba(100, 181, 246, 0.055)),
        rgba(255, 255, 255, 0.012);
    }}

    .header-copy {{
      display: grid;
      gap: 7px;
      min-width: 0;
    }}

    .tools-title {{
      margin: 0;
      background: linear-gradient(45deg, var(--accent-purple), var(--accent-primary) 55%, var(--accent-warning));
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      font-size: 38px;
      font-weight: 880;
      letter-spacing: 0;
      line-height: 0.98;
    }}

    .tools-subline {{
      margin: 0;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }}

    .tools-content {{
      position: relative;
      z-index: 1;
      min-height: 0;
      padding: 18px 42px;
    }}

    .tool-grid {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      grid-template-rows: repeat(2, minmax(0, 1fr));
      gap: 12px;
      width: 100%;
      height: 100%;
      min-height: 0;
    }}

    .tool-tile {{
      --tile-color: var(--accent-primary);
      --tile-border: rgba(100, 181, 246, 0.26);
      --tile-glow: rgba(100, 181, 246, 0.075);
      position: relative;
      display: grid;
      grid-template-columns: 185px minmax(0, 1fr);
      gap: 18px;
      align-items: center;
      min-width: 0;
      min-height: 0;
      overflow: hidden;
      padding: 18px;
      border: 1px solid var(--tile-border);
      border-radius: 8px;
      background:
        linear-gradient(135deg, var(--tile-glow), rgba(255, 255, 255, 0.015)),
        rgba(255, 255, 255, 0.026);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.045);
    }}

    .tool-tile.statistics {{
      --tile-color: var(--accent-primary);
      --tile-border: rgba(100, 181, 246, 0.32);
      --tile-glow: rgba(100, 181, 246, 0.11);
    }}

    .tool-tile.stamina {{
      --tile-color: var(--accent-warning);
      --tile-border: rgba(255, 183, 77, 0.23);
      --tile-glow: rgba(255, 183, 77, 0.07);
    }}

    .tool-tile.race {{
      --tile-color: var(--accent-pink);
      --tile-border: rgba(240, 98, 146, 0.22);
      --tile-glow: rgba(240, 98, 146, 0.066);
    }}

    .tool-tile.lineage {{
      --tile-color: var(--accent-secondary);
      --tile-border: rgba(129, 199, 132, 0.34);
      --tile-glow: rgba(129, 199, 132, 0.1);
    }}

    .tool-tile.wip {{
      filter: saturate(0.72);
    }}

    .tool-art {{
      display: grid;
      place-items: center;
      width: 185px;
      height: 185px;
      min-width: 0;
      overflow: hidden;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 8px;
      background:
        radial-gradient(circle at 44% 32%, color-mix(in srgb, var(--tile-color) 20%, transparent), transparent 48px),
        rgba(0, 0, 0, 0.24);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
    }}

    .tool-copy {{
      display: grid;
      gap: 11px;
      align-content: center;
      min-width: 0;
    }}

    .tool-head {{
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
    }}

    .status-pill {{
      display: inline-grid;
      place-items: center;
      height: 23px;
      padding: 0 9px;
      border-radius: 999px;
      color: var(--tile-color);
      background: color-mix(in srgb, var(--tile-color) 18%, transparent);
      border: 1px solid color-mix(in srgb, var(--tile-color) 46%, transparent);
      font-size: 10px;
      font-weight: 920;
      line-height: 1;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .status-pill.wip {{
      color: var(--accent-warning);
      background: rgba(255, 183, 77, 0.12);
      border-color: rgba(255, 183, 77, 0.42);
    }}

    .tool-title {{
      margin: 0;
      color: var(--text-primary);
      font-size: 25px;
      font-weight: 880;
      line-height: 1.05;
      display: -webkit-box;
      overflow: hidden;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 2;
    }}

    .tool-desc {{
      display: -webkit-box;
      overflow: hidden;
      max-width: 315px;
      margin: 0;
      color: var(--text-secondary);
      font-size: 13px;
      font-weight: 650;
      line-height: 1.28;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 2;
    }}

    .tool-tags {{
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
      min-width: 0;
    }}

    .tool-tags span {{
      display: inline-grid;
      place-items: center;
      height: 22px;
      padding: 0 8px;
      border-radius: 6px;
      background: rgba(255, 255, 255, 0.055);
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .stat-board {{
      display: grid;
      width: 150px;
      gap: 7px;
      align-content: center;
    }}

    .stat-icons {{
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 6px;
    }}

    .stat-icons img,
    .race-icons img {{
      width: 39px;
      height: 39px;
      object-fit: contain;
      display: block;
      filter: drop-shadow(0 3px 8px rgba(0, 0, 0, 0.5));
    }}

    .mini-bars {{
      display: grid;
      gap: 5px;
      padding-top: 3px;
    }}

    .mini-bars span {{
      position: relative;
      height: 8px;
      overflow: hidden;
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.08);
    }}

    .mini-bars span::before {{
      content: "";
      position: absolute;
      inset: 0 auto 0 0;
      width: var(--w);
      border-radius: inherit;
      background: linear-gradient(90deg, var(--accent-primary), var(--accent-teal));
    }}

    .stamina-art {{
      display: grid;
      justify-items: center;
      gap: 14px;
    }}

    .stamina-art img {{
      width: 70px;
      height: 70px;
      object-fit: contain;
      display: block;
      filter: drop-shadow(0 6px 16px rgba(255, 183, 77, 0.22));
    }}

    .stamina-calc {{
      display: grid;
      grid-template-columns: repeat(3, 42px);
      gap: 6px;
    }}

    .stamina-calc span,
    .race-lane span {{
      display: inline-grid;
      place-items: center;
      height: 24px;
      border-radius: 6px;
      color: var(--text-secondary);
      background: rgba(255, 255, 255, 0.06);
      font-size: 10px;
      font-weight: 920;
      line-height: 1;
      text-transform: uppercase;
    }}

    .race-art {{
      display: grid;
      align-content: center;
      justify-items: center;
      gap: 12px;
      width: 150px;
    }}

    .race-icons {{
      display: flex;
      align-items: center;
      gap: 8px;
    }}

    .race-lane {{
      position: relative;
      display: grid;
      grid-template-columns: repeat(3, 42px);
      gap: 6px;
      width: 140px;
      padding-top: 16px;
    }}

    .race-lane::before {{
      content: "";
      position: absolute;
      left: 4px;
      right: 4px;
      top: 5px;
      height: 2px;
      border-radius: 999px;
      background: linear-gradient(90deg, var(--accent-pink), var(--accent-warning));
      opacity: 0.72;
    }}

    .lineage-art {{
      position: relative;
      width: 160px;
      height: 160px;
    }}

    .lineage-art svg {{
      position: absolute;
      inset: 50px 24px 25px;
      width: calc(100% - 48px);
      height: 85px;
      overflow: visible;
    }}

    .lineage-art path {{
      fill: none;
      stroke: rgba(255, 255, 255, 0.48);
      stroke-width: 2;
    }}

    .lineage-node {{
      position: absolute;
      display: grid;
      justify-items: center;
      gap: 6px;
      width: 58px;
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
    }}

    .lineage-node.main {{
      top: 1px;
      left: 51px;
      color: var(--accent-primary);
    }}

    .lineage-node.left {{
      left: 15px;
      bottom: 0;
    }}

    .lineage-node.right {{
      right: 15px;
      bottom: 0;
    }}

    .lineage-portrait {{
      position: relative;
      display: grid;
      place-items: center;
      width: 52px;
      height: 52px;
    }}

    .lineage-node.main .lineage-portrait {{
      width: 62px;
      height: 62px;
    }}

    .lineage-portrait img {{
      width: 52px;
      height: 52px;
      object-fit: contain;
      display: block;
      border: 2px solid var(--node-border);
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.045);
      filter: drop-shadow(0 8px 14px rgba(0, 0, 0, 0.42));
    }}

    .lineage-node.main .lineage-portrait img {{
      width: 62px;
      height: 62px;
      --node-border: rgba(100, 181, 246, 0.78);
    }}

    .lineage-node.left .lineage-portrait img {{
      --node-border: rgba(240, 98, 146, 0.74);
    }}

    .lineage-node.right .lineage-portrait img {{
      --node-border: rgba(186, 104, 200, 0.72);
    }}

    .affinity-badge {{
      position: absolute;
      left: 50%;
      bottom: -5px;
      transform: translateX(-50%);
      display: inline-grid;
      place-items: center;
      height: 15px;
      min-width: 30px;
      padding: 0 5px;
      border-radius: 999px;
      color: var(--node-color);
      background:
        linear-gradient(rgba(0, 0, 0, 0.28), rgba(0, 0, 0, 0.28)),
        color-mix(in srgb, var(--node-color) 28%, #111);
      border: 1px solid color-mix(in srgb, var(--node-color) 46%, transparent);
      font-size: 9px;
      font-style: normal;
      font-weight: 950;
      line-height: 1;
      font-variant-numeric: tabular-nums;
      text-transform: none;
    }}
{brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-tools">
  <main class="tools-card {class_list} card-view-tools">
    <header class="tools-header">
      <div class="header-copy">
        <h1 class="tools-title">{title}</h1>
        <p class="tools-subline">Calculation tools and utilities for Umamusume trainers</p>
      </div>
      {brand}
    </header>

    <section class="tools-content">
      <div class="tool-grid">
        {tools}
      </div>
    </section>
  </main>
</body>
</html>
"#,
        class_list = class_list,
        title = title,
        brand = brand,
        brand_css = brand_css,
        tools = tools,
    )
}

fn render_tools_grid(asset_base: &str) -> String {
    [
        render_tool_tile(
            "statistics",
            "Team Stadium Statistics",
            "Interactive charts and aggregate training data for Team Stadium.",
            "Live",
            false,
            &["Statistics", "Decks", "Usage"],
            render_statistics_art(asset_base),
        ),
        render_tool_tile(
            "stamina wip",
            "Stamina Calculator",
            "Calculate race stamina requirements from distance, style, and scenario context.",
            "WIP",
            true,
            &["Coming soon", "Race stats"],
            render_stamina_art(asset_base),
        ),
        render_tool_tile(
            "race wip",
            "Race Simulator",
            "Simulate race outcomes and compare performance assumptions.",
            "WIP",
            true,
            &["Coming soon", "Simulation"],
            render_race_art(asset_base),
        ),
        render_tool_tile(
            "lineage",
            "Lineage Planner",
            "Plan inheritance trees across parents, sparks, and affinity.",
            "New",
            false,
            &["Planner", "Affinity", "Sparks"],
            render_lineage_art(asset_base),
        ),
    ]
    .join("")
}

fn render_tool_tile(
    class_name: &str,
    title: &str,
    description: &str,
    status: &str,
    wip: bool,
    tags: &[&str],
    art: String,
) -> String {
    let status_class = if wip {
        "status-pill wip"
    } else {
        "status-pill"
    };
    let tags = tags
        .iter()
        .map(|tag| format!(r#"<span>{}</span>"#, html_escape(tag)))
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<article class="tool-tile {class_name}">
          <div class="tool-art">{art}</div>
          <div class="tool-copy">
            <div class="tool-head"><span class="{status_class}">{status}</span></div>
            <h2 class="tool-title">{title}</h2>
            <p class="tool-desc">{description}</p>
            <div class="tool-tags">{tags}</div>
          </div>
        </article>"#,
        class_name = html_escape(class_name),
        art = art,
        status_class = status_class,
        status = html_escape(status),
        title = html_escape(title),
        description = html_escape(description),
        tags = tags,
    )
}

fn render_statistics_art(asset_base: &str) -> String {
    let icons = ["speed", "stamina", "power", "guts", "wit", "friend"]
        .into_iter()
        .map(|stat| stat_icon(asset_base, stat))
        .collect::<Vec<_>>()
        .join("");

    r#"<div class="stat-board">
      <div class="stat-icons">"#
        .to_string()
        + &icons
        + r#"</div>
      <div class="mini-bars"><span style="--w:92%"></span><span style="--w:68%"></span><span style="--w:54%"></span></div>
    </div>"#
}

fn render_stamina_art(asset_base: &str) -> String {
    format!(
        r#"<div class="stamina-art">
          <img src="{stamina}" alt="">
          <div class="stamina-calc"><span>Mile</span><span>Med</span><span>Long</span></div>
        </div>"#,
        stamina = html_escape(&stat_icon_url(asset_base, "stamina")),
    )
}

fn render_race_art(asset_base: &str) -> String {
    format!(
        r#"<div class="race-art">
          <div class="race-icons">{speed}{power}{guts}</div>
          <div class="race-lane"><span>Turf</span><span>Dirt</span><span>Final</span></div>
        </div>"#,
        speed = stat_icon(asset_base, "speed"),
        power = stat_icon(asset_base, "power"),
        guts = stat_icon(asset_base, "guts"),
    )
}

fn render_lineage_art(asset_base: &str) -> String {
    let main = character_image(asset_base, "101002");
    let left = character_image(asset_base, "105902");
    let right = character_image(asset_base, "103601");

    format!(
        r#"<div class="lineage-art">
          <svg viewBox="0 0 160 100" preserveAspectRatio="none" aria-hidden="true">
            <path d="M80 16 L80 45 M30 45 L130 45 M30 45 L30 96 M130 45 L130 96" vector-effect="non-scaling-stroke" />
          </svg>
          <span class="lineage-node main" style="--node-color: var(--accent-primary)"><span class="lineage-portrait"><img src="{main}" alt=""><em class="affinity-badge">+45</em></span><b>Main</b></span>
          <span class="lineage-node left" style="--node-color: var(--accent-pink)"><span class="lineage-portrait"><img src="{left}" alt=""><em class="affinity-badge">+19</em></span><b>GP</b></span>
          <span class="lineage-node right" style="--node-color: var(--accent-purple)"><span class="lineage-portrait"><img src="{right}" alt=""><em class="affinity-badge">+26</em></span><b>GP</b></span>
        </div>"#,
        main = html_escape(&main),
        left = html_escape(&left),
        right = html_escape(&right),
    )
}

fn stat_icon(asset_base: &str, stat: &str) -> String {
    format!(
        r#"<img src="{url}" alt="" onerror="this.style.visibility='hidden'">"#,
        url = html_escape(&stat_icon_url(asset_base, stat)),
    )
}

fn stat_icon_url(asset_base: &str, stat: &str) -> String {
    asset_url(asset_base, &format!("/images/icon/stats/{stat}.webp"))
}

fn character_image(asset_base: &str, id: &str) -> String {
    asset_url(
        asset_base,
        &format!("/images/character_stand/chara_stand_{id}.webp"),
    )
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    let asset_base = "https://uma.moe/assets";
    let speed = html_escape(&asset_url(asset_base, "/images/icon/stats/speed.webp"));
    let stamina = html_escape(&asset_url(asset_base, "/images/icon/stats/stamina.webp"));
    let power = html_escape(&asset_url(asset_base, "/images/icon/stats/power.webp"));
    let main = html_escape(&asset_url(
        asset_base,
        "/images/character_stand/chara_stand_101002.webp",
    ));

    format!(
        r#"<div class="visual-panel tools-visual">
        <div class="tools-grid-mini">
          <span class="tool-card-mini tool-statistics"><b>Statistics</b><small><img src="{speed}" alt=""><img src="{stamina}" alt=""><img src="{power}" alt=""></small></span>
          <span class="tool-card-mini tool-lineage"><b>Lineage Planner</b><small><img src="{main}" alt=""></small></span>
        </div>
      </div>"#
    )
}
