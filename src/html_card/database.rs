use crate::embed::{embed_class_list, DatabaseEmbedDetails, EmbedMetadata};

use super::{
    asset_url, display_title, format_number_grouped, format_trainer_id_display, html_escape,
    inheritance, truncate_chars,
};

pub(super) const CLASS_NAME: &str = "card-view-database";
const DEFAULT_ASSET_BASE: &str = "https://uma.moe/assets";

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    let path = super::canonical_path(&meta.canonical_url);
    let route_path = super::normalize_route_path(&path);

    meta.database.is_some() || matches!(route_path, "/database" | "/inheritance" | "/support-cards")
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let brand = super::render_brand_corner();
    let brand_css = super::brand_corner_css();
    let (title, subline, content) = match &meta.database {
        Some(database) => (
            html_escape(&truncate_chars(
                &display_trainer_name(&database.trainer_name),
                40,
            )),
            render_result_subline(database),
            render_result_content(database),
        ),
        None => (
            html_escape(&truncate_chars(&display_title(&meta.title), 40)),
            html_escape("Inheritance search / parents, factors, support cards"),
            render_default_content(meta),
        ),
    };

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=1200, initial-scale=1">
  <title>{title}</title>
  <style>
{database_css}
{brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-database">
  <main class="database-card {class_list} card-view-database">
    <header class="database-header">
      <div class="header-copy">
        <h1 class="database-title">{title}</h1>
        <p class="database-subline">{subline}</p>
      </div>
      {brand}
    </header>

    {content}
  </main>
</body>
</html>
"#,
        class_list = class_list,
        title = title,
        subline = subline,
        content = content,
        brand = brand,
        brand_css = brand_css,
        database_css = database_css(),
    )
}

pub(super) fn render_visual(_meta: &EmbedMetadata) -> String {
    format!(
        r#"<div class="visual-panel database-visual">
          {tree}
          {factors}
        </div>"#,
        tree = render_default_tree(),
        factors = render_default_factor_grid(),
    )
}

pub(super) fn render_stats(database: &DatabaseEmbedDetails) -> String {
    let mut items = Vec::new();

    if let Some(affinity) = database.affinity_score {
        items.push(render_stat_values(
            "Affinity",
            &format_number_grouped(affinity, ','),
            "affinity-stat",
        ));
    }
    if let Some(wins) = database.win_count {
        items.push(render_stat_values(
            "G1 Wins",
            &format_number_grouped(wins, ','),
            "wins-stat",
        ));
    }
    if let Some(white) = database.white_count {
        items.push(render_stat_values(
            "White Skills",
            &format_number_grouped(white, ','),
            "white-stat",
        ));
    }

    let rank_score = render_rank_score(database);
    if items.is_empty() && rank_score.is_empty() {
        return String::new();
    }

    format!(
        r#"<div class="record-header-stats"><div class="main-stats">{}</div>{rank_score}</div>"#,
        items.join("")
    )
}

pub(super) fn render_body(database: &DatabaseEmbedDetails) -> String {
    inheritance::render_body(database, inheritance::InheritanceRenderOptions::database())
}

fn render_default_content(_meta: &EmbedMetadata) -> String {
    let summary = html_escape(
        "Find borrowable parents with factor, affinity, support, race, and trainer filters.",
    );

    format!(
        r#"<section class="database-default-content">
      <div class="database-default-dashboard">
        <section class="database-landing-hero">
          <div class="panel-heading">
            <span class="panel-kicker">Database</span>
            <h2>Inheritance Database</h2>
            <p>{summary}</p>
          </div>
          <div class="database-mode-list">
            {modes}
          </div>
          <div class="database-flow-strip">
            <span><b>1</b><small>Filter</small></span>
            <span><b>2</b><small>Search</small></span>
            <span><b>3</b><small>Copy</small></span>
          </div>
        </section>

        <section class="database-tool-board">
          <div class="database-board-head">
            <div>
              <span class="panel-kicker">Filters</span>
              <h2>Filter Builder</h2>
            </div>
          </div>
          <div class="database-filter-overview">
            {filters}
          </div>
        </section>

        <section class="database-preview-card">
          <div class="panel-heading compact">
            <span class="panel-kicker">Preview</span>
            <h2>Borrow result</h2>
          </div>
          {preview}
        </section>
      </div>
    </section>"#,
        summary = summary,
        modes = render_default_mode_cards(),
        filters = render_default_filter_overview(),
        preview = render_default_result_preview(),
    )
}

fn render_default_mode_cards() -> String {
    [
        (
            "parents",
            "Parent Results",
            "Lineage and sparks",
            render_mode_lineage_icon(),
        ),
        (
            "advanced",
            "Factor Search",
            "Stats / aptitude / unique / skill",
            render_mode_advanced_icon(),
        ),
        (
            "borrowing",
            "Borrowing Filters",
            "Support card and LB",
            render_mode_friend_icon(),
        ),
    ]
    .into_iter()
    .map(|(class_name, title, text, icon)| {
        format!(
            r#"<article class="database-mode-card mode-{class_name}">
              <span class="database-mode-art">{icon}</span>
              <span class="database-mode-copy"><b>{title}</b><small>{text}</small></span>
            </article>"#,
            class_name = html_escape(class_name),
            icon = icon,
            title = html_escape(title),
            text = html_escape(text),
        )
    })
    .collect::<Vec<_>>()
    .join("")
}

fn render_mode_lineage_icon() -> String {
    let main =
        render_mode_character_chip("/images/character_stand/chara_stand_101002.webp", "main");
    let left =
        render_mode_character_chip("/images/character_stand/chara_stand_105902.webp", "left");
    let right =
        render_mode_character_chip("/images/character_stand/chara_stand_103601.webp", "right");

    format!(
        r#"<span class="mode-lineage-mini">
          <span class="mode-lineage-top">{main}</span>
          <span class="mode-lineage-bottom">{left}{right}</span>
        </span>"#
    )
}

fn render_mode_character_chip(path: &str, class_name: &str) -> String {
    format!(
        r#"<span class="mode-character-chip mode-character-{class_name}"><img src="{}" alt="" onerror="this.style.visibility='hidden';this.closest('.mode-character-chip').classList.add('asset-missing')"></span>"#,
        html_escape(&default_asset(path)),
        class_name = html_escape(class_name),
    )
}

fn render_mode_advanced_icon() -> String {
    format!(
        r#"<span class="mode-advanced-mini">
          {factors}
        </span>"#,
        factors = render_factor_type_chip_group(&[
            ("B", "Stats", "blue"),
            ("P", "Apt", "pink"),
            ("G", "Uniq", "green"),
            ("W", "Skill", "white"),
        ]),
    )
}

fn render_mode_friend_icon() -> String {
    format!(
        r#"<span class="mode-friend-mini">
          <span class="mode-friend-support"><img src="{support}" alt="" onerror="this.style.visibility='hidden';this.closest('.mode-friend-support').classList.add('asset-missing')"></span>
          <span class="mode-friend-tools">
            <span class="friend-lb-stack"><i></i><i></i><i></i><i></i></span>
            <b>LB</b>
          </span>
        </span>"#,
        support = html_escape(&default_asset(
            "/images/support_card/half/support_card_s_30102.webp"
        )),
    )
}

fn render_default_filter_overview() -> String {
    format!(
        r#"<article class="database-filter-panel factor-panel">
          <div class="filter-panel-head">
            <b>Factor Filters</b>
            <small>Full inheritance + main parent</small>
          </div>
          <div class="factor-filter-columns">
            {inheritance}
            {main_parent}
          </div>
        </article>
        <div class="database-overview-split">
          <article class="database-filter-panel affinity-panel">
            <div class="filter-panel-head">
              <b>Affinity</b>
              <small>target and legacy</small>
            </div>
            <div class="affinity-pair">
              <span class="affinity-slot target">Target</span>
              <span class="affinity-slot legacy">Legacy</span>
            </div>
          </article>
          <article class="database-filter-panel borrow-panel">
            <div class="filter-panel-head">
              <b>Borrowing</b>
              <small>friend + support</small>
            </div>
            <div class="borrow-filter-preview">
              {support}
              <span class="borrow-lb-diamonds"><i></i><i></i><i></i><i></i></span>
              <span class="borrow-filter-tags">{friend_tags}</span>
            </div>
          </article>
        </div>
        <div class="database-overview-split bottom">
          <article class="database-filter-panel criteria-panel">
            <div class="filter-panel-head">
              <b>Rules</b>
              <small>wins / rank / totals</small>
            </div>
            <div class="criteria-strip">
              {criteria}
            </div>
          </article>
          <article class="database-filter-panel race-panel">
            <div class="filter-panel-head">
              <b>Races</b>
              <small>schedule / grade</small>
            </div>
            {schedule}
          </article>
        </div>"#,
        inheritance =
            render_factor_filter_column("Inheritance", "Required", "Preferred", "Lineage"),
        main_parent = render_factor_filter_column("Main Parent", "Required", "Min Skill", "Totals"),
        support = render_support_art(),
        friend_tags = render_filter_chip_group(&[
            ("Allow", "green"),
            ("Hide", "red"),
            ("Trainer ID", "blue")
        ]),
        criteria = render_filter_chip_group(&[
            ("Wins", "gold"),
            ("Rank", "gray"),
            ("Followers", "purple"),
            ("Total Stars", "green"),
        ]),
        schedule = render_schedule_filter_icon_row(),
    )
}

fn render_filter_chip_group(chips: &[(&str, &str)]) -> String {
    chips
        .iter()
        .map(|(label, class_name)| {
            format!(
                r#"<span class="database-filter-chip chip-{class_name}">{label}</span>"#,
                class_name = html_escape(class_name),
                label = html_escape(label),
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_factor_type_chip_group(chips: &[(&str, &str, &str)]) -> String {
    chips
        .iter()
        .map(|(short, label, class_name)| render_factor_type_chip(short, label, class_name))
        .collect::<Vec<_>>()
        .join("")
}

fn render_factor_type_chip(short: &str, label: &str, class_name: &str) -> String {
    format!(
        r#"<span class="database-factor-type-chip chip-{class_name}"><b>{short}</b><em>{label}</em></span>"#,
        class_name = html_escape(class_name),
        short = html_escape(short),
        label = html_escape(label),
    )
}

fn render_factor_filter_column(
    title: &str,
    required: &str,
    preferred: &str,
    extra: &str,
) -> String {
    format!(
        r#"<div class="factor-filter-column">
          <strong>{title}</strong>
          <div class="factor-row">{stats}{aptitude}</div>
          <div class="factor-row">{unique}{skill}</div>
          <div class="factor-row compact">{required}{preferred}{extra}</div>
        </div>"#,
        title = html_escape(title),
        stats = render_factor_type_chip("B", "Stats", "blue"),
        aptitude = render_factor_type_chip("P", "Aptitude", "pink"),
        unique = render_factor_type_chip("G", "Unique", "green"),
        skill = render_factor_type_chip("W", "Skill", "white"),
        required = render_filter_chip_group(&[(required, "gray")]),
        preferred = render_filter_chip_group(&[(preferred, "gray")]),
        extra = render_filter_chip_group(&[(extra, "gray")]),
    )
}

fn render_support_art() -> String {
    format!(
        r#"<span class="filter-support-art"><img src="{}" alt="" onerror="this.style.visibility='hidden';this.closest('.filter-support-art').classList.add('asset-missing')"></span>"#,
        html_escape(&default_asset(
            "/images/support_card/half/support_card_s_30102.webp"
        )),
    )
}

fn render_schedule_filter_icon_row() -> String {
    format!(
        r#"<span class="schedule-mini">
          <span class="schedule-year-row">{years}</span>
          <span class="schedule-ranks">
            <i class="race-grade-g1">G1</i>
            <i class="race-grade-g2">G2</i>
            <i class="race-grade-g3">G3</i>
          </span>
        </span>"#,
        years = render_filter_chip_group(&[
            ("Junior", "blue"),
            ("Classic", "gold"),
            ("Senior", "green"),
        ]),
    )
}

fn render_default_result_preview() -> String {
    let support_image = default_asset("/images/support_card/half/support_card_s_30102.webp");

    format!(
        r#"<div class="database-preview-result">
          {affinity}
          <div class="database-preview-result-body">
            <div class="database-preview-visual-row">
              {lineage}
              <div class="database-support-preview">
                <span class="database-support-thumb"><img src="{support_image}" alt="" onerror="this.style.visibility='hidden';this.closest('.database-support-thumb').classList.add('asset-missing')"></span>
                <em><i></i><i></i><i></i><i></i></em>
              </div>
            </div>
            <div class="database-preview-sparks">{spark_preview}</div>
          </div>
        </div>"#,
        affinity = render_default_affinity_preview(),
        lineage = render_default_lineage_preview(),
        support_image = html_escape(&support_image),
        spark_preview = render_default_spark_preview(),
    )
}

fn render_default_affinity_preview() -> String {
    let stats = [
        ("158", "Affinity", "affinity"),
        ("17", "G1 Wins", "wins"),
        ("23", "White Skills", "white"),
    ]
    .into_iter()
    .map(|(value, label, class_name)| {
        format!(
            r#"<span class="database-preview-stat preview-stat-{class_name}"><b>{value}</b><small>{label}</small></span>"#,
            class_name = html_escape(class_name),
            value = html_escape(value),
            label = html_escape(label),
        )
    })
    .collect::<Vec<_>>()
    .join("");

    format!(
        r#"<div class="database-preview-affinity">{stats}<span class="database-preview-rank"><img src="{}" alt="" onerror="this.style.visibility='hidden'"><span class="database-preview-score-copy"><b>15K</b><small>Score</small></span></span></div>"#,
        html_escape(&default_asset("/images/icon/ranks/utx_txt_rank_14.webp")),
    )
}

fn render_default_lineage_preview() -> String {
    let portraits = [
        (
            "main",
            "/images/character_stand/chara_stand_101002.webp",
            "MAIN",
        ),
        (
            "left",
            "/images/character_stand/chara_stand_101002.webp",
            "GP",
        ),
        (
            "right",
            "/images/character_stand/chara_stand_105902.webp",
            "GP",
        ),
    ];
    let main = render_preview_portrait(portraits[0]);
    let left = render_preview_portrait(portraits[1]);
    let right = render_preview_portrait(portraits[2]);

    format!(
        r#"<div class="database-lineage-preview">
          {main}
          <svg class="database-preview-lines" viewBox="0 0 164 30" preserveAspectRatio="none" aria-hidden="true">
            <path d="M82 0 L82 10 M22 10 L142 10 M22 10 L22 30 M142 10 L142 30" vector-effect="non-scaling-stroke" />
          </svg>
          <div class="database-preview-parents">{left}{right}</div>
        </div>"#
    )
}

fn render_preview_portrait((class_name, path, label): (&str, &str, &str)) -> String {
    format!(
        r#"<div class="database-preview-portrait preview-{class_name}">
          <span class="database-preview-avatar"><img src="{}" alt="" onerror="this.style.visibility='hidden';this.closest('.database-preview-avatar').classList.add('asset-missing')"></span>
          <span class="preview-role">{label}</span>
        </div>"#,
        html_escape(&default_asset(path)),
        class_name = html_escape(class_name),
        label = html_escape(label),
    )
}

fn render_default_spark_preview() -> String {
    [
        ("blue", vec![("3", "Speed"), ("1", "Power")]),
        ("pink", vec![("3", "Mile"), ("2", "Front Runner")]),
        (
            "green",
            vec![("2", "Tokyo 1600"), ("2", "Victory Cheer!")],
        ),
        (
            "white",
            vec![
                ("2", "Corner Adept"),
                ("2", "Straightaway"),
                ("1", "URA Finale"),
                ("1", "Final Push"),
            ],
        ),
    ]
    .into_iter()
    .map(|(class_name, sparks)| {
        let chips = sparks
            .into_iter()
            .map(|(stars, name)| render_default_spark_chip(class_name, stars, name))
            .collect::<Vec<_>>()
            .join("");
        format!(
            r#"<div class="preview-spark-row"><span class="preview-spark-indicator {class_name}"></span><div class="preview-spark-list">{chips}</div></div>"#,
            class_name = html_escape(class_name),
            chips = chips,
        )
    })
    .collect::<Vec<_>>()
    .join("")
}

fn render_default_spark_chip(class_name: &str, stars: &str, name: &str) -> String {
    format!(
        r#"<span class="preview-spark preview-spark-{class_name}"><b>{stars}</b>{star}<strong>{name}</strong></span>"#,
        class_name = html_escape(class_name),
        stars = html_escape(stars),
        star = inheritance::render_spark_star(),
        name = html_escape(name),
    )
}

fn render_default_tree() -> String {
    r#"<div class="database-tree-preview">
      <div class="database-node database-node-main">Main</div>
      <svg class="database-tree-lines" viewBox="0 0 180 56" preserveAspectRatio="none" aria-hidden="true">
        <path d="M90 0 L90 18 M38 18 L142 18 M38 18 L38 56 M142 18 L142 56" vector-effect="non-scaling-stroke" />
      </svg>
      <div class="database-parent-row">
        <div class="database-node database-node-parent">GP</div>
        <div class="database-node database-node-parent">GP</div>
      </div>
    </div>"#
        .to_string()
}

fn render_default_factor_grid() -> String {
    let factors = [
        ("B", "Speed", "3"),
        ("P", "Mile", "3"),
        ("G", "Tokyo", "2"),
        ("W", "Corner", "3"),
        ("B", "Power", "2"),
        ("W", "Straight", "2"),
    ]
    .into_iter()
    .map(|(kind, name, level)| {
        format!(
            r#"<span class="database-factor database-factor-{kind_lower}"><b>{level}</b><span>{kind}</span><strong>{name}</strong></span>"#,
            kind_lower = kind.to_ascii_lowercase(),
            level = html_escape(level),
            kind = html_escape(kind),
            name = html_escape(name),
        )
    })
    .collect::<Vec<_>>()
    .join("");

    format!(r#"<div class="database-factor-grid">{factors}</div>"#)
}

fn default_asset(path: &str) -> String {
    asset_url(DEFAULT_ASSET_BASE, path)
}

fn render_result_content(database: &DatabaseEmbedDetails) -> String {
    let stats = render_stats(database);
    let body = render_body(database);
    let footer = render_result_footer(database);

    format!(
        r#"<section class="database-result-content">
      <article class="database-result-card">
        <header class="database-result-head">{stats}</header>
        {body}
        {footer}
      </article>
    </section>"#,
        stats = stats,
        body = body,
        footer = footer,
    )
}

fn render_result_footer(database: &DatabaseEmbedDetails) -> String {
    let date = database
        .last_updated
        .as_deref()
        .map(compact_result_date)
        .unwrap_or("recently")
        .to_string();
    let record_id = database
        .record_id
        .map(|record_id| format!("record {record_id}"))
        .unwrap_or_else(|| "inheritance".to_string());

    format!(
        r#"<footer class="database-footer" data-query="{}">
          <span class="verified-meta"><span class="verified-text">Verified</span></span>
          <span class="footer-dot">/</span>
          <span>{}</span>
          <span class="footer-dot">/</span>
          <span>{}</span>
        </footer>"#,
        html_escape(&database.query_label),
        html_escape(&record_id),
        html_escape(&date),
    )
}

fn render_result_subline(database: &DatabaseEmbedDetails) -> String {
    let trainer_id = format!("#{}", format_trainer_id_display(&database.trainer_id));
    let query = result_query_label(database);
    let result = result_total_label(database.result_total);

    format!(
        r#"<span class="database-trainer-id" title="Trainer ID">{}</span><span class="subline-dot"></span><span>{}</span><span class="subline-dot"></span><span class="subline-accent">{}</span>"#,
        html_escape(&trainer_id),
        html_escape(&truncate_chars(&query, 30)),
        html_escape(&result),
    )
}

fn result_query_label(database: &DatabaseEmbedDetails) -> String {
    let query = database.query_label.trim();
    let trainer_query = query.strip_prefix("trainer ").unwrap_or(query);
    if trainer_query == database.trainer_id {
        "Trainer ID".to_string()
    } else {
        query.to_string()
    }
}

fn compact_result_date(value: &str) -> &str {
    value
        .split_once('T')
        .map(|(date, _)| date)
        .or_else(|| value.split_once(' ').map(|(date, _)| date))
        .unwrap_or(value)
}

fn result_total_label(total: i64) -> String {
    if total <= 0 {
        "No results".to_string()
    } else if total > 10_000 {
        "Result 1 of 10k+".to_string()
    } else {
        format!("Result 1 of {}", format_number_grouped(total, ','))
    }
}

fn render_stat_values(label: &str, value: &str, class_name: &str) -> String {
    format!(
        r#"<span class="stat-pill {class_name}"><span class="stat-number">{}</span><span class="stat-label">{}</span></span>"#,
        html_escape(value),
        html_escape(label),
    )
}

fn render_rank_score(database: &DatabaseEmbedDetails) -> String {
    if database.parent_rank.is_none() && database.parent_rarity.is_none() {
        return String::new();
    }

    let rank = database.parent_rarity.map(|rarity| {
        format!(
            r#"<span class="rank-image-wrap"><img class="rank-image" src="{}" alt="" onerror="this.style.visibility='hidden'"></span>"#,
            html_escape(&rank_icon_url(database, rarity))
        )
    });
    let score = database.parent_rank.map(|score| {
        format!(
            r#"<span class="stat-pill score-stat"><span class="stat-number">{}</span><span class="stat-label">Score</span></span>"#,
            html_escape(&format_number_grouped(score, '.')),
        )
    });

    format!(
        r#"<div class="rank-score-section">{}{}</div>"#,
        rank.unwrap_or_default(),
        score.unwrap_or_default()
    )
}

fn rank_icon_url(database: &DatabaseEmbedDetails, rarity: i64) -> String {
    let rank_index = (rarity - 1).max(0);
    let filename = if rarity < 11 {
        format!("utx_txt_rank_{rank_index:02}.webp")
    } else {
        format!("utx_txt_rank_{rank_index}.webp")
    };

    asset_url(
        &database.asset_base_url,
        &format!("/images/icon/ranks/{filename}"),
    )
}

fn display_trainer_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "Unknown Trainer".to_string();
    }

    if let Some((name, club)) = trimmed
        .split_once('@')
        .or_else(|| trimmed.split_once('\u{ff20}'))
    {
        let name = name.trim();
        let club = club.trim();
        if !name.is_empty() && !club.is_empty() {
            return format!("{club} | {name}");
        }
    }

    trimmed.to_string()
}

fn database_css() -> &'static str {
    r#"
    :root {
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
    }

    * {
      box-sizing: border-box;
    }

    html,
    body {
      width: 1200px;
      height: 630px;
      margin: 0;
      overflow: hidden;
      background: var(--bg-primary);
    }

    .database-card {
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 88px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 18% 15%, rgba(100, 181, 246, 0.13), transparent 350px),
        radial-gradient(circle at 78% 12%, rgba(129, 199, 132, 0.11), transparent 330px),
        radial-gradient(circle at 55% 92%, rgba(233, 30, 99, 0.08), transparent 360px),
        var(--bg-primary);
    }

    .database-card::before {
      content: "";
      position: absolute;
      inset: 88px 0 0;
      background:
        linear-gradient(rgba(255, 255, 255, 0.026) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px);
      background-size: 74px 74px;
      mask-image: linear-gradient(90deg, transparent, #000 18%, #000 82%, transparent);
      opacity: 0.32;
      pointer-events: none;
    }

    .database-header {
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: center;
      gap: 32px;
      min-width: 0;
      padding: 13px 48px 10px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.08), rgba(255, 183, 77, 0.045)),
        rgba(255, 255, 255, 0.012);
    }

    .database-header .header-copy {
      display: grid;
      gap: 7px;
      min-width: 0;
    }

    .database-title {
      margin: 0;
      overflow: hidden;
      color: #76cfff;
      font-size: 38px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-subline {
      display: flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      margin: 0;
      overflow: hidden;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
      white-space: nowrap;
    }

    .database-subline span {
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .database-subline .database-trainer-id {
      flex: 0 0 auto;
      max-width: 260px;
      padding: 4px 8px;
      border: 1px solid rgba(118, 207, 255, 0.32);
      border-radius: 7px;
      background: rgba(118, 207, 255, 0.11);
      color: #e8f7ff;
      font-size: 15px;
      letter-spacing: 0;
      line-height: 1;
      text-transform: none;
      user-select: all;
      cursor: text;
    }

    .subline-dot {
      width: 4px;
      height: 4px;
      flex: 0 0 auto;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.24);
    }

    .subline-accent {
      color: var(--accent-primary);
    }

    .database-card .embed-brand-corner {
      transform: none;
      height: 70px;
    }

    .database-default-content,
    .database-result-content {
      position: relative;
      z-index: 1;
      min-width: 0;
      min-height: 0;
      padding: 18px 42px 16px;
    }

    .database-default-grid {
      display: grid;
      grid-template-columns: 392px minmax(0, 1fr) 328px;
      gap: 16px;
      width: 100%;
      height: 100%;
      min-height: 0;
    }

    .database-default-dashboard {
      display: grid;
      grid-template-columns: 318px minmax(0, 1fr) 348px;
      gap: 16px;
      width: 100%;
      height: 100%;
      min-height: 0;
    }

    .database-search-panel,
    .database-tree-panel,
    .database-result-panel,
    .database-landing-hero,
    .database-tool-board,
    .database-filter-stack,
    .database-match-board,
    .database-preview-card,
    .database-result-card {
      min-width: 0;
      min-height: 0;
      border: 1px solid var(--border-primary);
      border-radius: 8px;
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.05), rgba(255, 255, 255, 0.018)),
        rgba(255, 255, 255, 0.026);
      box-shadow: 0 18px 42px rgba(0, 0, 0, 0.22);
      overflow: hidden;
    }

    .database-search-panel,
    .database-result-panel,
    .database-landing-hero,
    .database-tool-board,
    .database-filter-stack,
    .database-match-board,
    .database-preview-card {
      display: grid;
      align-content: start;
      gap: 14px;
      padding: 18px;
    }

    .database-landing-hero {
      grid-template-rows: auto minmax(0, 1fr) auto;
      gap: 13px;
    }

    .database-tool-board {
      grid-template-rows: auto minmax(0, 1fr) auto;
      padding: 16px;
      gap: 12px;
    }

    .database-filter-stack {
      grid-template-rows: auto auto minmax(0, 1fr) auto;
    }

    .database-match-board {
      grid-template-rows: auto minmax(0, 1fr) auto;
      padding: 16px;
    }

    .database-preview-card {
      grid-template-rows: auto minmax(0, 1fr);
      padding: 16px;
      gap: 10px;
    }

    .database-tree-panel {
      display: grid;
      place-items: center;
      padding: 20px;
    }

    .panel-heading {
      display: grid;
      gap: 8px;
      min-width: 0;
    }

    .panel-heading.compact {
      gap: 5px;
    }

    .panel-kicker {
      width: fit-content;
      max-width: 100%;
      padding: 4px 8px;
      border: 1px solid rgba(100, 181, 246, 0.3);
      border-radius: 5px;
      background: rgba(100, 181, 246, 0.12);
      color: var(--accent-primary);
      font-size: 10px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .panel-heading h2 {
      margin: 0;
      overflow: hidden;
      color: var(--text-primary);
      font-size: 25px;
      font-weight: 850;
      line-height: 1.08;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-landing-hero .panel-heading h2 {
      overflow: visible;
      font-size: 23px;
      line-height: 1.08;
      text-overflow: clip;
      white-space: normal;
    }

    .panel-heading p {
      display: -webkit-box;
      margin: 0;
      overflow: hidden;
      color: var(--text-secondary);
      font-size: 14px;
      line-height: 1.38;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 3;
    }

    .database-landing-hero .panel-heading p {
      font-size: 13px;
      line-height: 1.34;
    }

    .database-mode-tabs {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 7px;
    }

    .database-mode-tabs span {
      min-width: 0;
      padding: 8px 7px;
      border: 1px solid rgba(100, 181, 246, 0.18);
      border-radius: 6px;
      background: rgba(255, 255, 255, 0.035);
      color: var(--text-muted);
      font-size: 11px;
      font-weight: 850;
      line-height: 1;
      text-align: center;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-mode-tabs .active {
      border-color: rgba(100, 181, 246, 0.42);
      background: rgba(100, 181, 246, 0.12);
      color: #90caf9;
    }

    .database-mode-list {
      display: grid;
      grid-template-rows: repeat(3, minmax(0, 1fr));
      gap: 12px;
      min-width: 0;
      min-height: 0;
      align-content: stretch;
    }

    .database-mode-card {
      display: grid;
      grid-template-columns: 84px minmax(0, 1fr);
      align-items: center;
      gap: 12px;
      min-width: 0;
      min-height: 0;
      height: 100%;
      padding: 10px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 7px;
      background:
        linear-gradient(90deg, rgba(100, 181, 246, 0.055), rgba(255, 255, 255, 0.018)),
        rgba(0, 0, 0, 0.15);
    }

    .database-mode-art {
      position: relative;
      display: grid;
      place-items: center;
      width: 84px;
      min-width: 0;
      height: 64px;
      overflow: hidden;
      border: 1px solid rgba(100, 181, 246, 0.38);
      border-radius: 7px;
      background: rgba(100, 181, 246, 0.08);
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.32);
    }

    .database-mode-card.asset-missing .database-mode-art,
    .mode-character-chip.asset-missing,
    .mode-support-thumb.asset-missing,
    .mode-friend-support.asset-missing,
    .tool-image-chip.asset-missing,
    .filter-support-art.asset-missing,
    .database-preview-avatar.asset-missing,
    .database-support-thumb.asset-missing,
    .trainer-rank-chip.asset-missing {
      background:
        radial-gradient(circle at 38% 28%, rgba(255, 255, 255, 0.12), transparent 18px),
        rgba(100, 181, 246, 0.08);
    }

    .database-mode-art img {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      object-position: center;
      color: transparent;
      font-size: 0;
    }

    .mode-lineage-mini {
      position: relative;
      display: grid;
      justify-items: center;
      gap: 2px;
      width: 74px;
      min-width: 0;
    }

    .mode-lineage-mini::before {
      content: "";
      position: absolute;
      top: 26px;
      left: 21px;
      width: 32px;
      height: 15px;
      border-top: 1px solid rgba(205, 214, 224, 0.4);
      border-left: 1px solid rgba(205, 214, 224, 0.4);
      border-right: 1px solid rgba(205, 214, 224, 0.4);
    }

    .mode-lineage-top,
    .mode-lineage-bottom {
      position: relative;
      z-index: 1;
      display: flex;
      justify-content: center;
      min-width: 0;
    }

    .mode-lineage-bottom {
      gap: 18px;
    }

    .mode-character-chip {
      display: block;
      width: 29px;
      height: 29px;
      overflow: hidden;
      border: 1px solid rgba(100, 181, 246, 0.46);
      border-radius: 50%;
      background: rgba(100, 181, 246, 0.1);
      box-shadow: 0 4px 9px rgba(0, 0, 0, 0.26);
    }

    .mode-character-left {
      border-color: rgba(255, 120, 178, 0.52);
    }

    .mode-character-right {
      border-color: rgba(206, 147, 216, 0.52);
    }

    .mode-affinity-mini {
      position: relative;
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      align-items: center;
      gap: 4px;
      width: 70px;
      min-width: 0;
      padding: 0 5px;
    }

    .mode-affinity-mini::before {
      content: "";
      position: absolute;
      left: 50%;
      top: 12px;
      width: 1px;
      height: 28px;
      background: rgba(255, 255, 255, 0.18);
    }

    .mode-affinity-mini span {
      display: grid;
      place-items: center;
      min-width: 0;
      height: 30px;
      padding: 0 4px;
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 7px;
      font-size: 8px;
      font-weight: 950;
      line-height: 1;
      text-align: center;
      text-transform: uppercase;
    }

    .affinity-target {
      background: rgba(255, 183, 77, 0.12);
      border-color: rgba(255, 183, 77, 0.34);
      color: #ffcf7a;
    }

    .affinity-legacy {
      background: rgba(206, 147, 216, 0.12);
      border-color: rgba(206, 147, 216, 0.34);
      color: #ce93d8;
    }

    .mode-advanced-mini {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      align-content: center;
      justify-items: center;
      gap: 4px;
      width: 74px;
      min-width: 0;
    }

    .mode-advanced-mini .database-filter-chip {
      height: 17px;
      padding: 0 5px;
      font-size: 7.5px;
    }

    .mode-advanced-mini .database-factor-type-chip {
      height: 20px;
      padding: 0 6px 0 4px;
      max-width: 36px;
      font-size: 0;
    }

    .mode-advanced-mini .database-factor-type-chip b {
      width: 15px;
      height: 14px;
      font-size: 8px;
    }

    .mode-friend-mini {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 7px;
      width: 74px;
      min-width: 0;
    }

    .mode-friend-support {
      display: block;
      width: 48px;
      height: 48px;
      overflow: hidden;
      border-radius: 7px;
      background: rgba(100, 181, 246, 0.08);
      box-shadow: 0 4px 10px rgba(0, 0, 0, 0.3);
    }

    .mode-friend-support img {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      object-position: center;
      color: transparent;
      font-size: 0;
    }

    .mode-friend-tools {
      display: grid;
      justify-items: center;
      gap: 3px;
    }

    .mode-friend-tools b {
      display: grid;
      place-items: center;
      width: 22px;
      height: 18px;
      border: 1px solid rgba(100, 181, 246, 0.34);
      border-radius: 5px;
      background: rgba(100, 181, 246, 0.1);
      color: #90caf9;
      font-size: 8px;
      font-weight: 950;
      line-height: 1;
    }

    .friend-lb-stack {
      display: flex;
      gap: 2px;
    }

    .friend-lb-stack i {
      width: 6px;
      height: 10px;
      background: #2196f3;
      clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
    }

    .mode-support-mini,
    .mode-trainer-mini {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 6px;
      width: 70px;
      min-width: 0;
    }

    .mode-support-mini .support-lb-preview {
      flex-direction: column;
      gap: 2px;
    }

    .mode-support-mini .support-lb-preview i {
      width: 8px;
      height: 12px;
    }

    .mode-support-thumb {
      display: block;
      width: 42px;
      height: 42px;
      overflow: hidden;
      border-radius: 7px;
      background: rgba(100, 181, 246, 0.08);
      box-shadow: 0 4px 10px rgba(0, 0, 0, 0.3);
    }

    .mode-trainer-mini .trainer-id-icon {
      width: 30px;
      height: 30px;
      font-size: 18px;
    }

    .mode-trainer-mini .trainer-rank-chip {
      width: 36px;
      height: 28px;
    }

    .mode-trainer-mini .trainer-rank-chip img {
      max-width: 36px;
      max-height: 28px;
    }

    .database-mode-copy {
      display: grid;
      align-content: center;
      gap: 5px;
      min-width: 0;
      min-height: 64px;
    }

    .database-mode-copy b,
    .database-mode-copy small {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-mode-copy b {
      color: var(--text-primary);
      font-size: 15px;
      font-weight: 850;
      line-height: 1;
    }

    .database-mode-copy small {
      color: var(--text-muted);
      font-size: 10.5px;
      font-weight: 780;
      line-height: 1.2;
      white-space: normal;
    }

    .database-flow-strip {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 7px;
      min-width: 0;
    }

    .database-flow-strip span {
      display: grid;
      grid-template-columns: 20px minmax(0, 1fr);
      align-items: center;
      gap: 5px;
      min-width: 0;
      padding: 7px 7px;
      border: 1px solid rgba(100, 181, 246, 0.16);
      border-radius: 6px;
      background: rgba(100, 181, 246, 0.06);
    }

    .database-flow-strip b {
      display: grid;
      place-items: center;
      width: 20px;
      height: 20px;
      border-radius: 50%;
      background: rgba(100, 181, 246, 0.16);
      color: #90caf9;
      font-size: 11px;
      font-weight: 900;
      line-height: 1;
    }

    .database-flow-strip small {
      min-width: 0;
      overflow: hidden;
      color: var(--text-secondary);
      font-size: 9px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }

    .database-filter-overview {
      display: grid;
      grid-template-rows: 156px 102px 102px;
      gap: 10px;
      min-width: 0;
      min-height: 0;
      align-content: stretch;
    }

    .database-filter-panel {
      display: grid;
      gap: 8px;
      min-width: 0;
      min-height: 0;
      padding: 10px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 7px;
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.045), rgba(255, 120, 178, 0.025)),
        rgba(0, 0, 0, 0.14);
      overflow: hidden;
    }

    .filter-panel-head {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: baseline;
      gap: 8px;
      min-width: 0;
    }

    .filter-panel-head b,
    .filter-panel-head small {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .filter-panel-head b {
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 900;
      line-height: 1;
    }

    .filter-panel-head small {
      color: var(--text-muted);
      font-size: 7.6px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }

    .database-overview-split .filter-panel-head {
      grid-template-columns: minmax(0, 1fr);
      gap: 3px;
    }

    .database-overview-split .filter-panel-head b,
    .database-overview-split .filter-panel-head small {
      text-overflow: clip;
    }

    .factor-panel {
      grid-template-rows: auto minmax(0, 1fr);
    }

    .factor-filter-columns {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
      min-width: 0;
      min-height: 0;
    }

    .factor-filter-column {
      display: grid;
      align-content: center;
      gap: 5px;
      min-width: 0;
      min-height: 0;
      padding: 8px;
      border: 1px solid rgba(255, 255, 255, 0.065);
      border-radius: 6px;
      background: rgba(0, 0, 0, 0.14);
    }

    .factor-filter-column strong {
      overflow: hidden;
      color: var(--text-secondary);
      font-size: 10px;
      font-weight: 900;
      line-height: 1;
      text-align: center;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }

    .factor-row {
      display: flex;
      flex-wrap: wrap;
      justify-content: center;
      gap: 5px;
      min-width: 0;
    }

    .factor-row.compact {
      gap: 3px;
    }

    .factor-row.compact .database-filter-chip {
      height: 16px;
      padding: 0 5px;
      font-size: 7.2px;
    }

    .database-overview-split {
      display: grid;
      grid-template-columns: 0.9fr 1.1fr;
      gap: 10px;
      min-width: 0;
      min-height: 0;
    }

    .database-overview-split.bottom {
      grid-template-columns: 1fr 1fr;
    }

    .affinity-panel,
    .borrow-panel,
    .criteria-panel,
    .race-panel {
      align-content: stretch;
    }

    .affinity-pair {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      align-items: center;
      gap: 8px;
      min-width: 0;
      min-height: 0;
    }

    .affinity-slot {
      display: grid;
      place-items: center;
      min-width: 0;
      height: 40px;
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 7px;
      color: var(--text-primary);
      font-size: 10px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
    }

    .affinity-slot.target {
      border-color: rgba(255, 183, 77, 0.38);
      background: rgba(255, 183, 77, 0.1);
      color: #ffcf7a;
    }

    .affinity-slot.legacy {
      border-color: rgba(206, 147, 216, 0.38);
      background: rgba(206, 147, 216, 0.1);
      color: #ce93d8;
    }

    .borrow-filter-preview {
      display: grid;
      grid-template-columns: 38px minmax(0, 1fr);
      grid-template-rows: 17px 18px;
      grid-template-areas:
        "support diamonds"
        "support tags";
      align-items: center;
      gap: 3px 8px;
      min-width: 0;
      min-height: 0;
    }

    .filter-support-art {
      grid-area: support;
      display: block;
      width: 38px;
      height: 38px;
      overflow: hidden;
      border: 1px solid rgba(100, 181, 246, 0.28);
      border-radius: 7px;
      background: rgba(100, 181, 246, 0.08);
      box-shadow: 0 4px 10px rgba(0, 0, 0, 0.32);
    }

    .filter-support-art img {
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      object-position: center;
      color: transparent;
      font-size: 0;
    }

    .borrow-lb-diamonds {
      grid-area: diamonds;
      display: flex;
      justify-content: center;
      gap: 4px;
      min-width: 0;
    }

    .borrow-lb-diamonds i {
      width: 9px;
      height: 15px;
      background: #2196f3;
      clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
    }

    .borrow-filter-tags,
    .criteria-strip,
    .schedule-year-row {
      display: flex;
      flex-wrap: wrap;
      align-content: center;
      align-items: center;
      justify-content: center;
      gap: 4px;
      min-width: 0;
    }

    .borrow-filter-tags {
      grid-area: tags;
      justify-content: center;
      gap: 4px;
    }

    .borrow-filter-tags .database-filter-chip {
      height: 16px;
      padding: 0 5px;
      font-size: 7.5px;
    }

    .criteria-strip {
      min-height: 42px;
    }

    .race-panel .schedule-mini {
      align-content: center;
      gap: 7px;
      min-height: 42px;
    }

    .database-tool-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      grid-template-rows: repeat(3, minmax(0, 1fr));
      gap: 10px;
      min-width: 0;
      min-height: 0;
      align-content: stretch;
    }

    .database-tool-tile {
      display: grid;
      grid-template-rows: 40px auto;
      justify-items: center;
      align-content: center;
      gap: 6px;
      min-width: 0;
      min-height: 0;
      padding: 8px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 7px;
      background:
        linear-gradient(135deg, rgba(255, 255, 255, 0.04), rgba(0, 0, 0, 0.1)),
        rgba(0, 0, 0, 0.12);
    }

    .database-tool-icon {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      align-content: center;
      justify-content: center;
      gap: 3px;
      width: 100%;
      min-width: 0;
      height: 40px;
      overflow: hidden;
    }

    .database-filter-chip {
      display: inline-grid;
      place-items: center;
      min-width: 0;
      height: 18px;
      padding: 0 6px;
      border: 1px solid rgba(255, 255, 255, 0.11);
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.055);
      color: var(--text-secondary);
      font-size: 8.5px;
      font-weight: 900;
      line-height: 1;
      white-space: nowrap;
    }

    .database-factor-type-chip {
      display: inline-flex;
      align-items: center;
      gap: 4px;
      min-width: 0;
      height: 19px;
      padding: 0 7px 0 3px;
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 5px;
      background: rgba(255, 255, 255, 0.045);
      color: var(--text-secondary);
      font-size: 8.2px;
      font-weight: 900;
      line-height: 1;
      white-space: nowrap;
    }

    .database-factor-type-chip b {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex: 0 0 auto;
      width: 15px;
      height: 14px;
      box-sizing: border-box;
      padding-top: 3px;
      border-radius: 4px;
      color: #ffffff;
      font-size: 8px;
      font-weight: 950;
      line-height: 1;
    }

    .database-factor-type-chip em {
      min-width: 0;
      overflow: hidden;
      font-style: normal;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-factor-type-chip.chip-blue b {
      background: rgba(33, 150, 243, 0.82);
    }

    .database-factor-type-chip.chip-pink b {
      background: rgba(233, 30, 99, 0.82);
    }

    .database-factor-type-chip.chip-green b {
      background: rgba(76, 175, 80, 0.82);
    }

    .database-factor-type-chip.chip-white b,
    .database-factor-type-chip.chip-gray b {
      background: rgba(189, 189, 189, 0.72);
      color: #101214;
    }

    .chip-blue {
      border-color: rgba(33, 150, 243, 0.38);
      background: rgba(33, 150, 243, 0.16);
      color: #90caf9;
    }

    .chip-pink {
      border-color: rgba(233, 30, 99, 0.4);
      background: rgba(233, 30, 99, 0.16);
      color: #f06292;
    }

    .chip-green {
      border-color: rgba(76, 175, 80, 0.4);
      background: rgba(76, 175, 80, 0.15);
      color: #81c784;
    }

    .chip-white,
    .chip-gray {
      border-color: rgba(189, 189, 189, 0.28);
      background: rgba(189, 189, 189, 0.12);
      color: #d0d0d0;
    }

    .chip-amber,
    .chip-gold {
      border-color: rgba(255, 183, 77, 0.42);
      background: rgba(255, 183, 77, 0.14);
      color: #ffcf7a;
    }

    .chip-purple {
      border-color: rgba(206, 147, 216, 0.42);
      background: rgba(206, 147, 216, 0.13);
      color: #ce93d8;
    }

    .chip-red {
      border-color: rgba(239, 83, 80, 0.42);
      background: rgba(239, 83, 80, 0.12);
      color: #ef9a9a;
    }

    .database-tool-icon .database-factor-b,
    .database-tool-icon .database-factor-p,
    .database-tool-icon .database-factor-g,
    .database-tool-icon .database-factor-w {
      display: grid;
      place-items: center;
      width: 24px;
      height: 24px;
      border-radius: 6px;
      font-size: 12px;
      font-weight: 900;
      line-height: 1;
    }

    .database-tool-icon .database-factor-b {
      background: rgba(33, 150, 243, 0.18);
      color: #64b5f6;
    }

    .database-tool-icon .database-factor-p {
      background: rgba(233, 30, 99, 0.18);
      color: #f06292;
    }

    .database-tool-icon .database-factor-g {
      background: rgba(76, 175, 80, 0.18);
      color: #81c784;
    }

    .database-tool-icon .database-factor-w {
      background: rgba(189, 189, 189, 0.16);
      color: #d0d0d0;
    }

    .tool-image-chip {
      position: relative;
      display: grid;
      place-items: center;
      overflow: hidden;
      flex: 0 0 auto;
      background: rgba(100, 181, 246, 0.08);
      color: #90caf9;
      font-size: 10px;
      font-weight: 900;
      line-height: 1;
    }

    .tool-image-chip span {
      position: absolute;
      inset: 0;
      display: grid;
      place-items: center;
      background:
        radial-gradient(circle at 42% 28%, rgba(255, 255, 255, 0.13), transparent 16px),
        rgba(100, 181, 246, 0.08);
    }

    .tool-image-chip img {
      position: relative;
      z-index: 1;
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      object-position: top;
      color: transparent;
      font-size: 0;
    }

    .tool-character {
      width: 40px;
      height: 40px;
      margin-left: -13px;
      border: 1px solid rgba(100, 181, 246, 0.38);
      border-radius: 50%;
    }

    .tool-character:first-child {
      margin-left: 0;
    }

    .race-filter-chip {
      display: grid;
      grid-template-rows: 19px auto;
      justify-items: center;
      gap: 3px;
      min-width: 34px;
      color: var(--text-secondary);
      font-size: 8px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
    }

    .race-filter-chip b {
      display: grid;
      place-items: center;
      width: 24px;
      height: 19px;
      border: 1px solid rgba(255, 255, 255, 0.12);
      border-radius: 999px;
      font-size: 11px;
      font-weight: 900;
      line-height: 1;
    }

    .race-filter-chip small {
      max-width: 38px;
      overflow: hidden;
      color: var(--text-muted);
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .race-turf b {
      border-color: rgba(129, 199, 132, 0.42);
      background: rgba(76, 175, 80, 0.15);
      color: #81c784;
    }

    .race-dirt b {
      border-color: rgba(255, 183, 77, 0.42);
      background: rgba(255, 183, 77, 0.13);
      color: #ffb74d;
    }

    .race-mile b,
    .race-long b {
      border-color: rgba(100, 181, 246, 0.4);
      background: rgba(100, 181, 246, 0.12);
      color: #90caf9;
    }

    .tool-support-card {
      width: 42px;
      height: 42px;
      border-radius: 7px;
      box-shadow: 0 3px 8px rgba(0, 0, 0, 0.35);
    }

    .support-lb-preview {
      display: flex;
      align-items: center;
      gap: 3px;
    }

    .support-lb-preview i {
      width: 10px;
      height: 17px;
      background: #2196f3;
      clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
    }

    .schedule-mini {
      display: grid;
      justify-items: center;
      align-content: center;
      gap: 5px;
      min-width: 0;
      width: 100%;
    }

    .schedule-ranks {
      display: flex;
      justify-content: center;
      gap: 4px;
      min-width: 0;
    }

    .schedule-ranks i {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 22px;
      height: 15px;
      box-sizing: border-box;
      padding-top: 3px;
      border-radius: 4px;
      color: #ffffff;
      font-size: 8px;
      font-style: normal;
      font-weight: 950;
      line-height: 1;
    }

    .race-grade-g1 {
      background: rgba(33, 150, 243, 0.34);
    }

    .race-grade-g2 {
      background: rgba(233, 30, 99, 0.34);
    }

    .race-grade-g3 {
      background: rgba(76, 175, 80, 0.34);
    }

    .database-tool-copy {
      display: grid;
      gap: 5px;
      justify-items: center;
      width: 100%;
      min-width: 0;
      text-align: center;
    }

    .database-tool-copy b,
    .database-tool-copy small {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-tool-copy b {
      color: var(--text-primary);
      font-size: 14px;
      font-weight: 850;
      line-height: 1;
    }

    .database-tool-copy small {
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 780;
      line-height: 1.05;
    }

    .trainer-id-icon {
      display: grid;
      place-items: center;
      width: 34px;
      height: 34px;
      border: 1px solid rgba(100, 181, 246, 0.35);
      border-radius: 8px;
      background: rgba(100, 181, 246, 0.12);
      color: #90caf9;
      font-size: 20px;
      font-weight: 900;
      line-height: 1;
    }

    .trainer-rank-chip {
      position: relative;
      display: grid;
      place-items: center;
      width: 44px;
      height: 32px;
      overflow: hidden;
      color: #ffdf78;
      font-size: 12px;
      font-weight: 900;
      line-height: 1;
    }

    .trainer-rank-chip img {
      position: relative;
      z-index: 1;
      display: block;
      max-width: 44px;
      max-height: 32px;
      object-fit: contain;
      filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.34));
    }

    .trainer-rank-chip span {
      position: absolute;
      inset: 0;
      display: grid;
      place-items: center;
    }

    .tool-spark-chip {
      display: inline-flex;
      align-items: center;
      gap: 3px;
      min-width: 0;
      max-width: 34px;
      height: 22px;
      padding: 0 6px;
      border: 1px solid rgba(189, 189, 189, 0.28);
      border-radius: 5px;
      background: rgba(255, 255, 255, 0.055);
      color: #d0d0d0;
      font-size: 9px;
      font-weight: 850;
      line-height: 1;
    }

    .tool-spark-chip b,
    .tool-spark-chip small {
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .tool-spark-chip b {
      color: var(--text-primary);
      font-weight: 900;
      font-variant-numeric: tabular-nums;
    }

    .tool-spark-chip small {
      font-size: 9px;
      font-weight: 820;
    }

    .tool-spark-chip .spark-star {
      width: 10px;
      height: 10px;
      flex: 0 0 auto;
      fill: currentColor;
    }

    .database-filter-board {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }

    .database-filter-stack-list {
      display: grid;
      gap: 9px;
      min-width: 0;
      min-height: 0;
      align-content: start;
    }

    .database-filter-board span,
    .database-filter-stack-list span,
    .database-metric-strip span {
      display: grid;
      gap: 4px;
      min-width: 0;
      padding: 10px;
      border: 1px solid rgba(255, 255, 255, 0.07);
      border-radius: 6px;
      background: rgba(0, 0, 0, 0.16);
    }

    .database-filter-board b,
    .database-filter-stack-list b,
    .database-metric-strip b {
      overflow: hidden;
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-filter-board small,
    .database-filter-stack-list small,
    .database-metric-strip small {
      overflow: hidden;
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 800;
      line-height: 1.15;
      text-overflow: ellipsis;
      white-space: nowrap;
      text-transform: uppercase;
    }

    .database-filter-stack-list em {
      justify-self: start;
      padding: 3px 6px;
      border-radius: 4px;
      background: rgba(255, 255, 255, 0.055);
      color: var(--text-secondary);
      font-size: 10px;
      font-style: normal;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }

    .database-filter-stack-list .filter-blue {
      border-left: 3px solid #2196f3;
    }

    .database-filter-stack-list .filter-pink {
      border-left: 3px solid #e91e63;
    }

    .database-filter-stack-list .filter-green {
      border-left: 3px solid #4caf50;
    }

    .database-filter-stack-list .filter-white {
      border-left: 3px solid #bdbdbd;
    }

    .database-metric-strip {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }

    .database-board-head {
      display: flex;
      align-items: end;
      justify-content: space-between;
      gap: 14px;
      min-width: 0;
    }

    .database-board-head div {
      display: grid;
      gap: 6px;
      min-width: 0;
    }

    .database-board-head h2 {
      margin: 0;
      overflow: hidden;
      color: var(--text-primary);
      font-size: 24px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-board-head strong {
      flex: 0 0 auto;
      padding: 6px 9px;
      border: 1px solid rgba(100, 181, 246, 0.24);
      border-radius: 6px;
      background: rgba(100, 181, 246, 0.08);
      color: #90caf9;
      font-size: 11px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
      white-space: nowrap;
    }

    .database-result-list {
      display: grid;
      gap: 9px;
      min-width: 0;
      min-height: 0;
      align-content: start;
    }

    .database-result-row {
      display: grid;
      grid-template-columns: 42px minmax(0, 1fr) 66px 58px 66px;
      align-items: center;
      gap: 8px;
      min-width: 0;
      min-height: 60px;
      padding: 9px 10px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 7px;
      background:
        linear-gradient(90deg, rgba(100, 181, 246, 0.06), rgba(255, 255, 255, 0.018)),
        rgba(0, 0, 0, 0.15);
    }

    .result-rank {
      color: var(--accent-warning);
      font-size: 18px;
      font-weight: 900;
      line-height: 1;
      text-align: center;
    }

    .result-name {
      display: grid;
      gap: 5px;
      min-width: 0;
    }

    .result-name b,
    .result-name small {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .result-name b {
      color: var(--text-primary);
      font-size: 14px;
      font-weight: 850;
      line-height: 1;
    }

    .result-name small {
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }

    .result-stat {
      display: grid;
      gap: 4px;
      min-width: 0;
      padding: 7px 6px;
      border: 1px solid rgba(255, 255, 255, 0.065);
      border-radius: 6px;
      background: rgba(255, 255, 255, 0.035);
      text-align: center;
    }

    .result-stat b,
    .result-stat small {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .result-stat b {
      font-size: 15px;
      font-weight: 900;
      line-height: 1;
      font-variant-numeric: tabular-nums;
    }

    .result-stat small {
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
    }

    .stat-affinity b {
      color: var(--accent-pink);
    }

    .stat-wins b {
      color: var(--accent-secondary);
    }

    .stat-white b {
      color: var(--accent-warning);
    }

    .database-factor-lane {
      min-width: 0;
      padding-top: 10px;
      border-top: 1px solid rgba(255, 255, 255, 0.065);
    }

    .database-factor-lane .database-factor-grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .database-filter-summary {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 6px;
      min-width: 0;
    }

    .database-filter-summary span {
      display: grid;
      gap: 4px;
      min-width: 0;
      padding: 7px 6px;
      border: 1px solid rgba(255, 255, 255, 0.065);
      border-radius: 5px;
      background: rgba(0, 0, 0, 0.13);
    }

    .database-filter-summary b,
    .database-filter-summary small {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-filter-summary b {
      color: var(--text-primary);
      font-size: 10px;
      font-weight: 900;
      line-height: 1;
    }

    .database-filter-summary small {
      color: var(--text-muted);
      font-size: 7.5px;
      font-weight: 820;
      line-height: 1;
      text-transform: uppercase;
    }

    .database-power-note {
      margin: 7px 0 0;
      overflow: hidden;
      color: rgba(176, 190, 197, 0.62);
      font-size: 8px;
      font-weight: 800;
      line-height: 1;
      text-align: right;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }

    .database-tree-preview {
      display: grid;
      grid-template-rows: 86px 62px 76px;
      justify-items: center;
      align-content: center;
      width: 100%;
      min-width: 0;
      min-height: 0;
    }

    .database-node {
      display: grid;
      place-items: center;
      width: 78px;
      height: 78px;
      border: 2px solid rgba(100, 181, 246, 0.58);
      border-radius: 50%;
      background: rgba(100, 181, 246, 0.09);
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 850;
      line-height: 1;
      text-transform: uppercase;
    }

    .database-node-parent {
      width: 64px;
      height: 64px;
      border-color: rgba(206, 147, 216, 0.54);
      background: rgba(206, 147, 216, 0.09);
      font-size: 12px;
    }

    .database-tree-lines {
      width: 210px;
      height: 62px;
    }

    .database-tree-lines path {
      fill: none;
      stroke: rgba(205, 214, 224, 0.52);
      stroke-width: 2;
      stroke-linecap: square;
    }

    .database-parent-row {
      display: flex;
      justify-content: space-between;
      width: 228px;
    }

    .database-preview-card .database-tree-preview {
      grid-template-rows: 72px 50px 62px;
      align-content: center;
    }

    .database-preview-card .database-node {
      width: 68px;
      height: 68px;
    }

    .database-preview-card .database-node-parent {
      width: 56px;
      height: 56px;
    }

    .database-preview-card .database-tree-lines {
      width: 178px;
      height: 50px;
    }

    .database-preview-card .database-parent-row {
      width: 194px;
    }

    .database-preview-result {
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      gap: 10px;
      min-width: 0;
      min-height: 0;
      overflow: hidden;
    }

    .database-preview-result-body {
      display: grid;
      grid-template-rows: 214px minmax(0, 1fr);
      align-items: start;
      gap: 12px;
      min-width: 0;
      min-height: 0;
      overflow: hidden;
    }

    .database-preview-visual-row {
      display: grid;
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: 144px 62px;
      align-items: stretch;
      gap: 8px;
      min-width: 0;
      min-height: 0;
      overflow: visible;
    }

    .database-preview-affinity {
      display: grid;
      grid-template-columns: repeat(3, 43px) minmax(0, 1fr) 96px;
      align-items: center;
      gap: 2px;
      min-width: 0;
      min-height: 36px;
      padding: 4px 9px;
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 6px;
      background: rgba(0, 0, 0, 0.14);
    }

    .database-preview-stat {
      display: grid;
      justify-items: start;
      align-content: center;
      gap: 2px;
      min-width: 0;
      padding: 0;
      line-height: 1;
    }

    .database-preview-stat b {
      color: var(--text-primary);
      font-size: 16px;
      font-weight: 900;
      font-variant-numeric: tabular-nums;
    }

    .database-preview-stat small {
      min-width: 0;
      overflow: hidden;
      color: var(--text-muted);
      font-size: 6.7px;
      font-weight: 900;
      line-height: 1;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }

    .preview-stat-affinity b {
      color: #ff78b2;
    }

    .preview-stat-wins b {
      color: #81c784;
    }

    .preview-stat-white b {
      color: #ffb74d;
    }

    .database-preview-rank {
      grid-column: 5;
      display: grid;
      grid-template-columns: 28px minmax(0, 1fr);
      align-items: center;
      column-gap: 7px;
      min-width: 0;
      height: 31px;
      padding: 0 0 0 8px;
      border-left: 1px solid rgba(255, 255, 255, 0.08);
      line-height: 1;
    }

    .database-preview-rank img {
      display: block;
      width: 28px;
      height: 28px;
      object-fit: contain;
      filter: drop-shadow(0 3px 5px rgba(0, 0, 0, 0.32));
      color: transparent;
      font-size: 0;
    }

    .database-preview-score-copy {
      display: grid;
      justify-items: start;
      align-content: center;
      gap: 2px;
      min-width: 0;
      line-height: 1;
    }

    .database-preview-rank b {
      min-width: 0;
      overflow: visible;
      color: #90caf9;
      font-size: 12px;
      font-weight: 900;
      line-height: 1;
      text-overflow: clip;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }

    .database-preview-rank small {
      min-width: 0;
      overflow: hidden;
      color: var(--text-muted);
      font-size: 6.7px;
      font-weight: 900;
      line-height: 1;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }

    .database-lineage-preview {
      position: relative;
      display: block;
      height: 144px;
      min-width: 0;
      min-height: 0;
      padding: 0;
      overflow: visible;
    }

    .database-preview-portrait {
      position: relative;
      display: grid;
      justify-items: center;
      align-content: start;
      gap: 2px;
      min-width: 0;
      overflow: visible;
    }

    .preview-main {
      position: absolute;
      top: 2px;
      left: 50%;
      z-index: 2;
      transform: translateX(-50%);
    }

    .database-preview-avatar {
      position: relative;
      display: grid;
      place-items: center;
      width: 48px;
      height: 48px;
      overflow: hidden;
      border: 2px solid rgba(100, 181, 246, 0.58);
      border-radius: 50%;
      background: rgba(100, 181, 246, 0.08);
      box-shadow: 0 5px 13px rgba(0, 0, 0, 0.34);
    }

    .database-preview-avatar .portrait-fallback {
      position: absolute;
      inset: 0;
      display: grid;
      place-items: center;
      background:
        radial-gradient(circle at 40% 28%, rgba(255, 255, 255, 0.14), transparent 18px),
        rgba(100, 181, 246, 0.08);
      color: #90caf9;
      font-size: 12px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
    }

    .database-preview-avatar img {
      position: relative;
      z-index: 1;
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      object-position: top;
      color: transparent;
      font-size: 0;
    }

    .preview-role {
      color: #90caf9;
      font-size: 8px;
      font-weight: 900;
      line-height: 1.15;
      margin-top: 2px;
      text-transform: uppercase;
    }

    .preview-left .database-preview-avatar {
      border-color: rgba(255, 120, 178, 0.56);
    }

    .preview-right .database-preview-avatar {
      border-color: rgba(206, 147, 216, 0.56);
    }

    .preview-left .preview-role,
    .preview-right .preview-role {
      color: var(--text-muted);
    }

    .preview-left .database-preview-avatar,
    .preview-right .database-preview-avatar {
      width: 44px;
      height: 44px;
    }

    .database-preview-lines {
      position: absolute;
      top: 64px;
      left: 50%;
      width: 164px;
      height: 30px;
      transform: translateX(-50%);
    }

    .database-preview-lines path {
      fill: none;
      stroke: rgba(205, 214, 224, 0.42);
      stroke-width: 1.8;
      stroke-linecap: square;
    }

    .database-preview-parents {
      position: absolute;
      top: 92px;
      left: 50%;
      display: flex;
      justify-content: space-between;
      width: 164px;
      min-width: 0;
      transform: translateX(-50%);
    }

    .database-factor-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
    }

    .database-factor {
      display: grid;
      grid-template-columns: 20px 20px minmax(0, 1fr);
      align-items: center;
      gap: 6px;
      min-width: 0;
      padding: 8px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 6px;
      background: rgba(0, 0, 0, 0.16);
      color: var(--text-secondary);
      font-size: 12px;
      font-weight: 760;
      line-height: 1;
    }

    .database-factor b,
    .database-factor span,
    .database-factor strong {
      display: inline-flex;
      align-items: center;
      min-width: 0;
      line-height: 1;
      white-space: nowrap;
    }

    .database-factor b {
      justify-content: flex-end;
      color: var(--text-primary);
      font-weight: 900;
      font-variant-numeric: tabular-nums;
    }

    .database-factor span {
      justify-content: center;
      height: 18px;
      border-radius: 4px;
      font-size: 10px;
      font-weight: 900;
    }

    .database-factor strong {
      overflow: hidden;
      text-overflow: ellipsis;
      font-weight: 800;
    }

    .database-factor-b span {
      background: rgba(33, 150, 243, 0.18);
      color: #64b5f6;
    }

    .database-factor-p span {
      background: rgba(233, 30, 99, 0.18);
      color: #f06292;
    }

    .database-factor-g span {
      background: rgba(76, 175, 80, 0.18);
      color: #81c784;
    }

    .database-factor-w span {
      background: rgba(189, 189, 189, 0.16);
      color: #d0d0d0;
    }

    .database-result-panel .database-factor-grid {
      grid-template-columns: minmax(0, 1fr);
    }

    .database-support-preview {
      display: grid;
      grid-template-columns: 48px minmax(0, 1fr) auto;
      align-items: center;
      gap: 10px;
      min-width: 0;
      min-height: 58px;
      padding: 7px 9px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 6px;
      background: rgba(0, 0, 0, 0.16);
    }

    .database-preview-visual-row .database-support-preview {
      grid-template-columns: minmax(0, 1fr) 48px 102px minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr);
      align-items: center;
      justify-items: stretch;
      gap: 12px;
      min-height: 62px;
      padding: 9px 10px 10px;
    }

    .database-support-preview div {
      display: grid;
      gap: 5px;
      min-width: 0;
    }

    .database-support-preview div span,
    .database-support-preview b {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .database-support-preview div span {
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
    }

    .database-support-preview b {
      color: var(--text-primary);
      font-size: 12px;
      font-weight: 850;
      line-height: 1;
    }

    .database-support-thumb {
      position: relative;
      display: grid;
      place-items: center;
      width: 48px;
      height: 48px;
      overflow: hidden;
      border-radius: 7px;
      background: rgba(100, 181, 246, 0.08);
      color: #90caf9;
      font-size: 10px;
      font-weight: 900;
      line-height: 1;
      box-shadow: 0 3px 8px rgba(0, 0, 0, 0.35);
    }

    .database-preview-visual-row .database-support-thumb {
      grid-column: 2;
      width: 46px;
      height: 46px;
      justify-self: center;
    }

    .database-support-thumb span {
      position: absolute;
      inset: 0;
      display: grid;
      place-items: center;
      background:
        radial-gradient(circle at 42% 28%, rgba(255, 255, 255, 0.13), transparent 16px),
        rgba(100, 181, 246, 0.08);
    }

    .database-support-thumb img {
      position: relative;
      z-index: 1;
      display: block;
      width: 100%;
      height: 100%;
      object-fit: cover;
      color: transparent;
      font-size: 0;
    }

    .database-support-preview em {
      display: flex;
      align-items: center;
      gap: 4px;
      flex: 0 0 auto;
      font-style: normal;
    }

    .database-preview-visual-row .database-support-preview em {
      grid-column: 3;
      grid-row: 1;
      gap: 9px;
      justify-content: center;
    }

    .database-support-preview i {
      width: 10px;
      height: 17px;
      background: #2196f3;
      clip-path: polygon(50% 0, 100% 50%, 50% 100%, 0 50%);
    }

    .database-preview-visual-row .database-support-preview i {
      width: 13px;
      height: 22px;
    }

    .database-preview-sparks {
      display: grid;
      align-content: start;
      gap: 5px;
      min-width: 0;
      min-height: 0;
      overflow: hidden;
    }

    .preview-spark-row {
      display: grid;
      grid-template-columns: 3px minmax(0, 1fr);
      align-items: stretch;
      gap: 5px;
      min-width: 0;
    }

    .preview-spark-indicator {
      width: 3px;
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.28);
    }

    .preview-spark-indicator.blue { background: #2196f3; }
    .preview-spark-indicator.pink { background: #e91e63; }
    .preview-spark-indicator.green { background: #4caf50; }
    .preview-spark-indicator.white { background: #bdbdbd; }

    .preview-spark-list {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 4px;
      min-width: 0;
      overflow: hidden;
    }

    .preview-spark {
      display: inline-flex;
      align-items: center;
      gap: 3px;
      max-width: 100%;
      min-width: 0;
      height: 20px;
      padding: 0 6px;
      border: 1px solid rgba(255, 255, 255, 0.09);
      border-radius: 5px;
      background: rgba(255, 255, 255, 0.05);
      color: var(--text-secondary);
      font-size: 9px;
      font-weight: 820;
      line-height: 1;
    }

    .preview-spark b,
    .preview-spark strong {
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .preview-spark b {
      color: var(--text-primary);
      font-weight: 900;
      font-variant-numeric: tabular-nums;
    }

    .preview-spark strong {
      font-weight: 820;
    }

    .preview-spark .spark-star {
      width: 11px;
      height: 11px;
      flex: 0 0 auto;
      fill: currentColor;
    }

    .preview-spark-blue {
      border-color: rgba(33, 150, 243, 0.44);
      background: rgba(33, 150, 243, 0.12);
      color: #64b5f6;
    }

    .preview-spark-pink {
      border-color: rgba(233, 30, 99, 0.44);
      background: rgba(233, 30, 99, 0.12);
      color: #f06292;
    }

    .preview-spark-green {
      border-color: rgba(76, 175, 80, 0.42);
      background: rgba(76, 175, 80, 0.11);
      color: #81c784;
    }

    .preview-spark-white {
      border-color: rgba(189, 189, 189, 0.28);
      background: rgba(255, 255, 255, 0.055);
      color: #d0d0d0;
    }

    .database-result-content {
      padding: 12px 18px 14px;
    }

    .database-result-card {
      display: grid;
      grid-template-rows: auto minmax(0, 1fr) 24px;
      width: 100%;
      height: 100%;
      padding: 13px 18px 10px;
      background: rgba(255, 255, 255, 0.04);
    }

    .database-result-head {
      min-width: 0;
      min-height: 0;
    }

    .record-header-stats {
      display: flex;
      align-items: center;
      flex-wrap: wrap;
      gap: 12px 28px;
      width: 100%;
      min-height: 54px;
      padding: 6px 12px;
      border: 1px solid rgba(255, 255, 255, 0.04);
      border-radius: 8px;
      background: rgba(0, 0, 0, 0.18);
    }

    .main-stats {
      display: flex;
      align-items: center;
      gap: 24px;
      min-width: 0;
      flex-wrap: wrap;
    }

    .stat-pill {
      display: inline-flex;
      flex-direction: column;
      align-items: flex-start;
      justify-content: center;
      gap: 4px;
      min-width: 0;
      line-height: 1;
    }

    .stat-number {
      max-width: 160px;
      overflow: hidden;
      color: var(--accent-primary);
      font-size: 23px;
      font-weight: 850;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-variant-numeric: tabular-nums;
    }

    .stat-label {
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 750;
      line-height: 1;
      text-transform: uppercase;
    }

    .affinity-stat .stat-number { color: var(--accent-pink); }
    .wins-stat .stat-number { color: var(--accent-secondary); }
    .white-stat .stat-number { color: var(--accent-warning); }
    .score-stat .stat-number { color: var(--accent-primary); }

    .rank-score-section {
      display: flex;
      align-items: center;
      gap: 14px;
      margin-left: auto;
      padding-left: 18px;
      border-left: 1px solid rgba(255, 255, 255, 0.07);
    }

    .rank-image-wrap {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 50px;
      height: 50px;
      flex: 0 0 auto;
    }

    .rank-image {
      display: block;
      max-width: 50px;
      max-height: 50px;
      object-fit: contain;
      filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.35));
      color: transparent;
      font-size: 0;
    }

    .inheritance-body {
      display: grid;
      grid-template-columns: 246px minmax(0, 1fr);
      gap: 14px;
      align-items: stretch;
      min-height: 0;
      margin-top: 9px;
      padding-top: 9px;
      border-top: 1px solid var(--border-primary);
      overflow: hidden;
    }

    .character-images {
      display: flex;
      flex-direction: column;
      align-items: stretch;
      justify-content: flex-start;
      gap: 13px;
      min-width: 0;
      min-height: 0;
      padding: 5px 10px;
      overflow: hidden;
    }

    .lineage-frame {
      display: flex;
      flex-direction: column;
      align-items: center;
      width: 100%;
      min-height: 188px;
      justify-content: flex-start;
    }

    .main-character,
    .parent-with-badges {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 4px;
      min-width: 0;
    }

    .parent-characters {
      display: flex;
      justify-content: space-between;
      gap: 0;
      width: 100%;
      padding: 0 16px;
    }

    .lineage-bracket {
      width: 168px;
      height: 21px;
      margin: 1px 0 2px;
    }

    .lineage-bracket path {
      fill: none;
      stroke: rgba(205, 214, 224, 0.52);
      stroke-width: 2;
      stroke-linecap: square;
    }

    .portrait-wrapper {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
      border: 2px solid rgba(255, 255, 255, 0.22);
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.045);
      box-shadow:
        inset 0 0 0 1px rgba(0, 0, 0, 0.3),
        0 6px 14px rgba(0, 0, 0, 0.28);
    }

    .portrait-main {
      width: 82px;
      height: 82px;
      padding: 2px;
      border-color: rgba(74, 168, 255, 0.58);
    }

    .portrait-gp {
      width: 62px;
      height: 62px;
      padding: 2px;
      border-color: rgba(206, 147, 216, 0.5);
    }

    .portrait-left {
      border-color: rgba(255, 120, 178, 0.55);
    }

    .portrait-right {
      border-color: rgba(206, 147, 216, 0.55);
    }

    .character-image {
      display: block;
      width: 100%;
      height: 100%;
      border-radius: 50%;
      object-fit: cover;
      object-position: top;
      color: transparent;
      font-size: 0;
    }

    .portrait-label {
      color: var(--text-primary);
      font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      font-size: 13px;
      font-weight: 800;
      line-height: 1;
    }

    .parent-affinity-badges {
      display: flex;
      justify-content: center;
      min-height: 17px;
    }

    .affinity-badge {
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
    }

    .affinity-badge.gp,
    .affinity-badge.gp-right {
      border-color: rgba(206, 147, 216, 0.25);
      background: rgba(206, 147, 216, 0.12);
      color: #ce93d8;
    }

    .affinity-badge.gp-left {
      border-color: rgba(233, 30, 99, 0.28);
      background: rgba(233, 30, 99, 0.12);
      color: #ff78b2;
    }

    .heart-icon {
      color: currentColor;
      font-size: 10px;
      line-height: 1;
    }

    .node-role-label {
      display: block;
      color: var(--text-disabled);
      font-size: 9px;
      font-weight: 800;
      line-height: 1;
      text-transform: uppercase;
    }

    .node-role-main {
      color: var(--accent-primary);
    }

    .support-card-section {
      position: relative;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 12px;
      min-height: 76px;
      padding: 9px 12px;
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.025);
    }

    .support-card-section.matched-filter {
      border-color: rgba(255, 214, 102, 0.72);
      background:
        linear-gradient(135deg, rgba(255, 214, 102, 0.16), rgba(33, 150, 243, 0.05)),
        rgba(255, 255, 255, 0.035);
      box-shadow: 0 0 0 1px rgba(255, 214, 102, 0.16), 0 8px 22px rgba(255, 214, 102, 0.08);
    }

    .support-filter-badge {
      position: absolute;
      top: 6px;
      right: 7px;
      color: #ffd666;
      font-size: 8px;
      font-weight: 900;
      line-height: 1;
      text-transform: uppercase;
    }

    .support-card-image {
      display: block;
      width: 64px;
      height: 64px;
      flex: 0 0 auto;
      border-radius: 7px;
      object-fit: cover;
      color: transparent;
      font-size: 0;
      box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
    }

    .card-limit-break {
      display: flex;
      flex-direction: row;
      gap: 3px;
      align-items: center;
      justify-content: center;
    }

    .limit-break-icon {
      display: block;
      width: 22px;
      height: 22px;
      flex: 0 0 auto;
    }

    .limit-break-icon path {
      fill: rgba(100, 181, 246, 0.58);
    }

    .limit-break-icon.filled path {
      fill: #2196f3;
    }

    .limit-break-icon.filled.matched-filter path {
      fill: #ffd666;
      filter: drop-shadow(0 0 5px rgba(255, 214, 102, 0.52));
    }

    .spark-arrays {
      display: flex;
      flex-direction: column;
      justify-content: flex-start;
      gap: 8px;
      min-width: 0;
      overflow: hidden;
      padding: 2px 0 0;
    }

    .spark-container {
      display: flex;
      flex-direction: column;
      gap: 8px;
      min-width: 0;
      overflow: hidden;
    }

    .spark-row {
      display: flex;
      align-items: stretch;
      gap: 10px;
      min-width: 0;
    }

    .spark-type-indicator {
      width: 4px;
      min-height: 100%;
      border-radius: 999px;
      flex-shrink: 0;
      opacity: 0.95;
    }

    .spark-type-indicator.blue { background: #2196f3; }
    .spark-type-indicator.pink { background: #e91e63; }
    .spark-type-indicator.green { background: #4caf50; }
    .spark-type-indicator.white { background: #bdbdbd; }

    .spark-list {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 6px;
      min-width: 0;
    }

    .spark-item {
      display: inline-flex;
      align-items: center;
      gap: 4px;
      max-width: 310px;
      padding: 4px 8px;
      border: 1px solid;
      border-radius: 8px;
      color: var(--text-secondary);
      font-size: 13px;
      font-weight: 500;
      line-height: 1;
      white-space: nowrap;
    }

    .spark-level {
      color: currentColor;
      font-weight: 600;
      font-variant-numeric: tabular-nums;
    }

    .spark-star {
      display: block;
      width: 13px;
      height: 13px;
      flex: 0 0 13px;
      color: currentColor;
    }

    .spark-star path {
      fill: currentColor;
    }

    .spark-name {
      min-width: 0;
      max-width: 185px;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .spark-pct {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      align-self: center;
      max-width: 72px;
      overflow: hidden;
      padding: 2px 5px;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 4px;
      background: rgba(0, 0, 0, 0.22);
      color: rgba(255, 255, 255, 0.78);
      font-size: 0.85em;
      font-weight: 700;
      line-height: 1;
      text-overflow: ellipsis;
      font-variant-numeric: tabular-nums;
    }

    .blue-spark {
      border-color: rgba(33, 150, 243, 0.48);
      background: rgba(33, 150, 243, 0.11);
      color: var(--accent-primary);
    }

    .pink-spark {
      border-color: rgba(233, 30, 99, 0.48);
      background: rgba(233, 30, 99, 0.11);
      color: #f06292;
    }

    .green-spark {
      border-color: rgba(76, 175, 80, 0.48);
      background: rgba(76, 175, 80, 0.11);
      color: var(--accent-secondary);
    }

    .white-spark {
      border-color: rgba(158, 158, 158, 0.46);
      background: rgba(158, 158, 158, 0.1);
      color: #cfcfcf;
    }

    .spark-item.matched-filter {
      border-color: #ffd666 !important;
      background: linear-gradient(45deg, rgba(255, 214, 102, 0.1), rgba(255, 214, 102, 0.035));
      box-shadow: inset 0 0 0 1px rgba(255, 214, 102, 0.28);
    }

    .parent-source {
      display: inline-flex;
      align-items: center;
      gap: 1px;
      margin-left: 0;
      color: #ff9100;
      line-height: 1;
    }

    .parent-icon {
      display: block;
      width: 14px;
      height: 14px;
      color: currentColor;
      flex: 0 0 auto;
      opacity: 0.9;
    }

    .parent-icon path {
      fill: currentColor;
    }

    .parent-contribution {
      display: inline-flex;
      align-items: center;
      gap: 1px;
      color: currentColor;
      font-size: 12px;
      font-weight: 700;
      line-height: 1;
      opacity: 0.9;
    }

    .parent-contribution::before {
      content: "(";
      opacity: 0.85;
    }

    .parent-contribution::after {
      content: ")";
      opacity: 0.85;
    }

    .parent-contribution .spark-star {
      width: 13px;
      height: 13px;
      flex-basis: 13px;
    }

    .database-footer {
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 8px;
      min-width: 0;
      overflow: hidden;
      padding-top: 5px;
      border-top: 1px solid rgba(255, 255, 255, 0.06);
      color: var(--text-muted);
      font-size: 11px;
      font-weight: 800;
      line-height: 1;
      white-space: nowrap;
    }

    .verified-meta {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      color: var(--text-muted);
      white-space: nowrap;
    }

    .verified-meta::before {
      content: "";
      width: 7px;
      height: 7px;
      border-radius: 50%;
      background: var(--accent-secondary);
      box-shadow: 0 0 9px rgba(129, 199, 132, 0.48);
    }

    .verified-text {
      color: var(--accent-secondary);
      font-size: 11px;
      font-weight: 800;
      text-transform: uppercase;
    }

    .footer-dot {
      color: var(--text-disabled);
    }
"#
}
