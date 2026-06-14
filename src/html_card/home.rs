use crate::embed::{embed_class_list, EmbedMetadata};

use super::{html_escape, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-home",
    render_visual,
};

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none() && super::canonical_path(&meta.canonical_url) == "/"
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let title = html_escape(&truncate_chars(
        meta.title.strip_prefix("uma.moe - ").unwrap_or("uma.moe"),
        48,
    ));
    let subtitle = html_escape("Umamusume resource hub for the global version");
    let stats = render_stat_tiles(meta);
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
      --bg-secondary: #121212;
      --surface: rgba(255, 255, 255, 0.045);
      --surface-strong: rgba(255, 255, 255, 0.08);
      --border-primary: rgba(255, 255, 255, 0.12);
      --border-strong: rgba(255, 255, 255, 0.2);
      --text-primary: #ffffff;
      --text-secondary: rgba(255, 255, 255, 0.72);
      --text-muted: rgba(255, 255, 255, 0.5);
      --accent-primary: #64b5f6;
      --accent-secondary: #81c784;
      --accent-warning: #ffb74d;
      --color-pink: #e91e63;
      --color-purple: #ba68c8;
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

    body {{
      display: block;
    }}

    .home-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      overflow: hidden;
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.11), transparent 38%),
        linear-gradient(225deg, rgba(129, 199, 132, 0.095), transparent 42%),
        linear-gradient(180deg, rgba(255, 183, 77, 0.045), rgba(0, 0, 0, 0.18)),
        var(--bg-primary);
    }}

    .home-card::before {{
      content: "";
      position: absolute;
      inset: 0;
      background:
        linear-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.025) 1px, transparent 1px);
      background-size: 72px 72px;
      mask-image: linear-gradient(90deg, transparent, #000 18%, #000 82%, transparent);
      opacity: 0.45;
      pointer-events: none;
    }}

    .home-content {{
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-rows: 166px minmax(0, 1fr) 92px;
      width: 100%;
      height: 100%;
      padding: 36px 54px 28px;
      gap: 18px;
    }}

    .home-topbar {{
      display: grid;
      justify-items: center;
      align-content: center;
      gap: 16px;
      min-width: 0;
      text-align: center;
    }}

    .home-heading {{
      display: grid;
      gap: 6px;
      min-width: 0;
    }}

    .home-hero {{
      display: grid;
      grid-template-columns: minmax(0, 1fr);
      align-items: center;
      justify-items: center;
      min-height: 0;
    }}

    .brand-panel {{
      display: none;
      justify-items: center;
      align-content: center;
      gap: 18px;
      min-height: 0;
      text-align: center;
    }}

    .logo-orbit {{
      position: relative;
      display: grid;
      place-items: center;
      width: 184px;
      height: 184px;
    }}

    .logo-orbit::before,
    .logo-orbit::after {{
      content: "";
      position: absolute;
      inset: 0;
      border-radius: 50%;
      border: 1px solid rgba(255, 255, 255, 0.16);
    }}

    .logo-orbit::after {{
      inset: 18px;
      border-color: rgba(129, 199, 132, 0.26);
      box-shadow: 0 0 42px rgba(100, 181, 246, 0.18);
    }}

    .home-logo {{
      position: relative;
      z-index: 2;
      width: 150px;
      height: 150px;
      object-fit: contain;
      border-radius: 50%;
      filter: drop-shadow(0 10px 24px rgba(0, 0, 0, 0.42));
    }}

    .home-logo-fallback {{
      position: absolute;
      z-index: 1;
      display: grid;
      place-items: center;
      width: 128px;
      height: 128px;
      border: 1px solid rgba(255, 255, 255, 0.2);
      border-radius: 50%;
      background:
        radial-gradient(circle at 42% 30%, rgba(255, 255, 255, 0.18), transparent 18px),
        linear-gradient(145deg, rgba(100, 181, 246, 0.28), rgba(129, 199, 132, 0.18));
      color: #ffffff;
      font-size: 70px;
      font-weight: 900;
      line-height: 1;
      text-shadow: 0 8px 18px rgba(0, 0, 0, 0.35);
      box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.08),
        0 16px 34px rgba(0, 0, 0, 0.26);
    }}

    .home-title {{
      margin: 0;
      color: #ffffff;
      font-size: 38px;
      font-weight: 850;
      line-height: 1;
      letter-spacing: 0;
      text-shadow: 0 12px 32px rgba(0, 0, 0, 0.38);
    }}

    .home-title span {{
      background: linear-gradient(45deg, #64b5f6, #81c784 56%, #ffb74d);
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
    }}

    .home-subtitle {{
      max-width: 760px;
      margin: 0;
      color: var(--text-secondary);
      font-size: 18px;
      font-weight: 500;
      line-height: 1.3;
    }}

    .quick-links {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      width: 800px;
      max-width: 100%;
      gap: 14px 24px;
      min-width: 0;
    }}

    .quick-link {{
      position: relative;
      display: grid;
      grid-template-columns: 44px minmax(0, 1fr);
      align-items: center;
      min-height: 78px;
      gap: 20px;
      padding: 14px 28px;
      border: 1px solid var(--border-primary);
      border-radius: 8px;
      background: var(--surface);
      box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.035),
        0 12px 30px rgba(0, 0, 0, 0.18);
      overflow: hidden;
    }}

    .quick-link::before {{
      display: none;
    }}

    .quick-icon {{
      display: grid;
      place-items: center;
      width: 44px;
      height: 44px;
      color: var(--tile-accent);
      line-height: 1;
    }}

    .quick-icon svg {{
      width: 30px;
      height: 30px;
      display: block;
    }}

    .quick-copy {{
      display: grid;
      gap: 5px;
      min-width: 0;
    }}

    .quick-title {{
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      color: var(--text-primary);
      font-size: 19px;
      font-weight: 800;
      line-height: 1;
      white-space: nowrap;
    }}

    .quick-desc {{
      color: var(--text-secondary);
      font-size: 13px;
      font-weight: 500;
      line-height: 1.25;
    }}

    .badge {{
      display: inline-flex;
      align-items: center;
      height: 18px;
      padding: 0 6px;
      border-radius: 4px;
      background: linear-gradient(45deg, #2196f3, #81c784);
      color: #fff;
      font-size: 9px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
    }}

    .database {{
      --tile-accent: #64b5f6;
      --tile-glow: rgba(100, 181, 246, 0.45);
      border-color: rgba(100, 181, 246, 0.34);
      background: linear-gradient(135deg, rgba(33, 150, 243, 0.17), rgba(129, 199, 132, 0.08));
      box-shadow:
        inset 0 1px 0 rgba(255, 255, 255, 0.045),
        0 0 30px rgba(100, 181, 246, 0.18),
        0 12px 30px rgba(0, 0, 0, 0.2);
    }}

    .clubs {{
      --tile-accent: #81c784;
      --tile-glow: rgba(129, 199, 132, 0.42);
    }}

    .rankings {{
      --tile-accent: #ff8a65;
      --tile-glow: rgba(255, 138, 101, 0.42);
    }}

    .tierlists {{
      --tile-accent: #ffb74d;
      --tile-glow: rgba(255, 183, 77, 0.4);
    }}

    .timeline {{
      --tile-accent: #e91e63;
      --tile-glow: rgba(233, 30, 99, 0.38);
    }}

    .tools {{
      --tile-accent: #ba68c8;
      --tile-glow: rgba(186, 104, 200, 0.38);
    }}

    .stats-strip {{
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      justify-self: center;
      width: 900px;
      max-width: 100%;
      gap: 16px;
      min-height: 0;
    }}

    .stat-card {{
      display: grid;
      align-content: center;
      justify-items: center;
      gap: 9px;
      min-width: 0;
      border: 1px solid var(--border-primary);
      border-radius: 8px;
      background: rgba(8, 8, 8, 0.82);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.035);
      text-align: center;
    }}

    .stat-icon {{
      display: grid;
      place-items: center;
      width: 24px;
      height: 24px;
      color: var(--stat-accent);
      line-height: 1;
    }}

    .stat-icon svg {{
      width: 22px;
      height: 22px;
      display: block;
    }}

    .stat-value {{
      max-width: 210px;
      overflow: hidden;
      color: var(--text-primary);
      font-size: 21px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .stat-label {{
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 800;
      line-height: 1;
      text-transform: uppercase;
    }}

    .home-footer {{
      position: absolute;
      right: 34px;
      bottom: 16px;
      display: inline-flex;
      align-items: center;
      gap: 8px;
      color: rgba(255, 255, 255, 0.42);
      font-size: 12px;
      font-weight: 750;
    }}

    .home-footer::before {{
      content: "";
      width: 7px;
      height: 7px;
      border-radius: 50%;
      background: var(--accent-secondary);
      box-shadow: 0 0 10px rgba(129, 199, 132, 0.5);
    }}
{brand_css}
    .home-card .embed-brand-corner {{
      justify-self: center;
      justify-content: center;
      gap: 32px;
      height: 118px;
      transform: none;
    }}

    .home-card .embed-brand-mark {{
      width: 118px;
      height: 118px;
      filter: drop-shadow(0 16px 34px rgba(0, 0, 0, 0.42));
    }}

    .home-card .embed-brand-url {{
      font-size: 72px;
      font-weight: 850;
      line-height: 1;
    }}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-home">
  <main class="home-card {class_list} card-view-home">
    <div class="home-content">
      <header class="home-topbar">
        {brand}
        <p class="home-subtitle">{subtitle}</p>
      </header>

      <section class="home-hero">
        <div class="quick-links" aria-hidden="true">
          {quick_links}
        </div>
      </section>

      <section class="stats-strip" aria-hidden="true">
        {stats}
      </section>
    </div>
  </main>
</body>
</html>
"#,
        class_list = class_list,
        brand_css = brand_css,
        brand = brand,
        title = title,
        subtitle = subtitle,
        quick_links = render_quick_links(),
        stats = stats,
    )
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    r#"<div class="visual-panel home-visual">
        <div class="home-hero-mini">
          <strong>uma.moe</strong>
          <span>Umamusume resource hub</span>
        </div>
        <div class="home-quick-grid">
          <span class="home-quick-link ql-database"><b>Database</b><small>Inheritance</small></span>
          <span class="home-quick-link ql-clubs"><b>Clubs</b><small>Fan progress</small></span>
          <span class="home-quick-link ql-rankings"><b>Rankings</b><small>Global fans</small></span>
          <span class="home-quick-link ql-tierlist"><b>Tierlist</b><small>Support cards</small></span>
          <span class="home-quick-link ql-timeline"><b>Timeline</b><small>Schedule</small></span>
          <span class="home-quick-link ql-tools"><b>Tools</b><small>Planning</small></span>
        </div>
        <div class="home-stats-mini">
          <span><b>Tasks</b><small>Today</small></span>
          <span><b>Updates</b><small>Live</small></span>
          <span><b>Umas</b><small>Tracked</small></span>
        </div>
      </div>"#
        .to_string()
}

fn render_quick_links() -> String {
    [
        (
            "database",
            "dataset",
            "Database",
            Some("Updated"),
            "Browse the Database",
        ),
        (
            "clubs",
            "groups",
            "Clubs",
            None,
            "Club Database and Fan Progression",
        ),
        (
            "rankings",
            "trending_up",
            "Rankings",
            None,
            "Global Trainer Fan Rankings",
        ),
        (
            "tierlists",
            "leaderboard",
            "Tierlists",
            None,
            "Support Card Tierlist",
        ),
        (
            "timeline",
            "schedule",
            "Timeline",
            None,
            "Estimated Release Schedule",
        ),
        (
            "tools",
            "build",
            "Tools & Analytics",
            None,
            "Advanced tools and statistics",
        ),
    ]
    .into_iter()
    .map(|(class_name, icon_name, title, badge, description)| {
        let badge = badge.map_or_else(String::new, |label| {
            format!(r#"<span class="badge">{}</span>"#, html_escape(label))
        });
        let icon = material_icon_svg(icon_name);
        format!(
            r#"<div class="quick-link {class_name}">
            <span class="quick-icon">{icon}</span>
            <span class="quick-copy"><span class="quick-title">{}{badge}</span><span class="quick-desc">{}</span></span>
          </div>"#,
            html_escape(title),
            html_escape(description),
        )
    })
    .collect::<Vec<_>>()
    .join("")
}

fn render_stat_tiles(meta: &EmbedMetadata) -> String {
    let stat_defs = [
        ("task_alt", "Tasks Today", "Tasks Today"),
        ("sync", "Updated Today", "Updated Today"),
        ("group", "Active 7d", "Active (7d)"),
        ("face", "Umas Tracked", "Total Umas Tracked"),
    ];

    stat_defs
        .into_iter()
        .enumerate()
        .map(|(index, (icon_name, lookup_label, display_label))| {
            let value = meta
                .metrics
                .iter()
                .find(|metric| metric.label.eq_ignore_ascii_case(lookup_label))
                .map(|metric| metric.value.as_str())
                .unwrap_or("uma.moe");
            let accent = match index {
                0 => "#64b5f6",
                1 => "#81c784",
                2 => "#ffb74d",
                _ => "#e91e63",
            };

            format!(
                r#"<div class="stat-card" style="--stat-accent: {accent}">
          <span class="stat-icon">{}</span>
          <span class="stat-value">{}</span>
          <span class="stat-label">{}</span>
        </div>"#,
                material_icon_svg(icon_name),
                html_escape(&truncate_chars(value, 18)),
                html_escape(display_label),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn material_icon_svg(name: &str) -> &'static str {
    match name {
        "dataset" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2Zm0 2v3h4V6H4Zm6 0v3h4V6h-4Zm6 0v3h4V6h-4ZM4 11v3h4v-3H4Zm6 0v3h4v-3h-4Zm6 0v3h4v-3h-4ZM4 16v2h4v-2H4Zm6 0v2h4v-2h-4Zm6 0v2h4v-2h-4Z"/></svg>"#
        }
        "groups" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5s-3 1.34-3 3 1.34 3 3 3Zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5 5 6.34 5 8s1.34 3 3 3Zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5C15 14.17 10.33 13 8 13Zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.98 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5Z"/></svg>"#
        }
        "trending_up" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="m16 6 2.29 2.29-4.88 4.88-4-4L2 16.59 3.41 18l6-6 4 4L19.7 9.71 22 12V6h-6Z"/></svg>"#
        }
        "leaderboard" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M7.5 21h-4c-.83 0-1.5-.67-1.5-1.5v-7c0-.83.67-1.5 1.5-1.5h4c.83 0 1.5.67 1.5 1.5v7c0 .83-.67 1.5-1.5 1.5Zm13 0h-4c-.83 0-1.5-.67-1.5-1.5v-13c0-.83.67-1.5 1.5-1.5h4c.83 0 1.5.67 1.5 1.5v13c0 .83-.67 1.5-1.5 1.5Zm-6.5 0h-4c-.83 0-1.5-.67-1.5-1.5v-16c0-.83.67-1.5 1.5-1.5h4c.83 0 1.5.67 1.5 1.5v16c0 .83-.67 1.5-1.5 1.5Z"/></svg>"#
        }
        "schedule" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M11.99 2C6.48 2 2 6.48 2 12s4.48 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2ZM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8Zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67V7Z"/></svg>"#
        }
        "build" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="m22.7 19-9.1-9.1c.9-2.3.4-5-1.5-6.9-2-2-5-2.4-7.4-1.3l4.1 4.1-3 3-4.2-4.1C.5 7.1.9 10.1 2.9 12.1c1.9 1.9 4.6 2.4 6.9 1.5l9.1 9.1c.4.4 1 .4 1.4 0l2.3-2.3c.5-.4.5-1.1.1-1.4Z"/></svg>"#
        }
        "task_alt" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M22 5.18 10.59 16.6l-4.24-4.24 1.41-1.41 2.83 2.83L20.59 3.77 22 5.18ZM19.79 10.22c.14.57.21 1.16.21 1.78 0 4.42-3.58 8-8 8s-8-3.58-8-8 3.58-8 8-8c1.58 0 3.04.46 4.28 1.25l1.44-1.44C16.1 2.67 14.13 2 12 2 6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10c0-1.19-.22-2.33-.6-3.39l-1.61 1.61Z"/></svg>"#
        }
        "sync" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.96-.7 2.8l1.46 1.46C19.54 15.03 20 13.57 20 12c0-4.42-3.58-8-8-8Zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.96.7-2.8L5.24 7.74C4.46 8.97 4 10.43 4 12c0 4.42 3.58 8 8 8v3l4-4-4-4v3Z"/></svg>"#
        }
        "group" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5s-3 1.34-3 3 1.34 3 3 3ZM8 11c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5 5 6.34 5 8s1.34 3 3 3Zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5C15 14.17 10.33 13 8 13Zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.98 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5Z"/></svg>"#
        }
        "face" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M9 11.75c.69 0 1.25-.56 1.25-1.25S9.69 9.25 9 9.25s-1.25.56-1.25 1.25S8.31 11.75 9 11.75Zm6 0c.69 0 1.25-.56 1.25-1.25S15.69 9.25 15 9.25s-1.25.56-1.25 1.25.56 1.25 1.25 1.25ZM12 17.5c2.33 0 4.31-1.46 5.11-3.5H6.89c.8 2.04 2.78 3.5 5.11 3.5ZM12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2Zm0 18c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8Z"/></svg>"#
        }
        _ => "",
    }
}
