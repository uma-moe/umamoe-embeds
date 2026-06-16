use std::collections::BTreeMap;

use base64::{
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
    Engine as _,
};
use serde::Deserialize;
use url::form_urlencoded;

use crate::embed::{embed_class_list, EmbedMetadata};

use super::{
    affinity::{self, SparkSource, SparkType},
    asset_url, html_escape, metric_value, truncate_chars,
};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-lineage-planner",
    render_visual,
};

struct TreeNode {
    class_name: &'static str,
    style: &'static str,
    role: &'static str,
    image_path: String,
    affinity: Option<i64>,
    filled: bool,
}

#[derive(Clone)]
struct SparkOddsSource {
    source: SparkSource,
}

struct SparkOddsRow {
    class_name: String,
    name: String,
    spark_type: SparkType,
    sources: Vec<SparkOddsSource>,
}

#[derive(Clone, Copy)]
struct PlannerSlot {
    position: &'static str,
    class_name: &'static str,
    style: &'static str,
    role: &'static str,
    layer: u8,
}

#[derive(Debug, Deserialize)]
struct LineageShareState {
    v: i64,
    #[serde(default)]
    n: Vec<LineageShareNode>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct LineageShareNode {
    p: usize,
    #[serde(default)]
    c: Option<i64>,
    #[serde(default)]
    s: Vec<i64>,
    #[serde(default)]
    r: Vec<i64>,
}

#[derive(Default)]
struct PlannerAffinitySummary {
    has_relation_data: bool,
    total: i64,
    relation_total: i64,
    race_total: i64,
    node_totals: BTreeMap<&'static str, i64>,
}

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none()
        && super::canonical_path_matches(&meta.canonical_url, "/tools/lineage-planner")
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let title = html_escape(&truncate_chars(&super::display_title(&meta.title), 42));
    let asset_base = metric_value(&meta.metrics, &["Asset Base"])
        .unwrap_or_else(|| "https://uma.moe/assets".to_string());
    let shared_state = shared_state_from_url(&meta.canonical_url);
    let shared_nodes = shared_state
        .as_ref()
        .map(index_share_nodes)
        .unwrap_or_default();
    let affinity_summary = if shared_state.is_some() {
        planner_affinity_summary(meta, &shared_nodes)
    } else {
        PlannerAffinitySummary::default()
    };
    let nodes = if shared_state.is_some() {
        render_shared_tree_nodes(meta, &asset_base, &shared_nodes, &affinity_summary)
    } else {
        render_tree_nodes(&asset_base)
    };
    let spark_table = if shared_state.is_some() {
        render_shared_spark_odds_table(meta, &shared_nodes, &affinity_summary)
    } else {
        render_spark_odds_table()
    };
    let stage_summary = if shared_state.is_some() {
        render_shared_stage_summary(&shared_nodes, &affinity_summary)
    } else {
        render_static_stage_summary()
    };
    let subline = if shared_state.is_some() {
        "Shared planner tree / decoded from URL"
    } else {
        "Full inheritance tree / combined spark probabilities"
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
      --surface-panel: rgba(13, 13, 13, 0.82);
      --border-subtle: rgba(255, 255, 255, 0.07);
      --border-strong: rgba(255, 255, 255, 0.13);
      --text-primary: #ffffff;
      --text-secondary: rgba(255, 255, 255, 0.72);
      --text-muted: rgba(255, 255, 255, 0.52);
      --accent-primary: #64b5f6;
      --accent-secondary: #81c784;
      --accent-warning: #ffb74d;
      --accent-pink: #f06292;
      --accent-purple: #ba68c8;
      --spark-blue: #64b5f6;
      --spark-pink: #f06292;
      --spark-green: #81c784;
      --spark-white: #d6d9df;
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

    .lineage-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 88px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 17% 14%, rgba(100, 181, 246, 0.14), transparent 330px),
        radial-gradient(circle at 77% 16%, rgba(240, 98, 146, 0.12), transparent 330px),
        radial-gradient(circle at 48% 88%, rgba(129, 199, 132, 0.11), transparent 360px),
        var(--bg-primary);
    }}

    .lineage-card::before {{
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

    .lineage-header {{
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
        linear-gradient(135deg, rgba(100, 181, 246, 0.09), rgba(129, 199, 132, 0.065), rgba(240, 98, 146, 0.055)),
        rgba(255, 255, 255, 0.012);
    }}

    .header-copy {{
      display: grid;
      gap: 6px;
      min-width: 0;
    }}

    .lineage-title {{
      overflow: hidden;
      margin: 0;
      background: linear-gradient(45deg, var(--accent-primary), var(--accent-secondary) 52%, var(--accent-pink));
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      font-size: 39px;
      font-weight: 760;
      letter-spacing: 0;
      line-height: 0.98;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .lineage-subline {{
      margin: 0;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 680;
      line-height: 1;
      text-transform: uppercase;
    }}

    .lineage-content {{
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-columns: minmax(0, 1fr) 268px;
      gap: 10px;
      min-height: 0;
      padding: 16px 42px 18px;
    }}

    .tree-stage,
    .spark-panel {{
      min-width: 0;
      min-height: 0;
      overflow: hidden;
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      background:
        linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.012)),
        var(--surface-panel);
    }}

    .tree-stage {{
      position: relative;
      background:
        linear-gradient(rgba(255, 255, 255, 0.024) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.018) 1px, transparent 1px),
        rgba(255, 255, 255, 0.014);
      background-size: 40px 40px;
    }}

    .tree-canvas {{
      position: relative;
      width: 820px;
      height: 428px;
      margin: 4px auto 0;
    }}

    .tree-lines {{
      position: absolute;
      inset: 0;
      width: 820px;
      height: 428px;
      filter: drop-shadow(0 0 8px rgba(100, 181, 246, 0.08));
    }}

    .tree-lines .branch {{
      fill: none;
      stroke: rgba(255, 255, 255, 0.36);
      stroke-width: 2;
      stroke-linecap: round;
      stroke-linejoin: round;
    }}

    .tree-lines .branch.soft {{
      stroke: rgba(255, 255, 255, 0.22);
      stroke-width: 1.5;
    }}

    .tree-node {{
      --node-color: var(--accent-primary);
      position: absolute;
      z-index: 2;
      width: var(--node-size);
      height: var(--node-size);
      min-width: 0;
      transform: translateZ(0);
    }}

    .node-ring {{
      position: relative;
      display: block;
      width: var(--node-size);
      height: var(--node-size);
      overflow: hidden;
      border: 2px solid color-mix(in srgb, var(--node-color), transparent 30%);
      border-radius: 50%;
      background:
        radial-gradient(circle at 45% 35%, rgba(255, 255, 255, 0.16), transparent 48%),
        rgba(0, 0, 0, 0.48);
      box-shadow:
        0 12px 26px rgba(0, 0, 0, 0.38),
        0 0 24px color-mix(in srgb, var(--node-color), transparent 82%);
    }}

    .node-ring img {{
      position: absolute;
      left: 50%;
      bottom: 6%;
      width: 88%;
      height: 88%;
      transform: translateX(-50%);
      object-fit: contain;
      object-position: bottom center;
    }}

    .tree-node.target {{
      --node-size: 88px;
    }}

    .tree-node.target .node-ring img,
    .tree-node.parent .node-ring img {{
      bottom: 9%;
      width: 84%;
      height: 84%;
    }}

    .tree-node.parent {{
      --node-size: 78px;
    }}

    .tree-node.gp {{
      --node-size: 68px;
    }}

    .tree-node.ggp {{
      --node-size: 50px;
    }}

    .tree-node.ggp .node-ring {{
      border-width: 1.5px;
      opacity: 0.96;
    }}

    .node-role {{
      position: absolute;
      left: 50%;
      top: calc(var(--node-size) + 4px);
      z-index: 4;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      height: 17px;
      min-width: 34px;
      padding: 1px 7px 0;
      transform: translateX(-50%);
      border: 1px solid color-mix(in srgb, var(--node-color), transparent 48%);
      border-radius: 999px;
      background: rgba(10, 10, 10, 0.88);
      color: color-mix(in srgb, var(--node-color), white 18%);
      font-size: 9px;
      font-weight: 720;
      line-height: 1;
      text-transform: uppercase;
    }}

    .tree-node.ggp .node-role {{
      top: calc(var(--node-size) + 3px);
      height: 14px;
      min-width: 26px;
      padding: 1px 5px 0;
      font-size: 7px;
      opacity: 0.8;
    }}

    .tree-node.empty .node-ring {{
      border-style: dashed;
      opacity: 0.36;
    }}

    .tree-node.empty .node-ring::after {{
      content: "";
      position: absolute;
      inset: 34%;
      border-radius: 50%;
      background: color-mix(in srgb, var(--node-color), transparent 72%);
    }}

    .affinity-badge {{
      position: absolute;
      left: 50%;
      top: calc(var(--node-size) - 8px);
      z-index: 5;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      height: 19px;
      min-width: 42px;
      padding: 0 7px;
      transform: translateX(-50%);
      border: 1px solid rgba(240, 98, 146, 0.32);
      border-radius: 999px;
      background: rgba(12, 12, 12, 0.9);
      color: var(--accent-pink);
      font-size: 10px;
      font-weight: 720;
      line-height: 1;
      box-shadow: 0 8px 18px rgba(0, 0, 0, 0.34);
    }}

    .tree-node.target .affinity-badge {{
      top: calc(var(--node-size) - 9px);
      border-color: rgba(100, 181, 246, 0.36);
      color: var(--accent-primary);
    }}

    .tree-node.gp .affinity-badge {{
      top: calc(var(--node-size) - 7px);
      min-width: 34px;
      color: var(--accent-secondary);
      border-color: rgba(129, 199, 132, 0.32);
    }}

    .stage-summary {{
      position: absolute;
      left: 14px;
      right: 14px;
      bottom: 12px;
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 8px;
      z-index: 3;
    }}

    .summary-cell {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      min-width: 0;
      height: 36px;
      padding: 0 10px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 7px;
      background: rgba(0, 0, 0, 0.3);
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 700;
      text-transform: uppercase;
    }}

    .summary-cell b {{
      color: var(--text-primary);
      font-size: 14px;
      font-weight: 720;
      font-variant-numeric: tabular-nums;
      text-transform: none;
    }}

    .spark-panel {{
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      gap: 6px;
      padding: 10px;
    }}

    .spark-head {{
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 10px;
      min-width: 0;
    }}

    .spark-head h2 {{
      margin: 0;
      color: var(--text-primary);
      font-size: 18px;
      font-weight: 760;
      line-height: 1.05;
    }}

    .spark-head span {{
      color: var(--text-muted);
      font-size: 10px;
      font-weight: 700;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .odds-table {{
      display: grid;
      align-content: start;
      gap: 3px;
      min-height: 0;
    }}

    .odds-header,
    .odds-row {{
      display: grid;
      grid-template-columns: minmax(0, 1fr) 31px 82px;
      gap: 5px;
      align-items: center;
      min-width: 0;
    }}

    .odds-header {{
      height: 13px;
      padding: 0 7px 0 9px;
      color: var(--text-muted);
      font-size: 8px;
      font-weight: 720;
      line-height: 1;
      text-transform: uppercase;
    }}

    .odds-header span:nth-child(2),
    .odds-header span:nth-child(3) {{
      text-align: right;
    }}

    .odds-row {{
      --row-color: var(--spark-white);
      position: relative;
      height: 22px;
      padding: 0 7px 0 9px;
      overflow: hidden;
      border: 1px solid rgba(255, 255, 255, 0.065);
      border-left: 2px solid var(--row-color);
      border-radius: 6px;
      background:
        linear-gradient(90deg, color-mix(in srgb, var(--row-color), transparent 92%), transparent 55%),
        rgba(255, 255, 255, 0.026);
    }}

    .odds-main {{
      display: block;
      min-width: 0;
    }}

    .odds-main b {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 9px;
      font-weight: 700;
      line-height: 1;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .odds-stars {{
      display: inline-flex;
      align-items: center;
      justify-content: flex-end;
      color: var(--row-color);
      font-size: 9px;
      font-weight: 720;
      line-height: 1;
      font-variant-numeric: tabular-nums;
      white-space: nowrap;
    }}

    .odds-rate {{
      display: inline-flex;
      align-items: center;
      justify-content: flex-end;
      gap: 5px;
      min-width: 0;
    }}

    .odds-rate b {{
      color: var(--text-primary);
      font-size: 9px;
      font-weight: 720;
      line-height: 1;
      font-variant-numeric: tabular-nums;
      min-width: 32px;
      text-align: right;
    }}

    .rate-bar {{
      position: relative;
      display: block;
      width: 36px;
      height: 3px;
      overflow: hidden;
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.09);
    }}

    .rate-bar::before {{
      content: "";
      position: absolute;
      inset: 0 auto 0 0;
      width: var(--rate);
      border-radius: inherit;
      background: linear-gradient(90deg, var(--row-color), color-mix(in srgb, var(--row-color), white 16%));
    }}

    .odds-row.blue {{ --row-color: var(--spark-blue); }}
    .odds-row.pink {{ --row-color: var(--spark-pink); }}
    .odds-row.green {{ --row-color: var(--spark-green); }}
    .odds-row.white {{ --row-color: var(--spark-white); }}

{brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-lineage-planner">
  <main class="lineage-card {class_list} card-view-lineage-planner">
    <header class="lineage-header">
      <div class="header-copy">
        <h1 class="lineage-title">{title}</h1>
        <p class="lineage-subline">{subline}</p>
      </div>
      {brand}
    </header>

    <section class="lineage-content">
      <section class="tree-stage" aria-label="Inheritance tree">
        <div class="tree-canvas">
          <svg class="tree-lines" viewBox="0 0 820 428" preserveAspectRatio="none" aria-hidden="true">
            <path class="branch" d="M410 98 V116 H180 V130 M410 116 H640 V130" />
            <path class="branch" d="M180 208 V224 H90 V238 M180 224 H270 V238" />
            <path class="branch" d="M640 208 V224 H550 V238 M640 224 H730 V238" />
            <path class="branch soft" d="M90 306 V344 H50 V365 M90 344 H130 V365" />
            <path class="branch soft" d="M270 306 V344 H230 V365 M270 344 H310 V365" />
            <path class="branch soft" d="M550 306 V344 H510 V365 M550 344 H590 V365" />
            <path class="branch soft" d="M730 306 V344 H690 V365 M730 344 H770 V365" />
          </svg>
          {nodes}
        </div>
        {stage_summary}
      </section>

      <aside class="spark-panel" aria-label="Spark Odds">
        <div class="spark-head"><h2>Spark Odds</h2><span>combined</span></div>
        {spark_table}
      </aside>
    </section>
  </main>
</body>
</html>"#,
        class_list = class_list,
        title = title,
        subline = html_escape(subline),
        brand = brand,
        brand_css = brand_css,
        nodes = nodes,
        spark_table = spark_table,
        stage_summary = stage_summary,
    )
}

fn render_tree_nodes(asset_base: &str) -> String {
    lineage_nodes()
        .iter()
        .map(|node| render_tree_node(node, asset_base))
        .collect::<Vec<_>>()
        .join("")
}

fn render_shared_tree_nodes(
    meta: &EmbedMetadata,
    asset_base: &str,
    shared_nodes: &BTreeMap<usize, LineageShareNode>,
    affinity_summary: &PlannerAffinitySummary,
) -> String {
    planner_slots()
        .iter()
        .map(|slot| {
            let node = shared_nodes
                .get(&slot_index(slot.position))
                .map(|entry| shared_tree_node(meta, slot, entry, affinity_summary))
                .unwrap_or_else(|| empty_tree_node(slot));
            render_tree_node(&node, asset_base)
        })
        .collect::<Vec<_>>()
        .join("")
}

fn lineage_nodes() -> Vec<TreeNode> {
    let static_images = [
        "101401", "100101", "100201", "100301", "100601", "100701", "100801", "100901", "101001",
        "101101", "101201", "101301", "101701", "103201", "105201",
    ];
    let static_affinity = [
        Some(180),
        Some(72),
        Some(68),
        Some(16),
        Some(14),
        Some(13),
        Some(11),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ];

    planner_slots()
        .iter()
        .enumerate()
        .map(|(index, slot)| TreeNode {
            class_name: slot.class_name,
            style: slot.style,
            role: slot.role,
            image_path: format!(
                "images/character_stand/chara_stand_{}.webp",
                static_images[index]
            ),
            affinity: static_affinity[index],
            filled: true,
        })
        .collect()
}

fn render_tree_node(node: &TreeNode, asset_base: &str) -> String {
    let image = if node.image_path.is_empty() {
        String::new()
    } else {
        let image_url = html_escape(&asset_url(asset_base, &node.image_path));
        format!(r#"<img src="{image_url}" alt="">"#)
    };
    let affinity_value = node.affinity.filter(|affinity| *affinity > 0);
    let affinity = affinity_value.map_or_else(String::new, |affinity| {
        format!(r#"<span class="affinity-badge">+{affinity}</span>"#)
    });
    let role = if affinity_value.is_some() {
        String::new()
    } else {
        format!(
            r#"<span class="node-role">{}</span>"#,
            html_escape(node.role)
        )
    };
    let state_class = if node.filled { "filled" } else { "empty" };

    format!(
        r#"<article class="tree-node {class_name} {state_class}" style="{style}">
          <span class="node-ring">{image}</span>
          {affinity}
          {role}
        </article>"#,
        class_name = node.class_name,
        state_class = state_class,
        style = node.style,
        image = image,
        role = role,
        affinity = affinity,
    )
}

fn shared_state_from_url(url: &str) -> Option<LineageShareState> {
    const MAX_SHARE_PARAM_LEN: usize = 16 * 1024;

    let query = url.split_once('?')?.1.split('#').next().unwrap_or_default();
    let tree = form_urlencoded::parse(query.as_bytes()).find_map(|(key, value)| {
        if key == "tree" {
            Some(value.into_owned())
        } else {
            None
        }
    })?;

    if tree.is_empty() || tree.len() > MAX_SHARE_PARAM_LEN {
        return None;
    }

    let bytes = URL_SAFE_NO_PAD
        .decode(tree.as_bytes())
        .or_else(|_| URL_SAFE.decode(tree.as_bytes()))
        .ok()?;
    let state = serde_json::from_slice::<LineageShareState>(&bytes).ok()?;
    if state.v == 1 && !state.n.is_empty() {
        Some(state)
    } else {
        None
    }
}

fn index_share_nodes(state: &LineageShareState) -> BTreeMap<usize, LineageShareNode> {
    state
        .n
        .iter()
        .filter(|node| node.p < planner_slots().len())
        .map(|node| (node.p, node.clone()))
        .collect()
}

fn shared_tree_node(
    meta: &EmbedMetadata,
    slot: &PlannerSlot,
    entry: &LineageShareNode,
    affinity_summary: &PlannerAffinitySummary,
) -> TreeNode {
    let image_path = entry
        .c
        .and_then(|card_id| {
            meta.resources
                .character_info(card_id)
                .map(|(_, image)| format!("images/character_stand/{image}"))
                .or_else(|| {
                    if card_id > 0 {
                        Some(format!("images/character_stand/chara_stand_{card_id}.webp"))
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| {
            if entry.s.is_empty() && entry.r.is_empty() {
                String::new()
            } else {
                String::new()
            }
        });

    TreeNode {
        class_name: slot.class_name,
        style: slot.style,
        role: slot.role,
        image_path,
        affinity: affinity_summary.node_totals.get(slot.position).copied(),
        filled: entry.c.is_some() || !entry.s.is_empty() || !entry.r.is_empty(),
    }
}

fn empty_tree_node(slot: &PlannerSlot) -> TreeNode {
    TreeNode {
        class_name: slot.class_name,
        style: slot.style,
        role: slot.role,
        image_path: String::new(),
        affinity: None,
        filled: false,
    }
}

fn planner_affinity_summary(
    meta: &EmbedMetadata,
    shared_nodes: &BTreeMap<usize, LineageShareNode>,
) -> PlannerAffinitySummary {
    let chara = |position: &str| -> Option<i64> {
        shared_nodes
            .get(&slot_index(position))
            .and_then(|node| node.c)
            .map(normalize_share_chara_id)
    };
    let wins = |position: &str| -> Vec<i64> {
        shared_nodes
            .get(&slot_index(position))
            .map(|node| clean_number_list(&node.r))
            .unwrap_or_default()
    };

    let target = chara("target");
    let p1 = chara("p1");
    let p2 = chara("p2");
    let gp11 = chara("p1-1");
    let gp12 = chara("p1-2");
    let gp21 = chara("p2-1");
    let gp22 = chara("p2-2");

    let aff2 = |a: Option<i64>, b: Option<i64>| -> i64 {
        a.zip(b)
            .and_then(|(a, b)| meta.resources.affinity2(a, b))
            .unwrap_or_default()
    };
    let aff3 = |a: Option<i64>, b: Option<i64>, c: Option<i64>| -> i64 {
        a.zip(b)
            .zip(c)
            .and_then(|((a, b), c)| meta.resources.affinity3(a, b, c))
            .unwrap_or_default()
    };

    let p1_pair = aff2(target, p1);
    let p2_pair = aff2(target, p2);
    let p1_left = aff3(target, p1, gp11);
    let p1_right = aff3(target, p1, gp12);
    let p2_left = aff3(target, p2, gp21);
    let p2_right = aff3(target, p2, gp22);
    let legacy = aff2(p1, p2);

    let p1_wins = wins("p1");
    let p2_wins = wins("p2");
    let gp11_wins = wins("p1-1");
    let gp12_wins = wins("p1-2");
    let gp21_wins = wins("p2-1");
    let gp22_wins = wins("p2-2");

    let p1_left_race = overlap_count(&p1_wins, &gp11_wins);
    let p1_right_race = overlap_count(&p1_wins, &gp12_wins);
    let p2_left_race = overlap_count(&p2_wins, &gp21_wins);
    let p2_right_race = overlap_count(&p2_wins, &gp22_wins);
    let p1_race = p1_left_race + p1_right_race;
    let p2_race = p2_left_race + p2_right_race;
    let race_total = p1_race + p2_race;

    let p1_total = p1_pair + p1_left + p1_right + p1_race;
    let p2_total = p2_pair + p2_left + p2_right + p2_race;
    let relation_total = p1_pair + p2_pair + p1_left + p1_right + p2_left + p2_right + legacy;
    let total = relation_total + race_total;

    let mut node_totals = BTreeMap::new();
    insert_positive(&mut node_totals, "target", total);
    insert_positive(&mut node_totals, "p1", p1_total);
    insert_positive(&mut node_totals, "p2", p2_total);
    insert_positive(&mut node_totals, "p1-1", p1_left + p1_left_race);
    insert_positive(&mut node_totals, "p1-2", p1_right + p1_right_race);
    insert_positive(&mut node_totals, "p2-1", p2_left + p2_left_race);
    insert_positive(&mut node_totals, "p2-2", p2_right + p2_right_race);

    PlannerAffinitySummary {
        has_relation_data: meta.resources.has_affinity(),
        total,
        relation_total,
        race_total,
        node_totals,
    }
}

fn insert_positive(map: &mut BTreeMap<&'static str, i64>, key: &'static str, value: i64) {
    if value > 0 {
        map.insert(key, value);
    }
}

fn normalize_share_chara_id(card_id: i64) -> i64 {
    if card_id >= 10_000 {
        card_id / 100
    } else {
        card_id
    }
}

fn overlap_count(primary: &[i64], secondary: &[i64]) -> i64 {
    primary
        .iter()
        .filter(|race_id| secondary.contains(race_id))
        .count() as i64
}

fn clean_number_list(values: &[i64]) -> Vec<i64> {
    let mut values = values
        .iter()
        .copied()
        .filter(|value| *value > 0)
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}

fn planner_slots() -> &'static [PlannerSlot] {
    const SLOTS: &[PlannerSlot] = &[
        PlannerSlot {
            position: "target",
            class_name: "target",
            style: "left: 366px; top: 10px; --node-color: var(--accent-primary);",
            role: "Target",
            layer: 0,
        },
        PlannerSlot {
            position: "p1",
            class_name: "parent",
            style: "left: 141px; top: 130px; --node-color: var(--accent-pink);",
            role: "P1",
            layer: 1,
        },
        PlannerSlot {
            position: "p2",
            class_name: "parent",
            style: "left: 601px; top: 130px; --node-color: var(--accent-purple);",
            role: "P2",
            layer: 1,
        },
        PlannerSlot {
            position: "p1-1",
            class_name: "gp",
            style: "left: 56px; top: 238px; --node-color: var(--accent-secondary);",
            role: "GP1",
            layer: 2,
        },
        PlannerSlot {
            position: "p1-2",
            class_name: "gp",
            style: "left: 236px; top: 238px; --node-color: var(--accent-secondary);",
            role: "GP2",
            layer: 2,
        },
        PlannerSlot {
            position: "p2-1",
            class_name: "gp",
            style: "left: 516px; top: 238px; --node-color: var(--accent-secondary);",
            role: "GP3",
            layer: 2,
        },
        PlannerSlot {
            position: "p2-2",
            class_name: "gp",
            style: "left: 696px; top: 238px; --node-color: var(--accent-secondary);",
            role: "GP4",
            layer: 2,
        },
        PlannerSlot {
            position: "p1-1-1",
            class_name: "ggp",
            style: "left: 25px; top: 365px; --node-color: var(--spark-blue);",
            role: "GGP",
            layer: 3,
        },
        PlannerSlot {
            position: "p1-1-2",
            class_name: "ggp",
            style: "left: 105px; top: 365px; --node-color: var(--spark-pink);",
            role: "GGP",
            layer: 3,
        },
        PlannerSlot {
            position: "p1-2-1",
            class_name: "ggp",
            style: "left: 205px; top: 365px; --node-color: var(--spark-blue);",
            role: "GGP",
            layer: 3,
        },
        PlannerSlot {
            position: "p1-2-2",
            class_name: "ggp",
            style: "left: 285px; top: 365px; --node-color: var(--spark-white);",
            role: "GGP",
            layer: 3,
        },
        PlannerSlot {
            position: "p2-1-1",
            class_name: "ggp",
            style: "left: 485px; top: 365px; --node-color: var(--spark-green);",
            role: "GGP",
            layer: 3,
        },
        PlannerSlot {
            position: "p2-1-2",
            class_name: "ggp",
            style: "left: 565px; top: 365px; --node-color: var(--spark-white);",
            role: "GGP",
            layer: 3,
        },
        PlannerSlot {
            position: "p2-2-1",
            class_name: "ggp",
            style: "left: 665px; top: 365px; --node-color: var(--spark-blue);",
            role: "GGP",
            layer: 3,
        },
        PlannerSlot {
            position: "p2-2-2",
            class_name: "ggp",
            style: "left: 745px; top: 365px; --node-color: var(--spark-pink);",
            role: "GGP",
            layer: 3,
        },
    ];

    SLOTS
}

fn slot_index(position: &str) -> usize {
    planner_slots()
        .iter()
        .position(|slot| slot.position == position)
        .unwrap_or_default()
}

fn render_spark_odds_table() -> String {
    render_spark_odds_rows(spark_odds_rows())
}

fn render_shared_spark_odds_table(
    meta: &EmbedMetadata,
    shared_nodes: &BTreeMap<usize, LineageShareNode>,
    affinity_summary: &PlannerAffinitySummary,
) -> String {
    let mut grouped = BTreeMap::<i64, SparkOddsRow>::new();
    for slot in planner_slots()
        .iter()
        .filter(|slot| slot.layer == 1 || slot.layer == 2)
    {
        let index = slot_index(slot.position);
        let Some(node) = shared_nodes.get(&index) else {
            continue;
        };
        let affinity = affinity_summary
            .node_totals
            .get(slot.position)
            .copied()
            .unwrap_or_default() as f32;

        for encoded_spark in clean_number_list(&node.s) {
            let Some((factor_id, level)) = decode_shared_spark(encoded_spark) else {
                continue;
            };
            let (name, factor_type) = meta.resources.factor_info(factor_id).unwrap_or_else(|| {
                (
                    fallback_factor_name(factor_id),
                    infer_factor_type(factor_id),
                )
            });
            let row = grouped.entry(factor_id).or_insert_with(|| SparkOddsRow {
                class_name: spark_class_name(factor_type).to_string(),
                name,
                spark_type: spark_type_from_factor_type(factor_type),
                sources: Vec::new(),
            });
            row.sources.push(SparkOddsSource {
                source: SparkSource::new(level, affinity),
            });
        }
    }

    let mut rows = grouped.into_values().collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        let a_metrics = spark_row_metrics(a);
        let b_metrics = spark_row_metrics(b);
        b_metrics
            .expected_procs
            .partial_cmp(&a_metrics.expected_procs)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.truncate(13);

    if rows.is_empty() {
        return r#"<div class="odds-table">
          <div class="odds-header"><span>Spark</span><span>Stars</span><span>Chance</span></div>
          <div class="odds-row white" style="--rate: 0%;">
            <span class="odds-main"><b>No shared sparks</b></span>
            <span class="odds-stars">0*</span>
            <span class="odds-rate"><b>0%</b><i class="rate-bar"></i></span>
          </div>
        </div>"#
            .to_string();
    }

    render_spark_odds_rows(rows)
}

fn render_spark_odds_rows(rows: Vec<SparkOddsRow>) -> String {
    let rows = rows
        .into_iter()
        .map(|row| {
            let total_stars: u32 = row
                .sources
                .iter()
                .map(|source| u32::from(source.source.level))
                .sum();
            let source_metrics = row
                .sources
                .iter()
                .map(|source| source.source)
                .collect::<Vec<_>>();
            let metrics = affinity::combined_spark_metrics(row.spark_type, &source_metrics, false);
            let chance = metrics.proc_chance_pct;
            let chance_display = affinity::format_probability(chance);

            format!(
                r#"<div class="odds-row {class_name}" style="--rate: {chance:.2}%;">
                  <span class="odds-main"><b>{name}</b></span>
                  <span class="odds-stars">{total_stars}*</span>
                  <span class="odds-rate"><b>{chance_display}</b><i class="rate-bar"></i></span>
                </div>"#,
                class_name = row.class_name,
                name = html_escape(&row.name),
                total_stars = total_stars,
                chance = chance,
                chance_display = chance_display,
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="odds-table">
          <div class="odds-header"><span>Spark</span><span>Stars</span><span>Chance</span></div>
          {rows}
        </div>"#
    )
}

fn render_static_stage_summary() -> String {
    r#"<div class="stage-summary">
          <span class="summary-cell"><span>Total affinity</span><b>180</b></span>
          <span class="summary-cell"><span>Parents</span><b>140</b></span>
          <span class="summary-cell"><span>Grandparents</span><b>54</b></span>
          <span class="summary-cell"><span>Race affinity</span><b>40</b></span>
        </div>"#
        .to_string()
}

fn render_shared_stage_summary(
    shared_nodes: &BTreeMap<usize, LineageShareNode>,
    affinity_summary: &PlannerAffinitySummary,
) -> String {
    let filled = shared_nodes
        .values()
        .filter(|node| node.c.is_some() || !node.s.is_empty() || !node.r.is_empty())
        .count();
    let spark_count: usize = shared_nodes.values().map(|node| node.s.len()).sum();
    let total_display = if affinity_summary.has_relation_data || affinity_summary.race_total > 0 {
        affinity_summary.total.to_string()
    } else {
        "--".to_string()
    };
    let relation_display = if affinity_summary.has_relation_data {
        affinity_summary.relation_total.to_string()
    } else {
        "--".to_string()
    };

    format!(
        r#"<div class="stage-summary">
          <span class="summary-cell"><span>Total affinity</span><b>{total}</b></span>
          <span class="summary-cell"><span>Relation</span><b>{relation}</b></span>
          <span class="summary-cell"><span>Nodes / sparks</span><b>{filled}/{spark_count}</b></span>
          <span class="summary-cell"><span>Race affinity</span><b>{race}</b></span>
        </div>"#,
        total = html_escape(&total_display),
        relation = html_escape(&relation_display),
        filled = filled,
        spark_count = spark_count,
        race = affinity_summary.race_total,
    )
}

fn spark_row_metrics(row: &SparkOddsRow) -> affinity::SparkDisplayMetrics {
    let source_metrics = row
        .sources
        .iter()
        .map(|source| source.source)
        .collect::<Vec<_>>();
    affinity::combined_spark_metrics(row.spark_type, &source_metrics, false)
}

fn decode_shared_spark(encoded_spark: i64) -> Option<(i64, u8)> {
    let factor_id = encoded_spark / 10;
    let level = encoded_spark % 10;
    if factor_id <= 0 || !(1..=3).contains(&level) {
        return None;
    }

    Some((factor_id, level as u8))
}

fn spark_type_from_factor_type(factor_type: i64) -> SparkType {
    match factor_type {
        0 => SparkType::Stats,
        1 => SparkType::Aptitude,
        5 => SparkType::Unique,
        2 => SparkType::Race,
        4 => SparkType::Scenario,
        _ => SparkType::Skill,
    }
}

fn spark_class_name(factor_type: i64) -> &'static str {
    match factor_type {
        0 => "blue",
        1 => "pink",
        5 => "green",
        _ => "white",
    }
}

fn infer_factor_type(factor_id: i64) -> i64 {
    if matches!(factor_id, 10 | 20 | 30 | 40 | 50) {
        0
    } else if (100..400).contains(&factor_id) {
        1
    } else if (100_000..200_000).contains(&factor_id) {
        2
    } else if (200_000..300_000).contains(&factor_id) {
        3
    } else if (300_000..400_000).contains(&factor_id) {
        4
    } else if factor_id >= 1_000_000 {
        5
    } else {
        3
    }
}

fn fallback_factor_name(factor_id: i64) -> String {
    match factor_id {
        10 => "Speed".to_string(),
        20 => "Stamina".to_string(),
        30 => "Power".to_string(),
        40 => "Guts".to_string(),
        50 => "Wit".to_string(),
        110 => "Turf".to_string(),
        120 => "Dirt".to_string(),
        210 => "Front".to_string(),
        220 => "Pace".to_string(),
        230 => "Late".to_string(),
        240 => "End".to_string(),
        310 => "Sprint".to_string(),
        320 => "Mile".to_string(),
        330 => "Medium".to_string(),
        340 => "Long".to_string(),
        _ => format!("Spark {factor_id}"),
    }
}

fn spark_odds_rows() -> Vec<SparkOddsRow> {
    vec![
        SparkOddsRow {
            class_name: "blue".to_string(),
            name: "Speed".to_string(),
            spark_type: SparkType::Stats,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(3, 72.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(3, 68.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "blue".to_string(),
            name: "Stamina".to_string(),
            spark_type: SparkType::Stats,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(2, 16.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(1, 14.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "blue".to_string(),
            name: "Power".to_string(),
            spark_type: SparkType::Stats,
            sources: vec![SparkOddsSource {
                source: SparkSource::new(1, 30.0),
            }],
        },
        SparkOddsRow {
            class_name: "blue".to_string(),
            name: "Guts".to_string(),
            spark_type: SparkType::Stats,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(2, 72.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(2, 11.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "pink".to_string(),
            name: "Mile".to_string(),
            spark_type: SparkType::Aptitude,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(3, 72.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(2, 68.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "pink".to_string(),
            name: "Turf".to_string(),
            spark_type: SparkType::Aptitude,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(2, 16.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(1, 11.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "pink".to_string(),
            name: "Long".to_string(),
            spark_type: SparkType::Aptitude,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(2, 68.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(1, 14.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "green".to_string(),
            name: "Victory Cheer!".to_string(),
            spark_type: SparkType::Unique,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(2, 68.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(1, 14.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "white".to_string(),
            name: "Tokyo 1600".to_string(),
            spark_type: SparkType::Race,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(2, 72.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(1, 13.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "white".to_string(),
            name: "Corner Adept".to_string(),
            spark_type: SparkType::Skill,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(3, 72.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(1, 13.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "white".to_string(),
            name: "Straightaway".to_string(),
            spark_type: SparkType::Skill,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(2, 68.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(1, 11.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "white".to_string(),
            name: "URA Finale".to_string(),
            spark_type: SparkType::Scenario,
            sources: vec![
                SparkOddsSource {
                    source: SparkSource::new(1, 30.0),
                },
                SparkOddsSource {
                    source: SparkSource::new(2, 14.0),
                },
            ],
        },
        SparkOddsRow {
            class_name: "white".to_string(),
            name: "Final Push".to_string(),
            spark_type: SparkType::Skill,
            sources: vec![SparkOddsSource {
                source: SparkSource::new(1, 16.0),
            }],
        },
    ]
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    r#"<div class="visual-panel planner-visual">
        <div class="planner-node target"><b>Target</b><small>affinity</small></div>
        <div class="planner-branch"></div>
        <div class="planner-node parent parent-a"><b>Parent</b><small>27</small></div>
        <div class="planner-node parent parent-b"><b>Parent</b><small>31</small></div>
        <div class="planner-node gp gp-a"><b>GP</b><small>race</small></div>
        <div class="planner-node gp gp-b"><b>GP</b><small>spark</small></div>
        <div class="planner-node gp gp-c"><b>GP</b><small>race</small></div>
        <div class="planner-node gp gp-d"><b>GP</b><small>spark</small></div>
      </div>"#
        .to_string()
}
