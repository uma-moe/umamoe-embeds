use std::time::{SystemTime, UNIX_EPOCH};

use crate::embed::{embed_class_list, EmbedMetadata, TimelineEventDetails};

use super::{asset_url, html_escape, metric_value, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-timeline",
    render_visual,
};

struct TimelineCard {
    marker: String,
    kind: String,
    label: String,
    title: String,
    date: String,
    details: Vec<String>,
    image_path: String,
    participants: Vec<TimelineParticipant>,
}

struct TimelineParticipant {
    name: String,
    image_path: String,
}

struct TimelineGroup {
    class_name: String,
    style: String,
    marker_style: String,
    dot_class: String,
    date_label: String,
    date_detail: String,
    cards: Vec<TimelineCard>,
}

struct TimelineViewModel {
    subline: String,
    today_detail: String,
    today_left: f64,
    groups: Vec<TimelineGroup>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SimpleDate {
    year: i32,
    month: u32,
    day: u32,
}

const TIMELINE_LEFT_EDGE: f64 = 32.0;
const TIMELINE_RIGHT_EDGE: f64 = 1172.0;
const TIMELINE_CARD_WIDTH: f64 = 224.0;
const TIMELINE_CARD_GAP: f64 = 14.0;
const TIMELINE_MIN_ANCHOR_GAP: f64 = 120.0;
const TIMELINE_MIN_LANE_CARD_GAP: f64 = 36.0;
const TIMELINE_FIRST_FUTURE_ANCHOR_GAP: f64 = 64.0;

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none() && super::canonical_path(&meta.canonical_url) == "/timeline"
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let title = html_escape(&truncate_chars(&super::display_title(&meta.title), 42));
    let asset_base = metric_value(&meta.metrics, &["Asset Base"])
        .unwrap_or_else(|| "https://uma.moe/assets".to_string());
    let frontend_origin = metric_value(&meta.metrics, &["Frontend Origin"]).unwrap_or_default();
    let view_model = timeline_view_model(meta);
    let groups = render_release_groups(&asset_base, &frontend_origin, &view_model.groups);
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
      --accent-purple: #ba68c8;
      --accent-pink: #e91e63;
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

    .timeline-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 88px minmax(0, 1fr);
      overflow: hidden;
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.08), transparent 34%),
        linear-gradient(225deg, rgba(129, 199, 132, 0.075), transparent 36%),
        linear-gradient(180deg, rgba(255, 183, 77, 0.045), rgba(0, 0, 0, 0.16)),
        var(--bg-primary);
    }}

    .timeline-header {{
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 32px;
      align-items: center;
      min-width: 0;
      padding: 13px 36px 10px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.08), rgba(255, 183, 77, 0.05)),
        rgba(255, 255, 255, 0.012);
    }}

    .header-copy {{
      display: grid;
      gap: 7px;
      min-width: 0;
    }}

    .timeline-title {{
      margin: 0;
      background: linear-gradient(45deg, var(--accent-primary), var(--accent-secondary) 54%, var(--accent-warning));
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      font-size: 38px;
      font-weight: 880;
      letter-spacing: 0;
      line-height: 0.98;
    }}

    .timeline-subline {{
      margin: 0;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }}

    .timeline-content {{
      position: relative;
      z-index: 1;
      display: block;
      height: 100%;
      min-height: 0;
      padding: 0;
      overflow: hidden;
      background:
        radial-gradient(circle at 46% 40%, rgba(129, 199, 132, 0.08), transparent 360px),
        radial-gradient(circle at 18% 82%, rgba(255, 183, 77, 0.055), transparent 320px),
        linear-gradient(rgba(255, 255, 255, 0.018) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.012) 1px, transparent 1px),
        rgba(255, 255, 255, 0.018);
      background-size: auto, auto, 64px 64px, 64px 64px, auto;
      background-position: center, center, 0 0, 0 0, center;
    }}

    .event-stage {{
      position: absolute;
      inset: 0;
      min-height: 0;
      overflow: hidden;
      border: 0;
      border-radius: 0;
      background: transparent;
    }}

    .timeline-axis {{
      position: absolute;
      z-index: 2;
      left: -2px;
      right: -2px;
      top: 270px;
      height: 4px;
      border-radius: 0;
      background: linear-gradient(90deg, var(--accent-primary), var(--accent-secondary), var(--accent-warning), var(--accent-pink));
      box-shadow: 0 0 24px rgba(100, 181, 246, 0.18);
    }}

    .event-group {{
      position: absolute;
      z-index: 4;
      display: flex;
      align-items: stretch;
      gap: 14px;
      height: 200px;
      overflow: visible;
    }}

    .event-group.top {{
      top: 28px;
    }}

    .event-group.bottom {{
      top: 312px;
    }}

    .event-group::after {{
      content: "";
      position: absolute;
      z-index: 0;
      left: var(--date-anchor, 50%);
      width: 2px;
      height: var(--connector-height, 42px);
      border-radius: 999px;
      background: linear-gradient(180deg, var(--event-border), rgba(255, 255, 255, 0.2));
      transform: translateX(-50%);
    }}

    .event-group.top::after {{
      top: 100%;
    }}

    .event-group.bottom::after {{
      bottom: 100%;
      background: linear-gradient(0deg, var(--event-border), rgba(255, 255, 255, 0.2));
    }}

    .event-card {{
      position: relative;
      z-index: 2;
      display: grid;
      grid-template-rows: 78px minmax(0, 1fr);
      flex: 0 0 224px;
      width: 224px;
      height: 200px;
      overflow: hidden;
      border: 0;
      border-radius: 8px;
      background:
        linear-gradient(135deg, var(--event-glow), rgba(255, 255, 255, 0.016)),
        rgba(17, 17, 17, 0.96);
      box-shadow:
        0 18px 34px rgba(0, 0, 0, 0.32);
    }}

    .event-card::before {{
      content: "";
      position: absolute;
      inset: 0 auto 0 0;
      z-index: 4;
      width: 4px;
      border-radius: 8px 0 0 8px;
      background: var(--event-color);
      pointer-events: none;
    }}

    .date-badge {{
      position: absolute;
      left: var(--date-anchor, 50%);
      z-index: 5;
      display: inline-grid;
      grid-template-columns: auto auto;
      align-items: center;
      gap: 6px;
      min-width: 84px;
      height: 24px;
      padding: 0 10px;
      border: 1px solid var(--event-border);
      border-radius: 999px;
      background:
        linear-gradient(135deg, rgba(255, 255, 255, 0.08), rgba(255, 255, 255, 0.025)),
        rgba(8, 8, 8, 0.92);
      color: var(--event-color);
      font-size: 11px;
      font-weight: 900;
      line-height: 1;
      text-align: center;
      white-space: nowrap;
      box-shadow:
        0 8px 20px rgba(0, 0, 0, 0.36),
        0 0 18px var(--event-glow);
      transform: translateX(-50%);
    }}

    .date-badge small {{
      color: var(--text-secondary);
      font-size: 9px;
      font-weight: 800;
    }}

    .event-group.top .date-badge {{
      bottom: -31px;
    }}

    .event-group.bottom .date-badge {{
      top: -31px;
    }}

    .rail-dot {{
      position: absolute;
      z-index: 20;
      top: 264px;
      width: 15px;
      height: 15px;
      border: 0;
      border-radius: 50%;
      background: var(--event-color);
      box-shadow:
        0 0 14px var(--event-glow),
        0 5px 12px rgba(0, 0, 0, 0.35);
      transform: translateX(-50%);
    }}

    .rail-dot::after {{
      display: none;
    }}

    .today-marker {{
      position: absolute;
      z-index: 24;
      left: 72px;
      top: 257px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 0;
      height: 28px;
      transform: translateX(-50%);
    }}

    .today-label {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 4px;
      min-width: 0;
      height: 28px;
      margin-left: 0;
      padding: 0 10px;
      border: 1px solid rgba(255, 185, 120, 0.7);
      border-radius: 999px;
      background:
        linear-gradient(135deg, #ff8a4c, #f05f2c),
        #ff7043;
      color: #fff7ef;
      font-size: 11px;
      font-weight: 900;
      line-height: 1;
      text-align: center;
      white-space: nowrap;
      box-shadow:
        0 8px 20px rgba(0, 0, 0, 0.35),
        0 0 18px rgba(255, 87, 34, 0.34),
        0 0 0 3px rgba(0, 0, 0, 0.34);
    }}

    .today-label small {{
      color: rgba(255, 243, 232, 0.9);
      font-size: 9px;
      font-weight: 800;
    }}

    .event-group.character,
    .event-card.character,
    .rail-dot.character {{
      --event-color: var(--accent-primary);
      --event-border: rgba(100, 181, 246, 0.42);
      --event-glow: rgba(100, 181, 246, 0.12);
    }}

    .event-group.support,
    .event-card.support,
    .rail-dot.support {{
      --event-color: var(--accent-secondary);
      --event-border: rgba(129, 199, 132, 0.42);
      --event-glow: rgba(129, 199, 132, 0.12);
    }}

    .event-group.story,
    .event-card.story,
    .rail-dot.story {{
      --event-color: var(--accent-warning);
      --event-border: rgba(255, 183, 77, 0.44);
      --event-glow: rgba(255, 183, 77, 0.12);
    }}

    .event-group.campaign,
    .event-card.campaign,
    .rail-dot.campaign {{
      --event-color: var(--accent-teal);
      --event-border: rgba(38, 166, 154, 0.48);
      --event-glow: rgba(38, 166, 154, 0.14);
    }}

    .event-group.champions,
    .event-card.champions,
    .rail-dot.champions {{
      --event-color: var(--accent-purple);
      --event-border: rgba(186, 104, 200, 0.44);
      --event-glow: rgba(186, 104, 200, 0.12);
    }}

    .event-group.legend,
    .event-card.legend,
    .rail-dot.legend {{
      --event-color: #ffd54f;
      --event-border: rgba(255, 213, 79, 0.44);
      --event-glow: rgba(255, 213, 79, 0.13);
    }}

    .event-group.mixed,
    .rail-dot.mixed {{
      --event-color: #9bd6c7;
      --event-border: rgba(155, 214, 199, 0.48);
      --event-glow: rgba(100, 181, 246, 0.13);
    }}

    .rail-dot.mixed {{
      background: var(--event-color);
      box-shadow:
        0 0 14px rgba(129, 199, 132, 0.24),
        0 5px 12px rgba(0, 0, 0, 0.35);
    }}

    .rail-dot.mixed::after {{
      display: none;
    }}

    .event-art {{
      position: relative;
      min-width: 0;
      height: 78px;
      margin: 0;
      overflow: hidden;
      border: 0;
      border-radius: 8px 8px 0 0;
      background:
        linear-gradient(135deg, rgba(255, 255, 255, 0.055), rgba(255, 255, 255, 0.012)),
        rgba(0, 0, 0, 0.24);
      box-shadow: none;
    }}

    .event-copy,
    .event-art {{
      position: relative;
      z-index: 1;
    }}

    .event-art img {{
      position: relative;
      z-index: 2;
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      object-position: center 57%;
      padding: 0;
      color: transparent;
      font-size: 0;
    }}

    .event-art.has-image .event-fallback {{
      position: absolute;
      inset: 0;
      z-index: 1;
      display: none;
    }}

    .event-art.has-image.is-missing .event-fallback {{
      display: grid;
    }}

    .event-art.has-image.is-missing img {{
      display: none;
    }}

    .event-card.champions,
    .event-card.campaign {{
      grid-template-rows: 78px minmax(0, 1fr);
    }}

    .event-card.champions .event-art,
    .event-card.campaign .event-art {{
      height: 78px;
      margin: 0;
      border: 0;
      border-radius: 0;
      box-shadow: none;
    }}

    .event-card.champions .event-art {{
      background:
        linear-gradient(135deg, rgba(186, 104, 200, 0.09), rgba(255, 255, 255, 0.018)),
        #1f1f1f;
    }}

    .event-card.campaign .event-art {{
      background:
        radial-gradient(circle at 30% 34%, rgba(38, 166, 154, 0.23), transparent 42px),
        radial-gradient(circle at 64% 38%, rgba(100, 181, 246, 0.18), transparent 54px),
        #121f1d;
    }}

    .event-fallback {{
      display: grid;
      place-items: center;
      width: 100%;
      height: 100%;
      color: var(--event-color);
      font-size: 30px;
      font-weight: 900;
      letter-spacing: 0;
    }}

    .event-fallback svg {{
      display: block;
      width: 46px;
      height: 46px;
      stroke: currentColor;
      fill: none;
      stroke-width: 2.2;
      stroke-linecap: round;
      stroke-linejoin: round;
    }}

    .campaign-mark {{
      display: inline-grid;
      place-items: center;
      color: #4dd0c6;
      font-size: 34px;
      font-weight: 900;
    }}

    .event-copy {{
      display: grid;
      grid-template-rows: auto auto auto minmax(0, 1fr);
      min-width: 0;
      min-height: 0;
      padding: 7px 11px 10px;
    }}

    .event-meta {{
      display: flex;
      align-items: center;
      justify-content: flex-start;
      gap: 6px;
      min-width: 0;
      margin-bottom: 5px;
      padding-bottom: 0;
      border-bottom: 0;
      color: var(--text-secondary);
      font-size: 9px;
      font-weight: 850;
      letter-spacing: 0.4px;
      line-height: 1;
      text-transform: uppercase;
    }}

    .event-symbol {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex: 0 0 15px;
      place-items: center;
      width: 15px;
      height: 15px;
      color: var(--event-color);
    }}

    .event-symbol svg {{
      display: block;
      width: 15px;
      height: 15px;
      stroke: currentColor;
      fill: none;
      stroke-width: 2.25;
      stroke-linecap: round;
      stroke-linejoin: round;
    }}

    .event-label {{
      overflow: hidden;
      color: var(--text-secondary);
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .event-date {{
      margin: 0;
      color: var(--accent-warning);
      font-size: 9px;
      font-weight: 850;
      line-height: 1;
      white-space: nowrap;
    }}

    .event-title {{
      overflow: hidden;
      margin: 0 0 4px;
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 880;
      line-height: 1.08;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
    }}

    .event-card.character .event-title,
    .event-card.support .event-title {{
      -webkit-line-clamp: 1;
    }}

    .event-details {{
      display: grid;
      gap: 3px;
      min-width: 0;
      margin: -1px 0 5px;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 700;
      line-height: 1.08;
    }}

    .event-participants {{
      display: flex;
      align-items: center;
      justify-content: flex-start;
      gap: 10px;
      min-width: 0;
      min-height: 0;
      overflow: visible;
      margin-top: 5px;
      padding-top: 0;
      border-top: 0;
    }}

    .event-participant {{
      flex: 0 0 auto;
      width: auto;
      height: 85%;
      aspect-ratio: 1;
      overflow: hidden;
      padding: 0;
      border: 0;
      border-radius: 7px;
      background: transparent;
      box-shadow: none;
    }}

    .event-participant img {{
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      object-position: center top;
      border-radius: inherit;
      color: transparent;
      font-size: 0;
    }}

    .event-card.character .event-participant img {{
      object-position: center 14%;
    }}

    .event-card.character .event-participant {{
      transform: translateY(5px);
    }}

    .event-card.support .event-participant {{
      height: 108%;
      aspect-ratio: 4 / 5;
      border-radius: 7px;
      padding: 0;
      transform: translateY(5px);
    }}

    .event-card.support .event-participant img {{
      object-fit: contain;
      object-position: center;
      border-radius: 5px;
    }}

{brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-timeline">
  <main class="timeline-card {class_list} card-view-timeline">
    <header class="timeline-header">
      <div class="header-copy">
        <h1 class="timeline-title">{title}</h1>
        <p class="timeline-subline">{subline}</p>
      </div>
      {brand}
    </header>

    <section class="timeline-content">
      <section class="event-stage" aria-label="Upcoming timeline events">
        <div class="timeline-axis" aria-hidden="true"></div>
        <span class="today-marker" style="left: {today_left}px;" aria-label="Today, {today_detail}">
          <span class="today-label">Today<small>{today_detail}</small></span>
        </span>
        {groups}
      </section>
    </section>
  </main>
</body>
</html>
"#,
        class_list = class_list,
        title = title,
        brand = brand,
        brand_css = brand_css,
        subline = html_escape(&view_model.subline),
        today_detail = html_escape(&view_model.today_detail),
        today_left = format!("{:.0}", view_model.today_left),
        groups = groups,
    )
}

fn render_release_groups(
    asset_base: &str,
    frontend_origin: &str,
    groups: &[TimelineGroup],
) -> String {
    groups
        .iter()
        .map(|group| render_release_group(group, asset_base, frontend_origin))
        .collect::<Vec<_>>()
        .join("")
}

fn timeline_view_model(meta: &EmbedMetadata) -> TimelineViewModel {
    let today = today_utc_date();
    let today_detail = format_short_date(today);

    if let Some((groups, subline, today_left)) = timeline_groups_from_resource(meta, today) {
        return TimelineViewModel {
            subline,
            today_detail,
            today_left,
            groups,
        };
    }

    TimelineViewModel {
        subline: "Upcoming global releases / June 2026".to_string(),
        today_detail,
        today_left: 72.0,
        groups: fallback_timeline_groups(),
    }
}

fn timeline_groups_from_resource(
    meta: &EmbedMetadata,
    today: SimpleDate,
) -> Option<(Vec<TimelineGroup>, String, f64)> {
    let timeline = meta.resources.timeline()?;
    let mut dated_events = timeline
        .events
        .iter()
        .filter_map(|event| {
            let start = parse_resource_date(&event.global_release_date)?;
            let end = event
                .estimated_end_date
                .as_deref()
                .and_then(parse_resource_date)
                .unwrap_or(start);
            Some((start, end, event))
        })
        .collect::<Vec<_>>();

    if dated_events.is_empty() {
        return None;
    }

    dated_events.sort_by(|(left_start, _, left), (right_start, _, right)| {
        left_start
            .cmp(right_start)
            .then_with(|| left.title.cmp(&right.title))
    });

    let current_start = dated_events
        .iter()
        .filter(|(start, end, _)| *start <= today && *end >= today)
        .map(|(start, _, _)| *start)
        .max();
    let has_current = current_start.is_some();
    let mut today_left = 72.0;
    let mut current_anchor = None;
    let mut groups = Vec::new();
    let mut subline_dates: Vec<SimpleDate> = Vec::new();
    let mut top_lane_right = TIMELINE_LEFT_EDGE - TIMELINE_MIN_LANE_CARD_GAP;
    let mut bottom_lane_right = TIMELINE_LEFT_EDGE - TIMELINE_MIN_LANE_CARD_GAP;

    if let Some(current_start) = current_start {
        let current_cards = dated_events
            .iter()
            .filter(|(start, end, _)| *start <= today && *end >= today)
            .filter_map(|(start, _, event)| timeline_card_from_event(event, *start))
            .collect::<Vec<_>>();
        let current_cards = fit_cards_to_width(current_cards, timeline_group_width(3));
        if !current_cards.is_empty() {
            let width = timeline_group_width(current_cards.len());
            let anchor_offset = centered_anchor_offset(current_cards.len());
            let anchor = TIMELINE_LEFT_EDGE + anchor_offset;
            top_lane_right = TIMELINE_LEFT_EDGE + width;
            current_anchor = Some(anchor);
            subline_dates.push(current_start);
            groups.push(make_anchored_timeline_group(
                current_start,
                current_cards,
                "top",
                anchor,
                anchor_offset,
                Some("Current".to_string()),
            ));
        }
    }

    let mut future_groups: Vec<(SimpleDate, Vec<&TimelineEventDetails>)> = Vec::new();
    for (start, _, event) in dated_events
        .iter()
        .filter(|(start, _, _)| *start > today)
        .copied()
    {
        if let Some((last_start, events)) = future_groups.last_mut() {
            if *last_start == start {
                events.push(event);
                continue;
            }
        }
        future_groups.push((start, vec![event]));
    }

    let mut next_anchor = if has_current {
        top_lane_right + TIMELINE_FIRST_FUTURE_ANCHOR_GAP
    } else {
        TIMELINE_LEFT_EDGE + future_anchor_offset(1)
    };
    let mut previous_anchor = current_anchor.unwrap_or(today_left);
    let mut first_future_anchor = None;
    let mut future_index = 0usize;
    for (date, events) in future_groups {
        let candidate_cards = events
            .iter()
            .filter_map(|event| timeline_card_from_event(event, date))
            .collect::<Vec<_>>();
        if candidate_cards.is_empty() {
            break;
        }

        let row_class = if (future_index + usize::from(!has_current)) % 2 == 0 {
            "bottom"
        } else {
            "top"
        };
        let lane_right = if row_class == "top" {
            top_lane_right
        } else {
            bottom_lane_right
        };
        let mut placement = None;
        for count in (1..=candidate_cards.len().min(3)).rev() {
            let anchor_offset = future_anchor_offset(count);
            let width = timeline_group_width(count);
            let anchor =
                resolve_timeline_anchor(next_anchor, previous_anchor, lane_right, anchor_offset);
            let left = anchor - anchor_offset;
            let right = left + width;
            if left >= TIMELINE_LEFT_EDGE - 0.5 && right <= TIMELINE_RIGHT_EDGE + 0.5 {
                placement = Some((count, anchor, anchor_offset, right));
                break;
            }
        }

        let Some((count, anchor, anchor_offset, right)) = placement else {
            break;
        };
        let cards = candidate_cards.into_iter().take(count).collect::<Vec<_>>();

        if first_future_anchor.is_none() {
            first_future_anchor = Some(anchor);
        }
        subline_dates.push(date);
        groups.push(make_anchored_timeline_group(
            date,
            cards,
            row_class,
            anchor,
            anchor_offset,
            None,
        ));
        if row_class == "top" {
            top_lane_right = right;
        } else {
            bottom_lane_right = right;
        }
        previous_anchor = anchor;
        next_anchor = anchor + TIMELINE_MIN_ANCHOR_GAP;
        future_index += 1;
    }

    if groups.is_empty() {
        return None;
    }

    if let (Some(left_anchor), Some(right_anchor)) = (current_anchor, first_future_anchor) {
        today_left = (left_anchor + right_anchor) / 2.0;
    }

    let subline = dynamic_subline(&subline_dates);
    Some((groups, subline, today_left))
}

fn timeline_card_from_event(
    event: &TimelineEventDetails,
    start: SimpleDate,
) -> Option<TimelineCard> {
    let (kind, marker, label) = match event.event_type.as_str() {
        "character_banner" => ("character", "C", "Character Banner"),
        "support_card_banner" => ("support", "S", "Support Card Banner"),
        "paid_banner" => ("campaign", "P", "Premium Banner"),
        "campaign" => ("campaign", "G", "Campaign"),
        "story_event" => ("story", "E", "Story Event"),
        "champions_meeting" => ("champions", "T", "Champions Meeting"),
        "legend_race" => ("legend", "L", "Legend Race"),
        _ => ("campaign", "?", "Event"),
    };

    let end = event
        .estimated_end_date
        .as_deref()
        .and_then(parse_resource_date)
        .unwrap_or(start);
    let mut details = Vec::new();
    if !event.is_confirmed {
        if let Some(score) = event.prediction_likelihood {
            details.push(format!(
                "Predicted {:.0}% fit",
                (score * 100.0).clamp(0.0, 100.0)
            ));
        } else if let Some(kind) = event.prediction_kind.as_deref() {
            details.push(format!("Predicted {kind}"));
        } else {
            details.push("Predicted".to_string());
        }
    }

    Some(TimelineCard {
        marker: marker.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        title: event.title.clone(),
        date: format_date_range(start, end),
        details,
        image_path: event
            .image_path
            .as_deref()
            .map(normalize_timeline_asset_path)
            .unwrap_or_default(),
        participants: timeline_participants_from_event(event),
    })
}

fn timeline_participants_from_event(event: &TimelineEventDetails) -> Vec<TimelineParticipant> {
    let (names, path_prefix, path_suffix): (&[String], &str, &str) = match event.event_type.as_str()
    {
        "support_card_banner" => (
            &event.related_support_cards,
            "images/support_card/half/support_card_s_",
            ".webp",
        ),
        "paid_banner" if !event.related_support_cards.is_empty() => (
            &event.related_support_cards,
            "images/support_card/half/support_card_s_",
            ".webp",
        ),
        _ => (
            &event.related_characters,
            "images/character_stand/chara_stand_",
            ".webp",
        ),
    };

    event
        .pickup_card_ids
        .iter()
        .take(2)
        .enumerate()
        .map(|(index, card_id)| TimelineParticipant {
            name: names
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("Pickup {}", index + 1)),
            image_path: format!("{path_prefix}{card_id}{path_suffix}"),
        })
        .collect()
}

fn normalize_timeline_asset_path(path: &str) -> String {
    let normalized = path
        .trim()
        .trim_start_matches('/')
        .strip_prefix("assets/")
        .unwrap_or_else(|| path.trim().trim_start_matches('/'))
        .to_string();

    if normalized.ends_with(".png")
        && (normalized.contains("/character/banner/") || normalized.contains("/support/banner/"))
    {
        format!("{}.webp", normalized.trim_end_matches(".png"))
    } else {
        normalized
    }
}

fn dynamic_subline(dates: &[SimpleDate]) -> String {
    let Some(first) = dates.first() else {
        return "Upcoming global releases".to_string();
    };
    let Some(last) = dates.last() else {
        return "Upcoming global releases".to_string();
    };

    if first.year == last.year && first.month == last.month {
        format!(
            "Upcoming global releases / {} {}",
            month_name(first.month),
            first.year
        )
    } else if first.year == last.year {
        format!(
            "Upcoming global releases / {}-{} {}",
            month_name(first.month),
            month_name(last.month),
            first.year
        )
    } else {
        format!("Upcoming global releases / {}+", first.year)
    }
}

fn fit_cards_to_width(cards: Vec<TimelineCard>, width: f64) -> Vec<TimelineCard> {
    let max_cards = (1..=cards.len().min(3))
        .rev()
        .find(|count| timeline_group_width(*count) <= width)
        .unwrap_or(0);

    cards.into_iter().take(max_cards).collect()
}

fn centered_anchor_offset(card_count: usize) -> f64 {
    timeline_group_width(card_count) / 2.0
}

fn future_anchor_offset(_card_count: usize) -> f64 {
    TIMELINE_CARD_WIDTH / 2.0
}

fn resolve_timeline_anchor(
    desired_anchor: f64,
    previous_anchor: f64,
    lane_right: f64,
    anchor_offset: f64,
) -> f64 {
    desired_anchor
        .max(previous_anchor + TIMELINE_MIN_ANCHOR_GAP)
        .max(TIMELINE_LEFT_EDGE + anchor_offset)
        .max(lane_right + TIMELINE_MIN_LANE_CARD_GAP + anchor_offset)
}

fn make_anchored_timeline_group(
    date: SimpleDate,
    cards: Vec<TimelineCard>,
    row_class: &str,
    anchor: f64,
    anchor_offset: f64,
    date_detail: Option<String>,
) -> TimelineGroup {
    let kind = timeline_group_kind(&cards);
    let date_detail = date_detail.unwrap_or_else(|| timeline_group_detail(&cards));
    let left = anchor - anchor_offset;

    TimelineGroup {
        class_name: format!("event-group {row_class} {kind}"),
        style: format!(
            "left: {:.0}px; --date-anchor: {:.0}px;",
            left, anchor_offset
        ),
        marker_style: format!("left: {:.0}px;", anchor),
        dot_class: kind,
        date_label: format_short_date(date),
        date_detail,
        cards,
    }
}

fn timeline_group_detail(cards: &[TimelineCard]) -> String {
    if cards.len() == 1 {
        if cards.first().is_some_and(|card| {
            card.details
                .iter()
                .any(|detail| detail.contains("Predicted"))
        }) {
            "Predicted".to_string()
        } else {
            "1 event".to_string()
        }
    } else {
        format!("{} events", cards.len())
    }
}

fn timeline_group_width(card_count: usize) -> f64 {
    let card_count = card_count.max(1) as f64;
    card_count * TIMELINE_CARD_WIDTH + (card_count - 1.0) * TIMELINE_CARD_GAP
}

fn timeline_group_kind(cards: &[TimelineCard]) -> String {
    let Some(first) = cards.first() else {
        return "mixed".to_string();
    };
    if cards.iter().all(|card| card.kind == first.kind) {
        first.kind.clone()
    } else {
        "mixed".to_string()
    }
}

fn fallback_timeline_groups() -> Vec<TimelineGroup> {
    vec![
        TimelineGroup {
            class_name: "event-group top mixed".to_string(),
            style: "left: 88px; --date-anchor: 323px;".to_string(),
            marker_style: "left: 411px;".to_string(),
            dot_class: "mixed".to_string(),
            date_label: "Jun 12".to_string(),
            date_detail: "3 events".to_string(),
            cards: vec![
                fallback_card(
                    "C",
                    "character",
                    "Character Banner",
                    "Taiki Shuttle + 1 more",
                    "Jun 12 - Jun 22, 2026",
                    "images/character/banner/2022_30098.webp",
                    vec![
                        participant(
                            "Taiki Shuttle",
                            "images/character_stand/chara_stand_101002.webp",
                        ),
                        participant(
                            "Mejiro Dober",
                            "images/character_stand/chara_stand_105902.webp",
                        ),
                    ],
                ),
                fallback_card(
                    "S",
                    "support",
                    "Support Card Banner",
                    "El Condor Pasa + 1 more",
                    "Jun 12 - Jun 22, 2026",
                    "images/support/banner/2022_30099.webp",
                    vec![
                        participant(
                            "El Condor Pasa",
                            "images/support_card/half/support_card_s_30102.webp",
                        ),
                        participant(
                            "Matikanetannhauser",
                            "images/support_card/half/support_card_s_30103.webp",
                        ),
                    ],
                ),
                fallback_card(
                    "E",
                    "story",
                    "Story Event",
                    "Seek, Solve, Summer Walk!",
                    "Jun 12 - Jun 23, 2026",
                    "images/story/06_seek_solve_summer_walk_banner.webp",
                    vec![],
                ),
            ],
        },
        TimelineGroup {
            class_name: "event-group bottom mixed".to_string(),
            style: "left: 454px; --date-anchor: 231px;".to_string(),
            marker_style: "left: 685px;".to_string(),
            dot_class: "mixed".to_string(),
            date_label: "Jun 19".to_string(),
            date_detail: "2 events".to_string(),
            cards: vec![
                fallback_card(
                    "C",
                    "character",
                    "Character Banner",
                    "Air Shakur",
                    "Jun 19 - Jun 27, 2026",
                    "images/character/banner/2022_30100.webp",
                    vec![participant(
                        "Air Shakur",
                        "images/character_stand/chara_stand_103601.webp",
                    )],
                ),
                fallback_card(
                    "S",
                    "support",
                    "Support Card Banner",
                    "Air Groove + Gold City",
                    "Jun 19 - Jun 27, 2026",
                    "images/support/banner/2022_30101.webp",
                    vec![
                        participant(
                            "Air Groove",
                            "images/support_card/half/support_card_s_30106.webp",
                        ),
                        participant(
                            "Gold City",
                            "images/support_card/half/support_card_s_20049.webp",
                        ),
                    ],
                ),
            ],
        },
        TimelineGroup {
            class_name: "event-group top champions".to_string(),
            style: "left: 920px; --date-anchor: 103px;".to_string(),
            marker_style: "left: 1023px;".to_string(),
            dot_class: "champions".to_string(),
            date_label: "Jun 22".to_string(),
            date_detail: "Cancer Cup".to_string(),
            cards: vec![TimelineCard {
                marker: "CM".to_string(),
                kind: "champions".to_string(),
                label: "Champions Meeting".to_string(),
                title: "Cancer Cup".to_string(),
                date: "Jun 22 - Jun 30, 2026".to_string(),
                details: vec![
                    "Hanshin Turf / 2200m".to_string(),
                    "Medium / Clockwise".to_string(),
                ],
                image_path: String::new(),
                participants: Vec::new(),
            }],
        },
    ]
}

fn fallback_card(
    marker: &str,
    kind: &str,
    label: &str,
    title: &str,
    date: &str,
    image_path: &str,
    participants: Vec<TimelineParticipant>,
) -> TimelineCard {
    TimelineCard {
        marker: marker.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        title: title.to_string(),
        date: date.to_string(),
        details: Vec::new(),
        image_path: image_path.to_string(),
        participants,
    }
}

fn participant(name: &str, image_path: &str) -> TimelineParticipant {
    TimelineParticipant {
        name: name.to_string(),
        image_path: image_path.to_string(),
    }
}

fn parse_resource_date(value: &str) -> Option<SimpleDate> {
    let date = value.get(0..10)?;
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    Some(SimpleDate { year, month, day })
}

fn today_utc_date() -> SimpleDate {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.as_secs() / 86_400) as i64)
        .unwrap_or_default();
    civil_date_from_unix_days(days)
}

fn civil_date_from_unix_days(days: i64) -> SimpleDate {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };

    SimpleDate {
        year: year as i32,
        month: month as u32,
        day: day as u32,
    }
}

fn format_short_date(date: SimpleDate) -> String {
    format!("{} {}", month_name(date.month), date.day)
}

fn format_date_range(start: SimpleDate, end: SimpleDate) -> String {
    if start == end {
        return format!("{} {}, {}", month_name(start.month), start.day, start.year);
    }

    if start.year == end.year && start.month == end.month {
        format!(
            "{} {} - {} {}, {}",
            month_name(start.month),
            start.day,
            month_name(end.month),
            end.day,
            start.year
        )
    } else if start.year == end.year {
        format!(
            "{} {} - {} {}, {}",
            month_name(start.month),
            start.day,
            month_name(end.month),
            end.day,
            start.year
        )
    } else {
        format!(
            "{} {}, {} - {} {}, {}",
            month_name(start.month),
            start.day,
            start.year,
            month_name(end.month),
            end.day,
            end.year
        )
    }
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "Date",
    }
}

fn render_release_group(group: &TimelineGroup, asset_base: &str, frontend_origin: &str) -> String {
    let cards = group
        .cards
        .iter()
        .map(|card| render_release_card(card, asset_base, frontend_origin))
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="{class_name}" style="{style}">
          {cards}
          <span class="date-badge">{date_label}<small>{date_detail}</small></span>
        </div>
        <span class="rail-dot {dot_class}" style="{marker_style}" aria-hidden="true"></span>"#,
        class_name = html_escape(&group.class_name),
        style = html_escape(&group.style),
        cards = cards,
        date_label = html_escape(&group.date_label),
        date_detail = html_escape(&group.date_detail),
        dot_class = html_escape(&group.dot_class),
        marker_style = html_escape(&group.marker_style),
    )
}

fn render_release_card(card: &TimelineCard, asset_base: &str, frontend_origin: &str) -> String {
    let image = if card.image_path.is_empty() {
        format!(
            r#"<div class="event-art"><span class="event-fallback">{}</span></div>"#,
            render_event_fallback(&card.kind, &card.marker)
        )
    } else {
        let image_url = html_escape(&timeline_event_art_url(
            asset_base,
            frontend_origin,
            &card.image_path,
        ));
        let fallback = render_event_fallback(&card.kind, &card.marker);
        format!(
            r#"<div class="event-art has-image"><img src="{image_url}" alt="{alt}" onerror="this.parentElement.classList.add('is-missing');this.remove();"><span class="event-fallback">{fallback}</span></div>"#,
            alt = html_escape(&card.title),
        )
    };

    let participants = card
        .participants
        .iter()
        .map(|participant| {
            let image_url = html_escape(&asset_url(asset_base, &participant.image_path));
            let name = html_escape(&participant.name);
            format!(
                r#"<span class="event-participant"><img src="{image_url}" alt="{name}"></span>"#
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let details = if card.details.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="event-details">{}</div>"#,
            card.details
                .iter()
                .map(|detail| format!("<span>{}</span>", html_escape(detail)))
                .collect::<Vec<_>>()
                .join("")
        )
    };
    let participants = if participants.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="event-participants">{participants}</div>"#)
    };

    let icon = render_event_icon(&card.kind, &card.marker);

    format!(
        r#"<article class="event-card {kind}">
          {image}
          <div class="event-copy">
            <div class="event-meta"><span class="event-symbol">{icon}</span><span class="event-label">{label}</span></div>
            <h2 class="event-title">{title}</h2>
            {details}
            <p class="event-date">{date}</p>
            {participants}
          </div>
        </article>"#,
        kind = html_escape(&card.kind),
        image = image,
        icon = icon,
        label = html_escape(&card.label),
        date = html_escape(&card.date),
        title = html_escape(&card.title),
        details = details,
        participants = participants,
    )
}

fn timeline_event_art_url(asset_base: &str, frontend_origin: &str, image_path: &str) -> String {
    let path = image_path.trim().trim_start_matches('/');
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }

    if !frontend_origin.trim().is_empty() {
        return asset_url(frontend_origin, &format!("assets/{path}"));
    }

    asset_url(asset_base, path)
}

fn render_event_fallback(kind: &str, marker: &str) -> String {
    if kind == "campaign" {
        r#"<span class="campaign-mark">G1</span>"#.to_string()
    } else {
        render_event_icon(kind, marker)
    }
}

fn render_event_icon(kind: &str, marker: &str) -> String {
    match kind {
        "character" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="7" r="3.2"></circle><path d="M5.8 20c.7-4 3-6.2 6.2-6.2S17.5 16 18.2 20"></path></svg>"#.to_string()
        }
        "support" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 6.5 13.5 4 17 15.8 8.5 18.3z"></path><path d="M10.2 6.3 18.5 8.7 15.3 20 7.1 17.6"></path></svg>"#.to_string()
        }
        "story" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 5.5h6.2c1.1 0 1.8.4 2.3 1.2.5-.8 1.3-1.2 2.3-1.2H21v13h-6.2c-1 0-1.8.4-2.3 1.2-.5-.8-1.2-1.2-2.3-1.2H4z"></path><path d="M12.5 6.7v13"></path></svg>"#.to_string()
        }
        "champions" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 4h8v4.8a4 4 0 0 1-8 0z"></path><path d="M8 6H5.5a2.5 2.5 0 0 0 2.8 3.6"></path><path d="M16 6h2.5a2.5 2.5 0 0 1-2.8 3.6"></path><path d="M12 13v4"></path><path d="M8.5 20h7"></path><path d="M10 17h4"></path></svg>"#.to_string()
        }
        "legend" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.5 14.4 8l5 .8-3.6 3.5.8 5-4.6-2.4-4.6 2.4.8-5L4.6 8.8l5-.8z"></path><path d="M8.7 18.5h6.6"></path><path d="M10 21h4"></path></svg>"#.to_string()
        }
        "campaign" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 13h3l8 4V7l-8 4H4z"></path><path d="M7 13v4"></path><path d="M18 10.2c.8.7 1.2 1.7 1.2 2.8s-.4 2.1-1.2 2.8"></path></svg>"#.to_string()
        }
        _ => format!("<span>{}</span>", html_escape(marker)),
    }
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    r#"<div class="visual-panel timeline-visual">
        <div class="timeline-line"></div>
        <div class="timeline-event-card event-character node-a"><span>Character</span><b>Taiki + Dober</b><small>Jun 12</small></div>
        <div class="timeline-event-card event-support node-b"><span>Support</span><b>SSR pickup</b><small>Jun 12</small></div>
        <div class="timeline-event-card event-story node-c"><span>Story Event</span><b>Summer Walk</b><small>Jun 12</small></div>
      </div>"#
        .to_string()
}
