use crate::embed::{embed_class_list, EmbedMetadata};

use super::{
    html_escape, js_number_array, js_string_array, metric_value, parse_display_number,
    truncate_chars,
};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-statistics",
    render_visual,
};

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none()
        && super::canonical_path_matches(&meta.canonical_url, "/tools/statistics")
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let title_text = match super::display_title(&meta.title).as_str() {
        "Statistics" => "Team Stadium Statistics".to_string(),
        title => title.to_string(),
    };
    let title = html_escape(&truncate_chars(&title_text, 44));
    let dataset =
        metric_value(&meta.metrics, &["Dataset"]).unwrap_or_else(|| "Latest dataset".to_string());
    let trained_umas =
        metric_value(&meta.metrics, &["Trained Umas"]).unwrap_or_else(|| "Live".to_string());
    let trainers = metric_value(&meta.metrics, &["Trainers"]).unwrap_or_else(|| "Live".to_string());
    let scope =
        metric_value(&meta.metrics, &["Statistics Scope"]).unwrap_or_else(|| "Overall".to_string());
    let scope_short = metric_value(&meta.metrics, &["Statistics Scope Short"])
        .unwrap_or_else(|| "Overall".to_string());
    let generated =
        metric_value(&meta.metrics, &["Generated"]).unwrap_or_else(|| "Snapshot".to_string());
    let generated_display = generated
        .trim()
        .strip_prefix("Updated ")
        .unwrap_or_else(|| generated.trim());
    let class_rows = render_metric_distribution_rows(
        meta,
        &[
            "Class 6", "Class 5", "Class 4", "Class 3", "Class 2", "Class 1",
        ],
        "class",
    );
    let deck_rows = render_deck_rows(meta);
    let umas = render_ranked_metric_rows(meta, "Uma", "uma");
    let supports = render_ranked_metric_rows(meta, "Support", "support");
    let brand = super::render_brand_corner();
    let brand_css = super::brand_corner_css();
    let chart_js = super::chart_js();
    let charts = render_statistics_charts(meta);

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
      --surface-soft: rgba(255, 255, 255, 0.026);
      --surface-panel: rgba(12, 12, 12, 0.78);
      --border-subtle: rgba(255, 255, 255, 0.07);
      --border-strong: rgba(255, 255, 255, 0.12);
      --text-primary: #ffffff;
      --text-secondary: rgba(255, 255, 255, 0.72);
      --text-muted: rgba(255, 255, 255, 0.52);
      --text-disabled: rgba(255, 255, 255, 0.36);
      --accent-primary: #64b5f6;
      --accent-secondary: #81c784;
      --accent-warning: #ffb74d;
      --accent-purple: #ba68c8;
      --accent-pink: #f06292;
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

    .statistics-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 88px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 16% 18%, rgba(100, 181, 246, 0.14), transparent 330px),
        radial-gradient(circle at 76% 20%, rgba(186, 104, 200, 0.11), transparent 320px),
        radial-gradient(circle at 44% 88%, rgba(129, 199, 132, 0.1), transparent 360px),
        linear-gradient(180deg, rgba(255, 183, 77, 0.035), transparent 34%),
        var(--bg-primary);
    }}

    .statistics-card::before {{
      content: "";
      position: absolute;
      inset: 88px 0 0;
      background:
        linear-gradient(rgba(255, 255, 255, 0.024) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.018) 1px, transparent 1px);
      background-size: 64px 64px;
      opacity: 0.5;
      pointer-events: none;
    }}

    .statistics-header {{
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 32px;
      align-items: center;
      min-width: 0;
      padding: 14px 48px 10px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.09), rgba(186, 104, 200, 0.06), rgba(255, 183, 77, 0.045)),
        rgba(255, 255, 255, 0.012);
    }}

    .header-copy {{
      display: grid;
      gap: 6px;
      min-width: 0;
    }}

    .statistics-title {{
      margin: 0;
      background: linear-gradient(45deg, var(--accent-primary), var(--accent-purple) 54%, var(--accent-warning));
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      font-size: 39px;
      font-weight: 760;
      letter-spacing: 0;
      line-height: 0.98;
    }}

    .statistics-subline {{
      margin: 0;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 680;
      line-height: 1;
      text-transform: uppercase;
    }}

    .statistics-content {{
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-rows: 50px minmax(0, 232px) minmax(0, 198px);
      gap: 10px;
      min-height: 0;
      padding: 14px 48px 18px;
    }}

    .dataset-strip {{
      display: grid;
      grid-template-columns: 1.2fr 1fr 1fr 1fr;
      gap: 10px;
      min-width: 0;
    }}

    .dataset-cell {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: center;
      gap: 10px;
      min-width: 0;
      padding: 0 13px;
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.026);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.035);
    }}

    .dataset-cell span {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 700;
      line-height: 1;
      text-transform: uppercase;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .dataset-cell b {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 16px;
      font-weight: 700;
      line-height: 1;
      text-align: right;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .dataset-cell.dataset b {{
      color: var(--accent-warning);
    }}

    .chart-row {{
      display: grid;
      grid-template-columns: 306px minmax(0, 1fr) 272px;
      gap: 12px;
      min-height: 0;
    }}

    .leader-row {{
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 12px;
      min-height: 0;
    }}

    .stat-panel,
    .leader-panel {{
      min-width: 0;
      min-height: 0;
      overflow: hidden;
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      background:
        linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.012)),
        var(--surface-panel);
    }}

    .stat-panel {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      gap: 8px;
      padding: 12px;
    }}

    .leader-panel {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      padding: 9px 12px 8px;
    }}

    .panel-head {{
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 12px;
      min-width: 0;
    }}

    .panel-head h2 {{
      overflow: hidden;
      margin: 0;
      color: var(--text-primary);
      font-size: 17px;
      font-weight: 760;
      line-height: 1.05;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .panel-head span {{
      flex: 0 0 auto;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 700;
      line-height: 1;
      text-transform: uppercase;
    }}

    .class-layout {{
      display: grid;
      grid-template-columns: 150px minmax(0, 1fr);
      align-items: center;
      gap: 12px;
      min-height: 0;
    }}

    .chart-box {{
      position: relative;
      min-width: 0;
      min-height: 0;
      padding: 4px;
      border-radius: 7px;
      background:
        linear-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px),
        rgba(255, 255, 255, 0.01);
      background-size: 100% 34px;
    }}

    .chart-box canvas {{
      display: block;
      width: 100%;
      height: 100%;
    }}

    .class-donut {{
      width: 150px;
      height: 150px;
      padding: 0;
      background: rgba(255, 255, 255, 0.012);
    }}

    .distribution-list {{
      display: grid;
      gap: 6px;
      min-height: 0;
    }}

    .dist-row {{
      display: grid;
      grid-template-columns: 10px minmax(0, 1fr) 44px;
      align-items: center;
      gap: 7px;
      min-width: 0;
      color: var(--text-secondary);
      font-size: 11px;
      font-weight: 760;
      line-height: 1;
    }}

    .dist-row b,
    .rank-row b {{
      overflow: hidden;
      color: var(--text-primary);
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .dist-dot {{
      width: 9px;
      height: 9px;
      border-radius: 999px;
      background: var(--dot-color);
    }}

    .dist-row span:last-child {{
      color: var(--text-secondary);
      font-size: 10px;
      font-weight: 700;
      text-align: right;
      font-variant-numeric: tabular-nums;
    }}

    .dist-bar,
    .rank-bar {{
      height: 7px;
      overflow: hidden;
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.065);
    }}

    .dist-bar i,
    .rank-bar i {{
      display: block;
      width: var(--bar-width);
      height: 100%;
      border-radius: inherit;
      background: linear-gradient(90deg, var(--bar-start), var(--bar-end));
    }}

    .rank-bar i {{
      margin-left: auto;
      background: linear-gradient(270deg, var(--bar-start), var(--bar-end));
    }}

    .dist-row.class {{
      --bar-start: var(--accent-primary);
      --bar-end: var(--accent-purple);
    }}

    .dist-row.class:nth-child(1) {{ --dot-color: #64b5f6; }}
    .dist-row.class:nth-child(2) {{ --dot-color: #81c784; }}
    .dist-row.class:nth-child(3) {{ --dot-color: #ffb74d; }}
    .dist-row.class:nth-child(4) {{ --dot-color: #ba68c8; }}
    .dist-row.class:nth-child(5) {{ --dot-color: #f06292; }}
    .dist-row.class:nth-child(6) {{ --dot-color: #26a69a; }}

    .deck-list {{
      display: grid;
      gap: 2px;
      min-height: 0;
      align-content: start;
    }}

    .deck-row {{
      display: grid;
      grid-template-columns: 24px minmax(0, 1fr) 82px 54px;
      align-items: center;
      gap: 4px;
      min-width: 0;
      min-height: 24px;
      padding: 2px 5px;
      border: 0;
      border-radius: 0;
      background: transparent;
    }}

    .deck-rank {{
      color: var(--text-disabled);
      font-size: 9px;
      font-weight: 720;
      line-height: 1;
      text-align: right;
      font-variant-numeric: tabular-nums;
    }}

    .deck-chips {{
      display: flex;
      flex-wrap: nowrap;
      gap: 1px;
      min-width: 0;
    }}

    .deck-chip {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 20px;
      height: 20px;
      padding: 0;
      border: 0;
      border-radius: 0;
      background: transparent;
      color: var(--chip-text);
      font-weight: 700;
      line-height: 1;
      text-transform: uppercase;
    }}

    .deck-chip img {{
      width: 100%;
      height: 100%;
      object-fit: contain;
      flex: 0 0 auto;
      filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.55));
    }}

    .deck-mini-bar {{
      display: block;
      height: 7px;
      overflow: hidden;
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.07);
    }}

    .deck-mini-bar i {{
      display: block;
      margin-left: auto;
      width: var(--bar-width);
      height: 100%;
      border-radius: inherit;
      background: linear-gradient(270deg, rgba(100, 181, 246, 0.9), rgba(38, 166, 154, 0.9));
      box-shadow: 0 0 14px rgba(100, 181, 246, 0.2);
    }}

    .deck-chip b {{
      color: var(--chip-text);
      font-size: 8px;
      font-weight: 700;
      line-height: 1;
      font-variant-numeric: tabular-nums;
    }}

    .deck-chip small {{
      color: var(--chip-text);
      font-size: 7px;
      font-weight: 700;
      line-height: 1;
      letter-spacing: 0;
    }}

    .deck-chip.speed {{ --chip-border: rgba(100, 181, 246, 0.55); --chip-bg: rgba(100, 181, 246, 0.16); --chip-text: #8ac8ff; }}
    .deck-chip.stamina {{ --chip-border: rgba(129, 199, 132, 0.55); --chip-bg: rgba(129, 199, 132, 0.15); --chip-text: #9be09f; }}
    .deck-chip.power {{ --chip-border: rgba(255, 183, 77, 0.55); --chip-bg: rgba(255, 183, 77, 0.15); --chip-text: #ffc66c; }}
    .deck-chip.guts {{ --chip-border: rgba(186, 104, 200, 0.58); --chip-bg: rgba(186, 104, 200, 0.16); --chip-text: #d889e4; }}
    .deck-chip.wisdom {{ --chip-border: rgba(38, 166, 154, 0.58); --chip-bg: rgba(38, 166, 154, 0.16); --chip-text: #58d1c6; }}
    .deck-chip.friend {{ --chip-border: rgba(240, 98, 146, 0.58); --chip-bg: rgba(240, 98, 146, 0.16); --chip-text: #ff8ab5; }}
    .deck-chip.group {{ --chip-border: rgba(255, 255, 255, 0.22); --chip-bg: rgba(255, 255, 255, 0.08); --chip-text: rgba(255, 255, 255, 0.78); }}

    .deck-value {{
      display: grid;
      gap: 2px;
      min-width: 0;
      text-align: right;
    }}

    .deck-value b {{
      color: var(--accent-warning);
      font-size: 11px;
      font-weight: 700;
      line-height: 1;
      font-variant-numeric: tabular-nums;
    }}

    .deck-value small {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 7px;
      font-weight: 680;
      line-height: 1;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .ranked-list {{
      display: grid;
      gap: 4px;
      align-content: start;
      min-height: 0;
      padding-top: 7px;
    }}

    .rank-row {{
      display: grid;
      grid-template-columns: 24px 32px minmax(0, 1fr) 104px 66px;
      align-items: center;
      gap: 8px;
      min-width: 0;
      min-height: 28px;
      color: var(--text-secondary);
      font-size: 11px;
      font-weight: 680;
      line-height: 1;
    }}

    .rank-row .rank {{
      color: var(--text-disabled);
      font-size: 10px;
      font-weight: 720;
      text-align: right;
    }}

    .rank-thumb {{
      position: relative;
      display: grid;
      place-items: center;
      width: 32px;
      height: 32px;
      overflow: hidden;
      border: 1px solid rgba(100, 181, 246, 0.35);
      border-radius: 7px;
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.18), rgba(255, 183, 77, 0.1)),
        rgba(255, 255, 255, 0.04);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.09);
    }}

    .rank-row.uma .rank-thumb.has-image {{
      overflow: hidden;
      border-color: transparent;
      border-radius: 999px;
      background: transparent;
      box-shadow: none;
    }}

    .rank-thumb img {{
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      object-fit: cover;
      display: block;
    }}

    .rank-row.uma .rank-thumb img {{
      inset: 50% auto auto 50%;
      width: 130%;
      height: 130%;
      object-fit: contain;
      transform: translate(-50%, -50%);
    }}

    .rank-thumb-fallback {{
      color: var(--text-secondary);
      font-size: 10px;
      font-weight: 720;
      line-height: 1;
    }}

    .rank-copy {{
      display: grid;
      gap: 2px;
      min-width: 0;
    }}

    .rank-copy b {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 11px;
      font-weight: 760;
      line-height: 1.05;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .rank-copy small {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 680;
      line-height: 1;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .rank-row .value {{
      display: grid;
      gap: 2px;
      color: var(--accent-warning);
      font-size: 11px;
      font-weight: 700;
      text-align: right;
      font-variant-numeric: tabular-nums;
    }}

    .rank-row .value b {{
      color: var(--accent-warning);
      font-size: 11px;
      font-weight: 700;
      line-height: 1;
    }}

    .rank-row .value small {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 7px;
      font-weight: 680;
      line-height: 1;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .rank-row.uma {{
      --bar-start: var(--accent-secondary);
      --bar-end: var(--accent-warning);
    }}

    .rank-row.support {{
      --bar-start: var(--accent-primary);
      --bar-end: var(--accent-teal);
    }}

    .scenario-summary {{
      display: grid;
      align-items: stretch;
      min-height: 0;
      height: 100%;
    }}

    .scenario-chart-box {{
      width: 100%;
      height: 100%;
      min-height: 0;
      padding: 3px 0 0;
    }}
{brand_css}
  </style>
  <script>{chart_js}</script>
</head>
<body class="embed-card-page {class_list} card-view-statistics">
  <main class="statistics-card {class_list} card-view-statistics">
    <header class="statistics-header">
      <div class="header-copy">
        <h1 class="statistics-title">{title}</h1>
        <p class="statistics-subline">Latest Team Stadium aggregate snapshot / {scope}</p>
      </div>
      {brand}
    </header>

    <section class="statistics-content">
      <div class="dataset-strip">
        <div class="dataset-cell dataset"><span>Dataset</span><b>{dataset}</b></div>
        <div class="dataset-cell"><span>Trained Umas</span><b>{trained_umas}</b></div>
        <div class="dataset-cell"><span>Trainers</span><b>{trainers}</b></div>
        <div class="dataset-cell"><span>Updated</span><b>{generated}</b></div>
      </div>

      <section class="chart-row">
        <article class="stat-panel class-panel">
          <div class="panel-head"><h2>Team Class Split</h2><span>{scope_short}</span></div>
          <div class="class-layout">
            <div class="chart-box class-donut"><canvas id="classChart" width="150" height="150" aria-label="Team class distribution chart"></canvas></div>
            <div class="distribution-list">{class_rows}</div>
          </div>
        </article>
        <article class="stat-panel">
          <div class="panel-head"><h2>Popular Deck Builds</h2><span>{scope_short}</span></div>
          <div class="deck-list">{deck_rows}</div>
        </article>
        <article class="stat-panel">
          <div class="panel-head"><h2>Scenario Split</h2></div>
          <div class="scenario-summary">
            <div class="scenario-chart-box"><canvas id="scenarioChart" width="254" height="164" aria-label="Scenario split bar chart"></canvas></div>
          </div>
        </article>
      </section>

      <section class="leader-row">
        <article class="leader-panel">
          <div class="panel-head"><h2>Most Popular Uma Musume</h2><span>{scope_short}</span></div>
          <div class="ranked-list">{umas}</div>
        </article>
        <article class="leader-panel">
          <div class="panel-head"><h2>Most Used Support Cards</h2><span>{scope_short}</span></div>
          <div class="ranked-list">{supports}</div>
        </article>
      </section>
    </section>
    {charts}
  </main>
</body>
</html>
"#,
        class_list = class_list,
        title = title,
        brand = brand,
        brand_css = brand_css,
        chart_js = chart_js,
        charts = charts,
        class_rows = class_rows,
        deck_rows = deck_rows,
        umas = umas,
        supports = supports,
        scope = html_escape(&scope),
        scope_short = html_escape(&scope_short),
        dataset = html_escape(&truncate_chars(&dataset, 24)),
        trained_umas = html_escape(&trained_umas),
        trainers = html_escape(&trainers),
        generated = html_escape(&truncate_chars(generated_display, 18)),
    )
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    r#"<div class="visual-panel statistics-visual">
        <div class="chart-card-mini">
          <span class="chart-title-mini">Stat averages by class</span>
          <div class="chart-bars"><span style="height:72%"></span><span style="height:48%"></span><span style="height:86%"></span><span style="height:60%"></span><span style="height:38%"></span></div>
        </div>
        <div class="stat-icons"><b>SPD</b><b>STA</b><b>POW</b><b>GUT</b><b>WIT</b></div>
      </div>"#
        .to_string()
}

fn render_metric_distribution_rows(
    meta: &EmbedMetadata,
    labels: &[&str],
    class_name: &str,
) -> String {
    let rows = labels
        .iter()
        .filter_map(|label| {
            let value = metric_value(&meta.metrics, &[*label])?;
            Some((label.to_string(), value))
        })
        .collect::<Vec<_>>();

    rows.iter()
        .map(|row| {
            format!(
                r#"<div class="dist-row {class_name}"><span class="dist-dot"></span><b>{label}</b><span>{value}</span></div>"#,
                class_name = html_escape(class_name),
                label = html_escape(&row.0),
                value = html_escape(&row.1),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_deck_rows(meta: &EmbedMetadata) -> String {
    let asset_base = metric_value(&meta.metrics, &["Asset Base"])
        .unwrap_or_else(|| "https://uma.moe/assets".to_string());
    let rows = (1..=7)
        .filter_map(|index| {
            let composition = metric_value(&meta.metrics, &[&format!("Deck {index}")])?;
            let value = metric_value(&meta.metrics, &[&format!("Deck Value {index}")])?;
            let count = metric_value(&meta.metrics, &[&format!("Deck Count {index}")])
                .unwrap_or_else(|| "runs".to_string());
            let width = parse_display_number(&value).unwrap_or_default();
            Some((composition, value, count, width))
        })
        .collect::<Vec<_>>();
    let max = rows
        .iter()
        .map(|(_, _, _, width)| *width)
        .fold(0.0, f64::max)
        .max(1.0);

    rows.iter()
        .enumerate()
        .map(|(index, (composition, value, count, raw_width))| {
            let chips = composition
                .split('/')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(|part| render_deck_chip(part, &asset_base))
                .collect::<Vec<_>>()
                .join("");
            let width = ((*raw_width / max) * 100.0).clamp(4.0, 100.0);
            format!(
                r#"<div class="deck-row"><span class="deck-rank">#{rank}</span><div class="deck-chips">{chips}</div><span class="deck-mini-bar" style="--bar-width:{width}"><i></i></span><span class="deck-value"><b>{value}</b><small>{count}</small></span></div>"#,
                rank = index + 1,
                chips = chips,
                width = format!("{width:.0}%"),
                value = html_escape(&value),
                count = html_escape(&count),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_deck_chip(part: &str, asset_base: &str) -> String {
    let mut pieces = part.split_whitespace();
    let count = pieces
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 6);
    let stat = pieces.next().unwrap_or_default();
    let (class_name, icon) = match stat {
        "SPD" => ("speed", "speed"),
        "STA" => ("stamina", "stamina"),
        "POW" => ("power", "power"),
        "GUT" => ("guts", "guts"),
        "WIT" => ("wisdom", "wit"),
        "FRD" => ("friend", "friend"),
        _ => ("group", "group"),
    };

    let image = super::asset_url(asset_base, &format!("/images/icon/stats/{icon}.webp"));
    (0..count)
        .map(|_| {
            format!(
                r#"<span class="deck-chip {class_name}"><img src="{image}" alt="" onerror="this.style.display='none'"></span>"#,
                class_name = html_escape(class_name),
                image = html_escape(&image),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_ranked_metric_rows(meta: &EmbedMetadata, prefix: &str, class_name: &str) -> String {
    let asset_base = metric_value(&meta.metrics, &["Asset Base"])
        .unwrap_or_else(|| "https://uma.moe/assets".to_string());
    let rows = (1..=5)
        .filter_map(|index| {
            let label = metric_value(&meta.metrics, &[&format!("{prefix} {index}")])?;
            let value = metric_value(&meta.metrics, &[&format!("{prefix} Value {index}")])?;
            let count = metric_value(&meta.metrics, &[&format!("{prefix} Count {index}")]);
            let id = metric_value(&meta.metrics, &[&format!("{prefix} Id {index}")]);
            let detail = metric_value(&meta.metrics, &[&format!("{prefix} Detail {index}")])
                .unwrap_or_else(|| {
                    if prefix == "Uma" {
                        String::new()
                    } else {
                        "support card".to_string()
                    }
                });
            let width = parse_display_number(&value).unwrap_or_default();
            Some((label, value, count, id, detail, width))
        })
        .collect::<Vec<_>>();
    let max = rows
        .iter()
        .map(|(_, _, _, _, _, width)| *width)
        .fold(0.0, f64::max)
        .max(1.0);

    rows.iter()
        .enumerate()
        .map(|(index, (label, value, count, id, detail, raw_width))| {
            let width = ((*raw_width / max) * 100.0).clamp(4.0, 100.0);
            let thumb = render_rank_thumb(id.as_deref(), label, prefix, &asset_base);
            let count = count.as_deref().unwrap_or("");
            let detail = if detail.trim().is_empty() {
                String::new()
            } else {
                format!(r#"<small>{}</small>"#, html_escape(detail))
            };
            format!(
                r#"<div class="rank-row {class_name}"><span class="rank">#{rank}</span>{thumb}<span class="rank-copy"><b>{label}</b>{detail}</span><span class="rank-bar" style="--bar-width:{width}"><i></i></span><span class="value"><b>{value}</b><small>{count}</small></span></div>"#,
                class_name = html_escape(class_name),
                rank = index + 1,
                thumb = thumb,
                label = html_escape(label),
                detail = detail,
                width = format!("{width:.0}%"),
                value = html_escape(value),
                count = html_escape(count),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_rank_thumb(id: Option<&str>, label: &str, prefix: &str, asset_base: &str) -> String {
    let Some(id) = id.filter(|id| !id.trim().is_empty()) else {
        return format!(
            r#"<span class="rank-thumb"><span class="rank-thumb-fallback">{}</span></span>"#,
            html_escape(&initials(label))
        );
    };
    let path = if prefix == "Uma" {
        format!("/images/character_stand/chara_stand_{id}.webp")
    } else {
        format!("/images/support_card/half/support_card_s_{id}.webp")
    };
    let image = super::asset_url(asset_base, &path);

    format!(
        r#"<span class="rank-thumb has-image"><img src="{image}" alt="" onerror="this.closest('.rank-thumb').innerHTML='<span class=&quot;rank-thumb-fallback&quot;>{fallback}</span>'"></span>"#,
        image = html_escape(&image),
        fallback = html_escape(&initials(label)),
    )
}

fn initials(label: &str) -> String {
    let mut initials = label
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();

    if initials.is_empty() {
        initials.push('?');
    }

    initials.to_ascii_uppercase()
}

fn render_statistics_charts(meta: &EmbedMetadata) -> String {
    let class_labels = ["C6", "C5", "C4", "C3", "C2", "C1"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let class_values = [
        "Class 6", "Class 5", "Class 4", "Class 3", "Class 2", "Class 1",
    ]
    .into_iter()
    .map(|label| {
        metric_value(&meta.metrics, &[label])
            .and_then(|value| parse_display_number(&value))
            .unwrap_or_default()
    })
    .collect::<Vec<_>>();
    let scenario_labels = ["URA", "Aoharu", "MANT"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let scenario_values = ["Scenario URA", "Scenario Aoharu", "Scenario MANT"]
        .into_iter()
        .map(|label| {
            metric_value(&meta.metrics, &[label])
                .and_then(|value| parse_display_number(&value))
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    format!(
        r#"<script>
        (() => {{
          if (!window.Chart) return;
          Chart.defaults.font.family = 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
          const baseOptions = {{
            responsive: false,
            animation: false,
            maintainAspectRatio: false,
            plugins: {{ legend: {{ display: false }}, tooltip: {{ enabled: false }} }}
          }};
          const classCanvas = document.getElementById('classChart');
          if (classCanvas) {{
            new Chart(classCanvas.getContext('2d'), {{
              type: 'doughnut',
              data: {{
                labels: {class_labels},
                datasets: [{{
                  data: {class_values},
                  backgroundColor: ['#64b5f6','#81c784','#ffb74d','#ba68c8','#f06292','#26a69a'],
                  borderColor: 'rgba(10,10,10,0.85)',
                  borderWidth: 3,
                  hoverOffset: 0
                }}]
              }},
              options: {{
                ...baseOptions,
                cutout: '62%',
                plugins: {{ legend: {{ display: false }}, tooltip: {{ enabled: false }} }}
              }}
            }});
          }}
          const scenarioCanvas = document.getElementById('scenarioChart');
          if (scenarioCanvas) {{
            const valueLabels = {scenario_values}.map((value) => `${{Number(value || 0).toFixed(1)}}%`);
            const scenarioValuePlugin = {{
              id: 'scenarioValueLabels',
              afterDatasetsDraw(chart) {{
                const {{ ctx, chartArea }} = chart;
                const meta = chart.getDatasetMeta(0);
                ctx.save();
                ctx.fillStyle = 'rgba(255,255,255,0.82)';
                ctx.font = '900 11px Inter, sans-serif';
                ctx.textBaseline = 'middle';
                meta.data.forEach((bar, index) => {{
                  const label = valueLabels[index] || '';
                  const metrics = ctx.measureText(label);
                  const x = Math.max(bar.x - metrics.width - 8, chartArea.left + 2);
                  ctx.fillText(label, x, bar.y);
                }});
                ctx.restore();
              }}
            }};
            new Chart(scenarioCanvas.getContext('2d'), {{
              type: 'bar',
              data: {{
                labels: {scenario_labels},
                datasets: [{{
                  data: {scenario_values},
                  backgroundColor: ['rgba(100,181,246,0.9)','rgba(129,199,132,0.9)','rgba(255,183,77,0.9)'],
                  borderColor: ['#64b5f6','#81c784','#ffb74d'],
                  borderWidth: 1,
                  borderRadius: 6,
                  borderSkipped: false,
                  barThickness: 22,
                  maxBarThickness: 22
                }}]
              }},
              plugins: [scenarioValuePlugin],
              options: {{
                ...baseOptions,
                indexAxis: 'y',
                scales: {{
                  x: {{
                    min: 0,
                    max: 100,
                    reverse: true,
                    grid: {{ display: false, drawTicks: false }},
                    border: {{ display: false }},
                    ticks: {{ display: false }}
                  }},
                  y: {{
                    position: 'right',
                    grid: {{ display: false }},
                    border: {{ display: false }},
                    ticks: {{
                      color: 'rgba(255,255,255,0.76)',
                      font: {{ size: 11, weight: 900 }},
                      padding: 6
                    }}
                  }}
                }},
                layout: {{ padding: {{ top: 2, right: 0, bottom: 0, left: 2 }} }},
                plugins: {{ legend: {{ display: false }}, tooltip: {{ enabled: false }} }}
              }}
            }});
          }}
        }})();
        </script>"#,
        class_labels = js_string_array(&class_labels),
        class_values = js_number_array(&class_values),
        scenario_labels = js_string_array(&scenario_labels),
        scenario_values = js_number_array(&scenario_values),
    )
}
