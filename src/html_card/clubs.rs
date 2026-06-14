use crate::embed::{embed_class_list, EmbedMetadata};

use super::{asset_url, display_title, html_escape, metric_value, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-clubs",
    render_visual,
};

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none() && super::canonical_path(&meta.canonical_url) == "/circles"
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let title = html_escape(&truncate_chars(&display_title(&meta.title), 42));
    let asset_base = metric_value(&meta.metrics, &["Asset Base"])
        .unwrap_or_else(|| "https://uma.moe/assets".to_string());
    let rows = club_rows(meta);
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
      --bg-secondary: #121212;
      --surface-1: rgba(255, 255, 255, 0.026);
      --surface-2: rgba(255, 255, 255, 0.05);
      --surface-3: rgba(255, 255, 255, 0.08);
      --border-subtle: rgba(255, 255, 255, 0.065);
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

    * {{ box-sizing: border-box; }}

    html,
    body {{
      width: 1200px;
      height: 630px;
      margin: 0;
      overflow: hidden;
      background: var(--bg-secondary);
    }}

    .clubs-card {{
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

    .clubs-header {{
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

    .clubs-title {{
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

    .clubs-subline {{
      margin: 0;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }}

    .clubs-body {{
      display: grid;
      grid-template-rows: 22px minmax(0, 1fr);
      gap: 6px;
      min-height: 0;
      padding: 8px 48px 12px;
    }}

    .clubs-head,
    .club-row {{
      display: grid;
      grid-template-columns: 72px 54px minmax(0, 1fr) 108px 132px 216px;
      gap: 12px;
      align-items: center;
      min-width: 0;
    }}

    .clubs-head {{
      padding: 0 14px;
      color: var(--text-disabled);
      font-size: 10px;
      font-weight: 850;
      text-transform: uppercase;
    }}

    .clubs-head span,
    .club-row > * {{
      min-width: 0;
    }}

    .clubs-head span:nth-child(1),
    .clubs-head span:nth-child(2) {{
      text-align: center;
    }}

    .clubs-head span:nth-child(4),
    .clubs-head span:nth-child(5),
    .clubs-head span:nth-child(6) {{
      text-align: right;
    }}

    .clubs-table {{
      display: grid;
      gap: 3px;
      min-height: 0;
      overflow: hidden;
    }}

    .club-row {{
      height: 46px;
      padding: 4px 14px;
      border: 1px solid var(--row-border);
      border-radius: 8px;
      background: var(--row-bg);
      box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.022);
    }}

    .club-row.rank-1 {{
      --row-border: rgba(255, 215, 0, 0.3);
      --row-bg: linear-gradient(135deg, rgba(255, 215, 0, 0.07), rgba(255, 215, 0, 0.015));
      --rank-color: #ffd86b;
    }}

    .club-row.rank-2 {{
      --row-border: rgba(192, 202, 212, 0.24);
      --row-bg: linear-gradient(135deg, rgba(192, 202, 212, 0.06), rgba(192, 202, 212, 0.014));
      --rank-color: #cfd8dc;
    }}

    .club-row.rank-3 {{
      --row-border: rgba(205, 127, 50, 0.27);
      --row-bg: linear-gradient(135deg, rgba(205, 127, 50, 0.064), rgba(205, 127, 50, 0.014));
      --rank-color: #d89b61;
    }}

    .club-row.rank-live {{
      --row-border: var(--border-subtle);
      --row-bg: var(--surface-1);
      --rank-color: var(--text-primary);
    }}

    .rank-stack {{
      display: grid;
      gap: 2px;
      justify-items: center;
      min-width: 0;
      text-align: center;
    }}

    .rank-main {{
      position: relative;
      display: block;
      width: 72px;
      min-width: 0;
    }}

    .rank-number {{
      display: block;
      width: 72px;
      color: var(--rank-color);
      font-size: 18px;
      font-weight: 950;
      line-height: 1;
      text-align: center;
      font-variant-numeric: tabular-nums;
      white-space: nowrap;
    }}

    .yesterday-rank {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      color: var(--text-muted);
      font-size: 7px;
      font-weight: 800;
      line-height: 1;
      white-space: nowrap;
    }}

    .rank-delta {{
      position: absolute;
      top: 50%;
      left: 5px;
      display: inline-flex;
      align-items: center;
      justify-content: flex-start;
      gap: 1px;
      transform: translateY(-50%);
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 950;
      line-height: 1;
      font-variant-numeric: tabular-nums;
    }}

    .rank-delta.up {{
      color: var(--accent-secondary);
    }}

    .rank-delta.down {{
      color: var(--accent-error);
    }}

    .rank-emblem {{
      position: relative;
      display: grid;
      place-items: center;
      justify-self: center;
      align-self: center;
      width: 36px;
      height: 36px;
      overflow: visible;
      color: var(--accent-primary);
      font-size: 12px;
      font-weight: 950;
    }}

    .rank-emblem img {{
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      object-fit: contain;
    }}

    .club-info {{
      display: grid;
      gap: 4px;
      min-width: 0;
      align-self: center;
    }}

    .club-name {{
      overflow: hidden;
      color: var(--accent-primary);
      font-size: 15px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .club-meta {{
      display: flex;
      align-items: center;
      gap: 6px;
      min-width: 0;
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 800;
      line-height: 1;
      white-space: nowrap;
    }}

    .leader {{
      overflow: hidden;
      text-overflow: ellipsis;
      max-width: 120px;
    }}

    .tag {{
      display: inline-flex;
      align-items: center;
      height: 18px;
      padding: 0 6px;
      border: 1px solid var(--tag-border);
      border-radius: 4px;
      background: var(--tag-bg);
      color: var(--tag-color);
      font-size: 9px;
      font-weight: 850;
      white-space: nowrap;
    }}

    .join-open {{
      --tag-border: rgba(129, 199, 132, 0.26);
      --tag-bg: rgba(129, 199, 132, 0.11);
      --tag-color: #81c784;
    }}

    .join-approval {{
      --tag-border: rgba(255, 183, 77, 0.25);
      --tag-bg: rgba(255, 183, 77, 0.105);
      --tag-color: #ffb74d;
    }}

    .join-closed {{
      --tag-border: rgba(239, 83, 80, 0.25);
      --tag-bg: rgba(239, 83, 80, 0.1);
      --tag-color: #ef5350;
    }}

    .policy {{
      display: inline-flex;
      align-items: center;
      height: 18px;
      max-width: 118px;
      padding: 0 7px;
      border-radius: 4px;
      overflow: hidden;
      background: rgba(156, 39, 176, 0.16);
      color: #ce93d8;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .members-box,
    .fans-box {{
      display: grid;
      justify-items: end;
      gap: 3px;
      min-width: 0;
      text-align: right;
    }}

    .metric-value {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 14px;
      font-weight: 900;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .members-box .metric-value {{
      color: var(--accent-primary);
    }}

    .gain-value.positive {{
      color: var(--accent-secondary);
    }}

    .gain-value.negative {{
      color: var(--accent-error);
    }}

    .gain-label {{
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 800;
      line-height: 1;
    }}

    .gain-value {{
      color: var(--text-secondary);
      overflow: visible;
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-overflow: clip;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }}

    .gains-row {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) 8px minmax(0, 1fr);
      align-items: center;
      justify-items: end;
      gap: 5px;
      min-width: 0;
      white-space: nowrap;
    }}

    .gain-item {{
      display: inline-flex;
      align-items: center;
      justify-content: flex-end;
      width: 100%;
      gap: 4px;
      min-width: 0;
    }}

    .gain-sep {{
      color: var(--text-disabled);
      font-size: 12px;
      font-weight: 800;
      line-height: 1;
    }}

    .metric-label {{
      color: var(--text-disabled);
      font-size: 8px;
      font-weight: 850;
      text-transform: uppercase;
      white-space: nowrap;
    }}
{brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-clubs">
  <main class="clubs-card {class_list} card-view-clubs">
    <header class="clubs-header">
      <div class="header-copy">
        <h1 class="clubs-title">{title}</h1>
        <p class="clubs-subline">Live club leaderboard</p>
      </div>
      {brand}
    </header>

    <section class="clubs-body">
      <div class="clubs-head"><span>Rank</span><span>Tier</span><span>Club</span><span>Members</span><span>Fans</span><span>Daily / Today</span></div>
      <div class="clubs-table">
        {rows_html}
      </div>
    </section>
  </main>
</body>
</html>
"#,
        class_list = class_list,
        brand_css = brand_css,
        brand = brand,
        title = title,
        rows_html = render_rows(&rows, &asset_base),
    )
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    super::overview::render_leaderboard_visual(
        "Club Database",
        &[
            ("#1", "Top Club", "Fans"),
            ("#2", "Live Rank", "Points"),
            ("#3", "Open Clubs", "Members"),
        ],
    )
}

fn club_rows(meta: &EmbedMetadata) -> Vec<ClubRow> {
    let mut rows = (1..=10)
        .filter_map(|row| row_from_metrics(meta, row))
        .collect::<Vec<_>>();

    if rows.is_empty() {
        rows = fallback_rows();
    }

    rows
}

fn row_from_metrics(meta: &EmbedMetadata, row: usize) -> Option<ClubRow> {
    let club = metric_value(&meta.metrics, &[&format!("Club {row}")])?;
    Some(ClubRow {
        class_name: rank_class(row),
        rank: metric_value(&meta.metrics, &[&format!("Rank {row}")])
            .unwrap_or_else(|| format!("#{row}")),
        yesterday_rank: metric_value(&meta.metrics, &[&format!("Yesterday Rank {row}")])
            .unwrap_or_else(|| "--".to_string()),
        delta: metric_value(&meta.metrics, &[&format!("Delta {row}")])
            .unwrap_or_else(|| "0".to_string()),
        name: club,
        leader: metric_value(&meta.metrics, &[&format!("Leader {row}")])
            .unwrap_or_else(|| "Leader".to_string()),
        members: metric_value(&meta.metrics, &[&format!("Members {row}")])
            .unwrap_or_else(|| "--/30".to_string()),
        join: metric_value(&meta.metrics, &[&format!("Join {row}")])
            .unwrap_or_else(|| "Unknown".to_string()),
        policy: metric_value(&meta.metrics, &[&format!("Policy {row}")])
            .unwrap_or_else(|| "Playstyle".to_string()),
        club_rank: metric_value(&meta.metrics, &[&format!("Club Rank {row}")])
            .unwrap_or_else(|| "Rank".to_string()),
        club_rank_id: metric_value(&meta.metrics, &[&format!("Club Rank Id {row}")])
            .unwrap_or_default(),
        points: metric_value(&meta.metrics, &[&format!("Points {row}")])
            .unwrap_or_else(|| "Fans".to_string()),
        daily: metric_value(&meta.metrics, &[&format!("Daily {row}")])
            .unwrap_or_else(|| "Daily".to_string()),
        today: metric_value(&meta.metrics, &[&format!("Today {row}")])
            .unwrap_or_else(|| "Live".to_string()),
    })
}

fn fallback_rows() -> Vec<ClubRow> {
    [
        (
            "#1",
            "#1",
            "+2",
            "Uma Utopia",
            "ItsJustWDSam",
            "30/30",
            "Approval",
            "Rank 20+",
            "SS",
            "11",
            "1.6B",
            "+38.0M",
            "+28.0M",
        ),
        (
            "#2",
            "#3",
            "+1",
            "Sprint Stars",
            "Bakushin!",
            "29/30",
            "Open",
            "Log in Daily",
            "SS",
            "11",
            "1.5B",
            "+26.0M",
            "+17.0M",
        ),
        (
            "#3",
            "#1",
            "-2",
            "Blue Roses",
            "RiceFan",
            "30/30",
            "Closed",
            "Rank 500+",
            "S+",
            "10",
            "1.4B",
            "+11.0M",
            "+6.0M",
        ),
        (
            "#4",
            "#4",
            "0",
            "Dream Gate",
            "TeioStep",
            "27/30",
            "Open",
            "Beginners Welcome",
            "S",
            "9",
            "1.3B",
            "+19.0M",
            "+10.0M",
        ),
        (
            "#5",
            "#7",
            "+2",
            "Morning Run",
            "Maya",
            "25/30",
            "Approval",
            "Active at Night",
            "S",
            "9",
            "1.3B",
            "+14.0M",
            "+14.0M",
        ),
        (
            "#6",
            "#6",
            "0",
            "Green Sprint",
            "Falcon",
            "22/30",
            "Open",
            "Let's Party!",
            "A+",
            "8",
            "1.2B",
            "+9.0M",
            "+6.0M",
        ),
        (
            "#7",
            "#5",
            "-2",
            "Training Camp",
            "McQueen",
            "30/30",
            "Approval",
            "Rank 100+",
            "A+",
            "8",
            "1.2B",
            "+5.0M",
            "+6.0M",
        ),
        (
            "#8",
            "#8",
            "0",
            "Victory Road",
            "Oguri",
            "28/30",
            "Open",
            "Rank 2000+",
            "A",
            "7",
            "1.1B",
            "+5.0M",
            "+6.0M",
        ),
        (
            "#9",
            "#10",
            "+1",
            "Starlight Derby",
            "Opera",
            "26/30",
            "Approval",
            "Log in Every 3 Days",
            "A",
            "7",
            "1.1B",
            "+8.0M",
            "+7.0M",
        ),
        (
            "#10",
            "#9",
            "-1",
            "Meadow Bells",
            "Urara",
            "24/30",
            "Open",
            "Laid-back",
            "B+",
            "6",
            "1.0B",
            "+3.0M",
            "+4.0M",
        ),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(
            index,
            (
                rank,
                yesterday_rank,
                delta,
                name,
                leader,
                members,
                join,
                policy,
                club_rank,
                club_rank_id,
                points,
                daily,
                today,
            ),
        )| ClubRow {
            class_name: rank_class(index + 1),
            rank: rank.to_string(),
            yesterday_rank: yesterday_rank.to_string(),
            delta: delta.to_string(),
            name: name.to_string(),
            leader: leader.to_string(),
            members: members.to_string(),
            join: join.to_string(),
            policy: policy.to_string(),
            club_rank: club_rank.to_string(),
            club_rank_id: club_rank_id.to_string(),
            points: points.to_string(),
            daily: daily.to_string(),
            today: today.to_string(),
        },
    )
    .collect()
}

fn render_rows(rows: &[ClubRow], asset_base: &str) -> String {
    rows.iter()
        .map(|row| render_row(row, asset_base))
        .collect::<Vec<_>>()
        .join("")
}

fn render_row(row: &ClubRow, asset_base: &str) -> String {
    let join_class = join_class(&row.join);
    let rank_icon = rank_icon(row, asset_base);
    let rank_delta = rank_delta(&row.delta);
    let gains = render_gains(row);

    format!(
        r#"<article class="club-row {class_name}">
          <div class="rank-stack"><span class="rank-main">{rank_delta}<span class="rank-number">{rank}</span></span><span class="yesterday-rank">Yesterday: {yesterday_rank}</span></div>
          {rank_icon}
          <div class="club-info">
            <strong class="club-name">{name}</strong>
            <div class="club-meta"><span class="tag {join_class}">{join}</span><span class="policy">{policy}</span><span class="leader">Leader: {leader}</span></div>
          </div>
          <div class="members-box"><span class="metric-value">{members}</span><span class="metric-label">members</span></div>
          <div class="fans-box"><span class="metric-value">{points}</span><span class="metric-label">fans</span></div>
          <div class="gains-row">{gains}</div>
        </article>"#,
        class_name = row.class_name,
        rank = html_escape(&row.rank),
        yesterday_rank = html_escape(&row.yesterday_rank),
        rank_delta = rank_delta,
        rank_icon = rank_icon,
        name = html_escape(&truncate_chars(&row.name, 28)),
        join_class = join_class,
        join = html_escape(&row.join),
        policy = html_escape(&truncate_chars(&row.policy, 18)),
        leader = html_escape(&truncate_chars(&row.leader, 18)),
        members = html_escape(&row.members),
        points = html_escape(&row.points),
        gains = gains,
    )
}

fn render_gains(row: &ClubRow) -> String {
    let daily = gain_item("Daily", &row.daily);
    let today = gain_item("Today", &row.today);

    format!(r#"{daily}<span class="gain-sep">&middot;</span>{today}"#)
}

fn gain_item(label: &str, value: &str) -> String {
    let class_name = gain_class(value);

    format!(
        r#"<span class="gain-item"><span class="gain-label">{label}</span><span class="metric-value gain-value {class_name}">{value}</span></span>"#,
        label = html_escape(label),
        value = html_escape(value),
        class_name = class_name,
    )
}

fn rank_delta(delta: &str) -> String {
    let trimmed = delta.trim();
    if trimmed.is_empty() || trimmed == "0" {
        return String::new();
    }

    let (class_name, glyph, value) = if let Some(value) = trimmed.strip_prefix('+') {
        ("up", "&#9650;", value)
    } else if let Some(value) = trimmed.strip_prefix('-') {
        ("down", "&#9660;", value)
    } else {
        ("up", "&#9650;", trimmed)
    };

    format!(
        r#"<span class="rank-delta {class_name}">{glyph}{value}</span>"#,
        class_name = class_name,
        glyph = glyph,
        value = html_escape(value),
    )
}

fn rank_icon(row: &ClubRow, asset_base: &str) -> String {
    let fallback = html_escape(&row.club_rank);
    let rank_id = row.club_rank_id.trim().parse::<i64>().ok();
    let Some(rank_id) = rank_id.filter(|rank_id| (1..=11).contains(rank_id)) else {
        return format!(r#"<span class="rank-emblem"><b>{fallback}</b></span>"#);
    };

    let url = asset_url(
        asset_base,
        &format!(
            "images/icon/circle_rank/utx_ico_circle_rank_{:02}.webp",
            rank_id
        ),
    );
    format!(
        r#"<span class="rank-emblem"><b>{fallback}</b><img src="{url}" alt="{fallback}" onload="this.previousElementSibling.style.display='none'" onerror="this.remove()"></span>"#,
        url = html_escape(&url),
    )
}

fn rank_class(row: usize) -> &'static str {
    match row {
        1 => "rank-1",
        2 => "rank-2",
        3 => "rank-3",
        _ => "rank-live",
    }
}

fn join_class(join: &str) -> &'static str {
    match join.to_ascii_lowercase().as_str() {
        "open" => "join-open",
        "closed" => "join-closed",
        "approval" => "join-approval",
        _ => "join-approval",
    }
}

fn gain_class(value: &str) -> &'static str {
    let value = value.trim();
    if value.starts_with('-') {
        "negative"
    } else if value.starts_with('+') {
        "positive"
    } else {
        "neutral"
    }
}

struct ClubRow {
    class_name: &'static str,
    rank: String,
    yesterday_rank: String,
    delta: String,
    name: String,
    leader: String,
    members: String,
    join: String,
    policy: String,
    club_rank: String,
    club_rank_id: String,
    points: String,
    daily: String,
    today: String,
}
