use serde::{Deserialize, Serialize};

use crate::embed::{embed_class_list, EmbedMetadata};

use super::{asset_url, display_title, html_escape, metric_value, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-club",
    render_visual,
};

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none() && super::canonical_path(&meta.canonical_url).starts_with("/circles/")
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let club = display_title(&meta.title);
    let rank = metric_value(&meta.metrics, &["Rank"]).unwrap_or_else(|| "#--".to_string());
    let members = metric_value(&meta.metrics, &["Members"]).unwrap_or_else(|| "--".to_string());
    let points =
        metric_value(&meta.metrics, &["Points"]).unwrap_or_else(|| "fan progress".to_string());
    let leader =
        metric_value(&meta.metrics, &["Leader"]).unwrap_or_else(|| "Public profile".to_string());
    let join = metric_value(&meta.metrics, &["Join"]).unwrap_or_else(|| "Approval".to_string());
    let policy =
        metric_value(&meta.metrics, &["Policy"]).unwrap_or_else(|| "Playstyle".to_string());
    let comment = metric_value(&meta.metrics, &["Comment"])
        .unwrap_or_else(|| "No public club comment available.".to_string());
    let (progress_secondary_label, progress_secondary_value, progress_secondary_class) =
        if let Some(last_month_points) = metric_value(&meta.metrics, &["Last Month Points"]) {
            ("Last Month", last_month_points, "previous")
        } else if let Some(today_gain) = metric_value(&meta.metrics, &["Today Gain"]) {
            ("Today Gain", today_gain, "gain")
        } else if let Some(live_points) = metric_value(&meta.metrics, &["Live Points"]) {
            ("Live Points", live_points, "live")
        } else {
            ("Current", points.clone(), "")
        };
    let club_rank =
        metric_value(&meta.metrics, &["Club Rank"]).unwrap_or_else(|| "Rank".to_string());
    let asset_base = metric_value(&meta.metrics, &["Asset Base"])
        .unwrap_or_else(|| "https://uma.moe/assets".to_string());
    let rank_emblem = render_club_rank_emblem(
        &asset_base,
        &club_rank,
        metric_value(&meta.metrics, &["Club Rank Id"]).as_deref(),
    );
    let needed =
        metric_value(&meta.metrics, &["Needed"]).unwrap_or_else(|| "Next tier".to_string());
    let buffer = metric_value(&meta.metrics, &["Buffer"]).unwrap_or_else(|| "Safe".to_string());
    let needed_delta = metric_value(&meta.metrics, &["Needed Delta"]);
    let buffer_delta = metric_value(&meta.metrics, &["Buffer Delta"]);
    let current_daily_avg = metric_value(
        &meta.metrics,
        &["Current Daily Avg", "Monthly Daily Avg", "Daily Avg"],
    )
    .unwrap_or_else(|| "--".to_string());
    let current_weekly_avg = metric_value(
        &meta.metrics,
        &[
            "Current Weekly Avg",
            "Current 7d Avg",
            "Weekly Avg",
            "7d Avg",
        ],
    )
    .unwrap_or_else(|| "--".to_string());
    let last_month_daily_avg = metric_value(
        &meta.metrics,
        &["Last Month Daily Avg", "Previous Daily Avg"],
    )
    .unwrap_or_else(|| "--".to_string());
    let last_month_weekly_avg = metric_value(
        &meta.metrics,
        &[
            "Last Month Weekly Avg",
            "Last Month 7d Avg",
            "Previous 7d Avg",
            "Previous Weekly Avg",
        ],
    )
    .unwrap_or_else(|| "--".to_string());
    let club_rank_id = metric_value(&meta.metrics, &["Club Rank Id"])
        .and_then(|rank_id| rank_id.trim().parse::<i64>().ok());
    let lower_cutoff_rank = metric_value(&meta.metrics, &["Lower Cutoff Rank", "Max Rank"]);
    let upper_cutoff_rank = metric_value(&meta.metrics, &["Upper Cutoff Rank", "Min Rank"]);
    let cutoff_rail = render_cutoff_rail(
        &asset_base,
        club_rank_id,
        &rank,
        &buffer,
        buffer_delta.as_deref(),
        &needed,
        needed_delta.as_deref(),
        lower_cutoff_rank.as_deref(),
        upper_cutoff_rank.as_deref(),
    );
    let brand = super::render_brand_corner();
    let brand_css = super::brand_corner_css();
    let chart_js = super::chart_js();
    let charts = render_club_charts(&meta.metrics);
    let member_legend = render_member_gain_legend(&meta.metrics);
    let member_period = metric_value(&meta.metrics, &["Member Gain Period"])
        .unwrap_or_else(|| "member gains".to_string());
    let progress_summary = render_progress_summary(&meta.metrics);

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
      --surface-2: rgba(255, 255, 255, 0.05);
      --surface-3: rgba(255, 255, 255, 0.08);
      --border-subtle: rgba(255, 255, 255, 0.07);
      --border-primary: rgba(255, 255, 255, 0.12);
      --text-primary: #ffffff;
      --text-secondary: rgba(255, 255, 255, 0.72);
      --text-muted: rgba(255, 255, 255, 0.5);
      --text-disabled: rgba(255, 255, 255, 0.32);
      --accent-primary: #64b5f6;
      --accent-secondary: #81c784;
      --accent-warning: #ffb74d;
      --accent-error: #ef5350;
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

    .club-card {{
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 88px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 14% 0%, rgba(100, 181, 246, 0.11), transparent 330px),
        radial-gradient(circle at 84% 0%, rgba(129, 199, 132, 0.1), transparent 320px),
        var(--bg-secondary);
    }}

    .club-header {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 22px;
      min-width: 0;
      padding: 14px 48px 10px;
      border-bottom: 1px solid var(--border-subtle);
      background: var(--surface-1);
    }}

    .header-main {{
      display: flex;
      align-items: center;
      gap: 15px;
      min-width: 0;
    }}

    .back-token {{
      display: grid;
      place-items: center;
      width: 36px;
      height: 36px;
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      color: var(--text-muted);
      font-size: 26px;
      line-height: 1;
    }}

    .club-rank-emblem {{
      display: grid;
      place-items: center;
      width: 58px;
      height: 58px;
      border: 0;
      background: transparent;
      color: #90caf9;
      font-size: 18px;
      font-weight: 900;
      box-shadow: 0 10px 24px rgba(0, 0, 0, 0.2);
    }}

    .club-rank-emblem img {{
      display: block;
      width: 58px;
      height: 58px;
      object-fit: contain;
      filter: drop-shadow(0 7px 14px rgba(0, 0, 0, 0.38));
    }}

    .title-block {{
      display: grid;
      gap: 7px;
      min-width: 0;
    }}

    h1 {{
      margin: 0;
      overflow: hidden;
      color: var(--text-primary);
      font-size: 26px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .badges {{
      display: flex;
      align-items: center;
      gap: 7px;
      min-width: 0;
    }}

    .badge {{
      display: inline-flex;
      align-items: center;
      height: 25px;
      padding: 0 10px;
      border: 1px solid var(--badge-border);
      border-radius: 6px;
      background: var(--badge-bg);
      color: var(--badge-color);
      font-size: 12px;
      font-weight: 850;
      line-height: 1;
      white-space: nowrap;
    }}

    .rank-badge {{
      --badge-border: rgba(100, 181, 246, 0.24);
      --badge-bg: rgba(100, 181, 246, 0.13);
      --badge-color: #90caf9;
    }}

    .members-badge {{
      --badge-border: var(--border-subtle);
      --badge-bg: var(--surface-2);
      --badge-color: var(--text-secondary);
    }}

    .join-open {{
      --badge-border: rgba(129, 199, 132, 0.25);
      --badge-bg: rgba(129, 199, 132, 0.12);
      --badge-color: #81c784;
    }}

    .join-approval {{
      --badge-border: rgba(255, 183, 77, 0.26);
      --badge-bg: rgba(255, 183, 77, 0.12);
      --badge-color: #ffb74d;
    }}

    .join-closed {{
      --badge-border: rgba(239, 83, 80, 0.26);
      --badge-bg: rgba(239, 83, 80, 0.12);
      --badge-color: #ef5350;
    }}

    .policy-badge {{
      --badge-border: rgba(255, 183, 77, 0.28);
      --badge-bg: rgba(255, 183, 77, 0.11);
      --badge-color: #ffb74d;
    }}

    .header-actions {{
      display: flex;
      align-items: center;
      gap: 10px;
      flex: 0 0 auto;
    }}

    .live-refresh-bar,
    .month-nav,
    .export-btn {{
      display: inline-flex;
      align-items: center;
      height: 34px;
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      background: var(--surface-2);
      color: var(--text-secondary);
      font-size: 13px;
      font-weight: 750;
      white-space: nowrap;
    }}

    .live-refresh-bar {{
      gap: 7px;
      padding: 0 10px;
      border-color: rgba(244, 67, 54, 0.16);
      background: rgba(244, 67, 54, 0.06);
    }}

    .live-dot {{
      width: 7px;
      height: 7px;
      border-radius: 50%;
      background: #f44336;
      box-shadow: 0 0 7px rgba(244, 67, 54, 0.72);
    }}

    .live-label {{
      color: #ef5350;
      font-size: 11px;
      font-weight: 900;
      text-transform: uppercase;
    }}

    .month-nav {{
      gap: 12px;
      padding: 0 12px;
    }}

    .month-arrow {{
      color: var(--text-muted);
      font-size: 18px;
      line-height: 1;
    }}

    .export-btn {{
      gap: 7px;
      padding: 0 12px;
    }}

    .club-body {{
      display: grid;
      grid-template-rows: 206px minmax(0, 1fr);
      gap: 6px;
      min-height: 0;
      padding: 10px 30px 4px;
    }}

    .top-grid {{
      display: grid;
      grid-template-columns: 390px minmax(0, 1fr);
      gap: 16px;
      min-height: 0;
    }}

    .details-card {{
      min-width: 0;
      border: 1px solid var(--border-subtle);
      border-radius: 10px;
      background: var(--surface-1);
      overflow: hidden;
    }}

    .card-header {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      height: 36px;
      padding: 0 18px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.045);
    }}

    .card-header h2 {{
      margin: 0;
      color: var(--text-primary);
      font-size: 16px;
      font-weight: 800;
      line-height: 1;
    }}

    .card-content {{
      padding: 6px 16px 8px;
    }}

    .info-card .card-content {{
      display: grid;
      min-height: calc(100% - 36px);
    }}

    .club-profile {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      gap: 8px;
      min-height: 0;
    }}

    .profile-leader {{
      display: grid;
      gap: 3px;
      min-width: 0;
      padding: 6px 10px;
      border: 1px solid rgba(255, 255, 255, 0.045);
      border-radius: 7px;
      background: rgba(255, 255, 255, 0.024);
    }}

    .profile-leader .value {{
      font-size: 15px;
      font-weight: 900;
    }}

    .profile-leader .label {{
      font-size: 10px;
      line-height: 1;
    }}

    .club-comment {{
      min-height: 0;
      margin: 0;
      overflow: hidden;
      padding: 9px 10px;
      border: 1px solid rgba(255, 255, 255, 0.045);
      border-radius: 7px;
      background: rgba(0, 0, 0, 0.12);
      color: var(--text-secondary);
      font-size: 12px;
      font-weight: 650;
      line-height: 1.34;
    }}

    .club-comment b {{
      display: block;
      margin-bottom: 5px;
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }}

    .club-comment span {{
      overflow: hidden;
      display: -webkit-box;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 4;
    }}

    .info-metrics {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      grid-auto-rows: minmax(0, 1fr);
      gap: 6px;
      min-width: 0;
    }}

    .info-metric {{
      min-width: 0;
      display: grid;
      align-content: center;
      gap: 5px;
      padding: 8px 11px;
      border: 1px solid rgba(255, 255, 255, 0.045);
      border-radius: 7px;
      background: rgba(255, 255, 255, 0.022);
    }}

    .info-metric .label {{
      display: block;
      margin-bottom: 0;
    }}

    .info-row {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 14px;
      min-width: 0;
      padding-bottom: 6px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.045);
    }}

    .info-row:last-child {{
      padding-bottom: 0;
      border-bottom: 0;
    }}

    .label {{
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 750;
      text-transform: uppercase;
    }}

    .value {{
      min-width: 0;
      overflow: hidden;
      color: var(--text-primary);
      font-size: 14px;
      font-weight: 750;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .highlight {{
      color: var(--accent-primary);
      font-size: 16px;
      font-weight: 900;
    }}

    .cutoff-rail {{
      display: grid;
      align-items: center;
      gap: 6px;
      min-width: 0;
      min-height: 72px;
      margin: 0 10px;
      padding: 4px 10px 8px;
      border: 1px solid rgba(255, 255, 255, 0.052);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.024);
    }}

    .cutoff-rail.both {{
      grid-template-columns: 44px 8px minmax(0, 1fr) 88px minmax(0, 1fr) 8px 44px;
    }}

    .cutoff-rail.buffer-only {{
      grid-template-columns: 44px 8px minmax(0, 1fr) 88px minmax(0, 1fr) 8px 44px;
    }}

    .cutoff-rail.needed-only {{
      grid-template-columns: 44px 8px minmax(0, 1fr) 88px minmax(0, 1fr) 8px 44px;
    }}

    .cutoff-node {{
      display: grid;
      justify-items: center;
      gap: 4px;
      min-width: 0;
      color: var(--text-muted);
      text-align: center;
    }}

    .cutoff-icon {{
      display: grid;
      place-items: center;
      width: 30px;
      height: 30px;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 750;
    }}

    .cutoff-node.dim .cutoff-icon {{
      opacity: 0.58;
      filter: grayscale(0.1);
    }}

    .cutoff-node.active .cutoff-icon {{
      width: 44px;
      height: 44px;
    }}

    .cutoff-node.active {{
      gap: 6px;
    }}

    .cutoff-node.active .cutoff-rank {{
      color: var(--text-secondary);
    }}

    .cutoff-node.empty {{
      visibility: hidden;
    }}

    .cutoff-icon img {{
      display: block;
      width: 100%;
      height: 100%;
      object-fit: contain;
      filter: drop-shadow(0 4px 9px rgba(0, 0, 0, 0.36));
    }}

    .cutoff-rank {{
      overflow: hidden;
      max-width: 72px;
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 600;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .cutoff-arrow {{
      color: var(--text-muted);
      font-size: 18px;
      font-weight: 600;
      line-height: 1;
      text-align: center;
    }}

    .cutoff-arrow.empty {{
      visibility: hidden;
    }}

    .cutoff-metric {{
      display: grid;
      justify-items: center;
      gap: 2px;
      min-width: 0;
      text-align: center;
    }}

    .cutoff-label {{
      color: var(--text-disabled);
      font-size: 9px;
      font-weight: 600;
      line-height: 1;
      text-transform: uppercase;
    }}

    .cutoff-value {{
      overflow: visible;
      max-width: 100%;
      font-size: 19px;
      font-weight: 600;
      line-height: 1;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .cutoff-value.safe {{
      color: var(--accent-secondary);
    }}

    .cutoff-value.needed {{
      color: var(--accent-warning);
    }}

    .cutoff-value.muted {{
      color: var(--text-muted);
    }}

    .cutoff-delta {{
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 600;
      line-height: 1;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .cutoff-delta.up {{
      color: var(--accent-secondary);
    }}

    .cutoff-delta.down {{
      color: var(--accent-error);
    }}

    .rank-center {{
      display: grid;
      justify-items: center;
      gap: 4px;
    }}

    .rank-center .club-rank-emblem {{
      width: 42px;
      height: 42px;
      font-size: 14px;
    }}

    .rank-center .club-rank-emblem img {{
      width: 42px;
      height: 42px;
    }}

    .rank-label {{
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 600;
    }}

    .chart-card {{
      position: relative;
    }}

    .chart-card .card-header .label {{
      max-width: 460px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .progress-summary {{
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 6px;
      min-width: 0;
    }}

    .progress-chip {{
      display: inline-flex;
      align-items: center;
      gap: 5px;
      height: 21px;
      padding: 0 8px;
      border: 1px solid rgba(255, 255, 255, 0.065);
      border-radius: 6px;
      background: rgba(0, 0, 0, 0.16);
      color: var(--text-secondary);
      font-size: 10px;
      font-weight: 600;
      line-height: 1;
      white-space: nowrap;
    }}

    .progress-chip strong {{
      color: var(--text-primary);
      font-size: 11px;
      font-weight: 600;
    }}

    .progress-chip.gain strong {{
      color: var(--accent-secondary);
    }}

    .progress-value-row {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      align-items: stretch;
      gap: 10px;
      height: 80px;
      margin: 0 10px 6px;
    }}

    .progress-value-tile {{
      min-width: 0;
      display: grid;
      align-content: center;
      justify-items: center;
      gap: 5px;
      padding: 6px 10px;
      border: 1px solid rgba(255, 255, 255, 0.052);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.024);
    }}

    .progress-value-tile .label {{
      font-size: 10px;
      line-height: 1;
    }}

    .progress-value-tile > strong {{
      min-width: 0;
      overflow: visible;
      color: var(--accent-primary);
      font-size: 25px;
      font-weight: 600;
      line-height: 1;
      white-space: nowrap;
    }}

    .progress-value-tile.live strong {{
      color: var(--accent-secondary);
    }}

    .progress-value-tile.gain strong {{
      color: var(--accent-secondary);
    }}

    .progress-value-tile.previous strong {{
      color: var(--accent-warning);
    }}

    .tile-averages {{
      min-width: 0;
      width: min(100%, 270px);
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      align-items: center;
      justify-content: center;
      gap: 7px;
    }}

    .tile-average {{
      min-width: 0;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 4px;
      height: 14px;
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 600;
      line-height: 1;
      text-transform: uppercase;
    }}

    .tile-average > span {{
      display: block;
      line-height: 1;
      white-space: nowrap;
    }}

    .tile-average b {{
      display: block;
      color: var(--accent-secondary);
      font-size: 10px;
      font-weight: 700;
      line-height: 1;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .chart-canvas {{
      position: relative;
      height: 104px;
      margin: 0 10px 9px;
      border: 1px solid rgba(100, 181, 246, 0.08);
      border-radius: 8px;
      background:
        linear-gradient(rgba(255, 255, 255, 0.035) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.025) 1px, transparent 1px);
      background-size: 100% 32px, 78px 100%;
      overflow: hidden;
    }}

    .chart-canvas canvas {{
      display: block;
      width: 100%;
      height: 100%;
    }}

    .member-chart {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) 178px;
      gap: 5px;
      height: calc(100% - 26px);
      min-height: 0;
    }}

    .full-width-chart .card-header {{
      height: 26px;
      padding: 0 18px;
      border-bottom: 0;
    }}

    .member-chart .chart-canvas {{
      height: auto;
      min-height: 0;
      margin: -2px 0 0 18px;
      border: 0;
      background: transparent;
    }}

    .legend-list {{
      display: grid;
      grid-template-columns: minmax(0, 1fr);
      align-content: start;
      gap: 3px;
      min-height: 0;
      padding: 0 4px 0 0;
    }}

    .legend-item {{
      display: flex;
      align-items: center;
      gap: 5px;
      min-width: 0;
      height: 17px;
      padding: 0 5px;
      border-radius: 5px;
      background: rgba(0, 0, 0, 0.16);
      color: var(--text-secondary);
      font-size: 9px;
      font-weight: 750;
    }}

    .legend-item.empty {{
      color: var(--text-muted);
    }}

    .legend-name {{
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .legend-item b {{
      margin-left: auto;
      font-size: 8px;
      font-weight: 900;
      white-space: nowrap;
    }}

    .legend-item b.positive {{
      color: var(--accent-secondary);
    }}

    .legend-item b.negative {{
      color: var(--accent-error);
    }}

    .legend-item b.neutral {{
      color: var(--text-muted);
    }}

    .color-indicator {{
      width: 7px;
      height: 7px;
      flex-shrink: 0;
      border-radius: 2px;
      background: var(--legend-color);
    }}

    .prior-club-badge,
    .toggle-pill {{
      display: inline-flex;
      align-items: center;
      height: 28px;
      padding: 0 10px;
      border: 1px solid var(--border-subtle);
      border-radius: 7px;
      background: var(--surface-2);
      color: var(--text-secondary);
      font-size: 12px;
      font-weight: 800;
    }}

    .prior-club-badge {{
      border-color: rgba(76, 175, 80, 0.28);
      background: rgba(76, 175, 80, 0.08);
      color: var(--accent-secondary);
    }}

    .members-grid {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
      min-height: 0;
      overflow: hidden;
    }}

    .member-card {{
      display: grid;
      gap: 5px;
      min-width: 0;
      padding: 7px 8px;
      border: 1px solid var(--member-border);
      border-radius: 9px;
      background: var(--member-bg);
    }}

    .member-card.rank-1 {{
      --member-border: rgba(255, 215, 0, 0.42);
      --member-bg: linear-gradient(145deg, rgba(255, 215, 0, 0.06), rgba(255, 255, 255, 0.022));
      --member-rank: #ffd700;
    }}

    .member-card.rank-2 {{
      --member-border: rgba(144, 164, 174, 0.38);
      --member-bg: linear-gradient(145deg, rgba(144, 164, 174, 0.06), rgba(255, 255, 255, 0.022));
      --member-rank: #b0bec5;
    }}

    .member-card.rank-3 {{
      --member-border: rgba(205, 127, 50, 0.38);
      --member-bg: linear-gradient(145deg, rgba(205, 127, 50, 0.06), rgba(255, 255, 255, 0.022));
      --member-rank: #cd7f32;
    }}

    .member-header {{
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      padding-bottom: 4px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.055);
    }}

    .member-rank {{
      color: var(--member-rank);
      font-size: 18px;
      font-weight: 900;
    }}

    .name-block {{
      display: grid;
      gap: 2px;
      min-width: 0;
      flex: 1;
    }}

    .member-name {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 14px;
      font-weight: 850;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .member-id {{
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 750;
    }}

    .role-badge {{
      display: inline-flex;
      align-items: center;
      height: 20px;
      padding: 0 7px;
      border-radius: 5px;
      background: rgba(255, 167, 38, 0.13);
      color: #ffa726;
      font-size: 10px;
      font-weight: 900;
      text-transform: uppercase;
    }}

    .member-stats {{
      display: grid;
      gap: 2px;
    }}

    .stat-row {{
      display: flex;
      justify-content: space-between;
      gap: 10px;
      color: var(--text-secondary);
      font-size: 12px;
      font-weight: 700;
    }}

    .stat-row.highlight-stat {{
      margin: 1px -8px 0;
      padding: 4px 8px;
      border-top: 1px solid rgba(255, 255, 255, 0.055);
      border-bottom: 1px solid rgba(255, 255, 255, 0.055);
      background: rgba(255, 255, 255, 0.024);
      color: var(--accent-primary);
    }}

    .gain.positive {{
      color: var(--accent-secondary);
    }}

    .club-footer {{
      position: absolute;
      right: 28px;
      bottom: 12px;
      color: var(--text-disabled);
      font-size: 11px;
      font-weight: 750;
    }}
{brand_css}
  </style>
  <script>{chart_js}</script>
</head>
<body class="embed-card-page {class_list} card-view-club">
  <main class="club-card {class_list} card-view-club">
    <header class="club-header">
      <div class="header-main">
        {rank_emblem}
        <div class="title-block">
          <h1>{club}</h1>
          <div class="badges">
            <span class="badge rank-badge">Rank {rank}</span>
            <span class="badge members-badge">{members}/30</span>
            <span class="badge {join_class}">{join}</span>
            <span class="badge policy-badge">{policy}</span>
          </div>
        </div>
      </div>
      {brand}
    </header>

    <section class="club-body">
      <div class="top-grid">
        <article class="details-card info-card">
          <header class="card-header"><h2>Club Information</h2></header>
          <div class="card-content">
            <div class="club-profile">
              <div class="profile-leader"><span class="label">Leader</span><span class="value">{leader}</span></div>
              <p class="club-comment"><b>Comment</b><span>{comment}</span></p>
            </div>
          </div>
        </article>

        <article class="details-card chart-card">
          <header class="card-header"><h2>Club Progression</h2>{progress_summary}</header>
          <div class="progress-value-row">
            <div class="progress-value-tile"><span class="label">Monthly Fans</span><strong>{points}</strong><span class="tile-averages"><span class="tile-average"><span>Daily Avg</span><b>{current_daily_avg}</b></span><span class="tile-average"><span>Weekly Avg</span><b>{current_weekly_avg}</b></span></span></div>
            <div class="progress-value-tile {progress_secondary_class}"><span class="label">{progress_secondary_label}</span><strong>{progress_secondary_value}</strong><span class="tile-averages"><span class="tile-average"><span>Daily Avg</span><b>{last_month_daily_avg}</b></span><span class="tile-average"><span>Weekly Avg</span><b>{last_month_weekly_avg}</b></span></span></div>
          </div>
          {cutoff_rail}
        </article>
      </div>

      <article class="details-card full-width-chart">
        <header class="card-header"><h2>Member Gains</h2><span class="label">{member_period}</span></header>
        <div class="member-chart">
          <div class="chart-canvas"><canvas id="clubMemberChart" width="982" height="292" aria-label="Club member progression chart"></canvas></div>
          <div class="legend-list">
            {member_legend}
          </div>
        </div>
      </article>
    </section>
    {charts}
  </main>
</body>
</html>
"#,
        title = html_escape(&truncate_chars(&club, 60)),
        class_list = class_list,
        brand_css = brand_css,
        chart_js = chart_js,
        brand = brand,
        club = html_escape(&truncate_chars(&club, 42)),
        rank = html_escape(&rank),
        members = html_escape(&members),
        join = html_escape(&join),
        join_class = join_class(&join),
        leader = html_escape(&truncate_chars(&leader, 32)),
        comment = html_escape(&truncate_chars(&comment, 150)),
        points = html_escape(&points),
        progress_secondary_label = html_escape(progress_secondary_label),
        progress_secondary_value = html_escape(&progress_secondary_value),
        progress_secondary_class = html_escape(progress_secondary_class),
        current_daily_avg = html_escape(&current_daily_avg),
        current_weekly_avg = html_escape(&current_weekly_avg),
        last_month_daily_avg = html_escape(&last_month_daily_avg),
        last_month_weekly_avg = html_escape(&last_month_weekly_avg),
        rank_emblem = rank_emblem,
        cutoff_rail = cutoff_rail,
        policy = html_escape(&truncate_chars(&policy, 24)),
        progress_summary = progress_summary,
        member_period = html_escape(&member_period),
        member_legend = member_legend,
        charts = charts,
    )
}

fn render_progress_summary(metrics: &[crate::embed::EmbedMetric]) -> String {
    let chips = [
        ("Today", metric_value(metrics, &["Today Gain"]), "gain"),
        ("Yday", metric_value(metrics, &["Yesterday Rank"]), ""),
        ("Last", metric_value(metrics, &["Last Month Rank"]), ""),
        ("Prev", metric_value(metrics, &["Last Month Points"]), ""),
    ]
    .into_iter()
    .filter_map(|(label, value, class_name)| {
        let value = value?;
        let class_attr = if class_name.is_empty() {
            String::new()
        } else {
            format!(" {class_name}")
        };
        Some(format!(
            r#"<span class="progress-chip{class_attr}">{label}<strong>{value}</strong></span>"#,
            label = html_escape(label),
            value = html_escape(&truncate_chars(&value, 9)),
        ))
    })
    .take(3)
    .collect::<String>();

    if chips.is_empty() {
        r#"<span class="progress-summary"><span class="progress-chip">Progress<strong>tracked</strong></span></span>"#.to_string()
    } else {
        format!(r#"<span class="progress-summary">{chips}</span>"#)
    }
}

fn render_club_charts(metrics: &[crate::embed::EmbedMetric]) -> String {
    let member_gain = member_gain_payload(metrics);

    format!(
        r#"<script>
        (() => {{
          if (!window.Chart) return;
          const baseOptions = {{
            responsive: false,
            animation: false,
            maintainAspectRatio: false,
            plugins: {{ legend: {{ display: false }}, tooltip: {{ enabled: false }} }},
            scales: {{
              x: {{ grid: {{ display: false }}, ticks: {{ color: 'rgba(255,255,255,0.56)', font: {{ size: 10, weight: '800' }} }} }},
              y: {{ grid: {{ color: 'rgba(255,255,255,0.05)' }}, ticks: {{ display: false }} }}
            }}
          }};
          const memberCanvas = document.getElementById('clubMemberChart');
          if (memberCanvas) {{
            const memberLabels = {member_labels};
            const memberSeries = {member_series};
            const ctx = memberCanvas.getContext('2d');
            if (!memberLabels.length || !memberSeries.length) {{
              ctx.save();
              ctx.fillStyle = 'rgba(255,255,255,0.62)';
              ctx.font = '800 17px Inter, sans-serif';
              ctx.textAlign = 'center';
              ctx.fillText('No member gain history for this month', memberCanvas.width / 2, memberCanvas.height / 2);
              ctx.restore();
              return;
            }}
            const gainPalette = {member_palette};
            const compact = (value) => new Intl.NumberFormat('en', {{ notation: 'compact', maximumFractionDigits: 1 }}).format(value);
            new Chart(ctx, {{
              type: 'line',
              data: {{
                labels: memberLabels,
                datasets: memberSeries.map((member, index) => {{
                  const color = gainPalette[index % gainPalette.length];
                  return {{
                    label: member.name,
                    data: member.data,
                    borderColor: color,
                    backgroundColor: 'transparent',
                    borderWidth: index < 6 ? 1.9 : 1.05,
                    pointRadius: 0,
                    pointHoverRadius: 0,
                    tension: 0.24,
                    spanGaps: false
                  }};
                }})
              }},
              options: {{
                ...baseOptions,
                layout: {{ padding: {{ top: 0, right: 40, bottom: 0, left: 0 }} }},
                elements: {{ line: {{ capBezierPoints: true }} }},
                scales: {{
                  x: {{ grid: {{ display: false }}, ticks: {{ color: 'rgba(255,255,255,0.5)', font: {{ size: 10, weight: '800' }}, maxRotation: 0, autoSkip: true, maxTicksLimit: 9, align: 'inner' }} }},
                  y: {{
                    min: 0,
                    border: {{ display: false }},
                    grid: {{
                      color: (ctx) => ctx.tick && ctx.tick.value === ctx.scale.max ? 'transparent' : 'rgba(255,255,255,0.055)'
                    }},
                    ticks: {{ color: 'rgba(255,255,255,0.48)', font: {{ size: 10, weight: '800' }}, callback: compact, maxTicksLimit: 5 }}
                  }}
                }}
              }}
            }});
          }}
        }})();
        </script>"#,
        member_labels = member_gain.labels_json,
        member_series = member_gain.series_json,
        member_palette =
            serde_json::to_string(MEMBER_GAIN_COLORS).unwrap_or_else(|_| "[]".to_string()),
    )
}

const MEMBER_GAIN_COLORS: &[&str] = &[
    "#64b5f6", "#81c784", "#ffb74d", "#f06292", "#ba68c8", "#4dd0e1", "#ffd54f", "#90a4ae",
    "#ff8a65", "#7986cb", "#aed581", "#4fc3f7", "#ce93d8", "#a5d6a7", "#ffcc80", "#ef9a9a",
    "#80cbc4", "#b39ddb", "#fff176", "#bcaaa4", "#f48fb1", "#9fa8da", "#c5e1a5", "#ffab91",
    "#4db6ac", "#e6ee9c", "#b0bec5", "#fdd835", "#9575cd", "#26c6da",
];

#[derive(Debug, Deserialize, Serialize)]
struct MemberGainDataset {
    name: String,
    data: Vec<Option<f64>>,
    total: f64,
}

struct MemberGainPayload {
    labels_json: String,
    series_json: String,
    series: Vec<MemberGainDataset>,
}

fn member_gain_payload(metrics: &[crate::embed::EmbedMetric]) -> MemberGainPayload {
    let labels = metric_value(metrics, &["Member Gain Labels"])
        .and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
        .unwrap_or_default();
    let series = metric_value(metrics, &["Member Gain Series"])
        .and_then(|value| serde_json::from_str::<Vec<MemberGainDataset>>(&value).ok())
        .unwrap_or_default();

    MemberGainPayload {
        labels_json: serde_json::to_string(&labels).unwrap_or_else(|_| "[]".to_string()),
        series_json: serde_json::to_string(&series).unwrap_or_else(|_| "[]".to_string()),
        series,
    }
}

fn render_member_gain_legend(metrics: &[crate::embed::EmbedMetric]) -> String {
    let payload = member_gain_payload(metrics);
    if payload.series.is_empty() {
        return r#"<span class="legend-item empty"><i class="color-indicator" style="--legend-color:rgba(255,255,255,.22)"></i>No member gains</span>"#.to_string();
    }

    payload
        .series
        .iter()
        .take(15)
        .enumerate()
        .map(|(index, member)| {
            let color = MEMBER_GAIN_COLORS[index % MEMBER_GAIN_COLORS.len()];
            format!(
                r#"<span class="legend-item"><i class="color-indicator" style="--legend-color:{color}"></i><span class="legend-name">{name}</span><b class="{gain_class}">{total}</b></span>"#,
                name = html_escape(&truncate_chars(&member.name, 16)),
                gain_class = gain_class(member.total),
                total = html_escape(&signed_compact_display(member.total)),
            )
        })
        .collect::<String>()
}

fn gain_class(value: f64) -> &'static str {
    if value > 0.0 {
        "positive"
    } else if value < 0.0 {
        "negative"
    } else {
        "neutral"
    }
}

fn signed_compact_display(value: f64) -> String {
    let rounded = value.round() as i64;
    if rounded > 0 {
        format!("+{}", compact_display(rounded))
    } else {
        compact_display(rounded)
    }
}

fn compact_display(value: i64) -> String {
    let absolute = value.abs() as f64;
    let sign = if value < 0 { "-" } else { "" };

    if absolute >= 1_000_000_000.0 {
        format!("{sign}{:.1}B", absolute / 1_000_000_000.0)
    } else if absolute >= 1_000_000.0 {
        format!("{sign}{:.1}M", absolute / 1_000_000.0)
    } else if absolute >= 1_000.0 {
        format!("{sign}{:.1}K", absolute / 1_000.0)
    } else {
        value.to_string()
    }
}

fn render_visual(meta: &EmbedMetadata) -> String {
    let club = display_title(&meta.title);
    let rank = metric_value(&meta.metrics, &["Rank"]).unwrap_or_else(|| "#--".to_string());
    let members = metric_value(&meta.metrics, &["Members"]).unwrap_or_else(|| "--".to_string());
    let points = metric_value(&meta.metrics, &["Points"]).unwrap_or_else(|| "--".to_string());

    format!(
        r#"<div class="visual-panel club-detail-visual">
        <div class="club-header-row">
          <div class="club-rank-token">{rank}</div>
          <div class="club-name-block"><strong>{club}</strong><span>Club activity board</span></div>
        </div>
        <div class="club-member-table">
          <span>Metric</span><span>Status</span><span>Value</span>
          <b>Monthly Points</b><em>ranked</em><strong>{points}</strong>
          <b>Members</b><em>active</em><strong>{members}</strong>
          <b>Leader</b><em>public</em><strong>Profile</strong>
        </div>
      </div>"#,
        rank = html_escape(&rank),
        club = html_escape(&truncate_chars(&club, 26)),
        points = html_escape(&points),
        members = html_escape(&members),
    )
}

fn render_cutoff_rail(
    asset_base: &str,
    current_rank_id: Option<i64>,
    current_rank: &str,
    buffer: &str,
    buffer_delta: Option<&str>,
    needed: &str,
    needed_delta: Option<&str>,
    lower_cutoff_rank: Option<&str>,
    upper_cutoff_rank: Option<&str>,
) -> String {
    let lower_rank_id = target_rank_id(current_rank_id, -1);
    let upper_rank_id = target_rank_id(current_rank_id, 1);
    let buffer_metric = render_cutoff_metric("Buffer", buffer, buffer_delta, "safe");
    let needed_metric = render_cutoff_metric("Needed", needed, needed_delta, "needed");
    let current = render_cutoff_node(
        asset_base,
        current_rank_id,
        &rank_position_caption(current_rank),
        "active",
    );
    let empty_node = r#"<span class="cutoff-node empty"></span>"#;
    let empty_arrow = r#"<span class="cutoff-arrow empty"></span>"#;

    match (lower_rank_id, upper_rank_id) {
        (Some(lower), Some(upper)) => {
            let lower = render_cutoff_node(
                asset_base,
                Some(lower),
                &cutoff_caption(lower_cutoff_rank, lower),
                "dim",
            );
            let upper = render_cutoff_node(
                asset_base,
                Some(upper),
                &cutoff_caption(upper_cutoff_rank, upper),
                "dim",
            );
            format!(
                r#"<div class="cutoff-rail both">{lower}<span class="cutoff-arrow">&lsaquo;</span>{buffer_metric}{current}{needed_metric}<span class="cutoff-arrow">&rsaquo;</span>{upper}</div>"#
            )
        }
        (Some(lower), None) => {
            let lower = render_cutoff_node(
                asset_base,
                Some(lower),
                &cutoff_caption(lower_cutoff_rank, lower),
                "dim",
            );
            let placeholder = render_cutoff_placeholder("Needed");
            format!(
                r#"<div class="cutoff-rail buffer-only">{lower}<span class="cutoff-arrow">&lsaquo;</span>{buffer_metric}{current}{placeholder}{empty_arrow}{empty_node}</div>"#
            )
        }
        (None, Some(upper)) => {
            let upper = render_cutoff_node(
                asset_base,
                Some(upper),
                &cutoff_caption(upper_cutoff_rank, upper),
                "dim",
            );
            let placeholder = render_cutoff_placeholder("Buffer");
            format!(
                r#"<div class="cutoff-rail needed-only">{empty_node}{empty_arrow}{placeholder}{current}{needed_metric}<span class="cutoff-arrow">&rsaquo;</span>{upper}</div>"#
            )
        }
        (None, None) => {
            let placeholder = render_cutoff_placeholder("Needed");
            format!(
                r#"<div class="cutoff-rail buffer-only">{empty_node}{empty_arrow}{buffer_metric}{current}{placeholder}{empty_arrow}{empty_node}</div>"#
            )
        }
    }
}

fn cutoff_caption(cutoff_rank: Option<&str>, fallback_rank_id: i64) -> String {
    cutoff_rank
        .map(str::trim)
        .filter(|rank| !rank.is_empty())
        .map(|rank| {
            if rank.to_ascii_lowercase().starts_with("rank ") {
                rank.to_string()
            } else if rank.starts_with('#') {
                format!("Rank {rank}")
            } else if rank.chars().all(|character| character.is_ascii_digit()) {
                format!("Rank #{rank}")
            } else {
                rank.to_string()
            }
        })
        .unwrap_or_else(|| format!("{} cutoff", club_rank_label_for_id(fallback_rank_id)))
}

fn rank_position_caption(rank: &str) -> String {
    let trimmed = rank.trim();
    if trimmed.is_empty() {
        "Rank --".to_string()
    } else if trimmed.to_ascii_lowercase().starts_with("rank ") {
        trimmed.to_string()
    } else {
        format!("Rank {trimmed}")
    }
}

fn render_cutoff_metric(label: &str, value: &str, delta: Option<&str>, class_name: &str) -> String {
    let delta = delta
        .filter(|delta| !delta.trim().is_empty() && delta.trim() != "0")
        .map(render_cutoff_delta)
        .unwrap_or_default();

    format!(
        r#"<span class="cutoff-metric"><span class="cutoff-label">{label}</span><strong class="cutoff-value {class_name}">{value}</strong>{delta}</span>"#,
        label = html_escape(label),
        value = html_escape(value),
        class_name = html_escape(class_name),
    )
}

fn render_cutoff_placeholder(label: &str) -> String {
    format!(
        r#"<span class="cutoff-metric placeholder"><span class="cutoff-label">{label}</span><strong class="cutoff-value muted">-</strong></span>"#,
        label = html_escape(label),
    )
}

fn render_cutoff_node(
    asset_base: &str,
    rank_id: Option<i64>,
    caption: &str,
    class_name: &str,
) -> String {
    let icon = render_cutoff_icon(asset_base, rank_id);

    format!(
        r#"<span class="cutoff-node {class_name}"><span class="cutoff-icon">{icon}</span><span class="cutoff-rank">{caption}</span></span>"#,
        class_name = html_escape(class_name),
        caption = html_escape(caption),
    )
}

fn render_cutoff_icon(asset_base: &str, rank_id: Option<i64>) -> String {
    let Some(rank_id) = rank_id else {
        return "--".to_string();
    };
    let clamped = rank_id.clamp(1, 11);
    let label = club_rank_label_for_id(clamped);
    let image = asset_url(
        asset_base,
        &format!("images/icon/circle_rank/utx_ico_circle_rank_{clamped:02}.webp"),
    );

    format!(
        r#"<img src="{image}" alt="{label}" onerror="this.replaceWith(document.createTextNode('{label}'))">"#,
        image = html_escape(&image),
        label = html_escape(&label),
    )
}

fn render_cutoff_delta(delta: &str) -> String {
    let trimmed = delta.trim();
    let class_name = if trimmed.starts_with('-') {
        "down"
    } else {
        "up"
    };

    format!(
        r#"<span class="cutoff-delta {class_name}">{value}</span>"#,
        class_name = class_name,
        value = html_escape(trimmed),
    )
}

fn target_rank_id(current: Option<i64>, offset: i64) -> Option<i64> {
    let target = current? + offset;
    (1..=11).contains(&target).then_some(target)
}

fn club_rank_label_for_id(value: i64) -> String {
    match value {
        1 => "D".to_string(),
        2 => "D+".to_string(),
        3 => "C".to_string(),
        4 => "C+".to_string(),
        5 => "B".to_string(),
        6 => "B+".to_string(),
        7 => "A".to_string(),
        8 => "A+".to_string(),
        9 => "S".to_string(),
        10 => "S+".to_string(),
        11 => "SS".to_string(),
        _ => format!("R{value}"),
    }
}

fn join_class(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "open" => "join-open",
        "closed" => "join-closed",
        _ => "join-approval",
    }
}

fn render_club_rank_emblem(asset_base: &str, fallback: &str, rank_id: Option<&str>) -> String {
    let Some(rank_id) = rank_id.and_then(|rank_id| rank_id.trim().parse::<i64>().ok()) else {
        return format!(
            r#"<div class="club-rank-emblem">{}</div>"#,
            html_escape(&truncate_chars(fallback, 6))
        );
    };
    let clamped = rank_id.clamp(1, 11);
    let image = asset_url(
        asset_base,
        &format!("images/icon/circle_rank/utx_ico_circle_rank_{clamped:02}.webp"),
    );
    format!(
        r#"<div class="club-rank-emblem"><img src="{image}" alt="{alt}"></div>"#,
        image = html_escape(&image),
        alt = html_escape(fallback),
    )
}
