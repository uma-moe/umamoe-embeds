use crate::embed::{embed_class_list, EmbedMetadata};

use super::{asset_url, display_title, html_escape, metric_value, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-rankings",
    render_visual,
};

struct RankingRow {
    rank: String,
    trainer: String,
    club: String,
    club_rank_id: Option<String>,
    primary_value: String,
    secondary_value: String,
    tertiary_value: String,
}

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none() && super::canonical_path_matches(&meta.canonical_url, "/rankings")
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let title_text = match display_title(&meta.title).as_str() {
        "Rankings" => "Trainer Rankings".to_string(),
        title => title.to_string(),
    };
    let title = html_escape(&truncate_chars(&title_text, 42));
    let tab = ranking_metric(meta, "Tab").unwrap_or_else(|| "Monthly".to_string());
    let period = ranking_metric(meta, "Period")
        .or_else(|| ranking_metric(meta, "Sort"))
        .unwrap_or_else(|| default_period(&tab).to_string());
    let total = ranking_metric(meta, "Total").unwrap_or_else(|| "Live".to_string());
    let primary_label = ranking_metric(meta, "Primary Label")
        .unwrap_or_else(|| default_primary_label(&tab).to_string());
    let secondary_label = ranking_metric(meta, "Secondary Label")
        .unwrap_or_else(|| default_secondary_label(&tab).to_string());
    let tertiary_label = ranking_metric(meta, "Tertiary Label")
        .unwrap_or_else(|| default_tertiary_label(&tab).to_string());
    let asset_base = metric_value(&meta.metrics, &["Asset Base"])
        .unwrap_or_else(|| "https://uma.moe/assets".to_string());
    let rows = render_rows(&ranking_rows(meta), &asset_base);
    let brand = super::render_brand_corner();
    let brand_css = super::brand_corner_css();
    let subline = format!("{tab} leaderboard / {period} / {total} entries");
    let mode_class = ranking_mode_class(&tab);

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=1200, initial-scale=1">
  <title>{title}</title>
  <style>
    :root {{
      --bg-secondary: #121212;
      --surface-1: rgba(255, 255, 255, 0.026);
      --border-subtle: rgba(255, 255, 255, 0.065);
      --text-primary: #ffffff;
      --text-muted: rgba(255, 255, 255, 0.5);
      --text-disabled: rgba(255, 255, 255, 0.34);
      --accent-warning: #ffb74d;
      --accent-primary: #64b5f6;
      --accent-secondary: #81c784;
      color-scheme: dark;
      font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: var(--bg-secondary);
      color: var(--text-primary);
    }}

    * {{ box-sizing: border-box; }}

    html,
    body {{
      width: 1200px;
      height: 630px;
      margin: 0;
      overflow: hidden;
      background: var(--bg-secondary);
    }}

    .rankings-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 88px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 15% 0%, rgba(100, 181, 246, 0.095), transparent 340px),
        radial-gradient(circle at 86% 0%, rgba(129, 199, 132, 0.09), transparent 350px),
        var(--bg-secondary);
    }}

    .rankings-header {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 30px;
      align-items: center;
      padding: 14px 48px 10px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.075), rgba(129, 199, 132, 0.06)),
        rgba(255, 255, 255, 0.012);
    }}

    .header-copy {{
      display: grid;
      gap: 7px;
      min-width: 0;
    }}

    .rankings-title {{
      margin: 0;
      background: linear-gradient(45deg, #64b5f6, #81c784 62%, #ffcc80);
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      font-size: 38px;
      font-weight: 850;
      line-height: 0.98;
      letter-spacing: 0;
      white-space: nowrap;
    }}

    .rankings-subline {{
      margin: 0;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }}

    .rankings-body {{
      display: grid;
      grid-template-rows: 22px minmax(0, 1fr);
      gap: 6px;
      min-height: 0;
      padding: 8px 48px 12px;
    }}

    .leader-head,
    .lb-row {{
      display: grid;
      grid-template-columns: 74px minmax(0, 1.08fr) minmax(190px, 0.9fr) 150px 120px 120px;
      gap: 12px;
      align-items: center;
      min-width: 0;
    }}

    .leader-head {{
      padding: 0 14px;
      color: var(--text-disabled);
      font-size: 10px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .leader-head span,
    .lb-row > * {{
      min-width: 0;
    }}

    .leader-head span:nth-child(1) {{ text-align: center; }}
    .leader-head span:nth-child(4),
    .leader-head span:nth-child(5),
    .leader-head span:nth-child(6) {{ text-align: right; }}

    .leaderboard {{
      display: grid;
      grid-template-rows: repeat(10, 46px);
      gap: 4px;
      min-height: 0;
      overflow: hidden;
    }}

    .lb-row {{
      height: 46px;
      padding: 4px 14px;
      border: 1px solid var(--row-border);
      border-radius: 8px;
      background: var(--row-bg);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.022);
    }}

    .lb-row.top-1 {{
      --row-border: rgba(255, 215, 0, 0.3);
      --row-bg: linear-gradient(135deg, rgba(255, 215, 0, 0.07), rgba(255, 215, 0, 0.015));
      --rank-color: #ffd86b;
    }}

    .lb-row.top-2 {{
      --row-border: rgba(192, 202, 212, 0.24);
      --row-bg: linear-gradient(135deg, rgba(192, 202, 212, 0.06), rgba(192, 202, 212, 0.014));
      --rank-color: #cfd8dc;
    }}

    .lb-row.top-3 {{
      --row-border: rgba(205, 127, 50, 0.27);
      --row-bg: linear-gradient(135deg, rgba(205, 127, 50, 0.064), rgba(205, 127, 50, 0.014));
      --rank-color: #d89b61;
    }}

    .lb-row.standard {{
      --row-border: var(--border-subtle);
      --row-bg: var(--surface-1);
      --rank-color: var(--text-primary);
    }}

    .lb-rank {{
      color: var(--rank-color);
      font-size: 20px;
      font-weight: 950;
      line-height: 1;
      text-align: center;
      font-variant-numeric: tabular-nums;
      white-space: nowrap;
    }}

    .trainer-name {{
      overflow: hidden;
      color: var(--accent-primary);
      font-size: 16px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .circle-link {{
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      overflow: hidden;
      color: rgba(100, 181, 246, 0.78);
      font-size: 12px;
      font-weight: 850;
      line-height: 1;
      white-space: nowrap;
    }}

    .circle-name {{
      overflow: hidden;
      min-width: 0;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .circle-emblem {{
      position: relative;
      display: grid;
      place-items: center;
      width: 28px;
      height: 28px;
      flex: 0 0 28px;
      border: 1px solid rgba(100, 181, 246, 0.32);
      border-radius: 50%;
      background:
        radial-gradient(circle at 36% 28%, rgba(129, 199, 132, 0.18), transparent 52%),
        rgba(100, 181, 246, 0.11);
      color: var(--accent-primary);
      font-size: 12px;
      font-weight: 950;
      line-height: 1;
      overflow: hidden;
    }}

    .circle-emblem.fallback::before {{
      content: "";
      position: absolute;
      width: 10px;
      height: 10px;
      border: 2px solid currentColor;
      border-top-color: transparent;
      border-radius: 50%;
      transform: rotate(-28deg);
      opacity: 0.95;
    }}

    .circle-emblem.fallback::after {{
      content: "";
      position: absolute;
      right: 7px;
      bottom: 7px;
      width: 5px;
      height: 5px;
      border-radius: 50%;
      background: currentColor;
      opacity: 0.95;
    }}

    .circle-emblem img {{
      width: 28px;
      height: 28px;
      object-fit: contain;
    }}

    .lb-stats {{
      display: contents;
    }}

    .stat {{
      display: flex;
      align-items: center;
      justify-content: flex-end;
      min-width: 0;
      text-align: right;
    }}

    .stat-value {{
      overflow: hidden;
      max-width: 100%;
      color: var(--text-primary);
      font-size: 14px;
      font-weight: 900;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .primary-stat .stat-value {{
      color: var(--accent-warning);
    }}

    .mode-gains .primary-stat .stat-value,
    .mode-gains .gain-stat .stat-value {{
      color: var(--accent-secondary);
    }}

{brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-rankings">
  <main class="rankings-card {class_list} card-view-rankings">
    <header class="rankings-header">
      <div class="header-copy">
        <h1 class="rankings-title">{title}</h1>
        <p class="rankings-subline">{subline}</p>
      </div>
      {brand}
    </header>

    <section class="rankings-body {mode_class}">
      <div class="leader-head"><span>Rank</span><span>Trainer</span><span>Club</span><span>{primary_label}</span><span>{secondary_label}</span><span>{tertiary_label}</span></div>
      <div class="leaderboard">{rows}</div>
    </section>
  </main>
</body>
</html>
"#,
        title = title,
        subline = html_escape(&truncate_chars(&subline, 72)),
        class_list = class_list,
        brand_css = brand_css,
        brand = brand,
        primary_label = html_escape(&truncate_chars(&primary_label, 14)),
        secondary_label = html_escape(&truncate_chars(&secondary_label, 14)),
        tertiary_label = html_escape(&truncate_chars(&tertiary_label, 14)),
        mode_class = mode_class,
        rows = rows,
    )
}

fn render_rows(rows: &[RankingRow], asset_base: &str) -> String {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            let class_name = match index {
                0 => "top-1",
                1 => "top-2",
                2 => "top-3",
                _ => "standard",
            };
            let club_icon = render_club_icon(row.club_rank_id.as_deref(), asset_base);
            format!(
                r#"<article class="lb-row {class_name}">
        <div class="lb-rank">{rank}</div>
        <div class="lb-identity"><span class="trainer-name">{trainer}</span></div>
        <div class="circle-link">{club_icon}<span class="circle-name">{club}</span></div>
        <div class="lb-stats">
          <div class="stat primary-stat gain-stat"><span class="stat-value">{primary}</span></div>
          <div class="stat gain-stat"><span class="stat-value">{secondary}</span></div>
          <div class="stat gain-stat"><span class="stat-value">{tertiary}</span></div>
        </div>
      </article>"#,
                class_name = class_name,
                rank = html_escape(&row.rank),
                trainer = html_escape(&truncate_chars(&row.trainer, 30)),
                club = html_escape(&truncate_chars(&row.club, 30)),
                club_icon = club_icon,
                primary = html_escape(&row.primary_value),
                secondary = html_escape(&row.secondary_value),
                tertiary = html_escape(&row.tertiary_value),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_club_icon(rank_id: Option<&str>, asset_base: &str) -> String {
    let Some(rank_id) = rank_id.and_then(|rank_id| rank_id.trim().parse::<i64>().ok()) else {
        return r#"<span class="circle-emblem fallback"></span>"#.to_string();
    };
    let rank_id = rank_id.clamp(1, 11);
    let image = asset_url(
        asset_base,
        &format!("/images/icon/circle_rank/utx_ico_circle_rank_{rank_id:02}.webp"),
    );
    format!(
        r#"<span class="circle-emblem"><img src="{image}" alt="" onerror="this.parentElement.className='circle-emblem fallback';this.remove()"></span>"#,
        image = html_escape(&image),
    )
}

fn ranking_rows(meta: &EmbedMetadata) -> Vec<RankingRow> {
    let mut rows = Vec::new();

    for index in 1..=10 {
        let Some(trainer) = ranking_metric(meta, &format!("Trainer {index}")) else {
            continue;
        };
        rows.push(RankingRow {
            rank: ranking_metric(meta, &format!("Rank {index}"))
                .unwrap_or_else(|| format!("#{index}")),
            trainer,
            club: ranking_metric(meta, &format!("Club {index}"))
                .unwrap_or_else(|| "No club".to_string()),
            club_rank_id: ranking_metric(meta, &format!("Club Rank Id {index}")),
            primary_value: ranking_metric(meta, &format!("Primary {index}"))
                .unwrap_or_else(|| "tracked".to_string()),
            secondary_value: ranking_metric(meta, &format!("Secondary {index}"))
                .unwrap_or_else(|| "fans".to_string()),
            tertiary_value: ranking_metric(meta, &format!("Tertiary {index}"))
                .unwrap_or_else(|| "active".to_string()),
        });
    }

    if rows.is_empty() {
        return fallback_rows();
    }

    rows
}

fn fallback_rows() -> Vec<RankingRow> {
    [
        (
            "#1",
            "Top trainer",
            "Circle affiliation",
            "live",
            "fans",
            "avg/day",
        ),
        (
            "#2",
            "Rising trainer",
            "Club link",
            "tracked",
            "fans",
            "active",
        ),
        (
            "#3",
            "Fan leader",
            "Public circle",
            "recent",
            "ranking",
            "daily",
        ),
        (
            "#4",
            "Active trainer",
            "Circle profile",
            "monthly",
            "total",
            "days",
        ),
        ("#5", "Global trainer", "Club data", "gain", "fans", "pace"),
        (
            "#6",
            "Steady runner",
            "Training camp",
            "tracked",
            "fans",
            "avg",
        ),
        (
            "#7",
            "Daily grinder",
            "Open club",
            "recent",
            "total",
            "days",
        ),
        (
            "#8",
            "Fan chaser",
            "Victory road",
            "monthly",
            "fans",
            "pace",
        ),
        ("#9", "Late runner", "Night sprint", "gain", "fans", "avg"),
        ("#10", "Club ace", "Top circle", "tracked", "total", "daily"),
    ]
    .into_iter()
    .map(
        |(rank, trainer, club, primary_value, secondary_value, tertiary_value)| RankingRow {
            rank: rank.to_string(),
            trainer: trainer.to_string(),
            club: club.to_string(),
            club_rank_id: None,
            primary_value: primary_value.to_string(),
            secondary_value: secondary_value.to_string(),
            tertiary_value: tertiary_value.to_string(),
        },
    )
    .collect()
}

fn default_period(tab: &str) -> &'static str {
    if tab.eq_ignore_ascii_case("gains") {
        "30-Day Gain"
    } else if tab.eq_ignore_ascii_case("all-time") || tab.eq_ignore_ascii_case("alltime") {
        "Avg/Month"
    } else {
        "Current Month"
    }
}

fn default_primary_label(tab: &str) -> &'static str {
    if tab.eq_ignore_ascii_case("gains") {
        "30d"
    } else if tab.eq_ignore_ascii_case("all-time") || tab.eq_ignore_ascii_case("alltime") {
        "Avg/Month"
    } else {
        "Monthly Gain"
    }
}

fn default_secondary_label(tab: &str) -> &'static str {
    if tab.eq_ignore_ascii_case("gains") {
        "7d"
    } else {
        "Fans"
    }
}

fn default_tertiary_label(tab: &str) -> &'static str {
    if tab.eq_ignore_ascii_case("gains") {
        "3d"
    } else if tab.eq_ignore_ascii_case("all-time") || tab.eq_ignore_ascii_case("alltime") {
        "Total Gain"
    } else {
        "Avg/Day"
    }
}

fn ranking_mode_class(tab: &str) -> &'static str {
    if tab.eq_ignore_ascii_case("gains") {
        "mode-gains"
    } else if tab.eq_ignore_ascii_case("all-time") || tab.eq_ignore_ascii_case("alltime") {
        "mode-alltime"
    } else {
        "mode-monthly"
    }
}

fn ranking_metric(meta: &EmbedMetadata, label: &str) -> Option<String> {
    meta.metrics
        .iter()
        .find(|metric| metric.label.eq_ignore_ascii_case(label))
        .map(|metric| metric.value.clone())
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    super::overview::render_leaderboard_visual(
        "Trainer Rankings",
        &[
            ("#1", "Monthly Fans", "+30d"),
            ("#2", "All-time Fans", "Total"),
            ("#3", "Recent Gains", "+7d"),
        ],
    )
}
