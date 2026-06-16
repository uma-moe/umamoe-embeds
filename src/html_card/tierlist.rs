use crate::embed::{embed_class_list, EmbedMetadata, TierlistCardDetails, TierlistRowDetails};

use super::{html_escape, truncate_chars};

pub(super) const VIEW: super::overview::CardView = super::overview::CardView {
    class_name: "card-view-tierlist",
    render_visual,
};

struct TierRow {
    tier: &'static str,
    range: &'static str,
    cards: &'static [TierCard],
}

struct TierCard {
    id: &'static str,
    name: &'static str,
    stat_type: &'static str,
    score: &'static str,
}

pub(super) fn renders_full_card(meta: &EmbedMetadata) -> bool {
    meta.database.is_none() && super::canonical_path_matches(&meta.canonical_url, "/tierlist")
}

pub(super) fn render_card_html(meta: &EmbedMetadata) -> String {
    let class_list = embed_class_list(meta);
    let title_text = match super::display_title(&meta.title).as_str() {
        "Tierlist" => "Support Card Tierlist".to_string(),
        title => title.to_string(),
    };
    let title = html_escape(&truncate_chars(&title_text, 44));
    let tiers = render_tier_rows(meta);
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
      --tier-s-plus: #ff1744;
      --tier-s: #ff6b35;
      --tier-a: #f7931e;
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

    .tierlist-card {{
      position: relative;
      width: 1200px;
      height: 630px;
      display: grid;
      grid-template-rows: 104px minmax(0, 1fr);
      overflow: hidden;
      background:
        radial-gradient(circle at 17% 12%, rgba(100, 181, 246, 0.13), transparent 350px),
        radial-gradient(circle at 78% 8%, rgba(129, 199, 132, 0.11), transparent 330px),
        radial-gradient(circle at 46% 86%, rgba(255, 183, 77, 0.11), transparent 360px),
        var(--bg-primary);
    }}

    .tierlist-card::before {{
      content: "";
      position: absolute;
      inset: 0;
      background:
        linear-gradient(rgba(255, 255, 255, 0.03) 1px, transparent 1px),
        linear-gradient(90deg, rgba(255, 255, 255, 0.022) 1px, transparent 1px);
      background-size: 72px 72px;
      mask-image: linear-gradient(90deg, transparent, #000 17%, #000 86%, transparent);
      opacity: 0.46;
      pointer-events: none;
    }}

    .tierlist-header {{
      position: relative;
      z-index: 1;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: start;
      gap: 36px;
      min-width: 0;
      padding: 24px 48px 14px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.075);
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.09), rgba(255, 183, 77, 0.055)),
        rgba(255, 255, 255, 0.012);
    }}

    .header-copy {{
      display: grid;
      gap: 0;
      min-width: 0;
    }}

    .tierlist-title-wrap {{
      display: flex;
      align-items: center;
      gap: 12px;
      min-width: 0;
      transform: translateY(4px);
    }}

    .tierlist-title {{
      overflow: hidden;
      margin: 0;
      background: linear-gradient(45deg, var(--accent-primary), var(--accent-secondary) 54%, var(--accent-warning));
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      font-size: 41px;
      font-weight: 760;
      letter-spacing: 0;
      line-height: 0.96;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .tierlist-kicker {{
      display: inline-flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      color: var(--text-secondary);
      font-size: 12px;
      font-weight: 680;
      line-height: 0.95;
      transform: translateY(-4px);
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .legacy-badge {{
      display: inline-flex;
      align-items: center;
      height: 22px;
      padding: 0 8px;
      border: 1px solid rgba(255, 183, 77, 0.34);
      border-radius: 6px;
      background: rgba(255, 183, 77, 0.1);
      color: var(--accent-warning);
      font-size: 10px;
      font-weight: 700;
      line-height: 1;
      flex-shrink: 0;
      text-transform: uppercase;
    }}

    .tierlist-content {{
      position: relative;
      z-index: 1;
      min-height: 0;
      padding: 12px 48px 26px;
    }}

    .tier-board {{
      display: grid;
      grid-template-rows: repeat(5, minmax(0, 1fr));
      gap: 8px;
      height: 100%;
      min-height: 0;
    }}

    .tier-row {{
      display: grid;
      grid-template-columns: 110px minmax(0, 1fr);
      min-height: 0;
      overflow: hidden;
      border: 1px solid var(--row-border);
      border-radius: 9px;
      background:
        linear-gradient(135deg, var(--row-glow), rgba(255, 255, 255, 0.018)),
        rgba(255, 255, 255, 0.026);
    }}

    .tier-row.s-plus {{ --row-color: var(--tier-s-plus); --row-border: rgba(255, 23, 68, 0.36); --row-glow: rgba(255, 23, 68, 0.095); }}
    .tier-row.s {{ --row-color: var(--tier-s); --row-border: rgba(255, 107, 53, 0.34); --row-glow: rgba(255, 107, 53, 0.09); }}
    .tier-row.a {{ --row-color: var(--tier-a); --row-border: rgba(247, 147, 30, 0.32); --row-glow: rgba(247, 147, 30, 0.085); }}
    .tier-row.b {{ --row-color: #ffd666; --row-border: rgba(255, 214, 102, 0.28); --row-glow: rgba(255, 214, 102, 0.075); }}
    .tier-row.c {{ --row-color: #81c784; --row-border: rgba(129, 199, 132, 0.26); --row-glow: rgba(129, 199, 132, 0.07); }}

    .tier-label {{
      position: relative;
      display: grid;
      justify-items: center;
      align-content: center;
      gap: 6px;
      min-height: 0;
      padding: 9px 8px;
      background:
        linear-gradient(135deg, color-mix(in srgb, var(--row-color) 72%, transparent), rgba(0, 0, 0, 0.18)),
        rgba(255, 255, 255, 0.03);
      color: #ffffff;
      text-align: center;
      text-shadow: 0 2px 9px rgba(0, 0, 0, 0.45);
    }}

    .tier-name {{
      font-size: 28px;
      font-weight: 700;
      line-height: 0.95;
    }}

    .tier-range {{
      color: rgba(255, 255, 255, 0.82);
      font-size: 10px;
      font-weight: 680;
      line-height: 1.15;
      text-transform: uppercase;
    }}

    .tier-cards {{
      display: grid;
      grid-template-columns: repeat(5, minmax(0, 1fr));
      gap: 8px;
      min-width: 0;
      min-height: 0;
      padding: 8px;
      background: rgba(0, 0, 0, 0.14);
    }}

    .support-card {{
      display: grid;
      grid-template-columns: 54px minmax(0, 1fr);
      gap: 8px;
      align-items: center;
      min-width: 0;
      min-height: 0;
      padding: 7px;
      border: 1px solid rgba(255, 255, 255, 0.075);
      border-radius: 8px;
      background: rgba(0, 0, 0, 0.22);
      overflow: hidden;
    }}

    .card-art {{
      position: relative;
      display: grid;
      place-items: center;
      width: 54px;
      height: 54px;
      overflow: hidden;
      border: 1px solid rgba(255, 255, 255, 0.11);
      border-radius: 7px;
      background:
        linear-gradient(135deg, rgba(100, 181, 246, 0.22), rgba(255, 183, 77, 0.14)),
        rgba(255, 255, 255, 0.04);
    }}

    .art-fallback {{
      color: rgba(255, 255, 255, 0.72);
      font-size: 14px;
      font-weight: 720;
      letter-spacing: 0;
      line-height: 1;
    }}

    .card-art img {{
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      object-fit: cover;
      display: block;
    }}

    .card-copy {{
      display: grid;
      gap: 5px;
      min-width: 0;
    }}

    .card-type {{
      overflow: hidden;
      color: var(--text-muted);
      font-size: 9px;
      font-weight: 700;
      line-height: 1;
      text-overflow: ellipsis;
      text-transform: uppercase;
      white-space: nowrap;
    }}

    .card-name {{
      overflow: hidden;
      color: var(--text-primary);
      font-size: 12px;
      font-weight: 680;
      line-height: 1.05;
      text-overflow: ellipsis;
      white-space: nowrap;
    }}

    .card-score {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 8px;
      min-width: 0;
      color: var(--text-secondary);
      font-size: 10px;
      font-weight: 680;
      line-height: 1;
    }}

    .card-score b {{
      color: var(--row-color);
      font-size: 13px;
      font-weight: 700;
      font-variant-numeric: tabular-nums;
    }}
{brand_css}
  </style>
</head>
<body class="embed-card-page {class_list} card-view-tierlist">
  <main class="tierlist-card {class_list} card-view-tierlist">
    <header class="tierlist-header">
      <div class="header-copy">
        <div class="tierlist-title-wrap"><h1 class="tierlist-title">{title}</h1><span class="legacy-badge">Legacy</span></div>
        <h5 class="tierlist-kicker">LB4 support cards</h5>
      </div>
      {brand}
    </header>

    <section class="tierlist-content">
      <section class="tier-board">
        {tiers}
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
        tiers = tiers,
    )
}

fn render_visual(_meta: &EmbedMetadata) -> String {
    let rows = [("S+", 5usize), ("S", 4usize), ("A", 3usize)]
        .into_iter()
        .map(|(tier, count)| {
            let cards = (0..count)
                .map(|index| format!(r#"<span class="tier-card tier-card-{}"></span>"#, index + 1))
                .collect::<Vec<_>>()
                .join("");
            format!(r#"<div class="tier-row-mini"><span>{tier}</span><div>{cards}</div></div>"#)
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="visual-panel tierlist-visual">
        <div class="tierlist-tabs-mini"><span>Speed</span><span>Stamina</span><span>Power</span></div>
        {rows}
      </div>"#
    )
}

fn render_tier_rows(meta: &EmbedMetadata) -> String {
    if let Some(tierlist) = meta
        .tierlist
        .as_ref()
        .filter(|tierlist| !tierlist.rows.is_empty())
    {
        return tierlist
            .rows
            .iter()
            .map(|row| render_dynamic_tier_row(row, &tierlist.asset_base_url))
            .collect::<Vec<_>>()
            .join("");
    }

    const S_PLUS: &[TierCard] = &[
        TierCard {
            id: "30028",
            name: "Kitasan Black",
            stat_type: "Speed",
            score: "3647",
        },
        TierCard {
            id: "30016",
            name: "Super Creek",
            stat_type: "Stamina",
            score: "2589",
        },
        TierCard {
            id: "30005",
            name: "Vodka",
            stat_type: "Power",
            score: "2921",
        },
        TierCard {
            id: "30011",
            name: "Ines Fujin",
            stat_type: "Guts",
            score: "2891",
        },
        TierCard {
            id: "30010",
            name: "Fine Motion",
            stat_type: "Intelligence",
            score: "2233",
        },
    ];
    const S: &[TierCard] = &[
        TierCard {
            id: "30015",
            name: "Sakura Bakushin O",
            stat_type: "Speed",
            score: "3414",
        },
        TierCard {
            id: "30043",
            name: "Nakayama Festa",
            stat_type: "Stamina",
            score: "2436",
        },
        TierCard {
            id: "30007",
            name: "El Condor Pasa",
            stat_type: "Power",
            score: "2838",
        },
        TierCard {
            id: "30083",
            name: "Sakura Bakushin O",
            stat_type: "Guts",
            score: "2865",
        },
        TierCard {
            id: "30097",
            name: "Mr. C.B.",
            stat_type: "Intelligence",
            score: "2207",
        },
    ];
    const A: &[TierCard] = &[
        TierCard {
            id: "30065",
            name: "Zenno Rob Roy",
            stat_type: "Speed",
            score: "3068",
        },
        TierCard {
            id: "30075",
            name: "Manhattan Cafe",
            stat_type: "Stamina",
            score: "2364",
        },
        TierCard {
            id: "30056",
            name: "King Halo",
            stat_type: "Power",
            score: "2804",
        },
        TierCard {
            id: "30019",
            name: "Haru Urara",
            stat_type: "Guts",
            score: "2795",
        },
        TierCard {
            id: "30082",
            name: "Nishino Flower",
            stat_type: "Intelligence",
            score: "2148",
        },
    ];
    const B: &[TierCard] = &[
        TierCard {
            id: "30039",
            name: "Kawakami Princess",
            stat_type: "Speed",
            score: "2688",
        },
        TierCard {
            id: "30009",
            name: "Tamamo Cross",
            stat_type: "Stamina",
            score: "2236",
        },
        TierCard {
            id: "30106",
            name: "Air Groove",
            stat_type: "Power",
            score: "2482",
        },
        TierCard {
            id: "30001",
            name: "Special Week",
            stat_type: "Guts",
            score: "2626",
        },
        TierCard {
            id: "30073",
            name: "Narita Taishin",
            stat_type: "Intelligence",
            score: "2089",
        },
    ];
    const C: &[TierCard] = &[
        TierCard {
            id: "30018",
            name: "Nishino Flower",
            stat_type: "Speed",
            score: "2476",
        },
        TierCard {
            id: "30022",
            name: "Mejiro McQueen",
            stat_type: "Stamina",
            score: "2119",
        },
        TierCard {
            id: "30032",
            name: "Yaeno Muteki",
            stat_type: "Power",
            score: "2290",
        },
        TierCard {
            id: "30040",
            name: "Hishi Akebono",
            stat_type: "Guts",
            score: "2340",
        },
        TierCard {
            id: "30041",
            name: "Mejiro Dober",
            stat_type: "Intelligence",
            score: "1952",
        },
    ];

    [
        TierRow {
            tier: "S+",
            range: "Top percentile",
            cards: S_PLUS,
        },
        TierRow {
            tier: "S",
            range: "High percentile",
            cards: S,
        },
        TierRow {
            tier: "A",
            range: "Strong picks",
            cards: A,
        },
        TierRow {
            tier: "B",
            range: "Strong picks",
            cards: B,
        },
        TierRow {
            tier: "C",
            range: "Strong picks",
            cards: C,
        },
    ]
    .into_iter()
    .map(render_static_tier_row)
    .collect::<Vec<_>>()
    .join("")
}

fn render_dynamic_tier_row(row: &TierlistRowDetails, asset_base_url: &str) -> String {
    let cards = row
        .cards
        .iter()
        .map(|card| render_dynamic_support_card(card, asset_base_url))
        .collect::<Vec<_>>()
        .join("");

    render_tier_row_shell(&row.tier, &row.range, &cards)
}

fn render_static_tier_row(row: TierRow) -> String {
    let cards = row
        .cards
        .iter()
        .map(|card| render_static_support_card(card))
        .collect::<Vec<_>>()
        .join("");

    render_tier_row_shell(row.tier, row.range, &cards)
}

fn render_tier_row_shell(tier: &str, range: &str, cards: &str) -> String {
    let row_class = match tier {
        "S+" => "s-plus",
        "S" => "s",
        "A" => "a",
        "B" => "b",
        "C" => "c",
        _ => "other",
    };
    format!(
        r#"<article class="tier-row {row_class}">
          <div class="tier-label"><span class="tier-name">{tier}</span><span class="tier-range">{range}</span></div>
          <div class="tier-cards">{cards}</div>
        </article>"#,
        row_class = row_class,
        tier = html_escape(tier),
        range = html_escape(range),
        cards = cards,
    )
}

fn render_dynamic_support_card(card: &TierlistCardDetails, asset_base_url: &str) -> String {
    render_support_card_parts(
        &card.id.to_string(),
        &card.name,
        &card.stat_type,
        &card.score.to_string(),
        asset_base_url,
    )
}

fn render_static_support_card(card: &TierCard) -> String {
    render_support_card_parts(
        card.id,
        card.name,
        card.stat_type,
        card.score,
        "https://uma.moe/assets",
    )
}

fn render_support_card_parts(
    id: &str,
    name: &str,
    stat_type: &str,
    score: &str,
    asset_base_url: &str,
) -> String {
    let image = super::asset_url(
        asset_base_url,
        &format!("/images/support_card/half/support_card_s_{id}.webp"),
    );
    let marker = match stat_type {
        "Speed" => "SPD",
        "Stamina" => "STA",
        "Power" => "PWR",
        "Guts" => "GUT",
        "Intelligence" => "INT",
        _ => "SSR",
    };

    format!(
        r#"<article class="support-card">
          <div class="card-art"><span class="art-fallback">{marker}</span><img src="{image}" alt="" onerror="this.style.display='none'"></div>
          <div class="card-copy">
            <span class="card-type">{stat_type}</span>
            <strong class="card-name">{name}</strong>
            <span class="card-score"><span>LB4</span><b>{score}</b></span>
          </div>
        </article>"#,
        image = html_escape(&image),
        marker = html_escape(marker),
        stat_type = html_escape(stat_type),
        name = html_escape(name),
        score = html_escape(score),
    )
}
