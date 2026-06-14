use crate::embed::{embed_class_list, EmbedMetadata};

use super::{display_title, html_escape, js_number_array, js_string_array, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-activity",
    render_visual,
};

struct ActivityRow {
    rank: String,
    trainer: String,
    viewer: String,
    club: String,
    facts: String,
    reason: String,
    reason_class: String,
    fan_gain: String,
    active: String,
    careers_rate: String,
    score: String,
    score_band: String,
    score_class: String,
}

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    let path = super::canonical_path(&meta.canonical_url);
    let route_path = super::normalize_route_path(&path);

    (matches!(route_path, "/activity" | "/shame")
        || route_path.starts_with("/activity/")
        || route_path.starts_with("/shame/"))
        && meta.database.is_none()
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let path = super::canonical_path(&meta.canonical_url);
    let route_path = super::normalize_route_path(&path);

    if route_path.starts_with("/activity/") || route_path.starts_with("/shame/") {
        return render_detail_card_html(meta);
    }

    let class_list = embed_class_list(meta);
    let title_text = match display_title(&meta.title).as_str() {
        "Activity" => "Top 100 Club Activity Reports".to_string(),
        title => title.to_string(),
    };
    let title = html_escape(&truncate_chars(&title_text, 48));
    let total = activity_metric(meta, "Total").unwrap_or_else(|| "Live".to_string());
    let updated = activity_metric(meta, "Updated").unwrap_or_else(|| "Snapshot data".to_string());
    let description = html_escape(&truncate_chars(
        &format!("Snapshot-based activity reports for {total} observed Top 100 club accounts; suspicion scores are context, not proof."),
        132,
    ));
    let rows = render_rows(&activity_rows(meta));
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
      --surface-1: rgba(255, 255, 255, 0.026);
      --surface-2: rgba(255, 255, 255, 0.052);
      --surface-3: rgba(255, 255, 255, 0.08);
      --border-subtle: rgba(255, 255, 255, 0.065);
      --border-primary: rgba(255, 255, 255, 0.12);
      --text-primary: #ffffff;
      --text-secondary: rgba(255, 255, 255, 0.72);
      --text-muted: rgba(255, 255, 255, 0.5);
      --text-disabled: rgba(255, 255, 255, 0.34);
      --accent-primary: #64b5f6;
      --accent-warning: #ffcc80;
      --accent-danger: #ef9a9a;
      --accent-purple: #ce93d8;
      --accent-green: #a5d6a7;
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

    .activity-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 88px 58px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 12% 4%, rgba(100, 181, 246, 0.09), transparent 340px),
        radial-gradient(circle at 90% 0%, rgba(239, 154, 154, 0.075), transparent 330px),
        var(--bg-secondary);
    }}

    .activity-header {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 30px;
      align-items: center;
      padding: 14px 48px 10px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.045), rgba(255, 204, 128, 0.055)),
        rgba(255, 255, 255, 0.012);
    }}

    .header-copy {{
      display: grid;
      gap: 8px;
      min-width: 0;
    }}

    .activity-title {{
      margin: 0;
      color: var(--accent-warning);
      font-size: 33px;
      font-weight: 850;
      line-height: 1.02;
      letter-spacing: 0;
    }}

    .activity-description {{
      max-width: 740px;
      margin: 0;
      color: var(--text-secondary);
      font-size: 13px;
      font-weight: 500;
      line-height: 1.25;
    }}

    .summary-grid {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
      min-width: 0;
    }}

    .summary-stat {{
      min-width: 0;
      padding: 10px 12px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 8px;
      background: rgba(10, 10, 10, 0.3);
    }}

    .summary-label {{
      display: block;
      margin-bottom: 5px;
      color: var(--text-disabled);
      font-size: 10px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .summary-value {{
      display: block;
      overflow: hidden;
      color: var(--text-primary);
      font-size: 18px;
      font-weight: 900;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .scope-notice {{
      display: grid;
      grid-template-columns: 34px minmax(0, 1fr) 230px;
      gap: 12px;
      align-items: center;
      min-width: 0;
      padding: 9px 48px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.045);
      background: rgba(100, 181, 246, 0.055);
    }}

    .info-icon {{
      display: grid;
      place-items: center;
      width: 32px;
      height: 32px;
      border: 1px solid rgba(100, 181, 246, 0.24);
      border-radius: 50%;
      color: #90caf9;
      font-size: 18px;
      font-weight: 900;
    }}

    .notice-copy {{
      min-width: 0;
    }}

    .notice-copy strong {{
      display: block;
      margin-bottom: 2px;
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 850;
    }}

    .notice-copy span {{
      display: block;
      overflow: hidden;
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 650;
      line-height: 1.15;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .notice-chip {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      height: 30px;
      padding: 0 11px;
      border: 1px solid rgba(255, 204, 128, 0.22);
      border-radius: 8px;
      background: rgba(255, 204, 128, 0.07);
      color: var(--accent-warning);
      font-size: 12px;
      font-weight: 850;
      white-space: nowrap;
    }}

    .activity-list {{
      display: grid;
      grid-template-rows: repeat(8, minmax(0, 1fr));
      gap: 4px;
      min-height: 0;
      padding: 10px 48px 12px;
    }}

    .activity-row {{
      display: grid;
      grid-template-columns: 48px minmax(0, 1fr) 285px 86px;
      gap: 10px;
      align-items: center;
      min-height: 0;
      padding: 0 10px;
      border: 1px solid var(--row-border);
      border-left: 3px solid var(--tier-color);
      border-radius: 8px;
      background: var(--row-bg);
    }}

    .score-critical {{
      --tier-color: rgba(229, 115, 115, 0.74);
      --row-border: rgba(229, 115, 115, 0.2);
      --row-bg: linear-gradient(135deg, rgba(229, 115, 115, 0.055), rgba(255, 255, 255, 0.02));
      --score-color: #ef9a9a;
    }}

    .score-high {{
      --tier-color: rgba(255, 204, 128, 0.68);
      --row-border: rgba(255, 204, 128, 0.18);
      --row-bg: linear-gradient(135deg, rgba(255, 204, 128, 0.052), rgba(255, 255, 255, 0.02));
      --score-color: #ffcc80;
    }}

    .score-elevated {{
      --tier-color: rgba(100, 181, 246, 0.62);
      --row-border: rgba(100, 181, 246, 0.16);
      --row-bg: linear-gradient(135deg, rgba(100, 181, 246, 0.045), rgba(255, 255, 255, 0.02));
      --score-color: #90caf9;
    }}

    .score-watch {{
      --tier-color: rgba(206, 147, 216, 0.58);
      --row-border: rgba(206, 147, 216, 0.16);
      --row-bg: linear-gradient(135deg, rgba(206, 147, 216, 0.04), rgba(255, 255, 255, 0.02));
      --score-color: #ce93d8;
    }}

    .score-low {{
      --tier-color: rgba(129, 199, 132, 0.52);
      --row-border: rgba(129, 199, 132, 0.13);
      --row-bg: rgba(255, 255, 255, 0.026);
      --score-color: #a5d6a7;
    }}

    .row-rank {{
      color: var(--text-secondary);
      font-size: 17px;
      font-weight: 900;
      text-align: center;
      white-space: nowrap;
    }}

    .entry-main {{
      display: grid;
      gap: 4px;
      min-width: 0;
    }}

    .entry-title {{
      display: flex;
      align-items: center;
      gap: 6px;
      min-width: 0;
    }}

    .entry-name {{
      overflow: hidden;
      color: var(--accent-primary);
      font-size: 14px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .viewer-id {{
      display: inline-flex;
      align-items: center;
      height: 17px;
      padding: 0 6px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 5px;
      background: rgba(255, 255, 255, 0.035);
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 750;
      white-space: nowrap;
    }}

    .entry-tags {{
      display: flex;
      align-items: center;
      gap: 5px;
      min-width: 0;
      overflow: hidden;
    }}

    .tag,
    .reason-chip {{
      display: inline-flex;
      align-items: center;
      min-width: 0;
      height: 17px;
      padding: 0 6px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 5px;
      background: rgba(255, 255, 255, 0.025);
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 750;
      white-space: nowrap;
    }}

    .circle-link {{
      color: rgba(100, 181, 246, 0.78);
    }}

    .reason-chip {{
      color: var(--reason-color);
      border-color: var(--reason-border);
      background: var(--reason-bg);
    }}

    .reason-critical,
    .reason-high {{
      --reason-color: #efb0b0;
      --reason-border: rgba(229, 115, 115, 0.2);
      --reason-bg: rgba(229, 115, 115, 0.04);
    }}

    .reason-medium {{
      --reason-color: #9fc9ec;
      --reason-border: rgba(100, 181, 246, 0.18);
      --reason-bg: rgba(100, 181, 246, 0.04);
    }}

    .reason-low,
    .reason-info {{
      --reason-color: #abd2ad;
      --reason-border: rgba(129, 199, 132, 0.16);
      --reason-bg: rgba(129, 199, 132, 0.035);
    }}

    .row-metrics {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 6px;
      min-width: 0;
      padding-left: 10px;
      border-left: 1px solid rgba(255, 255, 255, 0.065);
    }}

    .metric {{
      display: grid;
      justify-items: end;
      gap: 3px;
      min-width: 0;
    }}

    .metric strong {{
      overflow: hidden;
      max-width: 100%;
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .metric span {{
      color: var(--text-disabled);
      font-size: 8px;
      font-weight: 800;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .score-box {{
      display: grid;
      justify-items: end;
      gap: 2px;
      min-width: 0;
      padding-left: 10px;
      border-left: 1px solid rgba(255, 255, 255, 0.065);
      text-align: right;
    }}

    .score-band {{
      overflow: hidden;
      max-width: 100%;
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 800;
      line-height: 1.1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .score-line {{
      display: flex;
      align-items: baseline;
      justify-content: end;
      gap: 2px;
      color: var(--score-color);
    }}

    .score-value {{
      font-size: 25px;
      color: var(--score-color);
      font-weight: 900;
      line-height: 0.95;
      white-space: nowrap;
    }}

    .score-max {{
      color: var(--text-disabled);
      font-size: 9px;
      font-weight: 800;
    }}

    .activity-footer {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      min-width: 0;
      padding: 0 48px 12px;
    }}

    .paginator {{
      display: flex;
      align-items: center;
      gap: 10px;
      height: 30px;
      padding: 0 12px;
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.026);
      color: var(--text-secondary);
      font-size: 12px;
      font-weight: 800;
    }}

    .page-dot {{
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--accent-primary);
      box-shadow: 0 0 8px rgba(100, 181, 246, 0.5);
    }}

    .card-url {{
      overflow: hidden;
      color: var(--text-disabled);
      font-size: 11px;
      font-weight: 750;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}
{brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-activity">
  <main class="activity-card {class_list} card-view-activity">
    <header class="activity-header">
      <div class="header-copy">
        <h1 class="activity-title">{title}</h1>
        <p class="activity-description">{description}</p>
      </div>
      {brand}
    </header>

    <section class="scope-notice">
      <span class="info-icon">i</span>
      <div class="notice-copy">
        <strong>Limited snapshot reconstruction</strong>
        <span>Activity is inferred from Top 100 snapshots; Not proof of botting.</span>
      </div>
      <span class="notice-chip">{updated}</span>
    </section>

    <section class="activity-list">{rows}</section>
  </main>
</body>
</html>
"#,
        title = title,
        description = description,
        class_list = class_list,
        brand_css = brand_css,
        brand = brand,
        updated = html_escape(&truncate_chars(&updated, 22)),
        rows = rows,
    )
}

fn render_detail_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let viewer = activity_metric(meta, "Viewer").unwrap_or_else(|| "ID tracked".to_string());
    let trainer = activity_metric(meta, "Trainer").unwrap_or_else(|| viewer.clone());
    let score = activity_metric(meta, "Score").unwrap_or_else(|| "-".to_string());
    let score_band = activity_metric(meta, "Score Band").unwrap_or_else(|| "Review".to_string());
    let score_class_name = score_class(
        &activity_metric(meta, "Score Class").unwrap_or_else(|| "score-watch".to_string()),
    );
    let verdict =
        activity_metric(meta, "Verdict").unwrap_or_else(|| "Activity pattern".to_string());
    let total_fan_gain =
        activity_metric(meta, "Total Fan Gain").unwrap_or_else(|| "tracked".to_string());
    let total_active =
        activity_metric(meta, "Total Active").unwrap_or_else(|| "observed".to_string());
    let total_careers =
        activity_metric(meta, "Total Careers").unwrap_or_else(|| "careers".to_string());
    let recent_3d = activity_metric(meta, "Recent 3d").unwrap_or_else(|| "recent".to_string());
    let peak_daily = activity_metric(meta, "Peak Daily").unwrap_or_else(|| "fan gain".to_string());
    let heatmap = render_heatmap(
        activity_metric(meta, "Heatmap Pattern")
            .as_deref()
            .unwrap_or(""),
    );
    let daily_chart = render_daily_chart(meta);
    let runtime_chart = render_runtime_distribution_chart(meta);
    let brand = super::render_brand_corner();
    let brand_css = super::brand_corner_css();
    let chart_js = super::chart_js();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=1200, initial-scale=1">
  <title>{trainer}</title>
  <style>
    :root {{
      --bg-primary: #0a0a0a;
      --bg-secondary: #121212;
      --surface-1: rgba(255, 255, 255, 0.026);
      --surface-2: rgba(255, 255, 255, 0.052);
      --border-subtle: rgba(255, 255, 255, 0.065);
      --border-primary: rgba(255, 255, 255, 0.12);
      --text-primary: #ffffff;
      --text-secondary: rgba(255, 255, 255, 0.72);
      --text-muted: rgba(255, 255, 255, 0.5);
      --text-disabled: rgba(255, 255, 255, 0.34);
      --accent-primary: #64b5f6;
      --accent-warning: #ffcc80;
      --accent-danger: #ef9a9a;
      --accent-purple: #ce93d8;
      --accent-green: #a5d6a7;
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

    .detail-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 104px 78px 202px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 14% 0%, rgba(100, 181, 246, 0.09), transparent 330px),
        radial-gradient(circle at 88% 0%, rgba(239, 154, 154, 0.075), transparent 330px),
        var(--bg-secondary);
    }}

    .detail-lead {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 32px;
      align-items: center;
      padding: 18px 60px 16px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.04), rgba(255, 204, 128, 0.04)),
        rgba(255, 255, 255, 0.012);
    }}

    .lead-copy {{
      display: grid;
      gap: 10px;
      min-width: 0;
    }}

    .lead-kicker {{
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
    }}

    .verdict-badge,
    .stable-id {{
      display: inline-flex;
      align-items: center;
      height: 26px;
      padding: 0 10px;
      border: 1px solid var(--badge-border);
      border-radius: 7px;
      background: var(--badge-bg);
      color: var(--badge-color);
      font-size: 12px;
      font-weight: 850;
      white-space: nowrap;
    }}

    .verdict-badge {{
      --badge-border: rgba(255, 204, 128, 0.24);
      --badge-bg: rgba(255, 204, 128, 0.075);
      --badge-color: var(--accent-warning);
    }}

    .stable-id {{
      --badge-border: rgba(255, 255, 255, 0.08);
      --badge-bg: rgba(255, 255, 255, 0.035);
      --badge-color: var(--text-muted);
    }}

    .lead-title {{
      margin: 0;
      overflow: hidden;
      color: var(--text-primary);
      font-size: 38px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .lead-side {{
      display: grid;
      justify-items: end;
      align-content: center;
      min-width: 0;
    }}

    .score-critical {{ --ring-color: #ef9a9a; --ring-border: rgba(229, 115, 115, 0.34); --ring-bg: rgba(229, 115, 115, 0.075); }}
    .score-high {{ --ring-color: #ffcc80; --ring-border: rgba(255, 204, 128, 0.3); --ring-bg: rgba(255, 204, 128, 0.072); }}
    .score-elevated {{ --ring-color: #90caf9; --ring-border: rgba(100, 181, 246, 0.3); --ring-bg: rgba(100, 181, 246, 0.07); }}
    .score-watch {{ --ring-color: #ce93d8; --ring-border: rgba(206, 147, 216, 0.26); --ring-bg: rgba(206, 147, 216, 0.06); }}
    .score-low {{ --ring-color: #a5d6a7; --ring-border: rgba(129, 199, 132, 0.24); --ring-bg: rgba(129, 199, 132, 0.055); }}

    .metric-grid {{
      display: grid;
      grid-template-columns: repeat(5, minmax(0, 1fr));
      gap: 12px;
      min-height: 0;
      padding: 11px 60px 10px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.045);
    }}

    .metric-tile {{
      display: grid;
      align-content: center;
      gap: 5px;
      min-width: 0;
      padding: 10px 14px;
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.024);
    }}

    .metric-tile .label {{
      color: var(--text-disabled);
      font-size: 10px;
      font-weight: 850;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .metric-tile .value {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 18px;
      font-weight: 900;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .metric-tile.accent .value {{
      color: var(--accent-primary);
    }}

    .metric-tile.score .value {{
      color: var(--accent-warning);
    }}

    .upper-grid {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) 340px;
      gap: 16px;
      min-height: 0;
      padding: 16px 60px 0;
    }}

    .analysis-card {{
      min-width: 0;
      min-height: 0;
      border: 1px solid var(--border-subtle);
      border-radius: 9px;
      background: rgba(255, 255, 255, 0.022);
      overflow: hidden;
    }}

    .analysis-header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 16px;
      height: 36px;
      padding: 0 16px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    }}

    .analysis-header h3 {{
      margin: 0;
      flex-shrink: 0;
      color: var(--text-primary);
      font-size: 15px;
      font-weight: 850;
      line-height: 1;
      white-space: nowrap;
    }}

    .analysis-header > span {{
      color: var(--text-muted);
      font-size: 11px;
      font-weight: 800;
    }}

    .chart-key,
    .heatmap-legend {{
      display: inline-flex;
      align-items: center;
      gap: 10px;
      min-width: 0;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 850;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .key-item {{
      display: inline-flex;
      align-items: center;
      gap: 5px;
      min-width: 0;
    }}

    .line-sample {{
      width: 22px;
      height: 0;
      border-top: 3px solid var(--key-color);
      border-radius: 999px;
    }}

    .line-sample.dashed {{
      border-top-style: dashed;
    }}

    .peak-label {{
      color: var(--text-muted);
    }}

    .heatmap-grid {{
      display: grid;
      grid-template-columns: 31px repeat(24, minmax(0, 1fr));
      gap: 4px;
      padding: 10px 16px 8px;
      align-items: center;
    }}

    .heat-cell {{
      height: 10px;
      min-height: 0;
      border-radius: 2px;
      background: var(--heat-color);
      border: 1px solid var(--heat-border);
    }}

    .heatmap-corner,
    .hour-label,
    .day-label {{
      color: var(--text-disabled);
      font-size: 8px;
      font-weight: 800;
      line-height: 1;
    }}

    .hour-label {{
      min-height: 10px;
      text-align: center;
    }}

    .hour-label.major,
    .day-label {{
      color: var(--text-muted);
    }}

    .day-label {{
      padding-right: 2px;
      text-align: right;
    }}

    .level-0 {{ --heat-color: rgba(255, 255, 255, 0.04); --heat-border: rgba(255, 255, 255, 0.035); }}
    .level-1 {{ --heat-color: #0e4429; --heat-border: rgba(57, 211, 83, 0.18); }}
    .level-2 {{ --heat-color: #006d32; --heat-border: rgba(57, 211, 83, 0.24); }}
    .level-3 {{ --heat-color: #26a641; --heat-border: rgba(57, 211, 83, 0.3); }}
    .level-4 {{ --heat-color: #39d353; --heat-border: rgba(179, 255, 196, 0.35); }}

    .heatmap-legend {{
      padding: 0 16px 10px 47px;
      justify-content: flex-start;
      text-transform: none;
    }}

    .heatmap-scale {{
      display: inline-flex;
      gap: 4px;
    }}

    .heatmap-scale i {{
      width: 18px;
      height: 8px;
      border-radius: 2px;
      background: var(--heat-color);
    }}

    .heatmap-label {{
      display: inline-flex;
      align-items: center;
      gap: 4px;
    }}

    .heatmap-label i {{
      width: 12px;
      height: 8px;
      border-radius: 2px;
      background: var(--heat-color);
      border: 1px solid var(--heat-border);
    }}

    .daily-card {{
      display: grid;
      grid-template-rows: 32px minmax(0, 1fr);
      margin: 10px 60px 8px;
      min-height: 0;
    }}

    .daily-card .analysis-header {{
      height: 32px;
      padding-right: 14px;
    }}

    .daily-title-row {{
      display: inline-flex;
      align-items: center;
      gap: 11px;
      min-width: 0;
    }}

    .daily-title-row .chart-key {{
      gap: 6px;
      color: var(--text-muted);
      font-size: 10px;
      text-transform: none;
    }}

    .daily-title-row .key-item,
    .daily-card .peak-label {{
      height: 19px;
      padding: 0 8px;
      border: 1px solid rgba(255, 255, 255, 0.07);
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.028);
      line-height: 18px;
    }}

    .daily-card .peak-label {{
      color: var(--accent-warning);
      font-size: 10px;
      font-weight: 900;
      white-space: nowrap;
      border-color: rgba(255, 204, 128, 0.2);
      background: rgba(255, 204, 128, 0.055);
    }}

    .daily-title-row .line-sample {{
      width: 15px;
      border-top-width: 2px;
    }}

    .daily-chart {{
      position: relative;
      min-height: 0;
      padding: 4px 10px 7px;
    }}

    .daily-chart canvas {{
      width: 100%;
      height: 100%;
      display: block;
    }}

    .chart-empty {{
      display: grid;
      place-items: center;
      height: 100%;
      min-height: 84px;
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 800;
    }}

    .runtime-chart {{
      position: relative;
      height: 142px;
      padding: 10px 12px 13px;
    }}

    .runtime-chart canvas {{
      display: block;
      width: 100%;
      height: 100%;
    }}

    .card-url {{
      overflow: hidden;
      color: var(--text-disabled);
      font-size: 11px;
      font-weight: 750;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}
{brand_css}
  </style>
  <script>{chart_js}</script>
</head>
<body class="embed-card-page {class_list} card-view-activity">
  <main class="detail-card {class_list} card-view-activity {score_class_name}">
    <section class="detail-lead">
      <div class="lead-copy">
        <div class="lead-kicker"><span class="verdict-badge">{verdict}</span><span class="stable-id">{viewer}</span></div>
        <h1 class="lead-title">{trainer}</h1>
      </div>
      <div class="lead-side">{brand}</div>
    </section>

    <section class="metric-grid">
      <div class="metric-tile score"><span class="label">{score_band} score</span><span class="value">{score}/100</span></div>
      <div class="metric-tile accent"><span class="label">Total fan gain</span><span class="value">{total_fan_gain}</span></div>
      <div class="metric-tile"><span class="label">Recent 3d gain</span><span class="value">{recent_3d}</span></div>
      <div class="metric-tile"><span class="label">Careers</span><span class="value">{total_careers}</span></div>
      <div class="metric-tile"><span class="label">Total active</span><span class="value">{total_active}</span></div>
    </section>

    <section class="upper-grid">
      <article class="analysis-card heatmap-card">
        <header class="analysis-header"><h3>Weekly activity heatmap</h3><span>weekday x hour</span></header>
        <div class="heatmap-grid">{heatmap}</div>
        <div class="heatmap-legend"><span class="scale-caption">Less</span><span class="heatmap-label"><i class="level-0"></i></span><span class="heatmap-label"><i class="level-1"></i></span><span class="heatmap-label"><i class="level-2"></i></span><span class="heatmap-label"><i class="level-3"></i></span><span class="heatmap-label"><i class="level-4"></i></span><span class="scale-caption">More</span></div>
      </article>
      <article class="analysis-card runtime-card">
        <header class="analysis-header"><h3>Runtime Distribution</h3><div class="chart-key"><span class="key-item"><i class="line-sample" style="--key-color:rgba(100,181,246,0.76)"></i>Careers</span><span class="key-item"><i class="line-sample" style="--key-color:rgba(229,115,115,0.78)"></i>Weight</span></div></header>
        {runtime_chart}
      </article>
    </section>

    <article class="analysis-card daily-card">
      <header class="analysis-header"><div class="daily-title-row"><h3>Daily fan gain</h3><div class="chart-key"><span class="key-item"><i class="line-sample" style="--key-color:rgba(255,214,153,0.95)"></i>Fan gain</span><span class="key-item"><i class="line-sample" style="--key-color:rgba(255,255,255,0.82)"></i>Pace</span><span class="key-item"><i class="line-sample dashed" style="--key-color:rgba(148,196,248,0.88)"></i>Active time</span></div><span class="peak-label">Peak {peak_daily}</span></div></header>
      {daily_chart}
    </article>
  </main>
</body>
</html>
"#,
        class_list = class_list,
        brand_css = brand_css,
        brand = brand,
        trainer = html_escape(&truncate_chars(&trainer, 44)),
        viewer = html_escape(&viewer),
        score = html_escape(&score),
        score_band = html_escape(&score_band),
        score_class_name = score_class_name,
        verdict = html_escape(&truncate_chars(&verdict, 34)),
        total_fan_gain = html_escape(&total_fan_gain),
        total_active = html_escape(&total_active),
        total_careers = html_escape(&total_careers),
        recent_3d = html_escape(&recent_3d),
        peak_daily = html_escape(&peak_daily),
        heatmap = heatmap,
        daily_chart = daily_chart,
        runtime_chart = runtime_chart,
    )
}

fn render_daily_chart(meta: &EmbedMetadata) -> String {
    let fan_gains = parse_number_series(
        activity_metric(meta, "Daily Fan Gains")
            .as_deref()
            .unwrap_or(""),
    );
    let active_seconds = parse_number_series(
        activity_metric(meta, "Daily Active Seconds")
            .as_deref()
            .unwrap_or(""),
    );
    if fan_gains.is_empty() && active_seconds.is_empty() {
        return r#"<div class="chart-empty">Daily series unavailable</div>"#.to_string();
    }

    let len = fan_gains.len().max(active_seconds.len());
    let mut labels = activity_metric(meta, "Daily Labels")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|label| !label.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|labels| labels.len() == len)
        .unwrap_or_else(|| {
            (0..len)
                .map(|index| format!("D{}", index + 1))
                .collect::<Vec<_>>()
        });
    labels.resize(len, String::new());
    let mut fan_values = fan_gains;
    let mut active_values = active_seconds;
    fan_values.resize(len, 0.0);
    active_values.resize(len, 0.0);
    let raw_max_fans = fan_values.iter().copied().fold(0.0, f64::max).max(1.0);
    let raw_max_active = active_values.iter().copied().fold(0.0, f64::max).max(1.0);
    let max_fans = nice_axis_max(raw_max_fans, 4);
    let max_active = nice_time_axis_max(raw_max_active);
    let fan_series = fan_values
        .iter()
        .map(|value| (value / max_fans * 100.0).clamp(0.0, 100.0))
        .collect::<Vec<_>>();
    let active_percent_series = active_values
        .iter()
        .map(|value| (value / max_active * 100.0).clamp(0.0, 100.0))
        .collect::<Vec<_>>();
    let active_series = active_values
        .iter()
        .map(|value| -(value / max_active * 100.0).clamp(0.0, 100.0))
        .collect::<Vec<_>>();
    let bias_series = fan_series
        .iter()
        .zip(active_percent_series.iter())
        .map(|(fan, active)| (fan - active) * 0.34)
        .collect::<Vec<_>>();

    format!(
        r#"<div class="daily-chart"><canvas id="dailyActivityChart" width="1080" height="178" aria-label="Daily fan gain and activity chart"></canvas></div>
        <script>
        (() => {{
          const canvas = document.getElementById('dailyActivityChart');
          if (!canvas || !window.Chart) return;
          new Chart(canvas.getContext('2d'), {{
            data: {{
              labels: {labels},
              datasets: [
                {{
                  type: 'line',
                  label: 'Fan gain',
                  data: {fan_series},
                  backgroundColor: 'rgba(255,196,128,0.14)',
                  borderColor: 'rgba(255,214,153,0.95)',
                  borderWidth: 2,
                  pointRadius: 0,
                  tension: 0.28,
                  fill: 'origin',
                  yAxisID: 'mirror'
                }},
                {{
                  type: 'line',
                  label: 'Bias',
                  data: {bias_series},
                  borderColor: 'rgba(255,255,255,0.82)',
                  backgroundColor: 'transparent',
                  borderWidth: 1.8,
                  pointRadius: 0,
                  tension: 0.32,
                  fill: false,
                  yAxisID: 'mirror'
                }},
                {{
                  type: 'line',
                  label: 'Active time',
                  data: {active_series},
                  borderColor: 'rgba(148,196,248,0.88)',
                  backgroundColor: 'rgba(106,164,221,0.12)',
                  borderWidth: 2,
                  pointRadius: 0,
                  borderDash: [6, 4],
                  tension: 0.24,
                  fill: 'origin',
                  yAxisID: 'mirror'
                }}
              ]
            }},
            options: {{
              responsive: false,
              animation: false,
              maintainAspectRatio: false,
              layout: {{ padding: {{ top: 4, right: 8, bottom: 0, left: 4 }} }},
              plugins: {{ legend: {{ display: false }}, tooltip: {{ enabled: false }} }},
              scales: {{
                x: {{
                  grid: {{ display: false }},
                  border: {{ display: false }},
                  title: {{ display: false }},
                  ticks: {{
                    color: 'rgba(255,255,255,0.56)',
                    font: {{ size: 8, weight: '700' }},
                    padding: 0,
                    autoSkip: true,
                    maxTicksLimit: 4,
                    maxRotation: 0,
                    minRotation: 0
                  }}
                }},
                mirror: {{
                  position: 'left',
                  min: -100,
                  max: 100,
                  title: {{ display: false }},
                  grid: {{ color: context => context.tick.value === 0 ? 'rgba(255,255,255,0.18)' : 'rgba(255,255,255,0.06)' }},
                  border: {{ display: false }},
                  ticks: {{
                    stepSize: 50,
                    color: context => {{
                      const value = Number(context.tick.value);
                      if (value > 0) return 'rgba(255,214,153,0.76)';
                      if (value < 0) return 'rgba(148,196,248,0.78)';
                      return 'rgba(255,255,255,0.64)';
                    }},
                    font: {{ size: 9, weight: '750' }},
                    callback: value => {{
                      const numeric = Number(value);
                      const compact = number => {{
                        const abs = Math.abs(number);
                        if (abs >= 1000000) return `${{(number / 1000000).toFixed(abs >= 10000000 ? 0 : 1).replace('.0', '')}}M`;
                        if (abs >= 1000) return `${{Math.round(number / 1000)}}K`;
                        return `${{Math.round(number)}}`;
                      }};
                      const duration = seconds => {{
                        const hours = Math.floor(seconds / 3600);
                        const minutes = Math.floor((seconds % 3600) / 60);
                        if (hours > 0 && minutes > 0) return `${{hours}}h ${{minutes}}m`;
                        if (hours > 0) return `${{hours}}h`;
                        return `${{Math.max(minutes, 1)}}m`;
                      }};
                      if (numeric === 0) return '0';
                      if (numeric > 0) return '+' + compact({max_fans} * (numeric / 100));
                      return duration({max_active} * (Math.abs(numeric) / 100));
                    }}
                  }}
                }}
              }}
            }}
          }});
        }})();
        </script>"#,
        labels = js_string_array(&labels),
        fan_series = js_number_array(&fan_series),
        bias_series = js_number_array(&bias_series),
        active_series = js_number_array(&active_series),
        max_fans = max_fans,
        max_active = max_active,
    )
}

fn render_runtime_distribution_chart(meta: &EmbedMetadata) -> String {
    let counts = parse_number_series(
        activity_metric(meta, "Career Length Buckets")
            .as_deref()
            .unwrap_or(""),
    );
    if counts.is_empty() {
        return r#"<div class="chart-empty">Runtime buckets unavailable</div>"#.to_string();
    }

    let mut weights = parse_number_series(
        activity_metric(meta, "Short Fan Gain Score Buckets")
            .as_deref()
            .unwrap_or(""),
    );
    weights.resize(counts.len(), 0.0);
    let labels = (0..counts.len())
        .map(|index| {
            if index + 1 == counts.len() {
                format!("{}+", index * 5)
            } else {
                ((index + 1) * 5).to_string()
            }
        })
        .collect::<Vec<_>>();

    format!(
        r#"<div class="runtime-chart"><canvas id="runtimeDistributionChart" width="316" height="142" aria-label="Runtime distribution chart"></canvas></div>
        <script>
        (() => {{
          const canvas = document.getElementById('runtimeDistributionChart');
          if (!canvas || !window.Chart) return;
          new Chart(canvas.getContext('2d'), {{
            type: 'bar',
            data: {{
              labels: {labels},
              datasets: [
                {{
                  label: 'Careers',
                  data: {counts},
                  backgroundColor: 'rgba(100,181,246,0.76)',
                  borderColor: 'rgba(100,181,246,0.92)',
                  borderWidth: 1,
                  borderRadius: 5,
                  yAxisID: 'count'
                }},
                {{
                  label: 'Suspicion weight',
                  data: {weights},
                  backgroundColor: 'rgba(229,115,115,0.62)',
                  borderColor: 'rgba(229,115,115,0.86)',
                  borderWidth: 1,
                  borderRadius: 5,
                  yAxisID: 'weight'
                }}
              ]
            }},
            options: {{
              responsive: false,
              animation: false,
              maintainAspectRatio: false,
              plugins: {{ legend: {{ display: false }}, tooltip: {{ enabled: false }} }},
              scales: {{
                x: {{ grid: {{ display: false }}, ticks: {{ color: 'rgba(255,255,255,0.62)', font: {{ size: 9, weight: '800' }}, maxRotation: 0, padding: 6 }} }},
                count: {{ axis: 'y', display: false, beginAtZero: true }},
                weight: {{ axis: 'y', display: false, beginAtZero: true }}
              }}
            }}
          }});
        }})();
        </script>"#,
        labels = js_string_array(&labels),
        counts = js_number_array(&counts),
        weights = js_number_array(&weights),
    )
}

fn parse_number_series(value: &str) -> Vec<f64> {
    value
        .split(',')
        .filter_map(|part| part.trim().parse::<f64>().ok())
        .collect()
}

fn nice_axis_max(raw_max: f64, intervals: i32) -> f64 {
    if raw_max <= 0.0 || intervals <= 0 {
        return intervals.max(1) as f64;
    }

    let rough_step = raw_max / intervals as f64;
    let magnitude = 10_f64.powf(rough_step.log10().floor());
    let normalized = rough_step / magnitude;
    let nice_step = if normalized <= 1.0 {
        magnitude
    } else if normalized <= 2.0 {
        2.0 * magnitude
    } else if normalized <= 2.5 {
        2.5 * magnitude
    } else if normalized <= 5.0 {
        5.0 * magnitude
    } else {
        10.0 * magnitude
    };

    (raw_max / nice_step).ceil() * nice_step
}

fn nice_time_axis_max(raw_max_seconds: f64) -> f64 {
    const STEPS: [f64; 7] = [900.0, 1800.0, 3600.0, 7200.0, 10800.0, 14400.0, 21600.0];
    let step = STEPS
        .iter()
        .copied()
        .find(|step| step * 5.0 >= raw_max_seconds)
        .unwrap_or(STEPS[STEPS.len() - 1]);
    let tick_count = (raw_max_seconds / step).ceil().max(2.0);
    step * tick_count
}

fn render_rows(rows: &[ActivityRow]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                r#"<article class="activity-row {score_class}">
        <div class="row-rank">{rank}</div>
        <div class="entry-main">
          <div class="entry-title"><span class="entry-name">{trainer}</span><span class="viewer-id">{viewer}</span></div>
          <div class="entry-tags"><span class="tag circle-link">{club}</span><span class="tag">{facts}</span><span class="reason-chip {reason_class}">{reason}</span></div>
        </div>
        <div class="row-metrics">
          <div class="metric"><strong>{fan_gain}</strong><span>Fan gain</span></div>
          <div class="metric"><strong>{active}</strong><span>Active</span></div>
          <div class="metric"><strong>{careers_rate}</strong><span>Careers/hr</span></div>
        </div>
        <div class="score-box"><span class="score-band">{score_band}</span><span class="score-line"><strong class="score-value">{score}</strong><span class="score-max">/100</span></span></div>
      </article>"#,
                score_class = html_escape(&row.score_class),
                rank = html_escape(&row.rank),
                trainer = html_escape(&truncate_chars(&row.trainer, 28)),
                viewer = html_escape(&truncate_chars(&row.viewer, 18)),
                club = html_escape(&truncate_chars(&row.club, 28)),
                facts = html_escape(&truncate_chars(&row.facts, 30)),
                reason_class = html_escape(&row.reason_class),
                reason = html_escape(&truncate_chars(&row.reason, 34)),
                fan_gain = html_escape(&row.fan_gain),
                active = html_escape(&row.active),
                careers_rate = html_escape(&row.careers_rate),
                score_band = html_escape(&truncate_chars(&row.score_band, 18)),
                score = html_escape(&row.score),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_heatmap(pattern: &str) -> String {
    let fallback = "000000111100222000333000";
    let cells = if pattern.len() >= 168 {
        pattern.chars().take(168).collect::<Vec<_>>()
    } else {
        fallback.repeat(7).chars().take(168).collect::<Vec<_>>()
    };

    let mut html = String::new();
    html.push_str(r#"<span class="heatmap-corner">UTC</span>"#);
    for hour in 0..24 {
        if hour % 6 == 0 {
            html.push_str(&format!(r#"<span class="hour-label major">{hour}</span>"#));
        } else if hour == 23 {
            html.push_str(r#"<span class="hour-label major">24</span>"#);
        } else {
            html.push_str(r#"<span class="hour-label"></span>"#);
        }
    }

    let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    for (day_index, day_label) in days.iter().enumerate() {
        html.push_str(&format!(r#"<span class="day-label">{day_label}</span>"#));
        for hour in 0..24 {
            let level = cells[day_index * 24 + hour];
            let level = match level {
                '1' => '1',
                '2' => '2',
                '3' => '3',
                '4' => '4',
                _ => '0',
            };
            html.push_str(&format!(r#"<span class="heat-cell level-{level}"></span>"#));
        }
    }

    html
}

fn activity_rows(meta: &EmbedMetadata) -> Vec<ActivityRow> {
    let mut rows = Vec::new();

    for index in 1..=8 {
        let Some(trainer) = activity_metric(meta, &format!("Trainer {index}")) else {
            continue;
        };
        rows.push(ActivityRow {
            rank: activity_metric(meta, &format!("Rank {index}"))
                .unwrap_or_else(|| format!("#{index}")),
            trainer,
            viewer: activity_metric(meta, &format!("Viewer {index}"))
                .unwrap_or_else(|| "ID tracked".to_string()),
            club: activity_metric(meta, &format!("Club {index}"))
                .unwrap_or_else(|| "Club context".to_string()),
            facts: activity_metric(meta, &format!("Facts {index}"))
                .unwrap_or_else(|| "observed snapshots".to_string()),
            reason: activity_metric(meta, &format!("Reason {index}"))
                .unwrap_or_else(|| "snapshot context".to_string()),
            reason_class: reason_class(
                &activity_metric(meta, &format!("Reason Severity {index}"))
                    .unwrap_or_else(|| "medium".to_string()),
            )
            .to_string(),
            fan_gain: activity_metric(meta, &format!("Fan Gain {index}"))
                .unwrap_or_else(|| "tracked".to_string()),
            active: activity_metric(meta, &format!("Active {index}"))
                .unwrap_or_else(|| "observed".to_string()),
            careers_rate: activity_metric(meta, &format!("Careers/hr {index}"))
                .unwrap_or_else(|| "rate".to_string()),
            score: activity_metric(meta, &format!("Score {index}"))
                .unwrap_or_else(|| "-".to_string()),
            score_band: activity_metric(meta, &format!("Score Band {index}"))
                .unwrap_or_else(|| "Score".to_string()),
            score_class: score_class(
                &activity_metric(meta, &format!("Score Class {index}"))
                    .unwrap_or_else(|| "watch".to_string()),
            )
            .to_string(),
        });
    }

    if rows.is_empty() {
        return fallback_rows();
    }

    rows
}

fn fallback_rows() -> Vec<ActivityRow> {
    [
        (
            "#1",
            "Observed trainer",
            "ID tracked",
            "Circle context",
            "1d observed",
            "short-career signal",
            "high",
            "fan gain",
            "active",
            "rate",
            "-",
            "Review",
            "score-high",
        ),
        (
            "#2",
            "Activity report",
            "ID tracked",
            "Top 100 club",
            "snapshots",
            "fan-gain trend",
            "medium",
            "tracked",
            "duration",
            "careers",
            "-",
            "Context",
            "score-elevated",
        ),
        (
            "#3",
            "Snapshot row",
            "ID tracked",
            "Club rank",
            "careers",
            "login changes",
            "low",
            "observed",
            "window",
            "rate",
            "-",
            "Signal",
            "score-watch",
        ),
        (
            "#4",
            "Baseline account",
            "ID tracked",
            "Public club",
            "0+ score",
            "below threshold",
            "info",
            "context",
            "time",
            "pace",
            "-",
            "Low",
            "score-low",
        ),
        (
            "#5",
            "Observed account",
            "ID tracked",
            "Club snapshot",
            "days observed",
            "activity context",
            "info",
            "tracked",
            "active",
            "rate",
            "-",
            "Review",
            "score-low",
        ),
        (
            "#6",
            "Fan-gain row",
            "ID tracked",
            "Top 100 club",
            "snapshot data",
            "rate context",
            "medium",
            "fan gain",
            "window",
            "pace",
            "-",
            "Context",
            "score-elevated",
        ),
        (
            "#7",
            "Career signal",
            "ID tracked",
            "Public club",
            "careers",
            "training pattern",
            "low",
            "observed",
            "time",
            "rate",
            "-",
            "Signal",
            "score-watch",
        ),
        (
            "#8",
            "Snapshot account",
            "ID tracked",
            "Circle context",
            "activity",
            "baseline",
            "info",
            "tracked",
            "active",
            "pace",
            "-",
            "Low",
            "score-low",
        ),
    ]
    .into_iter()
    .map(
        |(
            rank,
            trainer,
            viewer,
            club,
            facts,
            reason,
            reason_class_name,
            fan_gain,
            active,
            careers_rate,
            score,
            score_band,
            score_class_name,
        )| ActivityRow {
            rank: rank.to_string(),
            trainer: trainer.to_string(),
            viewer: viewer.to_string(),
            club: club.to_string(),
            facts: facts.to_string(),
            reason: reason.to_string(),
            reason_class: reason_class(reason_class_name).to_string(),
            fan_gain: fan_gain.to_string(),
            active: active.to_string(),
            careers_rate: careers_rate.to_string(),
            score: score.to_string(),
            score_band: score_band.to_string(),
            score_class: score_class(score_class_name).to_string(),
        },
    )
    .collect()
}

fn reason_class(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "critical" => "reason-critical",
        "high" => "reason-high",
        "low" => "reason-low",
        "info" => "reason-info",
        _ => "reason-medium",
    }
}

fn score_class(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "critical" | "score-critical" => "score-critical",
        "high" | "score-high" => "score-high",
        "elevated" | "score-elevated" => "score-elevated",
        "low" | "score-low" => "score-low",
        _ => "score-watch",
    }
}

fn activity_metric(meta: &EmbedMetadata, label: &str) -> Option<String> {
    meta.metrics
        .iter()
        .find(|metric| metric.label.eq_ignore_ascii_case(label))
        .map(|metric| metric.value.clone())
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    r#"<div class="visual-panel activity-visual">
        <div class="activity-summary-row">
          <span class="activity-score">99</span>
          <div><b>Activity reports</b><small>snapshot cadence and fan gain signals</small></div>
        </div>
        <div class="activity-row severity-high"><span></span><b>Short Careers</b><strong>score</strong></div>
        <div class="activity-row severity-watch"><span></span><b>Fan Gain</b><strong>trend</strong></div>
        <div class="activity-row severity-low"><span></span><b>Login Changes</b><strong>trace</strong></div>
        <div class="activity-row severity-info"><span></span><b>Club Context</b><strong>rank</strong></div>
      </div>"#
        .to_string()
}
