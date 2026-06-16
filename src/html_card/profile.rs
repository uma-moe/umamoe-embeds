use crate::embed::{embed_class_list, DatabaseEmbedDetails, EmbedMetadata};

use super::{
    asset_url, display_title, format_number_grouped, html_escape, inheritance, metric_value,
    parse_display_number, short_id, truncate_chars,
};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-profile",
    render_visual,
};

struct FanHistoryRow {
    period: String,
    fans: String,
    gain: String,
    days: String,
    avg_day: String,
    rank: String,
    circle: String,
}

struct StadiumMemberRow {
    character_id: i64,
    distance_index: usize,
    score: Option<String>,
    running_style: Option<i64>,
}

const STADIUM_DISTANCE_LABELS: [&str; 5] = ["Sprint", "Mile", "Middle", "Long", "Dirt"];

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none() && super::canonical_path(&meta.canonical_url).starts_with("/profile/")
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let trainer = profile_value(meta, &["Trainer"], &display_title(&meta.title));
    let trainer_id = profile_value(meta, &["Trainer ID"], "trainer");
    let title = html_escape(&truncate_chars(&trainer, 44));
    let initials = profile_initials(&trainer);
    let avatar = render_profile_avatar(meta, &initials);
    let trainer_id_display = html_escape(&truncate_chars(&trainer_id, 28));
    let club = html_escape(&truncate_chars(
        &profile_value(meta, &["Club"], "No public circle"),
        28,
    ));
    let followers = html_escape(&truncate_chars(
        &profile_value(meta, &["Followers"], "followers"),
        18,
    ));
    let following = html_escape(&truncate_chars(
        &profile_value(meta, &["Following"], "following"),
        18,
    ));
    let team_class = html_escape(&truncate_chars(
        &profile_value(meta, &["Best Class", "Team Class"], "team"),
        18,
    ));
    let profile_content = if profile_uses_hidden_fallback(meta) {
        render_hidden_profile_state(meta)
    } else {
        render_profile_dashboard(meta)
    };
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
      --accent-secondary: #81c784;
      --accent-warning: #ffb74d;
      --accent-pink: #e91e63;
      --accent-purple: #ba68c8;
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

    .profile-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 104px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 15% 17%, rgba(100, 181, 246, 0.14), transparent 350px),
        radial-gradient(circle at 78% 12%, rgba(129, 199, 132, 0.12), transparent 330px),
        radial-gradient(circle at 48% 88%, rgba(186, 104, 200, 0.11), transparent 360px),
        var(--bg-primary);
    }}

    .profile-card::before {{
      content: "";
      position: absolute;
      inset: 104px 0 0;
      background:
        linear-gradient(rgba(255, 255, 255, 0.03) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.021) 1px, transparent 1px);
      background-size: 76px 76px;
      mask-image: linear-gradient(90deg, transparent, #000 15%, #000 88%, transparent);
      opacity: 0.44;
      pointer-events: none;
    }}

    .profile-header {{
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 24px;
      align-items: center;
      min-width: 0;
      padding: 10px 48px 8px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.09), rgba(129, 199, 132, 0.06), rgba(186, 104, 200, 0.05)),
        rgba(255, 255, 255, 0.012);
    }}

    .header-copy {{
      display: flex;
      align-items: center;
      gap: 16px;
      min-width: 0;
    }}

    .header-side {{
      display: grid;
      justify-items: end;
      align-content: center;
      gap: 5px;
      min-width: 0;
    }}

    .header-side .embed-brand-corner {{
      height: 50px;
      gap: 10px;
      transform: none;
    }}

    .header-side .embed-brand-mark {{
      width: 50px;
      height: 50px;
    }}

    .header-side .embed-brand-url {{
      font-size: 30px;
    }}

    .header-text {{
      display: grid;
      gap: 6px;
      min-width: 0;
    }}

    .profile-title {{
      overflow: hidden;
      margin: 0;
      background: linear-gradient(45deg, var(--accent-primary), var(--accent-secondary) 58%, var(--accent-purple));
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      font-size: 30px;
      font-weight: 760;
      letter-spacing: 0;
      line-height: 1.04;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .profile-subline {{
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 8px;
      margin: 0;
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 650;
      line-height: 1.22;
    }}

    .meta-dot {{
      width: 3px;
      height: 3px;
      flex: 0 0 auto;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.22);
    }}

    .meta-id {{
      color: rgba(255, 255, 255, 0.46);
      font-variant-numeric: tabular-nums;
    }}

    .meta-club {{
      color: var(--accent-primary);
      font-weight: 760;
    }}

    .profile-content {{
      position: relative;
      z-index: 1;
      display: grid;
      min-height: 0;
      padding: 12px 42px 16px;
    }}

    .metric-strip {{
      display: grid;
      grid-template-columns: repeat(6, minmax(0, 1fr));
      position: relative;
      z-index: 1;
      min-width: 0;
      padding: 0 48px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
      background: rgba(255, 255, 255, 0.014);
    }}

    .metric-tile {{
      position: relative;
      display: grid;
      align-content: center;
      justify-items: center;
      gap: 3px;
      min-width: 0;
      height: 58px;
      padding: 0 10px;
      overflow: hidden;
      border-right: 1px solid rgba(255, 255, 255, 0.045);
      text-align: center;
    }}

    .metric-tile::before {{
      display: none;
    }}

    .metric-tile:last-child {{
      border-right: 0;
    }}

    .metric-label {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 760;
      letter-spacing: 0.04em;
      text-transform: uppercase;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .metric-value {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 18px;
      font-weight: 760;
      letter-spacing: 0;
      line-height: 1.05;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .metric-note {{
      overflow: hidden;
      color: var(--text-disabled);
      font-size: 9px;
      font-weight: 600;
      line-height: 1.1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .metric-tile.fans {{ --tile-color: var(--accent-primary); }}
    .metric-tile.rank {{ --tile-color: var(--accent-warning); }}
    .metric-tile.gain {{ --tile-color: var(--accent-secondary); }}
    .metric-tile.followers {{ --tile-color: var(--accent-purple); }}
    .metric-tile.team {{ --tile-color: var(--accent-pink); }}

    .profile-dashboard {{
      display: grid;
      grid-template-rows: 168px minmax(0, 1fr);
      gap: 12px;
      min-height: 0;
    }}

    .identity-panel,
    .fan-panel,
    .feature-panel {{
      min-width: 0;
      min-height: 0;
      overflow: hidden;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 8px;
      background:
        linear-gradient(180deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.016)),
        rgba(255, 255, 255, 0.014);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.035);
    }}

    .identity-panel {{
      display: grid;
      grid-template-rows: auto auto auto minmax(0, 1fr);
      gap: 12px;
      padding: 18px 18px 16px;
    }}

    .profile-avatar-large {{
      position: relative;
      display: grid;
      place-items: center;
      width: 82px;
      height: 82px;
      flex: 0 0 82px;
      overflow: hidden;
      border: 2px solid rgba(100, 181, 246, 0.22);
      border-radius: 50%;
      background:
        radial-gradient(circle at 35% 22%, rgba(255, 255, 255, 0.18), transparent 42%),
        linear-gradient(135deg, rgba(100, 181, 246, 0.28), rgba(129, 199, 132, 0.18));
      color: var(--text-primary);
      font-size: 24px;
      font-weight: 760;
      letter-spacing: 0;
      box-shadow: 0 12px 26px rgba(0, 0, 0, 0.28);
    }}

    .profile-avatar-large img {{
      position: absolute;
      inset: 2px;
      z-index: 1;
      display: block;
      width: calc(100% - 4px);
      height: calc(100% - 4px);
      border-radius: 50%;
      object-fit: contain;
      object-position: center bottom;
      color: transparent;
      font-size: 0;
    }}

    .profile-avatar-fallback {{
      position: relative;
      z-index: 0;
      display: block;
      line-height: 1;
    }}

    .identity-copy {{
      display: grid;
      gap: 7px;
      min-width: 0;
    }}

    .identity-name {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 22px;
      font-weight: 700;
      line-height: 1.05;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .identity-id {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 750;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .section-badge {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: fit-content;
      max-width: 100%;
      height: 28px;
      padding: 0 10px;
      border: 1px solid rgba(129, 199, 132, 0.28);
      border-radius: 6px;
      background: rgba(129, 199, 132, 0.08);
      color: var(--accent-secondary);
      font-size: 11px;
      font-weight: 700;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }}

    .identity-facts {{
      display: grid;
      gap: 8px;
      min-height: 0;
    }}

    .identity-fact {{
      display: grid;
      gap: 4px;
      min-width: 0;
      padding: 10px 11px;
      border: 1px solid rgba(255, 255, 255, 0.065);
      border-radius: 7px;
      background: rgba(255, 255, 255, 0.022);
    }}

    .identity-fact span {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 700;
      line-height: 1;
      text-transform: uppercase;
      text-overflow: ellipsis;
      white-space: nowrap;
      text-align: center;
    }}

    .identity-fact b {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 15px;
      font-weight: 700;
      line-height: 1.05;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .fan-panel {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      padding: 11px 14px 10px;
    }}

    .panel-heading {{
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 14px;
      min-width: 0;
      margin-bottom: 6px;
    }}

    .panel-heading h2 {{
      margin: 0;
      color: var(--text-primary);
      font-size: 14px;
      font-weight: 760;
      letter-spacing: 0;
      line-height: 1.05;
    }}

    .panel-heading span {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 760;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .visibility-pill {{
      display: inline-flex;
      align-items: center;
      gap: 5px;
      height: 18px;
      padding: 0 8px;
      border: 1px solid rgba(129, 199, 132, 0.28);
      border-radius: 9px;
      background: rgba(129, 199, 132, 0.09);
      color: var(--accent-secondary);
      font-size: 8px;
      font-weight: 760;
      letter-spacing: 0.05em;
      line-height: 1;
      text-transform: uppercase;
    }}

    .visibility-pill::before {{
      content: "";
      width: 5px;
      height: 5px;
      border-radius: 50%;
      background: var(--accent-primary);
      box-shadow: 0 0 8px rgba(100, 181, 246, 0.45);
    }}

    .profile-hidden-state {{
      display: grid;
      align-content: center;
      justify-items: center;
      gap: 12px;
      min-height: 450px;
      padding: 48px;
      border-top: 1px solid rgba(255, 255, 255, 0.06);
      background:
        radial-gradient(circle at 50% 18%, rgba(100, 181, 246, 0.1), transparent 220px),
        rgba(255, 255, 255, 0.01);
      text-align: center;
    }}

    .profile-hidden-state svg {{
      width: 42px;
      height: 42px;
      color: var(--accent-primary);
      opacity: 0.95;
    }}

    .profile-hidden-state h2 {{
      margin: 0;
      color: var(--text-primary);
      font-size: 22px;
      font-weight: 680;
      line-height: 1;
    }}

    .profile-hidden-state p {{
      max-width: 430px;
      margin: 0;
      color: var(--text-secondary);
      font-size: 13px;
      font-weight: 620;
      line-height: 1.4;
    }}

    .hidden-profile-meta {{
      display: flex;
      justify-content: center;
      gap: 8px;
      min-width: 0;
    }}

    .hidden-profile-meta span {{
      display: inline-flex;
      align-items: center;
      height: 24px;
      padding: 0 10px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 7px;
      background: rgba(255, 255, 255, 0.03);
      color: var(--text-secondary);
      font-size: 10px;
      font-weight: 760;
      line-height: 1;
      white-space: nowrap;
    }}

    .fan-table-wrap {{
      min-height: 0;
      overflow: hidden;
    }}

    .fan-table {{
      width: 100%;
      border-collapse: collapse;
      table-layout: fixed;
    }}

    .fan-table th {{
      height: 21px;
      padding: 0 8px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.06);
      color: var(--text-disabled);
      font-size: 8px;
      font-weight: 680;
      line-height: 1.2;
      text-align: left;
      text-transform: uppercase;
    }}

    .fan-table td {{
      height: 25px;
      padding: 0 8px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.045);
      color: var(--text-secondary);
      font-size: 10px;
      font-weight: 620;
      line-height: 1.25;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .fan-table tr:last-child td {{
      border-bottom: 0;
    }}

    .fan-table .right {{
      text-align: right;
      font-variant-numeric: tabular-nums;
    }}

    .fan-table .gain {{
      color: var(--accent-secondary);
      font-weight: 760;
    }}

    .fan-table .circle-link {{
      color: var(--accent-primary);
      font-weight: 760;
    }}

    .overview-card-grid {{
      display: grid;
      grid-template-columns: minmax(230px, 0.74fr) minmax(0, 1.26fr) minmax(330px, 1fr);
      grid-template-rows: 116px minmax(0, 1fr);
      gap: 12px 14px;
      min-height: 0;
    }}

    .overview-card {{
      min-width: 0;
      min-height: 0;
      overflow: hidden;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 8px;
      background:
        linear-gradient(180deg, rgba(255, 255, 255, 0.038), rgba(255, 255, 255, 0.015)),
        rgba(255, 255, 255, 0.014);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.035);
      padding: 10px 13px;
    }}

    .summary-card {{
      grid-column: 1;
      grid-row: 1;
      padding: 9px 10px;
    }}

    .summary-card .overview-card-head {{
      margin-bottom: 6px;
    }}

    .circle-card {{
      grid-column: 1;
      grid-row: 2;
    }}

    .stadium-card {{
      display: grid;
      grid-column: 2;
      grid-row: 1 / span 2;
      grid-template-rows: auto minmax(0, 1fr);
      padding: 10px 12px;
    }}

    .borrow-card {{
      display: grid;
      grid-column: 3;
      grid-row: 1 / span 2;
      grid-template-rows: auto minmax(0, 1fr);
      padding: 10px 11px;
    }}

    .borrow-card .overview-card-head {{
      justify-content: space-between;
    }}

    .inheritance-head-totals {{
      display: inline-flex;
      align-items: center;
      justify-content: flex-end;
      gap: 3px;
      min-width: 0;
      margin-left: auto;
      overflow: hidden;
      font-variant-numeric: tabular-nums;
    }}

    .inheritance-total {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 2px;
      min-width: 27px;
      height: 18px;
      padding: 0 4px;
      border: 1px solid;
      border-radius: 5px;
      background: rgba(255, 255, 255, 0.025);
      font-size: 8px;
      font-weight: 760;
      line-height: 1.15;
      white-space: nowrap;
    }}

    .inheritance-total span {{
      display: inline-flex;
      align-items: center;
      height: 100%;
      font-weight: 700;
      line-height: 1;
      opacity: 0.92;
    }}

    .inheritance-total b {{
      display: inline-flex;
      align-items: center;
      height: 100%;
      color: var(--text-primary);
      font-size: 9px;
      font-weight: 680;
      line-height: 1;
    }}

    .inheritance-total-blue {{
      border-color: rgba(33, 150, 243, 0.6);
      color: var(--accent-primary);
    }}

    .inheritance-total-pink {{
      border-color: rgba(233, 30, 99, 0.6);
      color: #f06292;
    }}

    .inheritance-total-green {{
      border-color: rgba(76, 175, 80, 0.58);
      color: var(--accent-secondary);
    }}

    .inheritance-total-white {{
      border-color: rgba(189, 189, 189, 0.48);
      color: #d0d0d0;
    }}

    .overview-card-head {{
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      margin-bottom: 7px;
    }}

    .overview-card-head svg {{
      width: 16px;
      height: 16px;
      color: var(--accent-primary);
      flex: 0 0 auto;
    }}

    .overview-card-head h3 {{
      margin: 0;
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 760;
      line-height: 1.15;
    }}

    .stat-grid-mini {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
      min-height: 0;
    }}

    .summary-stat-grid {{
      display: grid;
      gap: 6px;
      min-height: 0;
    }}

    .rolling-stat-grid,
    .historic-stat-grid {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 5px;
      min-width: 0;
    }}

    .stat-block {{
      display: grid;
      align-content: center;
      justify-items: center;
      gap: 3px;
      min-width: 0;
      height: 36px;
      padding: 4px 6px;
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 7px;
      background: rgba(255, 255, 255, 0.022);
      text-align: center;
    }}

    .stat-block span {{
      overflow: hidden;
      max-width: 100%;
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 680;
      line-height: 1.16;
      text-transform: uppercase;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .stat-block b {{
      overflow: hidden;
      max-width: 100%;
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 760;
      line-height: 1.12;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .stat-block.gain b {{
      color: var(--accent-secondary);
    }}

    .historic-stat-grid .stat-block {{
      height: 36px;
      grid-template-columns: none;
      justify-items: center;
      align-content: center;
      gap: 3px;
      padding: 4px 6px;
    }}

    .historic-stat-grid .stat-block span {{
      font-size: 7px;
      text-align: center;
    }}

    .historic-stat-grid .stat-block b {{
      font-size: 12px;
      text-align: center;
    }}

    .profile-borrow-display {{
      min-width: 0;
      min-height: 0;
      overflow: hidden;
    }}

    .borrow-card .inheritance-body {{
      display: grid;
      grid-template-columns: 1fr;
      grid-template-rows: 80px minmax(0, 1fr);
      gap: 5px;
      min-width: 0;
      min-height: 0;
      height: 100%;
      margin: 0;
      padding: 0;
      border: 0;
      overflow: hidden;
    }}

    .borrow-card .character-images {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) 64px;
      gap: 6px;
      align-items: stretch;
      min-width: 0;
      min-height: 0;
      padding: 0;
    }}

    .borrow-card .lineage-frame {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      grid-template-rows: 12px minmax(0, 1fr);
      align-items: center;
      justify-items: center;
      min-width: 0;
      min-height: 0;
      overflow: hidden;
      padding: 2px 7px 3px;
      border: 1px solid rgba(100, 181, 246, 0.06);
      border-radius: 7px;
      background:
        radial-gradient(circle at 50% 18%, rgba(100, 181, 246, 0.055), transparent 62px),
        rgba(0, 0, 0, 0.12);
    }}

    .borrow-card .main-character,
    .borrow-card .parent-with-badges {{
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 2px;
      min-width: 0;
    }}

    .borrow-card .main-character {{
      grid-column: 1;
      grid-row: 2;
    }}

    .borrow-card .parent-characters {{
      display: grid;
      grid-column: 2 / 4;
      grid-row: 2;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      justify-items: center;
      gap: 0;
      width: 100%;
      min-width: 0;
    }}

    .borrow-card .lineage-bracket {{
      grid-column: 1 / 4;
      grid-row: 1;
      width: 100%;
      height: 13px;
      margin: 0;
      align-self: start;
    }}

    .borrow-card .lineage-bracket path {{
      fill: none;
      stroke: rgba(255, 255, 255, 0.46);
      stroke-width: 2;
      stroke-linecap: square;
    }}

    .borrow-card .portrait-wrapper {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
      border: 2px solid rgba(255, 255, 255, 0.2);
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.045);
      box-shadow:
        inset 0 0 0 1px rgba(0, 0, 0, 0.3),
        0 6px 14px rgba(0, 0, 0, 0.28);
    }}

    .borrow-card .portrait-main {{
      width: 36px;
      height: 36px;
      padding: 2px;
      border-color: rgba(100, 181, 246, 0.58);
    }}

    .borrow-card .portrait-gp {{
      width: 35px;
      height: 35px;
      padding: 2px;
      border-color: rgba(206, 147, 216, 0.52);
    }}

    .borrow-card .portrait-left {{
      border-color: rgba(255, 120, 178, 0.55);
    }}

    .borrow-card .portrait-label {{
      color: var(--text-primary);
      font-size: 10px;
      font-weight: 680;
      line-height: 1;
    }}

    .borrow-card .character-image {{
      display: block;
      width: 100%;
      height: 100%;
      border-radius: 50%;
      object-fit: contain;
      object-position: center;
      color: transparent;
      font-size: 0;
    }}

    .borrow-card .parent-affinity-badges {{
      display: flex;
      justify-content: center;
      min-height: 12px;
    }}

    .borrow-card .parent-affinity-badges:empty {{
      display: none;
      min-height: 0;
    }}

    .borrow-card .affinity-badge {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 3px;
      max-width: 66px;
      min-height: 12px;
      height: 12px;
      padding: 0 4px;
      border: 1px solid rgba(100, 181, 246, 0.25);
      border-radius: 10px;
      background: rgba(100, 181, 246, 0.12);
      color: var(--accent-primary);
      font-size: 7px;
      font-weight: 680;
      line-height: 1.1;
      white-space: nowrap;
    }}

    .borrow-card .heart-icon {{
      display: block;
      line-height: 1;
      transform: translateY(-0.5px);
    }}

    .borrow-card .affinity-badge.gp {{
      border-color: rgba(206, 147, 216, 0.25);
      background: rgba(206, 147, 216, 0.12);
      color: #ce93d8;
    }}

    .borrow-card .node-role-label {{
      display: block;
      color: var(--text-disabled);
      font-size: 8px;
      font-weight: 680;
      line-height: 1.12;
      text-transform: uppercase;
    }}

    .borrow-card .node-role-main {{
      color: var(--accent-primary);
    }}

    .borrow-card .support-card-section {{
      display: grid;
      grid-template-rows: 10px 40px 12px;
      justify-items: center;
      align-content: center;
      gap: 2px;
      min-width: 0;
      min-height: 0;
      padding: 5px;
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 7px;
      background:
        linear-gradient(180deg, rgba(255, 183, 77, 0.035), transparent),
        rgba(0, 0, 0, 0.12);
    }}

    .borrow-card .support-card-section::before {{
      content: "Support";
      display: block;
      height: 10px;
      overflow: hidden;
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 680;
      line-height: 10px;
      text-transform: uppercase;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .borrow-card .support-card-image {{
      display: block;
      width: 40px;
      height: 40px;
      border-radius: 7px;
      object-fit: cover;
      color: transparent;
      font-size: 0;
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
    }}

    .borrow-card .card-limit-break {{
      display: flex;
      justify-content: center;
      gap: 1px;
      min-width: 0;
      height: 12px;
    }}

    .borrow-card .limit-break-icon {{
      width: 12px;
      height: 12px;
      flex: 0 0 auto;
    }}

    .borrow-card .limit-break-icon path {{
      fill: rgba(100, 181, 246, 0.58);
    }}

    .borrow-card .limit-break-icon.filled path {{
      fill: #2196f3;
    }}

    .borrow-card .spark-arrays {{
      min-width: 0;
      min-height: 0;
      padding-bottom: 2px;
      overflow: hidden;
    }}

    .borrow-card .spark-container {{
      display: flex;
      flex-direction: column;
      gap: 3px;
      min-width: 0;
      min-height: 0;
      overflow: hidden;
    }}

    .borrow-card .spark-row {{
      display: flex;
      align-items: stretch;
      gap: 4px;
      min-width: 0;
    }}

    .borrow-card .spark-type-indicator {{
      width: 3px;
      min-height: 100%;
      flex: 0 0 3px;
      border-radius: 999px;
    }}

    .borrow-card .spark-type-indicator.blue {{ background: #2196f3; }}
    .borrow-card .spark-type-indicator.pink {{ background: #e91e63; }}
    .borrow-card .spark-type-indicator.green {{ background: #4caf50; }}
    .borrow-card .spark-type-indicator.white {{ background: #bdbdbd; }}

    .borrow-card .spark-list {{
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 3px 4px;
      min-width: 0;
    }}

    .borrow-card .spark-item {{
      display: inline-flex;
      align-items: center;
      justify-content: flex-start;
      gap: 2px;
      max-width: 100%;
      min-height: 18px;
      padding: 2px 5px;
      border: 1px solid;
      border-radius: 6px;
      color: var(--text-secondary);
      font-size: 8px;
      font-weight: 600;
      line-height: 1.18;
      white-space: nowrap;
    }}

    .borrow-card .spark-name {{
      min-width: 0;
      max-width: none;
      overflow: visible;
      line-height: 1.18;
      text-overflow: clip;
      white-space: nowrap;
    }}

    .borrow-card .spark-pct {{
      min-width: 0;
      max-width: 42px;
      overflow: hidden;
      padding: 1px 4px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 4px;
      background: rgba(0, 0, 0, 0.22);
      color: rgba(255, 255, 255, 0.78);
      font-size: 0.85em;
      font-weight: 760;
      line-height: 1;
      text-overflow: ellipsis;
      font-variant-numeric: tabular-nums;
    }}

    .borrow-card .spark-level {{
      display: inline-flex;
      align-items: center;
      color: currentColor;
      font-weight: 760;
      line-height: 1.15;
      font-variant-numeric: tabular-nums;
    }}

    .borrow-card .spark-star {{
      display: block;
      width: 9px;
      height: 9px;
      flex: 0 0 9px;
      color: currentColor;
      line-height: 1;
      transform: translateY(-1px);
    }}

    .borrow-card .spark-star path {{
      fill: currentColor;
    }}

    .borrow-card .blue-spark {{
      border-color: rgba(33, 150, 243, 0.38);
      background: rgba(11, 72, 120, 0.28);
      color: var(--accent-primary);
    }}

    .borrow-card .pink-spark {{
      border-color: rgba(233, 30, 99, 0.38);
      background: rgba(112, 14, 52, 0.3);
      color: #f06292;
    }}

    .borrow-card .green-spark {{
      border-color: rgba(76, 175, 80, 0.36);
      background: rgba(23, 82, 36, 0.3);
      color: var(--accent-secondary);
    }}

    .borrow-card .white-spark {{
      border-color: rgba(158, 158, 158, 0.34);
      background: rgba(72, 72, 72, 0.34);
      color: #cfcfcf;
    }}

    .borrow-card .parent-source {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 1px;
      color: #ff9100;
      line-height: 1;
    }}

    .borrow-card .parent-icon {{
      width: 9px;
      height: 9px;
      flex: 0 0 auto;
    }}

    .borrow-card .parent-icon path {{
      fill: currentColor;
    }}

    .borrow-card .parent-contribution {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 1px;
      color: currentColor;
      font-size: 8px;
      font-weight: 760;
      line-height: 1.15;
    }}

    .borrow-card .parent-contribution .spark-star {{
      width: 8px;
      height: 8px;
      flex-basis: 8px;
    }}

    .borrow-card .parent-contribution::before {{
      content: "(";
      opacity: 0.85;
    }}

    .borrow-card .parent-contribution::after {{
      content: ")";
      opacity: 0.85;
    }}

    .borrow-card .overflow-spark {{
      opacity: 0.78;
    }}

    .stadium-primary span,
    .stadium-distance-label,
    .stadium-stat-label {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 680;
      line-height: 1.2;
      text-transform: uppercase;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .stadium-distance-label,
    .stadium-stat-label {{
      text-align: center;
    }}

    .stadium-primary span {{
      text-align: left;
    }}

    .stadium-summary {{
      display: grid;
      grid-template-rows: 36px minmax(0, 1fr);
      gap: 6px;
      min-height: 0;
      height: 100%;
    }}

    .stadium-primary {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: center;
      gap: 8px;
      min-width: 0;
      padding: 1px 9px 0;
      border: 1px solid rgba(255, 255, 255, 0.065);
      border-radius: 7px;
      background: rgba(255, 255, 255, 0.024);
    }}

    .stadium-primary b {{
      overflow: hidden;
      color: var(--text-primary);
      font-weight: 680;
      line-height: 1.12;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .stadium-primary b {{
      color: var(--accent-secondary);
      font-size: 16px;
      text-align: right;
    }}

    .stadium-team-stack {{
      display: grid;
      grid-template-columns: repeat(5, minmax(0, 1fr));
      gap: 6px;
      min-width: 0;
      min-height: 0;
    }}

    .stadium-distance-column {{
      display: grid;
      grid-template-rows: 16px minmax(0, 1fr);
      justify-items: center;
      gap: 4px;
      min-width: 0;
      min-height: 0;
      overflow: visible;
      padding: 5px 5px 6px;
      border: 1px solid rgba(255, 255, 255, 0.058);
      border-radius: 6px;
      background: rgba(255, 255, 255, 0.02);
    }}

    .stadium-icon-stack {{
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 4px;
      width: 100%;
      min-width: 0;
      min-height: 0;
      overflow: visible;
    }}

    .stadium-character {{
      position: relative;
      display: block;
      width: min(61px, 100%);
      aspect-ratio: 1;
      flex: 0 0 auto;
      overflow: visible;
      border: 1px solid rgba(100, 181, 246, 0.38);
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.045);
      color: transparent;
      font-size: 0;
      box-shadow: 0 2px 7px rgba(0, 0, 0, 0.36);
    }}

    .stadium-character-icon {{
      display: block;
      position: absolute;
      inset: 3px;
      width: calc(100% - 6px);
      height: calc(100% - 6px);
      object-fit: contain;
      object-position: center;
      color: transparent;
      font-size: 0;
    }}

    .stadium-rank-badge {{
      position: absolute;
      top: -6px;
      right: -14px;
      z-index: 2;
      display: grid;
      place-items: center;
      width: 46px;
      height: 21px;
      overflow: hidden;
      color: #fff;
      font-size: 10px;
      font-weight: 720;
      line-height: 1;
      text-shadow: 0 1px 2px rgba(0, 0, 0, 0.65);
      filter: drop-shadow(0 3px 5px rgba(0, 0, 0, 0.5));
      pointer-events: none;
    }}

    .stadium-rank-badge img {{
      display: block;
      max-width: 46px;
      max-height: 21px;
      object-fit: contain;
    }}

    .stadium-runstyle-badge {{
      position: absolute;
      left: -6px;
      bottom: 0;
      z-index: 2;
      display: grid;
      place-items: center;
      width: 23px;
      height: 23px;
      overflow: hidden;
      border-radius: 50%;
      background: rgba(0, 0, 0, 0.42);
      box-shadow:
        0 2px 5px rgba(0, 0, 0, 0.4),
        inset 0 0 0 1px rgba(255, 255, 255, 0.16);
      pointer-events: none;
    }}

    .stadium-runstyle-badge img {{
      display: block;
      width: 20px;
      height: 20px;
      object-fit: contain;
    }}

    .stadium-empty-icons {{
      color: var(--text-disabled);
      font-size: 11px;
      font-weight: 760;
    }}

    .stadium-stat-value {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 12px;
      font-weight: 780;
      text-align: right;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .current-circle {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      gap: 10px;
      min-width: 0;
      min-height: 0;
    }}

    .circle-identity {{
      display: grid;
      grid-template-columns: 48px minmax(0, 1fr);
      align-items: center;
      gap: 10px;
      min-width: 0;
    }}

    .profile-circle-rank-emblem {{
      display: grid;
      place-items: center;
      width: 46px;
      height: 46px;
      color: var(--accent-primary);
      font-size: 14px;
      font-weight: 700;
      line-height: 1;
      filter: drop-shadow(0 6px 12px rgba(0, 0, 0, 0.34));
    }}

    .profile-circle-rank-emblem img {{
      display: block;
      width: 46px;
      height: 46px;
      object-fit: contain;
    }}

    .current-circle b {{
      overflow: hidden;
      color: var(--accent-primary);
      font-size: 18px;
      font-weight: 760;
      line-height: 1.05;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .circle-label,
    .circle-stat span {{
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 680;
      line-height: 1;
      text-transform: uppercase;
    }}

    .circle-main {{
      display: grid;
      gap: 4px;
      min-width: 0;
    }}

    .circle-stat-row {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 6px;
      min-width: 0;
      align-self: end;
    }}

    .circle-stat {{
      display: grid;
      gap: 4px;
      min-width: 0;
      padding: 8px 7px;
      border: 1px solid rgba(255, 255, 255, 0.058);
      border-radius: 7px;
      background: rgba(255, 255, 255, 0.022);
    }}

    .circle-stat strong {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 12px;
      font-weight: 790;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .fan-chart {{
      position: relative;
      min-height: 0;
      overflow: hidden;
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 8px;
      background:
        linear-gradient(rgba(255, 255, 255, 0.028) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px),
        rgba(255, 255, 255, 0.018);
      background-size: 52px 38px;
    }}

    .fan-chart canvas {{
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
    }}

    .chart-caption {{
      position: absolute;
      z-index: 2;
      left: 18px;
      top: 16px;
      display: grid;
      gap: 3px;
    }}

    .chart-caption span {{
      color: var(--text-disabled);
      font-size: 10px;
      font-weight: 700;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }}

    .chart-caption b {{
      color: var(--text-primary);
      font-size: 25px;
      font-weight: 700;
      letter-spacing: 0;
      font-variant-numeric: tabular-nums;
    }}

    .rolling-row {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 8px;
      margin-top: 10px;
    }}

    .rolling-stat {{
      display: grid;
      gap: 3px;
      min-width: 0;
      height: 56px;
      padding: 8px 9px;
      border: 1px solid rgba(255, 255, 255, 0.065);
      border-radius: 7px;
      background: rgba(255, 255, 255, 0.024);
    }}

    .rolling-stat span {{
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 700;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }}

    .rolling-stat b {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 15px;
      font-weight: 700;
      letter-spacing: 0;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .rolling-stat.gain b {{
      color: var(--accent-secondary);
    }}

    .feature-panel {{
      display: grid;
      gap: 9px;
      padding: 14px;
    }}

    .profile-nav-tabs {{
      position: relative;
      z-index: 1;
      display: flex;
      align-items: end;
      gap: 2px;
      padding: 0 48px;
      border-bottom: 2px solid var(--border-subtle);
      overflow: hidden;
    }}

    .nav-tab {{
      display: inline-flex;
      align-items: center;
      gap: 7px;
      height: 42px;
      padding: 0 16px;
      border-bottom: 2px solid transparent;
      margin-bottom: -2px;
      color: var(--text-muted);
      font-size: 12px;
      font-weight: 760;
      white-space: nowrap;
    }}

    .nav-tab.active {{
      color: var(--accent-primary);
      border-bottom-color: var(--accent-primary);
    }}

    .nav-tab svg {{
      width: 17px;
      height: 17px;
      display: block;
    }}

    .wip-chip {{
      display: inline-flex;
      align-items: center;
      height: 16px;
      padding: 0 5px;
      border-radius: 4px;
      background: rgba(255, 183, 77, 0.12);
      color: var(--accent-warning);
      font-size: 8px;
      font-weight: 680;
    }}

    .feature-tile {{
      position: relative;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 4px 10px;
      min-width: 0;
      min-height: 0;
      padding: 11px 12px;
      border: 1px solid rgba(255, 255, 255, 0.07);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.024);
      overflow: hidden;
    }}

    .feature-tile::before {{
      content: "";
      position: absolute;
      inset: 0 auto 0 0;
      width: 3px;
      background: var(--feature-color, var(--accent-primary));
      opacity: 0.84;
    }}

    .feature-kicker {{
      grid-column: 1 / -1;
      overflow: hidden;
      color: var(--text-disabled);
      font-size: 9px;
      font-weight: 700;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .feature-title {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 760;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .feature-value {{
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 700;
      font-variant-numeric: tabular-nums;
      white-space: nowrap;
    }}

    .feature-detail {{
      grid-column: 1 / -1;
      overflow: hidden;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 600;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .feature-tile.club {{ --feature-color: var(--accent-primary); }}
    .feature-tile.inheritance {{ --feature-color: var(--accent-pink); }}
    .feature-tile.support {{ --feature-color: var(--accent-warning); }}
    .feature-tile.stadium {{ --feature-color: var(--accent-secondary); }}

    {brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-profile">
  <article class="{class_list} profile-card card-view-profile" aria-label="{title}">
    <header class="profile-header">
      <div class="header-copy">
        <div class="profile-avatar-large">{avatar}</div>
        <div class="header-text">
          <h1 class="profile-title">{title}</h1>
          <p class="profile-subline">
            <span class="meta-id">#{trainer_id_display}</span>
            <span class="meta-dot"></span>
            <span>{followers} followers</span>
            <span class="meta-dot"></span>
            <span>{following} following</span>
            <span class="meta-dot"></span>
            <span class="meta-club">{club}</span>
            <span class="meta-dot"></span>
            <span>{team_class}</span>
          </p>
        </div>
      </div>
      <div class="header-side">
        {brand}
      </div>
    </header>

    {profile_content}
  </article>
</body>
</html>"#
    )
}

fn render_profile_dashboard(meta: &EmbedMetadata) -> String {
    format!(
        r#"<main class="profile-content">
      <section class="profile-dashboard">
        <section class="fan-panel" aria-label="Fan history">
          <div class="panel-heading">
            <h2>Fan History</h2>
          </div>
          {fan_history}
        </section>

        <section class="overview-card-grid" aria-label="Profile sections snapshot">
          {overview_cards}
        </section>
      </section>
    </main>"#,
        fan_history = render_fan_history_table(meta),
        overview_cards = render_overview_cards(meta),
    )
}

fn render_hidden_profile_state(meta: &EmbedMetadata) -> String {
    let trainer_id = html_escape(&truncate_chars(
        &profile_value(meta, &["Trainer ID"], "trainer"),
        28,
    ));
    let section = html_escape(&profile_value(meta, &["Section"], "Profile"));
    let visibility = html_escape(&profile_value(meta, &["Visibility"], "Hidden"));

    format!(
        r#"<main class="profile-content">
      <section class="profile-hidden-state" aria-label="Hidden profile">
        {icon}
        <h2>Profile Hidden</h2>
        <p>This trainer has not made profile details public, so uma.moe can only show the public account shell.</p>
        <div class="hidden-profile-meta">
          <span>#{trainer_id}</span>
          <span>{section}</span>
          <span>{visibility}</span>
        </div>
      </section>
    </main>"#,
        icon = material_icon_svg("shield"),
        trainer_id = trainer_id,
        section = section,
        visibility = visibility,
    )
}

fn profile_uses_hidden_fallback(meta: &EmbedMetadata) -> bool {
    if let Some(visibility) = metric_value(&meta.metrics, &["Visibility"]) {
        let visibility = visibility.to_ascii_lowercase();
        if visibility.contains("hidden") || visibility.contains("private") {
            return true;
        }
    }

    !has_any_metric(
        meta,
        &[
            "Fans",
            "Fan Month 1",
            "Inheritance",
            "Support",
            "Stadium",
            "Team",
            "Club Members",
            "Club Rank",
        ],
    )
}

fn has_any_metric(meta: &EmbedMetadata, labels: &[&str]) -> bool {
    labels
        .iter()
        .any(|label| metric_value(&meta.metrics, &[*label]).is_some())
}

fn profile_value(meta: &EmbedMetadata, labels: &[&str], fallback: &str) -> String {
    metric_value(&meta.metrics, labels).unwrap_or_else(|| fallback.to_string())
}

fn render_profile_avatar(meta: &EmbedMetadata, initials: &str) -> String {
    let image_ids = profile_avatar_image_ids(meta);
    let Some(image_id) = image_ids.first() else {
        return format!(
            r#"<span class="profile-avatar-fallback">{}</span>"#,
            html_escape(initials)
        );
    };

    let asset_base = profile_value(meta, &["Asset Base"], "https://uma.moe/assets");
    let image_url = profile_character_image_url(&asset_base, *image_id);
    let fallback_urls = image_ids
        .iter()
        .skip(1)
        .map(|image_id| profile_character_image_url(&asset_base, *image_id))
        .map(|url| html_escape(&url))
        .collect::<Vec<_>>()
        .join("|");
    let fallback_url = if fallback_urls.is_empty() {
        r#" onerror="this.style.display = 'none'""#.to_string()
    } else {
        format!(
            r#" data-alt-srcs="{}" onerror="const srcs = (this.dataset.altSrcs || '').split('|').filter(Boolean); if (srcs.length) {{ this.src = srcs.shift(); this.dataset.altSrcs = srcs.join('|'); }} else {{ this.style.display = 'none'; }}""#,
            fallback_urls
        )
    };

    format!(
        r#"<img src="{}" alt=""{}>"#,
        html_escape(&image_url),
        fallback_url,
    )
}

fn profile_avatar_image_ids(meta: &EmbedMetadata) -> Vec<i64> {
    let mut image_ids = Vec::new();
    for image_id in [
        metric_i64(meta, &["Leader Chara Dress"]),
        metric_i64(meta, &["Lineage Main"]),
    ]
    .into_iter()
    .flatten()
    {
        push_unique_image_id(&mut image_ids, image_id);
        push_unique_image_id(&mut image_ids, normalize_dress_asset_id(image_id));
    }
    image_ids
}

fn push_unique_image_id(image_ids: &mut Vec<i64>, image_id: i64) {
    if !image_ids.contains(&image_id) {
        image_ids.push(image_id);
    }
}

fn normalize_dress_asset_id(character_id: i64) -> i64 {
    if character_id > 100 {
        (character_id / 100) * 100 + 1
    } else {
        character_id
    }
}

fn profile_character_image_url(asset_base: &str, character_id: i64) -> String {
    asset_url(
        asset_base,
        &format!("/images/character_stand/chara_stand_{character_id}.webp"),
    )
}

fn profile_initials(name: &str) -> String {
    let mut initials = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();

    if initials.len() < 2 {
        initials = name.chars().take(2).collect();
    }

    let initials = initials.trim();
    if initials.is_empty() {
        "U".to_string()
    } else {
        initials.to_uppercase()
    }
}

fn material_icon_svg(name: &str) -> &'static str {
    match name {
        "person" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4Zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4Z"/></svg>"#
        }
        "history" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M13 3c-4.97 0-9 4.03-9 9H1l4 4 4-4H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.78-4.95-2.05L6.64 18.36C8.27 20 10.52 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9Zm-1 5v5l4.25 2.52.75-1.23-3.5-2.08V8H12Z"/></svg>"#
        }
        "emoji_events" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M19 5h-2V3H7v2H5c-1.1 0-2 .9-2 2v1c0 2.55 1.92 4.63 4.39 4.94A5.01 5.01 0 0 0 11 15.9V19H8v2h8v-2h-3v-3.1a5.01 5.01 0 0 0 3.61-2.96C19.08 12.63 21 10.55 21 8V7c0-1.1-.9-2-2-2ZM5 8V7h2v3.82C5.84 10.4 5 9.3 5 8Zm14 0c0 1.3-.84 2.4-2 2.82V7h2v1Z"/></svg>"#
        }
        "military_tech" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M17 10.43V2H7v8.43c0 .35.18.68.49.86l4.18 2.51-1 2.25-2.46.22 1.86 1.62-.56 2.42L12 19.07l2.49 1.24-.56-2.42 1.86-1.62-2.46-.22-1-2.25 4.18-2.51c.31-.18.49-.51.49-.86ZM9 4h6v5.87l-3 1.8-3-1.8V4Z"/></svg>"#
        }
        "workspace_premium" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M12 2 8.5 8.5 2 9.27l4.8 4.45L5.47 20 12 16.75 18.53 20l-1.33-6.28L22 9.27l-6.5-.77L12 2Zm0 4.2 1.9 3.54 3.92.46-2.88 2.67.8 3.78L12 14.69l-3.74 1.96.8-3.78-2.88-2.67 3.92-.46L12 6.2Z"/></svg>"#
        }
        "speed" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M20.38 8.57 19.66 10A8 8 0 0 1 20 12c0 4.42-3.58 8-8 8s-8-3.58-8-8 3.58-8 8-8c1.36 0 2.64.34 3.76.94l1.42-1.42A9.96 9.96 0 0 0 12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10c0-1.22-.22-2.39-.62-3.43ZM13 11.59l4.3-4.3-1.41-1.41-4.3 4.3A2 2 0 1 0 13 11.59Z"/></svg>"#
        }
        "shield" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M12 1 4 5v6c0 5.55 3.84 10.74 8 12 4.16-1.26 8-6.45 8-12V5l-8-4Zm0 2.24L18 6v5c0 4.49-2.94 8.74-6 9.89C8.94 19.74 6 15.49 6 11V6l6-2.76Z"/></svg>"#
        }
        "stadium" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M7 5c-2.76 0-5 1.79-5 4v6c0 2.21 2.24 4 5 4h10c2.76 0 5-1.79 5-4V9c0-2.21-2.24-4-5-4H7Zm0 2h10c1.66 0 3 .9 3 2s-1.34 2-3 2H7c-1.66 0-3-.9-3-2s1.34-2 3-2Zm-3 5.47c.84.34 1.87.53 3 .53h10c1.13 0 2.16-.19 3-.53V15c0 1.1-1.34 2-3 2H7c-1.66 0-3-.9-3-2v-2.53Z"/></svg>"#
        }
        "family_restroom" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M16 4c1.1 0 2 .9 2 2s-.9 2-2 2-2-.9-2-2 .9-2 2-2ZM8 4c1.1 0 2 .9 2 2s-.9 2-2 2-2-.9-2-2 .9-2 2-2Zm8 5c2.21 0 4 1.79 4 4v4h-2v5h-4v-5h-1.5l-1.68-4.64A4 4 0 0 1 14.5 9H16ZM8 9c1.86 0 3.43 1.27 3.87 3l1.63 5H12v5H8v-5H6v5H2v-5H0l1.63-5A4 4 0 0 1 5.5 9H8Z"/></svg>"#
        }
        _ => "",
    }
}

fn render_fan_history_table(meta: &EmbedMetadata) -> String {
    let rows = fan_history_rows(meta)
        .iter()
        .map(|row| {
            format!(
                r#"<tr>
                  <td>{period}</td>
                  <td class="right">{fans}</td>
                  <td class="right gain">{gain}</td>
                  <td class="right">{days}</td>
                  <td class="right">{avg_day}</td>
                  <td class="right">{rank}</td>
                  <td><span class="circle-link">{circle}</span></td>
                </tr>"#,
                period = html_escape(&row.period),
                fans = html_escape(&row.fans),
                gain = html_escape(&row.gain),
                days = html_escape(&row.days),
                avg_day = html_escape(&row.avg_day),
                rank = html_escape(&row.rank),
                circle = html_escape(&truncate_chars(&row.circle, 26)),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="fan-table-wrap">
            <table class="fan-table">
              <colgroup>
                <col style="width: 21%">
                <col style="width: 12%">
                <col style="width: 12%">
                <col style="width: 9%">
                <col style="width: 12%">
                <col style="width: 14%">
                <col style="width: 20%">
              </colgroup>
              <thead>
                <tr>
                  <th>Month</th>
                  <th class="right">Fans</th>
                  <th class="right">Gain</th>
                  <th class="right">Days</th>
                  <th class="right">Avg/day</th>
                  <th class="right">Rank</th>
                  <th>Circle</th>
                </tr>
              </thead>
              <tbody>{rows}</tbody>
            </table>
          </div>"#
    )
}

fn fan_history_rows(meta: &EmbedMetadata) -> Vec<FanHistoryRow> {
    let monthly_rows = monthly_fan_history_rows(meta);
    if !monthly_rows.is_empty() {
        return monthly_rows;
    }

    let fans = profile_value(meta, &["Fans"], "profile");
    let total_gain = profile_value(meta, &["Total Gain"], "+0");
    let gain_7d = profile_value(meta, &["7d Gain"], "+0");
    let gain_30d = profile_value(meta, &["30d Gain"], "+0");
    let active_days = profile_value(meta, &["Active Days"], "1");
    let avg_day = profile_value(meta, &["Avg Day"], "0");
    let rank = profile_value(meta, &["Fan Rank"], "-");
    let rank_7d = profile_value(meta, &["7d Rank"], "rolling");
    let rank_30d = profile_value(meta, &["30d Rank"], "rolling");
    let club = profile_value(meta, &["Club"], "-");

    vec![
        FanHistoryRow {
            period: "Current".to_string(),
            fans: fans.clone(),
            gain: total_gain,
            days: active_days,
            avg_day: avg_day.clone(),
            rank,
            circle: club.clone(),
        },
        FanHistoryRow {
            period: "7 days".to_string(),
            fans: estimate_previous_fans(&fans, &gain_7d),
            gain: gain_7d.clone(),
            days: "7".to_string(),
            avg_day: average_for_gain(&gain_7d, 7),
            rank: rank_7d,
            circle: club.clone(),
        },
        FanHistoryRow {
            period: "30 days".to_string(),
            fans: estimate_previous_fans(&fans, &gain_30d),
            gain: gain_30d.clone(),
            days: "30".to_string(),
            avg_day: average_for_gain(&gain_30d, 30),
            rank: rank_30d,
            circle: club.clone(),
        },
        FanHistoryRow {
            period: "All-time".to_string(),
            fans,
            gain: profile_value(meta, &["Total Gain"], "tracked"),
            days: profile_value(meta, &["Active Days"], "active"),
            avg_day,
            rank: profile_value(meta, &["Fan Rank"], "-"),
            circle: club,
        },
    ]
}

fn monthly_fan_history_rows(meta: &EmbedMetadata) -> Vec<FanHistoryRow> {
    (1..=4)
        .filter_map(|index| {
            let period = profile_metric_label(meta, &format!("Fan Month {index}"))?;
            Some(FanHistoryRow {
                period,
                fans: profile_metric_label(meta, &format!("Fan Month {index} Fans"))
                    .unwrap_or_else(|| "-".to_string()),
                gain: profile_metric_label(meta, &format!("Fan Month {index} Gain"))
                    .unwrap_or_else(|| "-".to_string()),
                days: profile_metric_label(meta, &format!("Fan Month {index} Days"))
                    .unwrap_or_else(|| "-".to_string()),
                avg_day: profile_metric_label(meta, &format!("Fan Month {index} Avg Day"))
                    .unwrap_or_else(|| "-".to_string()),
                rank: profile_metric_label(meta, &format!("Fan Month {index} Rank"))
                    .unwrap_or_else(|| "-".to_string()),
                circle: profile_metric_label(meta, &format!("Fan Month {index} Circle"))
                    .unwrap_or_else(|| "-".to_string()),
            })
        })
        .collect()
}

fn profile_metric_label(meta: &EmbedMetadata, label: &str) -> Option<String> {
    meta.metrics
        .iter()
        .find(|metric| metric.label.eq_ignore_ascii_case(label))
        .map(|metric| metric.value.clone())
}

fn render_overview_cards(meta: &EmbedMetadata) -> String {
    let asset_base = profile_value(meta, &["Asset Base"], "https://uma.moe/assets");
    let circle_tier = profile_value(meta, &["Club Tier"], "Rank");
    let circle_tier_id = profile_metric_label(meta, "Club Tier Id");
    let circle_emblem =
        render_profile_circle_rank_emblem(&asset_base, &circle_tier, circle_tier_id.as_deref());

    let summary = format!(
        r#"<section class="overview-card summary-card">
            <div class="overview-card-head">{speed_icon}<h3>Fan Progress</h3></div>
            <div class="summary-stat-grid">
              <div class="rolling-stat-grid">
                {gain_3d}
                {gain_7d}
                {gain_30d}
              </div>
              <div class="historic-stat-grid">
                {fans}
                {total_gain}
                {active_days}
              </div>
            </div>
          </section>"#,
        speed_icon = material_icon_svg("speed"),
        gain_3d = stat_block("3 days", &profile_value(meta, &["3d Gain"], "+0"), true),
        gain_7d = stat_block("7 days", &profile_value(meta, &["7d Gain"], "+0"), true),
        gain_30d = stat_block("30 days", &profile_value(meta, &["30d Gain"], "+0"), true),
        fans = stat_block(
            "Total fans",
            &profile_value(meta, &["Fans"], "profile"),
            false
        ),
        total_gain = stat_block(
            "Total gain",
            &profile_value(meta, &["Total Gain"], "+0"),
            true
        ),
        active_days = stat_block(
            "Active days",
            &profile_value(meta, &["Active Days"], "active"),
            false,
        ),
    );

    let current_circle = format!(
        r#"<section class="overview-card circle-card">
            <div class="overview-card-head">{shield_icon}<h3>Current Circle</h3></div>
            <div class="current-circle">
              <div class="circle-identity">
                {circle_emblem}
                <div class="circle-main">
                  <span class="circle-label">Active circle</span>
                  <b>{club}</b>
                </div>
              </div>
              <div class="circle-stat-row">
                <span class="circle-stat"><span>Members</span><strong>{members}</strong></span>
                <span class="circle-stat"><span>Rank</span><strong>{rank}</strong></span>
                <span class="circle-stat"><span>Fans</span><strong>{fans}</strong></span>
              </div>
            </div>
          </section>"#,
        shield_icon = material_icon_svg("shield"),
        circle_emblem = circle_emblem,
        club = html_escape(&truncate_chars(
            &profile_value(meta, &["Club"], "No public circle"),
            34,
        )),
        members = html_escape(&circle_members_display(&profile_value(
            meta,
            &["Club Members"],
            "-",
        ))),
        rank = html_escape(&profile_value(
            meta,
            &["Club Monthly Rank", "Club Rank"],
            "-",
        )),
        fans = html_escape(&profile_value(meta, &["Club Fans"], "-")),
    );

    let borrow = format!(
        r#"<section class="overview-card borrow-card">
            <div class="overview-card-head">{borrow_icon}<h3>Inheritance</h3>{summary}</div>
            {body}
          </section>"#,
        borrow_icon = material_icon_svg("family_restroom"),
        summary = render_inheritance_head_meta(meta),
        body = render_borrow_body(meta),
    );

    let stadium = format!(
        r#"<section class="overview-card stadium-card">
            <div class="overview-card-head">{stadium_icon}<h3>Team Stadium</h3></div>
            <div class="stadium-summary">
              <div class="stadium-primary"><span>Team eval</span><b>{team}</b></div>
              {team_rows}
            </div>
          </section>"#,
        stadium_icon = material_icon_svg("stadium"),
        team = html_escape(&truncate_chars(&profile_value(meta, &["Team"], "team"), 12)),
        team_rows = render_stadium_team_rows(meta),
    );

    [summary, current_circle, stadium, borrow].join("")
}

fn render_profile_circle_rank_emblem(
    asset_base: &str,
    fallback: &str,
    rank_id: Option<&str>,
) -> String {
    let Some(rank_id) = rank_id.and_then(|rank_id| rank_id.trim().parse::<i64>().ok()) else {
        return format!(
            r#"<span class="profile-circle-rank-emblem">{}</span>"#,
            html_escape(&truncate_chars(fallback, 5))
        );
    };
    let clamped = rank_id.clamp(1, 11);
    let image = asset_url(
        asset_base,
        &format!("images/icon/circle_rank/utx_ico_circle_rank_{clamped:02}.webp"),
    );
    format!(
        r#"<span class="profile-circle-rank-emblem"><img src="{image}" alt="{alt}"></span>"#,
        image = html_escape(&image),
        alt = html_escape(fallback),
    )
}

fn circle_members_display(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "-" || trimmed.contains('/') {
        trimmed.to_string()
    } else {
        format!("{trimmed}/30")
    }
}

fn render_inheritance_head_meta(meta: &EmbedMetadata) -> String {
    let sums = profile_metric_label(meta, "Spark Sums");
    let totals = [
        (
            "blue",
            "B",
            inheritance_spark_total(meta, sums.as_deref(), 'B', &["Blue Sparks"]),
        ),
        (
            "pink",
            "P",
            inheritance_spark_total(meta, sums.as_deref(), 'P', &["Pink Sparks"]),
        ),
        (
            "green",
            "G",
            inheritance_spark_total(meta, sums.as_deref(), 'G', &["Green Sparks"]),
        ),
        (
            "white",
            "W",
            inheritance_spark_total(meta, sums.as_deref(), 'W', &["White Sparks"]),
        ),
    ];
    if totals.iter().all(|(_, _, total)| total.is_none()) {
        return String::new();
    }

    let chips = totals
        .into_iter()
        .map(|(class_name, label, total)| {
            format!(
                r#"<span class="inheritance-total inheritance-total-{class_name}"><span>{label}</span><b>{total}</b></span>"#,
                class_name = class_name,
                label = label,
                total = html_escape(&format_number_grouped(total.unwrap_or_default(), ',')),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(r#"<span class="inheritance-head-totals">{chips}</span>"#)
}

fn inheritance_spark_total(
    meta: &EmbedMetadata,
    sums: Option<&str>,
    marker: char,
    fallback_labels: &[&str],
) -> Option<i64> {
    sums.and_then(|value| spark_sum_value(value, marker))
        .or_else(|| metric_i64(meta, fallback_labels))
}

fn spark_sum_value(value: &str, marker: char) -> Option<i64> {
    value.split_whitespace().find_map(|part| {
        part.strip_prefix(marker)
            .and_then(|number| number.parse::<i64>().ok())
    })
}

fn render_stadium_team_rows(meta: &EmbedMetadata) -> String {
    let members = stadium_members(meta);
    if members.is_empty() {
        return render_stadium_stat_rows(meta);
    }

    let asset_base = profile_value(meta, &["Asset Base"], "https://uma.moe/assets");
    let mut groups = (0..STADIUM_DISTANCE_LABELS.len())
        .map(|_| Vec::new())
        .collect::<Vec<Vec<&StadiumMemberRow>>>();
    for member in &members {
        groups[member.distance_index].push(member);
    }

    let rows = groups
        .iter()
        .enumerate()
        .map(|(index, members)| {
            let icons = if members.is_empty() {
                r#"<span class="stadium-empty-icons">-</span>"#.to_string()
            } else {
                members
                    .iter()
                    .map(|member| {
                        let image_url = asset_url(
                            &asset_base,
                            &format!(
                                "/images/character_stand/chara_stand_{}.webp",
                                member.character_id
                            ),
                        );
                        let title = member
                            .score
                            .as_ref()
                            .map(|score| format!("#{} {score}", member.character_id))
                            .unwrap_or_else(|| format!("#{}", member.character_id));
                        let rank_badge =
                            render_stadium_rank_badge(&asset_base, member.score.as_deref());
                        let running_style =
                            render_stadium_running_style(&asset_base, member.running_style);
                        format!(
                            r#"<span class="stadium-character" title="{title}"><img class="stadium-character-icon" src="{url}" alt="">{rank_badge}{running_style}</span>"#,
                            url = html_escape(&image_url),
                            title = html_escape(&title),
                            rank_badge = rank_badge,
                            running_style = running_style,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("")
            };

            format!(
                r#"<div class="stadium-distance-column">
                    <span class="stadium-distance-label">{label}</span>
                    <div class="stadium-icon-stack">{icons}</div>
                  </div>"#,
                label = html_escape(STADIUM_DISTANCE_LABELS[index]),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(r#"<div class="stadium-team-stack">{rows}</div>"#)
}

fn render_stadium_running_style(asset_base: &str, running_style: Option<i64>) -> String {
    let Some(style) = running_style else {
        return String::new();
    };
    let index = (style - 1).clamp(0, 3);
    let image_webp = asset_url(
        asset_base,
        &format!("/images/icon/common/utx_ico_runstyle_{index:02}.webp"),
    );
    let image_png = asset_url(
        asset_base,
        &format!("/images/icon/common/utx_ico_runstyle_{index:02}.png"),
    );

    format!(
        r#"<span class="stadium-runstyle-badge"><img src="{image_webp}" alt="" onerror="this.onerror=null;this.src='{image_png}'"></span>"#,
        image_webp = html_escape(&image_webp),
        image_png = html_escape(&image_png),
    )
}

fn render_stadium_rank_badge(asset_base: &str, score: Option<&str>) -> String {
    let Some(score) = score.and_then(parse_display_number) else {
        return String::new();
    };
    let rarity = stadium_rank_rarity(score);
    let label = stadium_rank_label(rarity);
    let image = rank_icon_asset_url(asset_base, rarity);

    format!(
        r#"<span class="stadium-rank-badge"><b>{label}</b><img src="{image}" alt="{label}" onload="this.previousElementSibling.style.display='none'" onerror="this.remove()"></span>"#,
        label = html_escape(label),
        image = html_escape(&image),
    )
}

fn rank_icon_asset_url(asset_base: &str, rarity: i64) -> String {
    let rank_index = (rarity - 1).max(0);
    let filename = if rarity < 11 {
        format!("utx_txt_rank_{rank_index:02}.webp")
    } else {
        format!("utx_txt_rank_{rank_index}.webp")
    };

    asset_url(asset_base, &format!("/images/icon/ranks/{filename}"))
}

fn stadium_rank_rarity(score: f64) -> i64 {
    let thresholds = [
        (0.0, 1),
        (300.0, 2),
        (600.0, 3),
        (900.0, 4),
        (1_300.0, 5),
        (1_800.0, 6),
        (2_300.0, 7),
        (2_900.0, 8),
        (3_500.0, 9),
        (4_900.0, 10),
        (6_500.0, 11),
        (8_200.0, 12),
        (10_000.0, 13),
        (12_000.0, 14),
        (14_500.0, 15),
        (15_900.0, 16),
        (17_500.0, 17),
        (20_000.0, 18),
        (22_000.0, 19),
        (24_000.0, 20),
        (26_000.0, 21),
        (28_000.0, 22),
        (30_000.0, 23),
    ];

    thresholds
        .iter()
        .rev()
        .find(|(minimum, _)| score >= *minimum)
        .map(|(_, rarity)| *rarity)
        .unwrap_or(1)
}

fn stadium_rank_label(rarity: i64) -> &'static str {
    match rarity {
        1 => "G",
        2 => "G+",
        3 => "F",
        4 => "F+",
        5 => "E",
        6 => "E+",
        7 => "D",
        8 => "D+",
        9 => "C",
        10 => "C+",
        11 => "B",
        12 => "B+",
        13 => "A",
        14 => "A+",
        15 => "S",
        16 => "S+",
        17 => "SS",
        18 => "SS+",
        19 => "UG",
        20 => "UG1",
        21 => "UG2",
        22 => "UG3",
        _ => "UG4",
    }
}

fn render_stadium_stat_rows(meta: &EmbedMetadata) -> String {
    let rows = [
        ("Roster", profile_value(meta, &["Stadium"], "team")),
        (
            "Distances",
            profile_value(meta, &["Stadium Distances"], "-"),
        ),
        ("Best uma", profile_value(meta, &["Best Uma"], "rank")),
    ]
    .into_iter()
    .map(|(label, value)| {
        format!(
            r#"<div class="stadium-distance-column">
                <span class="stadium-stat-label">{label}</span>
                <b class="stadium-stat-value">{value}</b>
              </div>"#,
            label = html_escape(label),
            value = html_escape(&truncate_chars(&value, 14)),
        )
    })
    .collect::<Vec<_>>()
    .join("");

    format!(r#"<div class="stadium-team-stack">{rows}</div>"#)
}

fn stadium_members(meta: &EmbedMetadata) -> Vec<StadiumMemberRow> {
    let raw = (1..=30)
        .filter_map(|index| {
            let character_label = format!("Stadium Member {index} Character");
            let distance_label = format!("Stadium Member {index} Distance");
            let score_label = format!("Stadium Member {index} Score");
            let running_style_label = format!("Stadium Member {index} Running Style");
            let character_id = metric_i64(meta, &[character_label.as_str()])?;
            let distance = metric_i64(meta, &[distance_label.as_str()])?;
            let score = profile_metric_label(meta, &score_label);
            let running_style = metric_i64(meta, &[running_style_label.as_str()]);
            Some((character_id, distance, score, running_style))
        })
        .collect::<Vec<_>>();

    let zero_based = raw.iter().any(|(_, distance, _, _)| *distance == 0);
    raw.into_iter()
        .filter_map(|(character_id, distance, score, running_style)| {
            normalize_stadium_distance(distance, zero_based).map(|distance_index| {
                StadiumMemberRow {
                    character_id,
                    distance_index,
                    score,
                    running_style,
                }
            })
        })
        .collect()
}

fn normalize_stadium_distance(distance: i64, zero_based: bool) -> Option<usize> {
    let normalized = if zero_based { distance } else { distance - 1 };
    usize::try_from(normalized)
        .ok()
        .filter(|index| *index < STADIUM_DISTANCE_LABELS.len())
}

fn render_borrow_body(meta: &EmbedMetadata) -> String {
    let details = profile_borrow_details(meta);
    format!(
        r#"<div class="profile-borrow-display">{}</div>"#,
        inheritance::render_body(&details, inheritance::InheritanceRenderOptions::profile())
    )
}

fn profile_borrow_details(meta: &EmbedMetadata) -> DatabaseEmbedDetails {
    let main_blue = metric_i64(meta, &["Lineage Main Blue"]);
    let left_blue = metric_i64(meta, &["Lineage Left Blue"]);
    let right_blue = metric_i64(meta, &["Lineage Right Blue"]);
    let main_pink = metric_i64(meta, &["Lineage Main Pink"]);
    let left_pink = metric_i64(meta, &["Lineage Left Pink"]);
    let right_pink = metric_i64(meta, &["Lineage Right Pink"]);
    let main_green = metric_i64(meta, &["Lineage Main Green"]);
    let left_green = metric_i64(meta, &["Lineage Left Green"]);
    let right_green = metric_i64(meta, &["Lineage Right Green"]);

    DatabaseEmbedDetails {
        asset_base_url: profile_value(meta, &["Asset Base"], "https://uma.moe/assets"),
        resources: meta.resources.clone(),
        query_label: "profile borrow".to_string(),
        result_total: 1,
        matched_factor_ids: Vec::new(),
        matched_main_factor_ids: Vec::new(),
        matched_support_card_id: None,
        matched_min_limit_break: None,
        trainer_name: profile_value(meta, &["Trainer"], &display_title(&meta.title)),
        trainer_id: profile_value(meta, &["Trainer ID"], "trainer"),
        record_id: None,
        main_parent_id: metric_i64(meta, &["Lineage Main"]),
        parent_left_id: metric_i64(meta, &["Lineage Left"]),
        parent_right_id: metric_i64(meta, &["Lineage Right"]),
        parent_rank: None,
        parent_rarity: None,
        affinity_score: metric_i64(meta, &["Affinity", "Inheritance"]),
        left_affinity_score: metric_i64(meta, &["Lineage Left Affinity"]),
        right_affinity_score: metric_i64(meta, &["Lineage Right Affinity"]),
        win_count: None,
        white_count: None,
        follower_num: None,
        support_card_id: metric_i64(meta, &["Support Card"]),
        limit_break_count: metric_i64(meta, &["Support LB"]),
        last_updated: None,
        blue_sparks: non_empty_or(
            metric_i64_list(meta, &["Blue Spark Ids"]),
            [main_blue, left_blue, right_blue],
        ),
        pink_sparks: non_empty_or(
            metric_i64_list(meta, &["Pink Spark Ids"]),
            [main_pink, left_pink, right_pink],
        ),
        green_sparks: non_empty_or(
            metric_i64_list(meta, &["Green Spark Ids"]),
            [main_green, left_green, right_green],
        ),
        white_sparks: metric_i64_list(meta, &["White Spark Ids"]),
        main_blue_factors: main_blue,
        main_pink_factors: main_pink,
        main_green_factors: main_green,
        main_white_factors: metric_i64_list(meta, &["Lineage Main White"]),
        left_blue_factors: left_blue,
        left_pink_factors: left_pink,
        left_green_factors: left_green,
        left_white_factors: metric_i64_list(meta, &["Lineage Left White"]),
        right_blue_factors: right_blue,
        right_pink_factors: right_pink,
        right_green_factors: right_green,
        right_white_factors: metric_i64_list(meta, &["Lineage Right White"]),
        main_win_saddles: metric_i64_list(meta, &["Lineage Main Wins"]),
        left_win_saddles: metric_i64_list(meta, &["Lineage Left Wins"]),
        right_win_saddles: metric_i64_list(meta, &["Lineage Right Wins"]),
    }
}

fn non_empty_or(values: Vec<i64>, fallback: [Option<i64>; 3]) -> Vec<i64> {
    if values.is_empty() {
        fallback.into_iter().flatten().collect()
    } else {
        values
    }
}

fn metric_i64(meta: &EmbedMetadata, labels: &[&str]) -> Option<i64> {
    metric_value(&meta.metrics, labels).and_then(|value| first_i64(&value))
}

fn metric_i64_list(meta: &EmbedMetadata, labels: &[&str]) -> Vec<i64> {
    metric_value(&meta.metrics, labels)
        .map(|value| {
            value
                .split(|ch: char| ch == ',' || ch.is_whitespace() || ch == '/')
                .filter_map(first_i64)
                .collect()
        })
        .unwrap_or_default()
}

fn first_i64(value: &str) -> Option<i64> {
    let mut digits = String::new();
    let mut started = false;

    for ch in value.chars() {
        if ch.is_ascii_digit() || (!started && ch == '-') {
            digits.push(ch);
            started = true;
        } else if started {
            break;
        }
    }

    digits.parse::<i64>().ok()
}

fn stat_block(label: &str, value: &str, is_gain: bool) -> String {
    let class_name = if is_gain {
        "stat-block gain"
    } else {
        "stat-block"
    };
    format!(
        r#"<span class="{class_name}"><span>{label}</span><b>{value}</b></span>"#,
        label = html_escape(label),
        value = html_escape(&truncate_chars(value, 14)),
    )
}

fn estimate_previous_fans(fans: &str, gain: &str) -> String {
    let Some(total) = parse_metric_number(fans) else {
        return fans.to_string();
    };
    let Some(gain) = parse_metric_number(gain) else {
        return fans.to_string();
    };

    compact_display((total - gain.abs()).max(0.0))
}

fn average_for_gain(gain: &str, days: u32) -> String {
    let Some(gain) = parse_metric_number(gain) else {
        return "-".to_string();
    };

    compact_display((gain.abs() / f64::from(days)).max(0.0))
}

fn parse_metric_number(value: &str) -> Option<f64> {
    parse_display_number(value.trim().trim_start_matches('+'))
}

fn compact_display(value: f64) -> String {
    let abs = value.abs();
    let (scaled, suffix) = if abs >= 1_000_000_000.0 {
        (value / 1_000_000_000.0, "B")
    } else if abs >= 1_000_000.0 {
        (value / 1_000_000.0, "M")
    } else if abs >= 1_000.0 {
        (value / 1_000.0, "K")
    } else {
        (value, "")
    };
    let mut text = format!("{scaled:.1}");
    if text.ends_with(".0") {
        text.truncate(text.len() - 2);
    }
    format!("{text}{suffix}")
}

fn render_visual(meta: &EmbedMetadata) -> String {
    let name =
        metric_value(&meta.metrics, &["Trainer"]).unwrap_or_else(|| display_title(&meta.title));
    let trainer_id =
        metric_value(&meta.metrics, &["Trainer ID"]).unwrap_or_else(|| "trainer".to_string());
    let fans =
        metric_value(&meta.metrics, &["Fans", "Fan Rank"]).unwrap_or_else(|| "profile".to_string());
    let followers =
        metric_value(&meta.metrics, &["Followers"]).unwrap_or_else(|| "public".to_string());
    let team = metric_value(&meta.metrics, &["Team"]).unwrap_or_else(|| "team".to_string());

    format!(
        r#"<div class="visual-panel profile-visual">
        <div class="profile-hero-strip">
          <div class="profile-avatar"><span>{avatar}</span></div>
          <div class="profile-lines">
            <strong>{name}</strong>
            <span>{trainer_id}</span>
          </div>
        </div>
        <div class="profile-stat-strip">
          <span><b>{fans}</b><small>Fans</small></span>
          <span><b>{followers}</b><small>Followers</small></span>
          <span><b>{team}</b><small>Team</small></span>
        </div>
        <div class="profile-tabs"><span>Profile</span><span>Veterans</span><span>CM</span><span>Titles</span></div>
      </div>"#,
        avatar = html_escape(&short_id(&name)),
        name = html_escape(&truncate_chars(&name, 24)),
        trainer_id = html_escape(&truncate_chars(&trainer_id, 22)),
        fans = html_escape(&truncate_chars(&fans, 12)),
        followers = html_escape(&truncate_chars(&followers, 12)),
        team = html_escape(&truncate_chars(&team, 12)),
    )
}
