use crate::embed::{DatabaseEmbedDetails, ResourceCatalog};

use super::{asset_url, format_decimal, format_number_grouped, html_escape, truncate_chars};

#[derive(Clone, Copy, Debug)]
enum OverflowLabel {
    Count,
    More,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct InheritanceRenderOptions {
    show_spark_chances: bool,
    color_limit: usize,
    white_limit: usize,
    overflow_label: OverflowLabel,
    spark_name_limit: Option<usize>,
    show_parent_icon: bool,
    compact_lineage: bool,
}

impl InheritanceRenderOptions {
    pub(super) const fn database() -> Self {
        Self {
            show_spark_chances: true,
            color_limit: 8,
            white_limit: 30,
            overflow_label: OverflowLabel::Count,
            spark_name_limit: Some(28),
            show_parent_icon: true,
            compact_lineage: false,
        }
    }

    pub(super) const fn profile() -> Self {
        Self {
            show_spark_chances: false,
            color_limit: 12,
            white_limit: 12,
            overflow_label: OverflowLabel::More,
            spark_name_limit: None,
            show_parent_icon: false,
            compact_lineage: true,
        }
    }
}

pub(super) fn render_body(
    database: &DatabaseEmbedDetails,
    options: InheritanceRenderOptions,
) -> String {
    let main_affinity = database.affinity_score.or_else(|| {
        positive_i64(
            shared_count(&database.main_win_saddles, &database.left_win_saddles)
                + shared_count(&database.main_win_saddles, &database.right_win_saddles),
        )
    });
    let main = render_character_node(
        database,
        database.main_parent_id,
        "main-character",
        "portrait-main",
        "main-character-img",
        "MAIN",
        "node-role-main",
        main_affinity,
        "base",
    );
    let left_affinity = database.left_affinity_score.or_else(|| {
        positive_i64(shared_count(
            &database.main_win_saddles,
            &database.left_win_saddles,
        ))
    });
    let left = render_character_node(
        database,
        database.parent_left_id,
        "parent-with-badges",
        "portrait-gp portrait-left",
        "parent-character-img",
        "GP",
        "node-role-gp",
        left_affinity,
        "base gp gp-left",
    );
    let right_affinity = database.right_affinity_score.or_else(|| {
        positive_i64(shared_count(
            &database.main_win_saddles,
            &database.right_win_saddles,
        ))
    });
    let right = render_character_node(
        database,
        database.parent_right_id,
        "parent-with-badges",
        "portrait-gp portrait-right",
        "parent-character-img",
        "GP",
        "node-role-gp",
        right_affinity,
        "base gp gp-right",
    );
    let support_card = render_support_card(database);
    let lineage_bracket = render_lineage_bracket(options);
    let chip_rows = render_spark_rows(database, options);

    format!(
        r#"<section class="inheritance-body">
        <div class="character-images">
          <div class="lineage-frame">
            {main}
            {lineage_bracket}
            <div class="parent-characters">{left}{right}</div>
          </div>
          {support_card}
        </div>
        <div class="spark-arrays"><div class="spark-container">{chip_rows}</div></div>
      </section>"#,
        main = main,
        lineage_bracket = lineage_bracket,
        left = left,
        right = right,
        support_card = support_card,
        chip_rows = chip_rows,
    )
}

fn render_lineage_bracket(options: InheritanceRenderOptions) -> &'static str {
    if options.compact_lineage {
        r#"<svg class="lineage-bracket lineage-bracket-compact" viewBox="0 0 100 24" preserveAspectRatio="none" aria-hidden="true">
              <path d="M16 8 L84 8" vector-effect="non-scaling-stroke" />
              <path d="M16 8 L16 24" vector-effect="non-scaling-stroke" />
              <path d="M50 8 L50 24" vector-effect="non-scaling-stroke" />
              <path d="M84 8 L84 24" vector-effect="non-scaling-stroke" />
            </svg>"#
    } else {
        r#"<svg class="lineage-bracket" viewBox="0 0 100 24" preserveAspectRatio="none" aria-hidden="true">
              <path d="M50 0 L50 9" vector-effect="non-scaling-stroke" />
              <path d="M15 9 L85 9" vector-effect="non-scaling-stroke" />
              <path d="M15 9 L15 24" vector-effect="non-scaling-stroke" />
              <path d="M85 9 L85 24" vector-effect="non-scaling-stroke" />
            </svg>"#
    }
}

fn render_character_node(
    database: &DatabaseEmbedDetails,
    character_id: Option<i64>,
    node_class: &str,
    portrait_class: &str,
    image_class: &str,
    role: &str,
    role_class: &str,
    affinity: Option<i64>,
    badge_class: &str,
) -> String {
    let portrait = match character_id {
        Some(character_id) => format!(
            r#"<img src="{}" alt="" class="character-image {}" onerror="this.style.visibility='hidden'">"#,
            html_escape(&character_image_url(database, character_id)),
            image_class,
        ),
        None => r#"<span class="portrait-label">?</span>"#.to_string(),
    };
    let affinity_badge = affinity.map_or_else(String::new, |affinity| {
        format!(
            r#"<span class="affinity-badge {badge_class}"><span class="heart-icon">&#9829;</span>{}</span>"#,
            html_escape(&format_number_grouped(affinity, ',')),
        )
    });

    format!(
        r#"<div class="{node_class}">
          <div class="portrait-wrapper {portrait_class}">{portrait}</div>
          <div class="parent-affinity-badges">{affinity_badge}</div>
          <span class="node-role-label {role_class}">{role}</span>
        </div>"#,
        node_class = node_class,
        portrait_class = portrait_class,
        portrait = portrait,
        affinity_badge = affinity_badge,
        role_class = role_class,
        role = role,
    )
}

fn render_support_card(database: &DatabaseEmbedDetails) -> String {
    let Some(support_card_id) = database.support_card_id else {
        return String::new();
    };
    let support_matches = database.matched_support_card_id == Some(support_card_id);
    let mut class_names = vec!["support-card-section"];
    if support_matches {
        class_names.push("matched-filter");
    }
    let match_badge = if support_matches {
        r#"<span class="support-filter-badge">Filter</span>"#
    } else {
        ""
    };

    format!(
        r#"<div class="{class_names}">
          <img src="{image_url}" alt="" class="support-card-image" onerror="this.style.visibility='hidden'">
          {limit_breaks}
          {match_badge}
        </div>"#,
        class_names = class_names.join(" "),
        image_url = html_escape(&support_card_image_url(database, support_card_id)),
        limit_breaks = render_limit_breaks(
            database.limit_break_count.unwrap_or_default(),
            database.matched_min_limit_break.filter(|_| support_matches),
        ),
        match_badge = match_badge,
    )
}

fn render_limit_breaks(count: i64, matched_min_limit_break: Option<i64>) -> String {
    let count = count.clamp(0, 4);
    let matched_count = matched_min_limit_break.unwrap_or_default().clamp(0, 4);
    let icons = (0..4)
        .map(|index| {
            if index < count {
                let matched_class = if index < matched_count {
                    " matched-filter"
                } else {
                    ""
                };
                format!(r#"<svg class="limit-break-icon filled{matched_class}" viewBox="0 -960 960 960" aria-hidden="true"><path d="M480-64 224-480l256-416 256 416L480-64Z"/></svg>"#)
            } else {
                r#"<svg class="limit-break-icon" viewBox="0 -960 960 960" aria-hidden="true"><path d="M480-64 224-480l256-416 256 416L480-64Zm0-139 170-277-170-278-169 278 169 277Zm0-277Z"/></svg>"#.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("");

    format!(r#"<div class="card-limit-break">{icons}</div>"#)
}

fn render_spark_rows(database: &DatabaseEmbedDetails, options: InheritanceRenderOptions) -> String {
    let blue = color_factor_ids(
        [
            database.main_blue_factors,
            database.left_blue_factors,
            database.right_blue_factors,
        ],
        &database.blue_sparks,
    );
    let pink = color_factor_ids(
        [
            database.main_pink_factors,
            database.left_pink_factors,
            database.right_pink_factors,
        ],
        &database.pink_sparks,
    );
    let green = color_factor_ids(
        [
            database.main_green_factors,
            database.left_green_factors,
            database.right_green_factors,
        ],
        &database.green_sparks,
    );
    let white = white_factor_ids(database);

    let rows = [
        render_spark_row(
            database,
            options,
            "blue",
            "blue-spark",
            combine_spark_ids(&blue, &database.resources),
            database.affinity_score.unwrap_or_default(),
            options.color_limit,
        ),
        render_spark_row(
            database,
            options,
            "pink",
            "pink-spark",
            combine_spark_ids(&pink, &database.resources),
            database.affinity_score.unwrap_or_default(),
            options.color_limit,
        ),
        render_spark_row(
            database,
            options,
            "green",
            "green-spark",
            combine_spark_ids(&green, &database.resources),
            database.affinity_score.unwrap_or_default(),
            options.color_limit,
        ),
        render_spark_row(
            database,
            options,
            "white",
            "white-spark",
            combine_spark_ids(&white, &database.resources),
            database.affinity_score.unwrap_or_default(),
            options.white_limit,
        ),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if rows.is_empty() {
        return r#"<div class="spark-row"><div class="spark-type-indicator blue"></div><div class="spark-list"><span class="spark-item blue-spark"><span class="spark-name">No sparks in preview</span></span></div></div>"#.to_string();
    }

    rows.join("")
}

fn render_spark_row(
    database: &DatabaseEmbedDetails,
    options: InheritanceRenderOptions,
    indicator: &str,
    chip_class: &str,
    mut sparks: Vec<SparkDisplay>,
    affinity: i64,
    limit: usize,
) -> Option<String> {
    if sparks.is_empty() {
        return None;
    }

    sparks.sort_by(|left, right| {
        right
            .total_level
            .cmp(&left.total_level)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.factor_id.cmp(&right.factor_id))
    });

    let overflow = sparks.len().saturating_sub(limit);
    let mut chips = sparks
        .into_iter()
        .take(limit)
        .map(|spark| render_spark_chip(database, options, chip_class, &spark, affinity))
        .collect::<Vec<_>>();

    if overflow > 0 {
        let label = match options.overflow_label {
            OverflowLabel::Count => format!("+{overflow} more"),
            OverflowLabel::More => "... more".to_string(),
        };
        chips.push(format!(
            r#"<span class="spark-item {chip_class} overflow-spark"><span class="spark-name">{label}</span></span>"#,
            chip_class = chip_class,
            label = html_escape(&label),
        ));
    }

    Some(format!(
        r#"<div class="spark-row"><div class="spark-type-indicator {indicator}"></div><div class="spark-list">{}</div></div>"#,
        chips.join("")
    ))
}

fn render_spark_chip(
    database: &DatabaseEmbedDetails,
    options: InheritanceRenderOptions,
    chip_class: &str,
    spark: &SparkDisplay,
    affinity: i64,
) -> String {
    let matched = database.matched_factor_ids.contains(&spark.factor_id);
    let query_main_matched = database.matched_main_factor_ids.contains(&spark.factor_id);
    let main_level = main_parent_level(database, spark.factor_id);
    let source_marker = main_level.map_or_else(String::new, |level| {
        render_main_parent_marker(level, options.show_parent_icon)
    });
    let chance = if options.show_spark_chances {
        format!(
            r#"<span class="spark-pct">{}</span>"#,
            html_escape(&spark_display_value(spark, affinity))
        )
    } else {
        String::new()
    };
    let mut classes = vec!["spark-item", chip_class];
    if matched {
        classes.push("matched-filter");
    }
    if main_level.is_some() || query_main_matched {
        classes.push("from-main-parent");
    }

    format!(
        r#"<span class="{}"><span class="spark-level">{}</span>{}<span class="spark-name">{}</span>{}{}</span>"#,
        classes.join(" "),
        spark.total_level,
        render_spark_star(),
        html_escape(&spark_name(spark, options)),
        chance,
        source_marker,
    )
}

fn spark_name(spark: &SparkDisplay, options: InheritanceRenderOptions) -> String {
    options.spark_name_limit.map_or_else(
        || spark.name.clone(),
        |limit| truncate_chars(&spark.name, limit),
    )
}

fn main_parent_level(database: &DatabaseEmbedDetails, factor_id: i64) -> Option<i64> {
    let mut level = 0;

    for id in [
        database.main_blue_factors,
        database.main_pink_factors,
        database.main_green_factors,
    ]
    .into_iter()
    .flatten()
    {
        if factor_id_from_spark(id) == factor_id {
            level += spark_level(id);
        }
    }

    for id in &database.main_white_factors {
        if factor_id_from_spark(*id) == factor_id {
            level += spark_level(*id);
        }
    }

    positive_i64(level)
}

fn render_main_parent_marker(level: i64, show_icon: bool) -> String {
    let icon = if show_icon {
        r#"<svg class="parent-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 12c2.2 0 4-1.8 4-4s-1.8-4-4-4-4 1.8-4 4 1.8 4 4 4Zm0 2c-2.7 0-8 1.3-8 4v2h16v-2c0-2.7-5.3-4-8-4Z"/></svg>"#
    } else {
        ""
    };
    format!(
        r#"<span class="parent-source">{icon}<span class="parent-contribution">{}{}</span></span>"#,
        level,
        render_spark_star()
    )
}

pub(super) fn render_spark_star() -> &'static str {
    r#"<svg class="spark-star" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 17.27 18.18 21l-1.64-7.03L22 9.24l-7.19-.61L12 2 9.19 8.63 2 9.24l5.46 4.73L5.82 21 12 17.27Z"/></svg>"#
}

fn color_factor_ids(parent_ids: [Option<i64>; 3], fallback: &[i64]) -> Vec<i64> {
    let ids = parent_ids.into_iter().flatten().collect::<Vec<_>>();
    if ids.is_empty() {
        fallback.to_vec()
    } else {
        ids
    }
}

fn white_factor_ids(database: &DatabaseEmbedDetails) -> Vec<i64> {
    let mut ids = Vec::new();
    ids.extend(database.main_white_factors.iter().copied());
    ids.extend(database.left_white_factors.iter().copied());
    ids.extend(database.right_white_factors.iter().copied());

    if ids.is_empty() {
        database.white_sparks.clone()
    } else {
        ids
    }
}

fn shared_count(primary: &[i64], secondary: &[i64]) -> i64 {
    primary
        .iter()
        .filter(|race_id| secondary.contains(race_id))
        .count() as i64
}

fn positive_i64(value: i64) -> Option<i64> {
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

#[derive(Clone, Debug)]
struct SparkDisplay {
    factor_id: i64,
    name: String,
    type_id: i64,
    total_level: i64,
    source_levels: Vec<i64>,
}

fn combine_spark_ids(ids: &[i64], resources: &ResourceCatalog) -> Vec<SparkDisplay> {
    let mut sparks: Vec<SparkDisplay> = Vec::new();

    for id in ids.iter().copied().filter(|id| *id > 0) {
        let factor_id = factor_id_from_spark(id);
        let level = spark_level(id);
        let (name, type_id) = resources
            .factor_info(factor_id)
            .unwrap_or_else(|| factor_info(factor_id));

        if let Some(existing) = sparks.iter_mut().find(|spark| spark.factor_id == factor_id) {
            existing.total_level += level;
            existing.source_levels.push(level);
        } else {
            sparks.push(SparkDisplay {
                factor_id,
                name,
                type_id,
                total_level: level,
                source_levels: vec![level],
            });
        }
    }

    sparks
}

fn factor_id_from_spark(id: i64) -> i64 {
    id / 10
}

fn spark_level(id: i64) -> i64 {
    let level = id.rem_euclid(10);
    if level == 0 {
        1
    } else {
        level
    }
}

fn spark_display_value(spark: &SparkDisplay, affinity: i64) -> String {
    let mut expected = 0.0;
    let mut miss_all = 1.0;

    for level in &spark.source_levels {
        let chance = spark_source_chance(spark.type_id, *level, affinity) / 100.0;
        expected += chance;
        miss_all *= 1.0 - chance;
    }

    let proc_chance = (1.0 - miss_all) * 100.0;
    if matches!(spark.type_id, 2..=4) {
        format_decimal(proc_chance, "%")
    } else if expected >= 1.0 {
        format_decimal(expected, "x")
    } else {
        format_decimal(proc_chance, "%")
    }
}

fn spark_source_chance(type_id: i64, level: i64, affinity: i64) -> f64 {
    let level_index = level.clamp(0, 3) as usize;
    let base = match type_id {
        0 => [0.0, 70.0, 80.0, 90.0][level_index],
        1 => [0.0, 1.0, 3.0, 5.0][level_index],
        2 => [0.0, 1.0, 2.0, 3.0][level_index],
        3 | 4 => [0.0, 3.0, 6.0, 9.0][level_index],
        5 => [0.0, 5.0, 10.0, 15.0][level_index],
        _ => [0.0, 3.0, 6.0, 9.0][level_index],
    };

    (base * (1.0 + affinity.max(0) as f64 / 100.0)).min(100.0)
}

fn factor_info(factor_id: i64) -> (String, i64) {
    (format!("Factor {factor_id}"), infer_factor_type(factor_id))
}

fn infer_factor_type(factor_id: i64) -> i64 {
    if matches!(factor_id, 10 | 20 | 30 | 40 | 50) {
        0
    } else if (100..400).contains(&factor_id) {
        1
    } else if (100000..200000).contains(&factor_id) {
        2
    } else if (200000..300000).contains(&factor_id) {
        3
    } else if (300000..400000).contains(&factor_id) {
        4
    } else if factor_id >= 1_000_000 {
        5
    } else {
        3
    }
}

fn character_image_url(database: &DatabaseEmbedDetails, character_id: i64) -> String {
    asset_url(
        &database.asset_base_url,
        &format!("/images/character_stand/chara_stand_{character_id}.webp"),
    )
}

fn support_card_image_url(database: &DatabaseEmbedDetails, support_card_id: i64) -> String {
    asset_url(
        &database.asset_base_url,
        &format!("/images/support_card/half/support_card_s_{support_card_id}.webp"),
    )
}
