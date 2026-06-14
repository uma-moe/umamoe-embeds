use crate::embed::{EmbedMetadata, EmbedMetric};

use super::{canonical_path, html_escape, truncate_chars};

#[derive(Clone, Copy)]
pub(super) struct CardView {
    pub(super) class_name: &'static str,
    pub(super) render_visual: fn(&EmbedMetadata) -> String,
}

const DATABASE_VIEW: CardView = CardView {
    class_name: super::database::CLASS_NAME,
    render_visual: super::database::render_visual,
};

pub(super) fn card_view_class(meta: &EmbedMetadata) -> &'static str {
    card_view(meta).class_name
}

pub(super) fn render_body(meta: &EmbedMetadata) -> String {
    render_overview_body(meta, card_view(meta))
}

fn card_view(meta: &EmbedMetadata) -> CardView {
    if meta.database.is_some() {
        return DATABASE_VIEW;
    }

    let path = canonical_path(&meta.canonical_url);
    let route_path = super::normalize_route_path(&path);

    match route_path {
        "/" => super::home::VIEW,
        path if path.starts_with("/profile/") => super::profile::VIEW,
        "/circles" => super::clubs::VIEW,
        path if path.starts_with("/circles/") => super::club::VIEW,
        "/database" | "/inheritance" | "/support-cards" => DATABASE_VIEW,
        "/rankings" => super::rankings::VIEW,
        "/activity" | "/shame" => super::activity::VIEW,
        path if path.starts_with("/activity/") || path.starts_with("/shame/") => {
            super::activity::VIEW
        }
        "/timeline" => super::timeline::VIEW,
        "/tierlist" => super::tierlist::VIEW,
        "/tools" => super::tools::VIEW,
        "/tools/statistics" => super::statistics::VIEW,
        "/tools/lineage-planner" => super::lineage_planner::VIEW,
        _ => super::page::VIEW,
    }
}

fn render_overview_body(meta: &EmbedMetadata, view: CardView) -> String {
    let summary_title = html_escape(&truncate_chars(&super::display_title(&meta.title), 42));
    let summary_text = html_escape(&truncate_chars(&meta.description, 150));
    let label = html_escape(&meta.kind_label);
    let metric_chips = render_overview_metric_chips(meta);
    let visual = (view.render_visual)(meta);

    format!(
        r#"<section class="overview-body {view_class}">
        <div class="overview-copy">
          <span class="overview-label">{label}</span>
          <strong class="overview-title">{summary_title}</strong>
          <p class="overview-text">{summary_text}</p>
          {metric_chips}
        </div>
        <div class="overview-visual">{visual}</div>
      </section>"#,
        view_class = view.class_name,
        label = label,
        summary_title = summary_title,
        summary_text = summary_text,
        metric_chips = metric_chips,
        visual = visual,
    )
}

fn render_overview_metric_chips(meta: &EmbedMetadata) -> String {
    let chips = meta
        .metrics
        .iter()
        .filter(|metric| should_show_overview_metric(metric))
        .take(4)
        .map(|metric| {
            format!(
                r#"<span class="overview-metric"><span class="overview-metric-value">{}</span><span class="overview-metric-label">{}</span></span>"#,
                html_escape(&truncate_chars(&metric.value, 18)),
                html_escape(&truncate_chars(&metric.label, 20)),
            )
        })
        .collect::<Vec<_>>();

    if chips.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="overview-metrics">{}</div>"#, chips.join(""))
    }
}

fn should_show_overview_metric(metric: &EmbedMetric) -> bool {
    !matches!(
        metric.label.to_ascii_lowercase().as_str(),
        "site" | "view" | "use" | "data" | "focus"
    )
}

pub(super) fn render_leaderboard_visual(title: &str, rows: &[(&str, &str, &str)]) -> String {
    let rows = rows
        .iter()
        .enumerate()
        .map(|(index, (rank, name, value))| {
            format!(
                r#"<div class="leader-row leader-top-{}"><span class="leader-rank">{}</span><span class="leader-name">{}</span><strong>{}</strong></div>"#,
                index + 1,
                html_escape(rank),
                html_escape(name),
                html_escape(value),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="visual-panel leaderboard-visual"><span class="visual-kicker">{}</span>{rows}</div>"#,
        html_escape(title)
    )
}
