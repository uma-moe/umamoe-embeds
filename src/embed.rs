use reqwest::Client;
use serde::Deserialize;

use crate::config::Config;

#[derive(Clone, Debug)]
pub struct EmbedMetadata {
    pub title: String,
    pub description: String,
    pub canonical_url: String,
    pub image_url: String,
    pub image_alt: String,
    pub kind_label: String,
    pub metrics: Vec<EmbedMetric>,
}

#[derive(Clone, Debug)]
pub struct EmbedMetric {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
struct UserProfileResponse {
    trainer: TrainerProfile,
    #[serde(default)]
    circle: Option<ProfileCircleInfo>,
    #[serde(default)]
    fan_history: Option<FanHistory>,
}

#[derive(Debug, Deserialize, Default)]
struct TrainerProfile {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    follower_num: Option<i64>,
    #[serde(default)]
    team_evaluation_point: Option<i64>,
    #[serde(default)]
    rank_score: Option<i64>,
    #[serde(default)]
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileCircleInfo {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FanHistory {
    #[serde(default)]
    alltime: Option<FanHistoryAlltime>,
}

#[derive(Debug, Deserialize)]
struct FanHistoryAlltime {
    #[serde(default)]
    total_fans: Option<i64>,
    #[serde(default)]
    rank_total_fans: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CircleDetailsResponse {
    circle: CircleDetails,
}

#[derive(Debug, Deserialize, Default)]
struct CircleDetails {
    #[serde(default)]
    circle_id: Option<i64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    leader_name: Option<String>,
    #[serde(default)]
    member_count: Option<i64>,
    #[serde(default)]
    monthly_rank: Option<i64>,
    #[serde(default)]
    monthly_point: Option<i64>,
    #[serde(default)]
    live_rank: Option<i64>,
    #[serde(default)]
    live_points: Option<i64>,
}

pub async fn metadata_for_path(
    client: &Client,
    config: &Config,
    path: &str,
) -> Option<EmbedMetadata> {
    if should_never_embed(path) {
        return None;
    }

    let normalized_path = normalize_path(path);
    let segments = path_segments(&normalized_path);

    match segments.as_slice() {
        [] => Some(page_metadata(config, "home", "/", Some("Home"))),
        ["profile", account_id] => {
            Some(profile_metadata(client, config, account_id, "/profile").await)
        }
        ["profile", account_id, subsection] => {
            Some(profile_metadata(client, config, account_id, subsection).await)
        }
        ["circles"] => Some(page_metadata(config, "circles", "/circles", Some("Clubs"))),
        ["circles", circle_id] | ["circles", circle_id, _] => {
            Some(circle_metadata(client, config, circle_id).await)
        }
        ["database"] => Some(page_metadata(
            config,
            "database",
            "/database",
            Some("Database"),
        )),
        ["inheritance"] | ["support-cards"] => Some(page_metadata(
            config,
            "database",
            "/database",
            Some("Database"),
        )),
        ["timeline"] => Some(page_metadata(
            config,
            "timeline",
            "/timeline",
            Some("Timeline"),
        )),
        ["tierlist"] => Some(page_metadata(
            config,
            "tierlist",
            "/tierlist",
            Some("Tierlist"),
        )),
        ["rankings"] => Some(page_metadata(
            config,
            "rankings",
            "/rankings",
            Some("Rankings"),
        )),
        ["activity"] | ["activity", _] | ["shame"] | ["shame", _] => Some(page_metadata(
            config,
            "activity",
            &normalized_path,
            Some("Activity"),
        )),
        ["tools"] => Some(page_metadata(config, "tools", "/tools", Some("Tools"))),
        ["tools", "statistics"] => Some(page_metadata(
            config,
            "statistics",
            "/tools/statistics",
            Some("Statistics"),
        )),
        ["tools", "lineage-planner"] => Some(page_metadata(
            config,
            "lineage-planner",
            "/tools/lineage-planner",
            Some("Lineage Planner"),
        )),
        ["privacy-policy"] => Some(page_metadata(
            config,
            "privacy-policy",
            "/privacy-policy",
            Some("Privacy"),
        )),
        _ => Some(generic_metadata(config, &normalized_path)),
    }
}

pub async fn metadata_for_image(
    client: &Client,
    config: &Config,
    kind: &str,
    raw_id: &str,
) -> Option<EmbedMetadata> {
    let id = strip_png_suffix(raw_id);
    let id = urlencoding::decode(id).ok()?.into_owned();

    match kind {
        "profile" => Some(profile_metadata(client, config, &id, "/profile").await),
        "circle" => Some(circle_metadata(client, config, &id).await),
        "page" => Some(page_metadata_by_slug(config, &id)),
        _ => None,
    }
}

pub fn render_embed_html(meta: &EmbedMetadata) -> String {
    let title = truncate_chars(&meta.title, 90);
    let description = truncate_chars(&meta.description, 200);
    let escaped_title = html_escape(&title);
    let escaped_description = html_escape(&description);
    let escaped_url = html_escape(&meta.canonical_url);
    let escaped_image = html_escape(&meta.image_url);
    let escaped_alt = html_escape(&meta.image_alt);

    format!(
        r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="canonical" href="{url}">
  <meta name="description" content="{description}">
  <meta name="theme-color" content="#4aa8ff">

  <meta property="og:type" content="website">
  <meta property="og:site_name" content="uma.moe">
  <meta property="og:title" content="{title}">
  <meta property="og:description" content="{description}">
  <meta property="og:url" content="{url}">
  <meta property="og:image" content="{image}">
  <meta property="og:image:secure_url" content="{image}">
  <meta property="og:image:type" content="image/png">
  <meta property="og:image:width" content="1200">
  <meta property="og:image:height" content="630">
  <meta property="og:image:alt" content="{alt}">

  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{title}">
  <meta name="twitter:description" content="{description}">
  <meta name="twitter:image" content="{image}">
  <meta name="twitter:image:alt" content="{alt}">
</head>
<body>
  <a href="{url}">{title}</a>
</body>
</html>
"##,
        title = escaped_title,
        description = escaped_description,
        url = escaped_url,
        image = escaped_image,
        alt = escaped_alt,
    )
}

async fn profile_metadata(
    client: &Client,
    config: &Config,
    account_id: &str,
    subsection: &str,
) -> EmbedMetadata {
    let profile = fetch_profile(client, config, account_id).await;

    let (name, comment, circle, fans, rank, followers, team_score) = match profile {
        Some(profile) => {
            let trainer = profile.trainer;
            let circle = profile.circle.and_then(|circle| circle.name);
            let fan_history = profile.fan_history.as_ref();
            let fans = fan_history
                .and_then(|history| history.alltime.as_ref())
                .and_then(|alltime| alltime.total_fans);
            let rank = fan_history
                .and_then(|history| history.alltime.as_ref())
                .and_then(|alltime| alltime.rank_total_fans);

            (
                trainer
                    .name
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| format!("Trainer {account_id}")),
                trainer.comment,
                circle,
                fans,
                rank,
                trainer.follower_num,
                trainer.team_evaluation_point.or(trainer.rank_score),
            )
        }
        None => (
            format!("Trainer {account_id}"),
            None,
            None,
            None,
            None,
            None,
            None,
        ),
    };

    let section_label = match subsection {
        "veterans" => "Veterans",
        "cm" => "Career Menu",
        "achievements" => "Achievements",
        "titles" => "Titles",
        _ => "Profile",
    };

    let mut description_parts = vec![format!("{section_label} page for {name}.")];
    if let Some(circle) = &circle {
        description_parts.push(format!("Club: {circle}."));
    }
    if let Some(fans) = fans {
        description_parts.push(format!("Total fans: {}.", format_number(fans)));
    }
    if let Some(comment) = comment.filter(|comment| !comment.trim().is_empty()) {
        description_parts.push(comment);
    }

    let mut metrics = vec![EmbedMetric {
        label: "Trainer ID".to_string(),
        value: account_id.to_string(),
    }];
    if let Some(fans) = fans {
        metrics.push(EmbedMetric {
            label: "Fans".to_string(),
            value: compact_number(fans),
        });
    }
    if let Some(rank) = rank {
        metrics.push(EmbedMetric {
            label: "Fan Rank".to_string(),
            value: format!("#{rank}"),
        });
    }
    if let Some(followers) = followers {
        metrics.push(EmbedMetric {
            label: "Followers".to_string(),
            value: format_number(followers),
        });
    }
    if let Some(team_score) = team_score {
        metrics.push(EmbedMetric {
            label: "Team".to_string(),
            value: compact_number(team_score),
        });
    }

    EmbedMetadata {
        title: format!("{name} | uma.moe"),
        description: description_parts.join(" "),
        canonical_url: absolute_url(config, &format!("/profile/{account_id}")),
        image_url: image_url(config, "profile", account_id),
        image_alt: format!("uma.moe profile preview for {name}"),
        kind_label: section_label.to_string(),
        metrics,
    }
}

async fn circle_metadata(client: &Client, config: &Config, circle_id: &str) -> EmbedMetadata {
    let circle = fetch_circle(client, config, circle_id).await;

    let circle = circle.unwrap_or_else(|| CircleDetails {
        circle_id: circle_id.parse::<i64>().ok(),
        name: Some(format!("Club {circle_id}")),
        ..CircleDetails::default()
    });

    let name = circle
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("Club {circle_id}"));

    let mut description_parts = vec![format!(
        "Club profile for {name}, including ranking, member activity, and fan progress."
    )];
    if let Some(comment) = circle.comment.filter(|comment| !comment.trim().is_empty()) {
        description_parts.push(comment);
    }

    let mut metrics = vec![EmbedMetric {
        label: "Club ID".to_string(),
        value: circle
            .circle_id
            .map_or_else(|| circle_id.to_string(), |id| id.to_string()),
    }];
    if let Some(rank) = circle.live_rank.or(circle.monthly_rank) {
        metrics.push(EmbedMetric {
            label: "Rank".to_string(),
            value: format!("#{rank}"),
        });
    }
    if let Some(points) = circle.live_points.or(circle.monthly_point) {
        metrics.push(EmbedMetric {
            label: "Points".to_string(),
            value: compact_number(points),
        });
    }
    if let Some(members) = circle.member_count {
        metrics.push(EmbedMetric {
            label: "Members".to_string(),
            value: format_number(members),
        });
    }
    if let Some(leader) = circle.leader_name {
        metrics.push(EmbedMetric {
            label: "Leader".to_string(),
            value: leader,
        });
    }

    EmbedMetadata {
        title: format!("{name} Club | uma.moe"),
        description: description_parts.join(" "),
        canonical_url: absolute_url(config, &format!("/circles/{circle_id}")),
        image_url: image_url(config, "circle", circle_id),
        image_alt: format!("uma.moe club preview for {name}"),
        kind_label: "Club".to_string(),
        metrics,
    }
}

fn page_metadata_by_slug(config: &Config, slug: &str) -> EmbedMetadata {
    match slug {
        "home" => page_metadata(config, "home", "/", Some("Home")),
        "database" => page_metadata(config, "database", "/database", Some("Database")),
        "timeline" => page_metadata(config, "timeline", "/timeline", Some("Timeline")),
        "tierlist" => page_metadata(config, "tierlist", "/tierlist", Some("Tierlist")),
        "rankings" => page_metadata(config, "rankings", "/rankings", Some("Rankings")),
        "activity" => page_metadata(config, "activity", "/activity", Some("Activity")),
        "circles" => page_metadata(config, "circles", "/circles", Some("Clubs")),
        "tools" => page_metadata(config, "tools", "/tools", Some("Tools")),
        "statistics" => page_metadata(
            config,
            "statistics",
            "/tools/statistics",
            Some("Statistics"),
        ),
        "lineage-planner" => page_metadata(
            config,
            "lineage-planner",
            "/tools/lineage-planner",
            Some("Lineage Planner"),
        ),
        "privacy-policy" => {
            page_metadata(config, "privacy-policy", "/privacy-policy", Some("Privacy"))
        }
        _ => generic_metadata(config, "/"),
    }
}

fn page_metadata(
    config: &Config,
    slug: &str,
    path: &str,
    kind_label: Option<&str>,
) -> EmbedMetadata {
    let (title, description, metrics) = match slug {
        "home" => (
            "uma.moe - Umamusume Database & Tools",
            "A practical Umamusume companion site for inheritance search, release tracking, rankings, clubs, profiles, and planning tools.",
            vec![
                metric("Database", "Inheritance"),
                metric("Tools", "Planner"),
                metric("Live", "Rankings"),
            ],
        ),
        "database" => (
            "Database | uma.moe",
            "Find useful Umamusume inheritance parents with filters for factors, characters, races, support cards, trainer IDs, and affinity.",
            vec![
                metric("Focus", "Inheritance"),
                metric("Filters", "Advanced"),
                metric("Use", "Borrowing"),
            ],
        ),
        "timeline" => (
            "Timeline | uma.moe",
            "Track expected global releases for Umamusume characters, support cards, banners, events, campaigns, and major updates.",
            vec![metric("View", "Schedule"), metric("Server", "Global")],
        ),
        "tierlist" => (
            "Tierlist | uma.moe",
            "Explore precomputed support card tierlists and scoring views for Umamusume planning.",
            vec![metric("Cards", "Support"), metric("Mode", "Ranked")],
        ),
        "rankings" => (
            "Rankings | uma.moe",
            "Browse trainer rankings, leaderboard data, and progress comparisons for Umamusume.",
            vec![metric("View", "Leaders"), metric("Data", "Live")],
        ),
        "activity" => (
            "Activity | uma.moe",
            "Review trainer and club activity, short careers, fan gains, and ranking evidence.",
            vec![metric("View", "Activity"), metric("Data", "Trainers")],
        ),
        "circles" => (
            "Clubs | uma.moe",
            "Search Umamusume clubs by rank, points, leader, membership, and activity data.",
            vec![metric("View", "Clubs"), metric("Sort", "Rank")],
        ),
        "tools" => (
            "Tools | uma.moe",
            "Use practical Umamusume calculators and planning utilities for daily account decisions.",
            vec![metric("Tools", "Planning"), metric("Use", "Daily")],
        ),
        "statistics" => (
            "Statistics | uma.moe",
            "Explore aggregate Umamusume statistics, account trends, usage data, and comparisons.",
            vec![metric("View", "Stats"), metric("Data", "Aggregate")],
        ),
        "lineage-planner" => (
            "Lineage Planner | uma.moe",
            "Plan complete inheritance trees across parents and grandparents with saved veterans, manual entries, imports, and exports.",
            vec![metric("Tool", "Planner"), metric("Tree", "Inheritance")],
        ),
        "privacy-policy" => (
            "Privacy Policy | uma.moe",
            "Read how uma.moe handles optional accounts, cookies, analytics consent, and privacy controls.",
            vec![metric("Privacy", "Controls"), metric("Cookies", "Consent")],
        ),
        _ => (
            "uma.moe",
            "Umamusume database, timeline, tierlists, clubs, rankings, profiles, and planning tools.",
            vec![metric("Site", "uma.moe")],
        ),
    };

    EmbedMetadata {
        title: title.to_string(),
        description: description.to_string(),
        canonical_url: absolute_url(config, path),
        image_url: image_url(config, "page", slug),
        image_alt: format!("{title} preview image"),
        kind_label: kind_label.unwrap_or("uma.moe").to_string(),
        metrics,
    }
}

fn generic_metadata(config: &Config, path: &str) -> EmbedMetadata {
    EmbedMetadata {
        title: "uma.moe".to_string(),
        description:
            "Umamusume database, timeline, tierlists, clubs, rankings, profiles, and planning tools."
                .to_string(),
        canonical_url: absolute_url(config, path),
        image_url: image_url(config, "page", "home"),
        image_alt: "uma.moe preview image".to_string(),
        kind_label: "uma.moe".to_string(),
        metrics: vec![metric("Site", "uma.moe")],
    }
}

async fn fetch_profile(
    client: &Client,
    config: &Config,
    account_id: &str,
) -> Option<UserProfileResponse> {
    let url = format!(
        "{}/api/v4/user/profile/{}",
        config.api_base_url,
        urlencoding::encode(account_id)
    );

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    response.json::<UserProfileResponse>().await.ok()
}

async fn fetch_circle(client: &Client, config: &Config, circle_id: &str) -> Option<CircleDetails> {
    let url = format!(
        "{}/api/v4/circles?circle_id={}",
        config.api_base_url,
        urlencoding::encode(circle_id)
    );

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    response
        .json::<CircleDetailsResponse>()
        .await
        .ok()
        .map(|response| response.circle)
}

fn should_never_embed(path: &str) -> bool {
    let path = path.to_ascii_lowercase();

    path.starts_with("/api/")
        || path.starts_with("/assets/")
        || path.starts_with("/resources/")
        || path.starts_with("/ingest/")
        || path.starts_with("/status-api/")
        || path.starts_with("/__embeds/")
        || path == "/healthz"
        || has_file_extension(&path)
}

fn has_file_extension(path: &str) -> bool {
    let last_segment = path.rsplit('/').next().unwrap_or_default();
    last_segment.contains('.') && !last_segment.ends_with(".html")
}

fn normalize_path(path: &str) -> String {
    let path = path.trim();

    if path.is_empty() || path == "/" {
        return "/".to_string();
    }

    format!("/{}", path.trim_matches('/'))
}

fn path_segments(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn absolute_url(config: &Config, path: &str) -> String {
    if path == "/" {
        return config.public_base_url.clone();
    }

    format!("{}{}", config.public_base_url, path)
}

fn image_url(config: &Config, kind: &str, id: &str) -> String {
    format!(
        "{}/__embeds/images/{}/{}.png",
        config.public_base_url,
        kind,
        urlencoding::encode(id)
    )
}

fn strip_png_suffix(id: &str) -> &str {
    id.strip_suffix(".png").unwrap_or(id)
}

fn metric(label: &str, value: &str) -> EmbedMetric {
    EmbedMetric {
        label: label.to_string(),
        value: value.to_string(),
    }
}

fn format_number(value: i64) -> String {
    let mut chars: Vec<char> = value.abs().to_string().chars().rev().collect();
    let mut formatted = String::new();

    for (index, ch) in chars.drain(..).enumerate() {
        if index > 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(ch);
    }

    let formatted: String = formatted.chars().rev().collect();
    if value < 0 {
        format!("-{formatted}")
    } else {
        formatted
    }
}

fn compact_number(value: i64) -> String {
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

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            public_base_url: "https://uma.moe".to_string(),
            frontend_origin: "http://127.0.0.1:4200".to_string(),
            api_base_url: "https://uma.moe".to_string(),
            bot_user_agent_tokens: vec![],
            debug_query_key: "__embed".to_string(),
        }
    }

    #[test]
    fn static_route_uses_canonical_same_url() {
        let meta = page_metadata(&config(), "timeline", "/timeline", Some("Timeline"));
        assert_eq!(meta.canonical_url, "https://uma.moe/timeline");
        assert!(meta
            .image_url
            .starts_with("https://uma.moe/__embeds/images/"));
    }

    #[test]
    fn ignores_assets_and_api() {
        assert!(should_never_embed("/assets/app.js"));
        assert!(should_never_embed("/api/v4/circles"));
        assert!(!should_never_embed("/circles/772781438"));
    }
}
