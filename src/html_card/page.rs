use crate::embed::EmbedMetadata;

use super::{html_escape, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-page",
    render_visual,
};

pub(super) fn render_visual(meta: &EmbedMetadata) -> String {
    let rows = meta
        .metrics
        .iter()
        .take(3)
        .map(|metric| {
            format!(
                r#"<div class="page-visual-row"><span>{}</span><strong>{}</strong></div>"#,
                html_escape(&truncate_chars(&metric.label, 16)),
                html_escape(&truncate_chars(&metric.value, 20)),
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(r#"<div class="visual-panel page-visual">{rows}</div>"#)
}
