use std::{
    collections::BTreeMap,
    io::Read,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use flate2::read::GzDecoder;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use tracing::{debug, info, warn};
use url::form_urlencoded;

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
    pub database: Option<DatabaseEmbedDetails>,
    pub tierlist: Option<TierlistEmbedDetails>,
    pub resources: ResourceCatalog,
}

#[derive(Clone, Debug)]
pub struct EmbedMetric {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct TierlistEmbedDetails {
    pub asset_base_url: String,
    pub generated_at: Option<String>,
    pub rows: Vec<TierlistRowDetails>,
}

#[derive(Clone, Debug)]
pub struct TierlistRowDetails {
    pub tier: String,
    pub range: String,
    pub cards: Vec<TierlistCardDetails>,
}

#[derive(Clone, Debug)]
pub struct TierlistCardDetails {
    pub id: i64,
    pub name: String,
    pub stat_type: String,
    pub score: i64,
}

#[derive(Clone, Debug)]
pub struct TimelineEmbedDetails {
    pub events: Vec<TimelineEventDetails>,
}

#[derive(Clone, Debug)]
pub struct TimelineEventDetails {
    pub event_type: String,
    pub title: String,
    pub description: Option<String>,
    pub image_path: Option<String>,
    pub global_release_date: String,
    pub estimated_end_date: Option<String>,
    pub is_confirmed: bool,
    pub pickup_card_ids: Vec<i64>,
    pub related_characters: Vec<String>,
    pub related_support_cards: Vec<String>,
    pub prediction_kind: Option<String>,
    pub prediction_likelihood: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct DatabaseEmbedDetails {
    pub asset_base_url: String,
    pub resources: ResourceCatalog,
    pub query_label: String,
    pub result_total: i64,
    pub matched_factor_ids: Vec<i64>,
    pub matched_main_factor_ids: Vec<i64>,
    pub matched_support_card_id: Option<i64>,
    pub matched_min_limit_break: Option<i64>,
    pub trainer_name: String,
    pub trainer_id: String,
    pub record_id: Option<i64>,
    pub main_parent_id: Option<i64>,
    pub parent_left_id: Option<i64>,
    pub parent_right_id: Option<i64>,
    pub parent_rank: Option<i64>,
    pub parent_rarity: Option<i64>,
    pub affinity_score: Option<i64>,
    pub left_affinity_score: Option<i64>,
    pub right_affinity_score: Option<i64>,
    pub win_count: Option<i64>,
    pub white_count: Option<i64>,
    pub follower_num: Option<i64>,
    pub support_card_id: Option<i64>,
    pub limit_break_count: Option<i64>,
    pub last_updated: Option<String>,
    pub blue_sparks: Vec<i64>,
    pub pink_sparks: Vec<i64>,
    pub green_sparks: Vec<i64>,
    pub white_sparks: Vec<i64>,
    pub main_blue_factors: Option<i64>,
    pub main_pink_factors: Option<i64>,
    pub main_green_factors: Option<i64>,
    pub main_white_factors: Vec<i64>,
    pub left_blue_factors: Option<i64>,
    pub left_pink_factors: Option<i64>,
    pub left_green_factors: Option<i64>,
    pub left_white_factors: Vec<i64>,
    pub right_blue_factors: Option<i64>,
    pub right_pink_factors: Option<i64>,
    pub right_green_factors: Option<i64>,
    pub right_white_factors: Vec<i64>,
    pub main_win_saddles: Vec<i64>,
    pub left_win_saddles: Vec<i64>,
    pub right_win_saddles: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct UserProfileResponse {
    trainer: TrainerProfile,
    #[serde(default)]
    circle: Option<ProfileCircleInfo>,
    #[serde(default)]
    circle_history: Vec<Value>,
    #[serde(default)]
    fan_history: Option<FanHistory>,
    #[serde(default)]
    inheritance: Option<ProfileInheritance>,
    #[serde(default)]
    support_card: Option<ProfileSupportCard>,
    #[serde(default)]
    team_stadium: Vec<ProfileTeamStadiumMember>,
}

#[derive(Debug, Deserialize, Default)]
struct TrainerProfile {
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_i64",
        alias = "leaderCharaDressId"
    )]
    leader_chara_dress_id: Option<i64>,
    #[serde(default)]
    follower_num: Option<i64>,
    #[serde(default)]
    own_follow_num: Option<i64>,
    #[serde(default)]
    team_evaluation_point: Option<i64>,
    #[serde(default)]
    team_class: Option<i64>,
    #[serde(default)]
    best_team_class: Option<i64>,
    #[serde(default)]
    rank_score: Option<i64>,
    #[serde(default)]
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileCircleInfo {
    #[serde(default)]
    circle_id: Option<i64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    member_count: Option<i64>,
    #[serde(default)]
    monthly_rank: Option<i64>,
    #[serde(default)]
    monthly_point: Option<i64>,
    #[serde(default)]
    live_points: Option<i64>,
    #[serde(default)]
    live_rank: Option<i64>,
    #[serde(default)]
    club_rank: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct FanHistory {
    #[serde(default)]
    alltime: Option<FanHistoryAlltime>,
    #[serde(default)]
    rolling: Option<FanHistoryRolling>,
    #[serde(default)]
    monthly: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct FanHistoryAlltime {
    #[serde(default)]
    total_fans: Option<i64>,
    #[serde(default)]
    rank_total_fans: Option<i64>,
    #[serde(default)]
    total_gain: Option<i64>,
    #[serde(default)]
    active_days: Option<i64>,
    #[serde(default)]
    avg_day: Option<f64>,
    #[serde(default)]
    avg_week: Option<f64>,
    #[serde(default)]
    avg_month: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct FanHistoryRolling {
    #[serde(default)]
    gain_3d: Option<i64>,
    #[serde(default)]
    gain_7d: Option<i64>,
    #[serde(default)]
    gain_30d: Option<i64>,
    #[serde(default)]
    rank_7d: Option<i64>,
    #[serde(default)]
    rank_30d: Option<i64>,
}

#[derive(Debug)]
struct ProfileFanMonthMetric {
    original_index: usize,
    sort_key: Option<(i64, i64)>,
    period: String,
    fans: String,
    gain: String,
    days: String,
    avg_day: String,
    rank: String,
    circle: String,
}

#[derive(Debug, Default)]
struct ProfileCircleHistoryMetric {
    rank: Option<i64>,
    points: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProfileInheritance {
    #[serde(default)]
    blue_stars_sum: Option<i64>,
    #[serde(default)]
    pink_stars_sum: Option<i64>,
    #[serde(default)]
    green_stars_sum: Option<i64>,
    #[serde(default)]
    white_stars_sum: Option<i64>,
    #[serde(default)]
    affinity_score: Option<i64>,
    #[serde(default, alias = "mainParentId")]
    main_parent_id: Option<i64>,
    #[serde(default, alias = "parentLeftId")]
    parent_left_id: Option<i64>,
    #[serde(default, alias = "parentRightId")]
    parent_right_id: Option<i64>,
    #[serde(default, alias = "blueSparks")]
    blue_sparks: Vec<i64>,
    #[serde(default, alias = "pinkSparks")]
    pink_sparks: Vec<i64>,
    #[serde(default, alias = "greenSparks")]
    green_sparks: Vec<i64>,
    #[serde(default, alias = "whiteSparks")]
    white_sparks: Vec<i64>,
    #[serde(default, alias = "mainBlueFactors")]
    main_blue_factors: Option<i64>,
    #[serde(default, alias = "mainPinkFactors")]
    main_pink_factors: Option<i64>,
    #[serde(default, alias = "mainGreenFactors")]
    main_green_factors: Option<i64>,
    #[serde(default, alias = "mainWhiteFactors")]
    main_white_factors: Vec<i64>,
    #[serde(default, alias = "leftBlueFactors")]
    left_blue_factors: Option<i64>,
    #[serde(default, alias = "leftPinkFactors")]
    left_pink_factors: Option<i64>,
    #[serde(default, alias = "leftGreenFactors")]
    left_green_factors: Option<i64>,
    #[serde(default, alias = "leftWhiteFactors")]
    left_white_factors: Vec<i64>,
    #[serde(default, alias = "rightBlueFactors")]
    right_blue_factors: Option<i64>,
    #[serde(default, alias = "rightPinkFactors")]
    right_pink_factors: Option<i64>,
    #[serde(default, alias = "rightGreenFactors")]
    right_green_factors: Option<i64>,
    #[serde(default, alias = "rightWhiteFactors")]
    right_white_factors: Vec<i64>,
    #[serde(default, alias = "mainWinSaddles")]
    main_win_saddles: Vec<i64>,
    #[serde(default, alias = "leftWinSaddles")]
    left_win_saddles: Vec<i64>,
    #[serde(default, alias = "rightWinSaddles")]
    right_win_saddles: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct ProfileSupportCard {
    #[serde(default)]
    support_card_id: Option<i64>,
    #[serde(default)]
    limit_break_count: Option<i64>,
    #[serde(default)]
    experience: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ProfileTeamStadiumMember {
    #[serde(
        default,
        deserialize_with = "deserialize_optional_i64",
        alias = "chara_id",
        alias = "charaId",
        alias = "character_id",
        alias = "characterId",
        alias = "uma_id",
        alias = "umaId"
    )]
    character_id: Option<i64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_i64",
        alias = "cardId"
    )]
    card_id: Option<i64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_i64",
        alias = "trainedCharaId"
    )]
    trained_chara_id: Option<i64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_i64",
        alias = "distanceType"
    )]
    distance_type: Option<i64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_i64",
        alias = "rankScore"
    )]
    rank_score: Option<i64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_i64",
        alias = "runningStyle"
    )]
    running_style: Option<i64>,
}

impl ProfileTeamStadiumMember {
    fn stadium_character_asset_id(&self) -> Option<i64> {
        self.card_id
            .or(self.character_id)
            .or_else(|| self.trained_chara_id.filter(|id| *id >= 10_000))
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
struct SiteStatsResponse {
    #[serde(default)]
    today: TodayActivityStats,
    #[serde(default)]
    freshness: DataFreshnessStats,
}

#[derive(Clone, Debug, Deserialize, Default)]
struct TodayActivityStats {
    #[serde(default)]
    tasks_24h: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Default)]
struct DataFreshnessStats {
    #[serde(default)]
    accounts_24h: Option<i64>,
    #[serde(default)]
    accounts_7d: Option<i64>,
    #[serde(default)]
    umas_tracked: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct StatisticsDatasetsResponse {
    #[serde(default)]
    datasets: Vec<StatisticsDatasetInfo>,
}

#[derive(Debug, Deserialize, Default)]
struct StatisticsDatasetInfo {
    #[serde(default, rename = "basePath")]
    base_path: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    format_version: Option<i64>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    index: Option<StatisticsDatasetIndex>,
}

#[derive(Debug, Deserialize, Default)]
struct StatisticsDatasetIndex {
    #[serde(default)]
    format_version: Option<i64>,
    #[serde(default)]
    total_entries: Option<i64>,
    #[serde(default)]
    total_trainers: Option<i64>,
    #[serde(default)]
    generated_at: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct MonthlyRankingsResponse {
    #[serde(default)]
    rankings: Vec<UserFanRankingMonthly>,
    #[serde(default)]
    total: i64,
    #[serde(default)]
    year: Option<i64>,
    #[serde(default)]
    month: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct UserFanRankingMonthly {
    #[serde(default)]
    viewer_id: Option<i64>,
    #[serde(default)]
    trainer_name: Option<String>,
    #[serde(default)]
    total_fans: Option<i64>,
    #[serde(default)]
    monthly_gain: Option<i64>,
    #[serde(default)]
    active_days: Option<i64>,
    #[serde(default)]
    avg_daily: Option<f64>,
    #[serde(default)]
    rank: Option<i64>,
    #[serde(default)]
    circle_name: Option<String>,
    #[serde(default)]
    circle_rank: Option<i64>,
    #[serde(default)]
    club_rank: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct AlltimeRankingsResponse {
    #[serde(default)]
    rankings: Vec<UserFanRankingAlltime>,
    #[serde(default)]
    total: i64,
}

#[derive(Debug, Deserialize, Default)]
struct UserFanRankingAlltime {
    #[serde(default)]
    viewer_id: Option<i64>,
    #[serde(default)]
    trainer_name: Option<String>,
    #[serde(default)]
    total_fans: Option<i64>,
    #[serde(default)]
    total_gain: Option<i64>,
    #[serde(default)]
    avg_day: Option<f64>,
    #[serde(default)]
    avg_week: Option<f64>,
    #[serde(default)]
    avg_month: Option<f64>,
    #[serde(default)]
    rank: Option<i64>,
    #[serde(default)]
    rank_total_gain: Option<i64>,
    #[serde(default)]
    rank_total_fans: Option<i64>,
    #[serde(default)]
    rank_avg_day: Option<i64>,
    #[serde(default)]
    rank_avg_week: Option<i64>,
    #[serde(default)]
    rank_avg_month: Option<i64>,
    #[serde(default)]
    circle_name: Option<String>,
    #[serde(default)]
    circle_rank: Option<i64>,
    #[serde(default)]
    club_rank: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct GainsRankingsResponse {
    #[serde(default)]
    rankings: Vec<UserFanRankingGains>,
    #[serde(default)]
    total: i64,
    #[serde(default)]
    sort_by: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct UserFanRankingGains {
    #[serde(default)]
    viewer_id: Option<i64>,
    #[serde(default)]
    trainer_name: Option<String>,
    #[serde(default)]
    gain_3d: Option<i64>,
    #[serde(default)]
    gain_7d: Option<i64>,
    #[serde(default)]
    gain_30d: Option<i64>,
    #[serde(default)]
    rank_3d: Option<i64>,
    #[serde(default)]
    rank_7d: Option<i64>,
    #[serde(default)]
    rank_30d: Option<i64>,
    #[serde(default)]
    circle_name: Option<String>,
    #[serde(default)]
    circle_rank: Option<i64>,
    #[serde(default)]
    club_rank: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct ActivityHallResponse {
    #[serde(default)]
    entries: Vec<ActivityHallEntry>,
    #[serde(default)]
    total: i64,
    #[serde(default)]
    last_refreshed_at: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ActivityHallEntry {
    #[serde(default)]
    viewer_id: Option<i64>,
    #[serde(default)]
    trainer_name: Option<String>,
    #[serde(default)]
    circle_name: Option<String>,
    #[serde(default)]
    circle_monthly_rank: Option<i64>,
    #[serde(default)]
    days_observed: Option<i64>,
    #[serde(default)]
    total_active_seconds: Option<i64>,
    #[serde(default)]
    total_fan_gain: Option<i64>,
    #[serde(default)]
    total_careers: Option<i64>,
    #[serde(default)]
    careers_per_active_hour: Option<f64>,
    #[serde(default)]
    avg_career_length_last20_seconds: Option<f64>,
    #[serde(default)]
    short_high_fan_careers: Option<i64>,
    #[serde(default)]
    short_fan_gain_score: Option<f64>,
    #[serde(default)]
    career_length_buckets: Vec<i64>,
    #[serde(default)]
    short_fan_gain_score_buckets: Vec<f64>,
    #[serde(default)]
    short_career_avg_fan_gain: Option<f64>,
    #[serde(default)]
    short_career_p95_fan_gain: Option<f64>,
    #[serde(default)]
    recent_fan_gain_3d: Option<i64>,
    #[serde(default)]
    peak_fans_per_minute: Option<f64>,
    #[serde(default)]
    distinct_weekly_hour_buckets: Option<i64>,
    #[serde(default)]
    last_seen: Option<String>,
    #[serde(default)]
    suspicion_score: Option<i64>,
    #[serde(default)]
    evidence: Option<ActivityEvidenceSummary>,
}

#[derive(Debug, Deserialize, Default)]
struct ActivityEvidenceSummary {
    #[serde(default)]
    verdict: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    reasons: Vec<ActivityEvidenceReason>,
}

#[derive(Debug, Deserialize, Default)]
struct ActivityEvidenceReason {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    severity: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ActivityViewerReport {
    #[serde(default)]
    score: Option<ActivityHallEntry>,
    #[serde(default)]
    daily: Vec<ActivityDailyPoint>,
    #[serde(default)]
    heatmap: Vec<ActivityHeatmapCell>,
    #[serde(default)]
    last_refreshed_at: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ActivityDailyPoint {
    #[serde(default)]
    day: Option<String>,
    #[serde(default)]
    fan_gain: Option<i64>,
    #[serde(default)]
    active_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct ActivityHeatmapCell {
    #[serde(default)]
    dow: Option<usize>,
    #[serde(default)]
    hour: Option<usize>,
    #[serde(default)]
    active_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CircleDetailsResponse {
    circle: CircleDetails,
    #[serde(default)]
    members: Vec<CircleMemberMonthlyData>,
    #[serde(default)]
    club_rank: Option<i64>,
    #[serde(
        default,
        alias = "minRank",
        alias = "minimum_rank",
        alias = "minimumRank",
        alias = "tier_min_rank",
        alias = "tierMinRank",
        alias = "cutoff_min_rank",
        alias = "cutoffMinRank",
        alias = "rank_min",
        alias = "rankMin",
        alias = "upper_cutoff_rank",
        alias = "upperCutoffRank",
        deserialize_with = "deserialize_optional_i64"
    )]
    min_rank: Option<i64>,
    #[serde(
        default,
        alias = "maxRank",
        alias = "maximum_rank",
        alias = "maximumRank",
        alias = "tier_max_rank",
        alias = "tierMaxRank",
        alias = "cutoff_max_rank",
        alias = "cutoffMaxRank",
        alias = "rank_max",
        alias = "rankMax",
        alias = "lower_cutoff_rank",
        alias = "lowerCutoffRank",
        deserialize_with = "deserialize_optional_i64"
    )]
    max_rank: Option<i64>,
    #[serde(default)]
    fans_to_next_tier: Option<i64>,
    #[serde(default)]
    fans_to_lower_tier: Option<i64>,
    #[serde(default)]
    yesterday_fans_to_next_tier: Option<i64>,
    #[serde(default)]
    yesterday_fans_to_lower_tier: Option<i64>,
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
    leader_viewer_id: Option<i64>,
    #[serde(default)]
    member_count: Option<i64>,
    #[serde(default)]
    members: Vec<CircleMemberMonthlyData>,
    #[serde(default)]
    join_style: Option<i64>,
    #[serde(default)]
    policy: Option<i64>,
    #[serde(default)]
    monthly_rank: Option<i64>,
    #[serde(default)]
    monthly_point: Option<i64>,
    #[serde(default)]
    yesterday_rank: Option<i64>,
    #[serde(default)]
    yesterday_points: Option<i64>,
    #[serde(default)]
    last_month_rank: Option<i64>,
    #[serde(default)]
    last_month_point: Option<i64>,
    #[serde(default)]
    live_rank: Option<i64>,
    #[serde(default)]
    live_points: Option<i64>,
    #[serde(default)]
    club_rank: Option<i64>,
    #[serde(
        default,
        alias = "minRank",
        alias = "minimum_rank",
        alias = "minimumRank",
        alias = "tier_min_rank",
        alias = "tierMinRank",
        alias = "cutoff_min_rank",
        alias = "cutoffMinRank",
        alias = "rank_min",
        alias = "rankMin",
        alias = "upper_cutoff_rank",
        alias = "upperCutoffRank",
        deserialize_with = "deserialize_optional_i64"
    )]
    min_rank: Option<i64>,
    #[serde(
        default,
        alias = "maxRank",
        alias = "maximum_rank",
        alias = "maximumRank",
        alias = "tier_max_rank",
        alias = "tierMaxRank",
        alias = "cutoff_max_rank",
        alias = "cutoffMaxRank",
        alias = "rank_max",
        alias = "rankMax",
        alias = "lower_cutoff_rank",
        alias = "lowerCutoffRank",
        deserialize_with = "deserialize_optional_i64"
    )]
    max_rank: Option<i64>,
    #[serde(default)]
    fans_to_next_tier: Option<i64>,
    #[serde(default)]
    fans_to_lower_tier: Option<i64>,
    #[serde(default)]
    yesterday_fans_to_next_tier: Option<i64>,
    #[serde(default)]
    yesterday_fans_to_lower_tier: Option<i64>,
    #[serde(default)]
    last_updated: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct CircleMemberMonthlyData {
    #[serde(default)]
    viewer_id: Option<i64>,
    #[serde(default)]
    trainer_name: Option<String>,
    #[serde(default)]
    year: Option<i64>,
    #[serde(default)]
    month: Option<i64>,
    #[serde(default)]
    daily_fans: Vec<i64>,
    #[serde(default)]
    next_month_start: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct CircleListResponse {
    #[serde(default)]
    circles: Vec<CircleDetails>,
    #[serde(default)]
    list: Vec<CircleDetails>,
    #[serde(default)]
    total: Option<Value>,
    #[serde(default)]
    total_count: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct DatabaseSearchResponse {
    #[serde(default)]
    items: Vec<DatabaseAccountRecord>,
    #[serde(default, deserialize_with = "deserialize_i64")]
    total: i64,
}

#[derive(Debug, Deserialize)]
struct DatabaseAccountRecord {
    #[serde(default)]
    account_id: String,
    #[serde(default)]
    trainer_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    follower_num: Option<i64>,
    #[serde(default)]
    last_updated: Option<String>,
    #[serde(default)]
    support_card: Option<DatabaseSupportCardRecord>,
    #[serde(default)]
    inheritance: Option<DatabaseInheritanceRecord>,
}

#[derive(Debug, Deserialize, Default)]
struct DatabaseSupportCardRecord {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    support_card_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    limit_break_count: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct DatabaseInheritanceRecord {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    inheritance_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    main_parent_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    parent_left_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    parent_right_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    parent_rank: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    parent_rarity: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    win_count: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    white_count: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    affinity_score: Option<i64>,
    #[serde(default)]
    blue_sparks: Vec<i64>,
    #[serde(default)]
    pink_sparks: Vec<i64>,
    #[serde(default)]
    green_sparks: Vec<i64>,
    #[serde(default)]
    white_sparks: Vec<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    main_blue_factors: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    main_pink_factors: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    main_green_factors: Option<i64>,
    #[serde(default)]
    main_white_factors: Vec<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    left_blue_factors: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    left_pink_factors: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    left_green_factors: Option<i64>,
    #[serde(default)]
    left_white_factors: Vec<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    right_blue_factors: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    right_pink_factors: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    right_green_factors: Option<i64>,
    #[serde(default)]
    right_white_factors: Vec<i64>,
    #[serde(default)]
    main_win_saddles: Vec<i64>,
    #[serde(default)]
    left_win_saddles: Vec<i64>,
    #[serde(default)]
    right_win_saddles: Vec<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    support_card_id: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    limit_break_count: Option<i64>,
    #[serde(default)]
    last_updated: Option<String>,
}

#[derive(Debug)]
struct DatabaseSearchPreview {
    total: i64,
    top_result: Option<DatabaseAccountRecord>,
}

struct DatabaseQueryHighlights {
    matched_factor_ids: Vec<i64>,
    matched_main_factor_ids: Vec<i64>,
    matched_support_card_id: Option<i64>,
    matched_min_limit_break: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default)]
struct ResourceAffinityBreakdown {
    main_total: i64,
    left_total: i64,
    right_total: i64,
    race_total: i64,
    relation_available: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ResourceCatalog {
    characters: BTreeMap<i64, ResourceCharacter>,
    factors: BTreeMap<i64, ResourceFactor>,
    skills: BTreeMap<i64, ResourceSkill>,
    support_cards: BTreeMap<i64, ResourceSupportCard>,
    affinity: Option<AffinityMatrix>,
    race_instance_saddles: BTreeMap<i64, Vec<i64>>,
    timeline: Option<TimelineEmbedDetails>,
}

impl ResourceCatalog {
    fn has_any_data(&self) -> bool {
        !self.characters.is_empty()
            || !self.factors.is_empty()
            || !self.skills.is_empty()
            || !self.support_cards.is_empty()
            || self.affinity.is_some()
            || !self.race_instance_saddles.is_empty()
            || self.timeline.is_some()
    }

    pub fn timeline(&self) -> Option<&TimelineEmbedDetails> {
        self.timeline.as_ref()
    }

    pub fn character_info(&self, card_id: i64) -> Option<(String, String)> {
        self.characters
            .get(&card_id)
            .map(|character| (character.name.clone(), character.image.clone()))
    }

    pub fn character_name(&self, card_id: i64) -> Option<&str> {
        self.characters
            .get(&card_id)
            .map(|character| character.name.as_str())
    }

    pub fn support_card_name(&self, support_card_id: i64) -> Option<&str> {
        self.support_cards
            .get(&support_card_id)
            .map(|support_card| support_card.name.as_str())
    }

    pub fn factor_info(&self, factor_id: i64) -> Option<(String, i64)> {
        if let Some(factor) = self.factors.get(&factor_id) {
            return Some((factor.text.clone(), factor.factor_type));
        }

        self.skills
            .get(&factor_id)
            .map(|skill| (skill.name.clone(), infer_resource_factor_type(factor_id)))
    }

    pub fn affinity2(&self, a: i64, b: i64) -> Option<i64> {
        self.affinity.as_ref()?.aff2(
            normalize_affinity_chara_id(a),
            normalize_affinity_chara_id(b),
        )
    }

    pub fn affinity3(&self, a: i64, b: i64, c: i64) -> Option<i64> {
        self.affinity.as_ref()?.aff3(
            normalize_affinity_chara_id(a),
            normalize_affinity_chara_id(b),
            normalize_affinity_chara_id(c),
        )
    }

    pub fn has_affinity(&self) -> bool {
        self.affinity.is_some()
    }

    fn saddle_ids_for_race_instance(&self, race_instance_id: i64) -> &[i64] {
        self.race_instance_saddles
            .get(&race_instance_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn affinity_breakdown_from_params(
        &self,
        main_parent_id: Option<i64>,
        parent_left_id: Option<i64>,
        parent_right_id: Option<i64>,
        main_win_saddles: &[i64],
        left_win_saddles: &[i64],
        right_win_saddles: &[i64],
    ) -> Option<ResourceAffinityBreakdown> {
        let left_race = shared_win_count(main_win_saddles, left_win_saddles);
        let right_race = shared_win_count(main_win_saddles, right_win_saddles);
        let race_total = left_race + right_race;

        let main = main_parent_id.map(normalize_affinity_chara_id);
        let left = parent_left_id.map(normalize_affinity_chara_id);
        let right = parent_right_id.map(normalize_affinity_chara_id);

        let mut left_base = 0;
        let mut right_base = 0;
        let mut main_base = 0;
        let mut relation_available = false;
        if let (Some(affinity), Some(main)) = (self.affinity.as_ref(), main) {
            relation_available = true;
            left_base = left
                .map(|left| affinity.aff2(main, left).unwrap_or_default())
                .unwrap_or_default();
            right_base = right
                .map(|right| affinity.aff2(main, right).unwrap_or_default())
                .unwrap_or_default();
            main_base = left_base + right_base;
        }

        let breakdown = ResourceAffinityBreakdown {
            main_total: main_base + race_total,
            left_total: left_base + left_race,
            right_total: right_base + right_race,
            race_total,
            relation_available,
        };

        if breakdown.main_total > 0 || breakdown.left_total > 0 || breakdown.right_total > 0 {
            Some(breakdown)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct ResourceCharacter {
    name: String,
    image: String,
}

#[derive(Clone, Debug)]
struct ResourceFactor {
    text: String,
    factor_type: i64,
}

#[derive(Clone, Debug)]
struct ResourceSkill {
    name: String,
}

#[derive(Clone, Debug)]
struct ResourceSupportCard {
    name: String,
}

#[derive(Clone, Debug)]
struct AffinityMatrix {
    chars: Vec<i64>,
    index: BTreeMap<i64, usize>,
    aff2: Vec<u8>,
    aff3: Vec<u8>,
}

impl AffinityMatrix {
    fn new(chars: Vec<i64>, aff2: Vec<u8>, aff3: Vec<u8>) -> Self {
        let index = chars
            .iter()
            .enumerate()
            .map(|(index, chara_id)| (*chara_id, index))
            .collect();

        Self {
            chars,
            index,
            aff2,
            aff3,
        }
    }

    fn aff2(&self, a: i64, b: i64) -> Option<i64> {
        let n = self.chars.len();
        let a = *self.index.get(&a)?;
        let b = *self.index.get(&b)?;
        self.aff2
            .get(a.checked_mul(n)?.checked_add(b)?)
            .map(|value| *value as i64)
    }

    fn aff3(&self, a: i64, b: i64, c: i64) -> Option<i64> {
        let n = self.chars.len();
        let a = *self.index.get(&a)?;
        let b = *self.index.get(&b)?;
        let c = *self.index.get(&c)?;
        let index = a
            .checked_mul(n)?
            .checked_mul(n)?
            .checked_add(b.checked_mul(n)?)?
            .checked_add(c)?;
        self.aff3.get(index).map(|value| *value as i64)
    }
}

#[derive(Debug, Deserialize)]
struct ResourceCharacterRaw {
    id: Value,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResourceFactorRaw {
    id: String,
    text: String,
    #[serde(rename = "type", default)]
    factor_type: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ResourceSkillRaw {
    #[serde(default)]
    skill_id: Option<Value>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResourceSupportCardRaw {
    id: Value,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ResourceAffinityRaw {
    #[serde(default)]
    chars: Vec<i64>,
    #[serde(default)]
    aff2: Vec<u8>,
    #[serde(default)]
    aff3: Vec<u8>,
}

#[derive(Debug, Deserialize, Default)]
struct ResourceRaceProgramRaw {
    #[serde(default)]
    races: BTreeMap<String, ResourceRaceProgramEntryRaw>,
}

#[derive(Debug, Deserialize)]
struct ResourceRaceProgramEntryRaw {
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    race_instance_id: Option<Value>,
}

#[cfg(test)]
#[derive(Debug, Deserialize, Default)]
struct BannerTimelineRaw {
    #[serde(default)]
    events: Vec<BannerTimelineEventRaw>,
}

#[cfg(test)]
#[derive(Debug, Deserialize, Default)]
struct BannerTimelineEventRaw {
    #[serde(default, rename = "type")]
    event_type: String,
    #[serde(default)]
    title: String,
    #[serde(
        default,
        alias = "content",
        alias = "detail",
        alias = "details",
        alias = "race_description",
        alias = "raceDescription",
        alias = "race_conditions",
        alias = "raceConditions",
        alias = "conditions"
    )]
    description: Option<String>,
    #[serde(
        default,
        alias = "image",
        alias = "imagePath",
        alias = "image_url",
        alias = "imageUrl",
        alias = "image_webp",
        alias = "imageWebp",
        alias = "webp",
        alias = "banner",
        alias = "banner_url",
        alias = "bannerUrl",
        alias = "banner_image",
        alias = "bannerImage",
        alias = "banner_image_path",
        alias = "bannerImagePath",
        alias = "story_banner",
        alias = "storyBanner",
        alias = "story_image",
        alias = "storyImage",
        alias = "event_image",
        alias = "eventImage",
        alias = "thumbnail",
        alias = "thumbnail_url",
        alias = "thumbnailUrl"
    )]
    image_path: Option<String>,
    #[serde(default)]
    global_release_date: Option<String>,
    #[serde(default)]
    estimated_end_date: Option<String>,
    #[serde(default)]
    is_confirmed: bool,
    #[serde(default)]
    pickup_card_ids: Vec<i64>,
    #[serde(default)]
    related_characters: Vec<String>,
    #[serde(default)]
    related_support_cards: Vec<String>,
    #[serde(default)]
    prediction: Option<BannerTimelinePredictionRaw>,
}

#[cfg(test)]
#[derive(Debug, Deserialize, Default)]
struct BannerTimelinePredictionRaw {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    calendar_likelihood: Option<BannerTimelineLikelihoodRaw>,
}

#[cfg(test)]
#[derive(Debug, Deserialize, Default)]
struct BannerTimelineLikelihoodRaw {
    #[serde(default)]
    score: Option<f64>,
}

#[derive(Clone, Debug)]
struct ResourceCacheEntry {
    base_url: String,
    token: Option<String>,
    catalog: ResourceCatalog,
}

static RESOURCE_CACHE: OnceLock<Mutex<Option<ResourceCacheEntry>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct BannerTimelineCacheEntry {
    base_url: String,
    token: Option<String>,
    details: Option<TimelineEmbedDetails>,
}

static BANNER_TIMELINE_CACHE: OnceLock<Mutex<Option<BannerTimelineCacheEntry>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct SiteStatsCacheEntry {
    api_base_url: String,
    fetched_at: Instant,
    stats: SiteStatsResponse,
}

static SITE_STATS_CACHE: OnceLock<Mutex<Option<SiteStatsCacheEntry>>> = OnceLock::new();
const SITE_STATS_CACHE_TTL: Duration = Duration::from_secs(60);

struct TierlistCacheEntry {
    asset_base_url: String,
    details: TierlistEmbedDetails,
}

static TIERLIST_CACHE: OnceLock<Mutex<Option<TierlistCacheEntry>>> = OnceLock::new();

#[derive(Debug, PartialEq, Eq)]
enum MetadataRoute {
    Home,
    Profile {
        account_id: String,
        subsection: String,
    },
    Clubs,
    Circle {
        circle_id: String,
    },
    Database,
    Timeline,
    Tierlist,
    Rankings,
    Activity,
    ActivityDetail {
        viewer_id: String,
    },
    Tools,
    Statistics,
    LineagePlanner,
    PrivacyPolicy,
    Generic {
        normalized_path: String,
    },
}

impl MetadataRoute {
    #[cfg(test)]
    fn kind_label(&self) -> &'static str {
        match self {
            MetadataRoute::Home => "Home",
            MetadataRoute::Profile { subsection, .. } => match subsection.as_str() {
                "veterans" => "Veterans",
                "cm" => "Career Menu",
                "achievements" => "Achievements",
                "titles" => "Titles",
                _ => "Profile",
            },
            MetadataRoute::Clubs => "Clubs",
            MetadataRoute::Circle { .. } => "Club",
            MetadataRoute::Database => "Database",
            MetadataRoute::Timeline => "Timeline",
            MetadataRoute::Tierlist => "Tierlist",
            MetadataRoute::Rankings => "Rankings",
            MetadataRoute::Activity | MetadataRoute::ActivityDetail { .. } => "Activity",
            MetadataRoute::Tools => "Tools",
            MetadataRoute::Statistics => "Statistics",
            MetadataRoute::LineagePlanner => "Lineage Planner",
            MetadataRoute::PrivacyPolicy => "Privacy Policy",
            MetadataRoute::Generic { .. } => "uma.moe",
        }
    }
}

fn metadata_route_for_path(path: &str) -> Option<MetadataRoute> {
    if should_never_embed(path) {
        return None;
    }

    let normalized_path = normalize_path(path);
    let segments = path_segments(&normalized_path);

    Some(match segments.as_slice() {
        [] => MetadataRoute::Home,
        ["profile", account_id] => MetadataRoute::Profile {
            account_id: (*account_id).to_string(),
            subsection: "/profile".to_string(),
        },
        ["profile", account_id, subsection] => MetadataRoute::Profile {
            account_id: (*account_id).to_string(),
            subsection: (*subsection).to_string(),
        },
        ["circles"] => MetadataRoute::Clubs,
        ["circles", circle_id] | ["circles", circle_id, _] => MetadataRoute::Circle {
            circle_id: (*circle_id).to_string(),
        },
        ["database"] | ["inheritance"] | ["support-cards"] => MetadataRoute::Database,
        ["timeline"] => MetadataRoute::Timeline,
        ["tierlist"] => MetadataRoute::Tierlist,
        ["rankings"] => MetadataRoute::Rankings,
        ["activity"] | ["shame"] => MetadataRoute::Activity,
        ["activity", viewer_id] | ["shame", viewer_id] => MetadataRoute::ActivityDetail {
            viewer_id: (*viewer_id).to_string(),
        },
        ["tools"] => MetadataRoute::Tools,
        ["tools", "statistics"] => MetadataRoute::Statistics,
        ["tools", "lineage-planner"] => MetadataRoute::LineagePlanner,
        ["privacy-policy"] => MetadataRoute::PrivacyPolicy,
        _ => MetadataRoute::Generic { normalized_path },
    })
}

pub async fn metadata_for_path(
    client: &Client,
    config: &Config,
    path: &str,
    query: Option<&str>,
) -> Option<EmbedMetadata> {
    match metadata_route_for_path(path)? {
        MetadataRoute::Home => Some(home_metadata(client, config).await),
        MetadataRoute::Profile {
            account_id,
            subsection,
        } => Some(profile_metadata(client, config, &account_id, &subsection).await),
        MetadataRoute::Clubs => Some(circles_metadata(client, config, query).await),
        MetadataRoute::Circle { circle_id } => {
            Some(circle_metadata(client, config, &circle_id).await)
        }
        MetadataRoute::Database => Some(database_metadata(client, config, query).await),
        MetadataRoute::Timeline => Some(timeline_metadata(client, config).await),
        MetadataRoute::Tierlist => Some(tierlist_metadata(client, config, query).await),
        MetadataRoute::Rankings => Some(rankings_metadata(client, config, query).await),
        MetadataRoute::Activity => Some(activity_metadata(client, config, query).await),
        MetadataRoute::ActivityDetail { viewer_id } => {
            Some(activity_detail_metadata(client, config, &viewer_id).await)
        }
        MetadataRoute::Tools => Some(tools_metadata(client, config).await),
        MetadataRoute::Statistics => Some(statistics_metadata(client, config).await),
        MetadataRoute::LineagePlanner => {
            Some(lineage_planner_metadata(client, config, query).await)
        }
        MetadataRoute::PrivacyPolicy => Some(page_metadata(
            config,
            "privacy-policy",
            "/privacy-policy",
            Some("Privacy Policy"),
        )),
        MetadataRoute::Generic { normalized_path } => {
            Some(generic_metadata(config, &normalized_path))
        }
    }
}

pub async fn metadata_for_image(
    client: &Client,
    config: &Config,
    kind: &str,
    raw_id: &str,
    query: Option<&str>,
) -> Option<EmbedMetadata> {
    let id = strip_png_suffix(raw_id);
    let id = urlencoding::decode(id).ok()?.into_owned();

    match kind {
        "profile" => Some(profile_metadata(client, config, &id, "/profile").await),
        "circle" => Some(circle_metadata(client, config, &id).await),
        "activity" => Some(activity_detail_metadata(client, config, &id).await),
        "database" => Some(database_metadata(client, config, query).await),
        "page" => Some(page_metadata_by_slug(client, config, &id, query).await),
        _ => None,
    }
}

pub async fn warm_static_caches(client: &Client, config: &Config) {
    let started_at = Instant::now();
    let (catalog, timeline, tierlist) = tokio::join!(
        fetch_resource_catalog(client, config),
        fetch_banner_timeline_details(client, config),
        fetch_tierlist_details(client, config),
    );

    info!(
        characters = catalog.characters.len(),
        factors = catalog.factors.len(),
        skills = catalog.skills.len(),
        support_cards = catalog.support_cards.len(),
        affinity_loaded = catalog.affinity.is_some(),
        race_instances = catalog.race_instance_saddles.len(),
        timeline_events = timeline
            .as_ref()
            .map(|details| details.events.len())
            .unwrap_or_default(),
        tierlist_rows = tierlist
            .as_ref()
            .map(|details| details.rows.len())
            .unwrap_or_default(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "warmed embed static metadata caches"
    );
}

pub fn render_embed_html(meta: &EmbedMetadata, redirect_humans: bool) -> String {
    let title = truncate_chars(&meta.title, 90);
    let description = truncate_chars(&meta.description, 200);
    let escaped_title = html_escape(&title);
    let escaped_description = html_escape(&description);
    let escaped_url = html_escape(&meta.canonical_url);
    let escaped_image = html_escape(&meta.image_url);
    let escaped_alt = html_escape(&meta.image_alt);
    let escaped_kind = html_escape(&meta.kind_label);
    let redirect_script = if redirect_humans {
        format!(
            "  <script>\n    window.location.replace({});\n  </script>\n",
            js_string_literal(&meta.canonical_url)
        )
    } else {
        String::new()
    };
    let class_list = embed_class_list(meta);
    let metrics = render_preview_metrics(&meta.metrics);

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
{redirect_script}
  <style>
    :root {{
      color-scheme: dark;
      font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: #0f1114;
      color: #f5f7fb;
    }}

    * {{
      box-sizing: border-box;
    }}

    body {{
      min-height: 100vh;
      margin: 0;
      display: grid;
      place-items: center;
      padding: 24px;
      background:
        radial-gradient(circle at 50% 0%, rgba(74, 168, 255, 0.16), transparent 34rem),
        linear-gradient(180deg, #12171a 0%, #090a0c 100%);
    }}

    main {{
      width: min(100%, 980px);
      display: grid;
      gap: 14px;
    }}

    .preview {{
      display: block;
      overflow: hidden;
      border: 1px solid rgba(74, 168, 255, 0.36);
      border-radius: 8px;
      background: #15181c;
      box-shadow: 0 18px 50px rgba(0, 0, 0, 0.34);
    }}

    img {{
      display: block;
      width: 100%;
      height: auto;
      aspect-ratio: 1200 / 630;
      background: #111418;
    }}

    .details {{
      display: grid;
      gap: 8px;
      padding: 16px 18px 18px;
      border-top: 1px solid rgba(255, 255, 255, 0.08);
    }}

    .kind {{
      width: fit-content;
      padding: 3px 8px;
      border-radius: 4px;
      background: rgba(74, 168, 255, 0.16);
      color: #6ec1ff;
      font-size: 11px;
      font-weight: 800;
      line-height: 1;
      text-transform: uppercase;
      letter-spacing: 0;
    }}

    h1 {{
      margin: 0;
      font-size: 21px;
      line-height: 1.2;
    }}

    p {{
      margin: 0;
      max-width: 68ch;
      color: #bec7d3;
      font-size: 14px;
      line-height: 1.45;
    }}

    .metrics {{
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      padding: 0;
      margin: 6px 0 0;
      list-style: none;
    }}

    .metrics li {{
      display: grid;
      gap: 3px;
      min-width: 96px;
      padding: 8px 10px;
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 6px;
      background: rgba(255, 255, 255, 0.04);
    }}

    .metrics span {{
      color: #8b97a5;
      font-size: 10px;
      font-weight: 800;
      text-transform: uppercase;
    }}

    .metrics strong {{
      color: #f5f7fb;
      font-size: 14px;
      line-height: 1.1;
    }}

    .open-link {{
      color: #6ec1ff;
      font-size: 13px;
      font-weight: 700;
      text-decoration: none;
    }}

    .open-link:hover {{
      color: #8dd0ff;
      text-decoration: underline;
    }}
  </style>
</head>
<body class="embed-redirect-page {class_list}">
  <main class="embed-preview-shell {class_list}">
    <a class="preview" href="{url}" aria-label="Open {title}">
      <img src="{image}" alt="{alt}" width="1200" height="630">
    </a>
    <section class="details" aria-label="Embed preview details">
      <span class="kind">{kind}</span>
      <h1>{title}</h1>
      <p>{description}</p>
      {metrics}
      <a class="open-link" href="{url}">Open on uma.moe</a>
    </section>
  </main>
</body>
</html>
"##,
        title = escaped_title,
        description = escaped_description,
        url = escaped_url,
        image = escaped_image,
        alt = escaped_alt,
        kind = escaped_kind,
        class_list = class_list,
        redirect_script = redirect_script,
        metrics = metrics,
    )
}

pub fn embed_class_list(meta: &EmbedMetadata) -> String {
    format!(
        "{} {} {}",
        embed_kind_class(meta),
        embed_type_class(meta),
        embed_route_class(meta)
    )
}

pub fn embed_kind_class(meta: &EmbedMetadata) -> String {
    let slug = if meta.database.is_some() {
        "database".to_string()
    } else {
        class_slug(&meta.kind_label)
    };

    format!("embed-kind-{slug}")
}

pub fn embed_type_class(meta: &EmbedMetadata) -> String {
    format!("embed-type-{}", embed_type_slug(meta))
}

pub fn embed_route_class(meta: &EmbedMetadata) -> String {
    format!("embed-route-{}", embed_route_slug(meta))
}

fn embed_type_slug(meta: &EmbedMetadata) -> &'static str {
    if meta.database.is_some() {
        return "database";
    }

    match canonical_path(&meta.canonical_url).as_str() {
        "/" => "home",
        path if path.starts_with("/profile/") => "profile",
        "/circles" => "clubs",
        path if path.starts_with("/circles/") => "club",
        "/database" | "/inheritance" | "/support-cards" => "database",
        "/timeline" => "timeline",
        "/tierlist" => "tierlist",
        "/rankings" => "rankings",
        "/activity" | "/shame" => "activity",
        path if path.starts_with("/activity/") || path.starts_with("/shame/") => "activity",
        "/tools" => "tools",
        "/tools/statistics" => "statistics",
        "/tools/lineage-planner" => "lineage-planner",
        "/privacy-policy" => "privacy-policy",
        _ => "page",
    }
}

fn embed_route_slug(meta: &EmbedMetadata) -> String {
    let path = canonical_path(&meta.canonical_url);
    match path.as_str() {
        "/" => "home".to_string(),
        path if path.starts_with("/profile/") => "profile".to_string(),
        "/circles" => "circles".to_string(),
        path if path.starts_with("/circles/") => "club".to_string(),
        "/database" => "database".to_string(),
        "/inheritance" => "inheritance".to_string(),
        "/support-cards" => "support-cards".to_string(),
        path => class_slug(path.trim_matches('/')),
    }
}

fn canonical_path(url: &str) -> String {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path_start = after_scheme.find('/').unwrap_or(after_scheme.len());
    let path = &after_scheme[path_start..];
    let path = path
        .split(['?', '#'])
        .next()
        .filter(|path| !path.is_empty())
        .unwrap_or("/");

    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn class_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "unknown".to_string()
    } else {
        slug
    }
}

fn render_preview_metrics(metrics: &[EmbedMetric]) -> String {
    if metrics.is_empty() {
        return String::new();
    }

    let items = metrics
        .iter()
        .take(6)
        .map(|metric| {
            format!(
                "<li><span>{}</span><strong>{}</strong></li>",
                html_escape(&metric.label),
                html_escape(&truncate_chars(&metric.value, 32))
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(r#"<ul class="metrics">{items}</ul>"#)
}

async fn profile_metadata(
    client: &Client,
    config: &Config,
    account_id: &str,
    subsection: &str,
) -> EmbedMetadata {
    let profile = fetch_profile(client, config, account_id).await;

    let section_label = match subsection {
        "veterans" => "Veterans",
        "cm" => "Career Menu",
        "achievements" => "Achievements",
        "titles" => "Titles",
        _ => "Profile",
    };

    let mut metrics = Vec::new();
    let mut profile_resources = ResourceCatalog::default();
    let (name, comment, circle, fans) = match profile {
        Some(profile) => {
            let trainer = profile.trainer;
            let profile_account_id = trainer.account_id.as_deref().unwrap_or(account_id);
            let name = trainer
                .name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| format!("Trainer {profile_account_id}"));
            let circle = profile.circle;
            let circle_id = circle.as_ref().and_then(|circle| circle.circle_id);
            let circle_details = match circle_id {
                Some(circle_id) => fetch_circle(client, config, &circle_id.to_string()).await,
                None => None,
            };
            let circle_history = profile_circle_history_latest(&profile.circle_history, circle_id);
            let circle_name = circle
                .as_ref()
                .and_then(|circle| circle.name.as_deref())
                .filter(|name| !name.trim().is_empty())
                .or_else(|| {
                    circle_details
                        .as_ref()
                        .and_then(|circle| circle.name.as_deref())
                        .filter(|name| !name.trim().is_empty())
                })
                .map(str::to_string);
            let circle_member_count = circle
                .as_ref()
                .and_then(|circle| circle.member_count)
                .or_else(|| {
                    circle_details
                        .as_ref()
                        .and_then(|circle| circle.member_count)
                });
            let circle_rank = circle
                .as_ref()
                .and_then(|circle| circle.monthly_rank.or(circle.live_rank))
                .or_else(|| {
                    circle_details
                        .as_ref()
                        .and_then(|circle| circle.monthly_rank.or(circle.live_rank))
                })
                .or(circle_history.rank);
            let circle_points = circle
                .as_ref()
                .and_then(|circle| {
                    circle
                        .live_points
                        .and_then(positive_i64)
                        .or_else(|| circle.monthly_point.and_then(positive_i64))
                })
                .or_else(|| {
                    circle_details.as_ref().and_then(|circle| {
                        circle
                            .live_points
                            .and_then(positive_i64)
                            .or_else(|| circle.monthly_point.and_then(positive_i64))
                    })
                })
                .or_else(|| circle_history.points.and_then(positive_i64));
            let circle_tier = circle
                .as_ref()
                .and_then(|circle| circle.club_rank)
                .or_else(|| circle_details.as_ref().and_then(|circle| circle.club_rank));
            let fan_history = profile.fan_history.as_ref();
            let alltime = fan_history.and_then(|history| history.alltime.as_ref());
            let rolling = fan_history.and_then(|history| history.rolling.as_ref());
            let fans = alltime.and_then(|alltime| alltime.total_fans);
            let rank = alltime.and_then(|alltime| alltime.rank_total_fans);

            metrics.push(metric("Trainer", &name));
            metrics.push(metric("Trainer ID", profile_account_id));
            metrics.push(metric("Section", section_label));
            if let Some(leader_chara_dress_id) = trainer.leader_chara_dress_id {
                metrics.push(metric(
                    "Leader Chara Dress",
                    &format!("#{leader_chara_dress_id}"),
                ));
            }

            if let Some(circle_id) = circle_id {
                metrics.push(metric("Club ID", &circle_id.to_string()));
            }
            if let Some(circle_name) = &circle_name {
                metrics.push(metric("Club", circle_name));
            }
            if let Some(member_count) = circle_member_count {
                metrics.push(metric("Club Members", &format_number(member_count)));
            }
            if let Some(monthly_rank) = circle_rank {
                metrics.push(metric("Club Rank", &format!("#{monthly_rank}")));
                metrics.push(metric("Club Monthly Rank", &format!("#{monthly_rank}")));
            }
            if let Some(points) = circle_points {
                metrics.push(metric("Club Fans", &compact_number(points)));
            }
            if let Some(club_rank) = circle_tier {
                metrics.push(metric("Club Tier", &club_rank_label(club_rank)));
                metrics.push(metric("Club Tier Id", &club_rank.to_string()));
            }
            if let Some(fans) = fans {
                metrics.push(metric("Fans", &compact_number(fans)));
            }
            if let Some(rank) = rank {
                metrics.push(metric("Fan Rank", &format!("#{rank}")));
            }
            if let Some(total_gain) = alltime.and_then(|alltime| alltime.total_gain) {
                metrics.push(metric("Total Gain", &signed_compact_number(total_gain)));
            }
            if let Some(active_days) = alltime.and_then(|alltime| alltime.active_days) {
                metrics.push(metric("Active Days", &format_number(active_days)));
            }
            if let Some(avg_day) = alltime.and_then(|alltime| alltime.avg_day) {
                metrics.push(metric("Avg Day", &compact_float(avg_day)));
            }
            if let Some(avg_week) = alltime.and_then(|alltime| alltime.avg_week) {
                metrics.push(metric("Avg Week", &compact_float(avg_week)));
            }
            if let Some(avg_month) = alltime.and_then(|alltime| alltime.avg_month) {
                metrics.push(metric("Avg Month", &compact_float(avg_month)));
            }
            if let Some(rolling) = rolling {
                if let Some(gain_3d) = rolling.gain_3d {
                    metrics.push(metric("3d Gain", &signed_compact_number(gain_3d)));
                }
                if let Some(gain_7d) = rolling.gain_7d {
                    metrics.push(metric("7d Gain", &signed_compact_number(gain_7d)));
                }
                if let Some(gain_30d) = rolling.gain_30d {
                    metrics.push(metric("30d Gain", &signed_compact_number(gain_30d)));
                }
                if let Some(rank_7d) = rolling.rank_7d {
                    metrics.push(metric("7d Rank", &format!("#{rank_7d}")));
                }
                if let Some(rank_30d) = rolling.rank_30d {
                    metrics.push(metric("30d Rank", &format!("#{rank_30d}")));
                }
            }
            if let Some(history) = fan_history {
                if !history.monthly.is_empty() {
                    metrics.push(metric("Fan Months", &history.monthly.len().to_string()));
                    push_profile_month_history_metrics(
                        &mut metrics,
                        &history.monthly,
                        circle_name.as_deref(),
                    );
                }
            }
            if let Some(followers) = trainer.follower_num {
                metrics.push(metric("Followers", &format_number(followers)));
            }
            if let Some(following) = trainer.own_follow_num {
                metrics.push(metric("Following", &format_number(following)));
            }
            if let Some(team_score) = trainer.team_evaluation_point.or(trainer.rank_score) {
                metrics.push(metric("Team", &compact_number(team_score)));
            }
            if let Some(team_class) = trainer.team_class {
                metrics.push(metric("Team Class", &format!("Class {team_class}")));
            }
            if let Some(best_team_class) = trainer.best_team_class {
                metrics.push(metric("Best Class", &format!("Class {best_team_class}")));
            }
            if let Some(inheritance) = profile.inheritance {
                let inheritance_resources = fetch_resource_catalog(client, config).await;
                let affinity_breakdown = inheritance_resources.affinity_breakdown_from_params(
                    inheritance.main_parent_id,
                    inheritance.parent_left_id,
                    inheritance.parent_right_id,
                    &inheritance.main_win_saddles,
                    &inheritance.left_win_saddles,
                    &inheritance.right_win_saddles,
                );
                let effective_affinity = inheritance.affinity_score.or_else(|| {
                    affinity_breakdown.and_then(|breakdown| positive_i64(breakdown.main_total))
                });
                let left_affinity_score =
                    affinity_breakdown.and_then(|breakdown| positive_i64(breakdown.left_total));
                let right_affinity_score =
                    affinity_breakdown.and_then(|breakdown| positive_i64(breakdown.right_total));
                profile_resources = inheritance_resources;
                let spark_sums = format!(
                    "B{} P{} G{} W{}",
                    inheritance.blue_stars_sum.unwrap_or(0),
                    inheritance.pink_stars_sum.unwrap_or(0),
                    inheritance.green_stars_sum.unwrap_or(0),
                    inheritance.white_stars_sum.unwrap_or(0)
                );
                metrics.push(metric(
                    "Inheritance",
                    &effective_affinity
                        .map(|score| format!("AFF {score}"))
                        .unwrap_or_else(|| "Available".to_string()),
                ));
                if let Some(affinity) = effective_affinity {
                    metrics.push(metric("Affinity", &format_number(affinity)));
                }
                push_optional_metric(&mut metrics, "Lineage Left Affinity", left_affinity_score);
                push_optional_metric(&mut metrics, "Lineage Right Affinity", right_affinity_score);
                metrics.push(metric("Spark Sums", &spark_sums));
                push_csv_metric(&mut metrics, "Blue Spark Ids", &inheritance.blue_sparks);
                push_csv_metric(&mut metrics, "Pink Spark Ids", &inheritance.pink_sparks);
                push_csv_metric(&mut metrics, "Green Spark Ids", &inheritance.green_sparks);
                push_csv_metric(&mut metrics, "White Spark Ids", &inheritance.white_sparks);
                if let Some(blue_total) = inheritance
                    .blue_stars_sum
                    .or_else(|| count_as_i64(&inheritance.blue_sparks))
                {
                    metrics.push(metric("Blue Sparks", &format!("B{blue_total}")));
                }
                if let Some(pink_total) = inheritance
                    .pink_stars_sum
                    .or_else(|| count_as_i64(&inheritance.pink_sparks))
                {
                    metrics.push(metric("Pink Sparks", &format!("P{pink_total}")));
                }
                if let Some(main_parent_id) = inheritance.main_parent_id {
                    metrics.push(metric("Lineage Main", &format!("#{main_parent_id}")));
                }
                if let Some(parent_left_id) = inheritance.parent_left_id {
                    metrics.push(metric("Lineage Left", &format!("#{parent_left_id}")));
                }
                if let Some(parent_right_id) = inheritance.parent_right_id {
                    metrics.push(metric("Lineage Right", &format!("#{parent_right_id}")));
                }
                if let (Some(main), Some(left), Some(right)) = (
                    inheritance.main_parent_id,
                    inheritance.parent_left_id,
                    inheritance.parent_right_id,
                ) {
                    metrics.push(metric("Lineage", &format!("#{main} / #{left} + #{right}")));
                }
                push_optional_metric(
                    &mut metrics,
                    "Lineage Main Blue",
                    inheritance.main_blue_factors,
                );
                push_optional_metric(
                    &mut metrics,
                    "Lineage Main Pink",
                    inheritance.main_pink_factors,
                );
                push_optional_metric(
                    &mut metrics,
                    "Lineage Main Green",
                    inheritance.main_green_factors,
                );
                push_csv_metric(
                    &mut metrics,
                    "Lineage Main White",
                    &inheritance.main_white_factors,
                );
                push_optional_metric(
                    &mut metrics,
                    "Lineage Left Blue",
                    inheritance.left_blue_factors,
                );
                push_optional_metric(
                    &mut metrics,
                    "Lineage Left Pink",
                    inheritance.left_pink_factors,
                );
                push_optional_metric(
                    &mut metrics,
                    "Lineage Left Green",
                    inheritance.left_green_factors,
                );
                push_csv_metric(
                    &mut metrics,
                    "Lineage Left White",
                    &inheritance.left_white_factors,
                );
                push_optional_metric(
                    &mut metrics,
                    "Lineage Right Blue",
                    inheritance.right_blue_factors,
                );
                push_optional_metric(
                    &mut metrics,
                    "Lineage Right Pink",
                    inheritance.right_pink_factors,
                );
                push_optional_metric(
                    &mut metrics,
                    "Lineage Right Green",
                    inheritance.right_green_factors,
                );
                push_csv_metric(
                    &mut metrics,
                    "Lineage Right White",
                    &inheritance.right_white_factors,
                );
                push_csv_metric(
                    &mut metrics,
                    "Lineage Main Wins",
                    &inheritance.main_win_saddles,
                );
                push_csv_metric(
                    &mut metrics,
                    "Lineage Left Wins",
                    &inheritance.left_win_saddles,
                );
                push_csv_metric(
                    &mut metrics,
                    "Lineage Right Wins",
                    &inheritance.right_win_saddles,
                );
            }
            if let Some(support_card) = profile.support_card {
                let support_label = match (
                    support_card.support_card_id,
                    support_card.limit_break_count,
                    support_card.experience,
                ) {
                    (Some(card_id), Some(limit_break), _) => {
                        format!("#{card_id} LB{limit_break}")
                    }
                    (Some(card_id), None, _) => format!("#{card_id}"),
                    (None, Some(limit_break), _) => format!("LB{limit_break} support"),
                    (None, None, Some(experience)) => format!("EXP {experience}"),
                    _ => "Support card".to_string(),
                };
                if let Some(card_id) = support_card.support_card_id {
                    metrics.push(metric("Support Card", &format!("#{card_id}")));
                }
                if let Some(limit_break) = support_card.limit_break_count {
                    metrics.push(metric("Support LB", &format!("LB{limit_break}")));
                }
                if let Some(experience) = support_card.experience {
                    metrics.push(metric("Support EXP", &format_number(experience)));
                }
                metrics.push(metric("Support", &support_label));
            }
            if !profile.team_stadium.is_empty() {
                metrics.push(metric(
                    "Stadium",
                    &format!("{} Umas", profile.team_stadium.len()),
                ));
                let mut distances = profile
                    .team_stadium
                    .iter()
                    .filter_map(|member| member.distance_type)
                    .collect::<Vec<_>>();
                distances.sort_unstable();
                distances.dedup();
                if !distances.is_empty() {
                    metrics.push(metric(
                        "Stadium Distances",
                        &format!("{} types", distances.len()),
                    ));
                }
                if let Some(best_score) = profile
                    .team_stadium
                    .iter()
                    .filter_map(|member| member.rank_score)
                    .max()
                {
                    metrics.push(metric("Best Uma", &compact_number(best_score)));
                }
                for (index, member) in profile.team_stadium.iter().enumerate() {
                    let number = index + 1;
                    if let Some(character_id) = member.stadium_character_asset_id() {
                        metrics.push(metric(
                            &format!("Stadium Member {number} Character"),
                            &format!("#{character_id}"),
                        ));
                    }
                    if let Some(distance_type) = member.distance_type {
                        metrics.push(metric(
                            &format!("Stadium Member {number} Distance"),
                            &distance_type.to_string(),
                        ));
                    }
                    if let Some(rank_score) = member.rank_score {
                        metrics.push(metric(
                            &format!("Stadium Member {number} Score"),
                            &compact_number(rank_score),
                        ));
                    }
                    if let Some(running_style) = member.running_style {
                        metrics.push(metric(
                            &format!("Stadium Member {number} Running Style"),
                            &running_style.to_string(),
                        ));
                    }
                }
            }
            metrics.push(metric("Asset Base", &config.asset_base_url));

            (name, trainer.comment, circle_name, fans)
        }
        None => {
            let name = format!("Trainer {account_id}");
            metrics.push(metric("Trainer", &name));
            metrics.push(metric("Trainer ID", account_id));
            metrics.push(metric("Section", section_label));
            metrics.push(metric("Visibility", "Hidden"));
            (name, None, None, None)
        }
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

    EmbedMetadata {
        title: format!("{name} | uma.moe"),
        description: description_parts.join(" "),
        canonical_url: absolute_url(config, &format!("/profile/{account_id}")),
        image_url: image_url(config, "profile", account_id),
        image_alt: format!("uma.moe profile preview for {name}"),
        kind_label: section_label.to_string(),
        metrics,
        database: None,
        tierlist: None,
        resources: profile_resources,
    }
}

fn push_profile_month_history_metrics(
    metrics: &mut Vec<EmbedMetric>,
    monthly: &[Value],
    fallback_circle: Option<&str>,
) {
    let mut rows = monthly
        .iter()
        .enumerate()
        .filter_map(|(index, value)| profile_fan_month_metric(index, value, fallback_circle))
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| match (left.sort_key, right.sort_key) {
        (Some(left_key), Some(right_key)) => right_key.cmp(&left_key),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.original_index.cmp(&right.original_index),
    });

    for (index, row) in rows.into_iter().take(4).enumerate() {
        let slot = index + 1;
        metrics.push(metric(&format!("Fan Month {slot}"), &row.period));
        metrics.push(metric(&format!("Fan Month {slot} Fans"), &row.fans));
        metrics.push(metric(&format!("Fan Month {slot} Gain"), &row.gain));
        metrics.push(metric(&format!("Fan Month {slot} Days"), &row.days));
        metrics.push(metric(&format!("Fan Month {slot} Avg Day"), &row.avg_day));
        metrics.push(metric(&format!("Fan Month {slot} Rank"), &row.rank));
        metrics.push(metric(&format!("Fan Month {slot} Circle"), &row.circle));
    }
}

fn count_as_i64(values: &[i64]) -> Option<i64> {
    if values.is_empty() {
        None
    } else {
        i64::try_from(values.len()).ok()
    }
}

fn push_optional_metric(metrics: &mut Vec<EmbedMetric>, label: &str, value: Option<i64>) {
    if let Some(value) = value {
        metrics.push(metric(label, &value.to_string()));
    }
}

fn push_csv_metric(metrics: &mut Vec<EmbedMetric>, label: &str, values: &[i64]) {
    if !values.is_empty() {
        metrics.push(metric(label, &csv_numbers(values)));
    }
}

fn profile_circle_history_latest(
    rows: &[Value],
    circle_id: Option<i64>,
) -> ProfileCircleHistoryMetric {
    rows.iter()
        .filter(|value| match circle_id {
            Some(circle_id) => {
                profile_month_i64(value, &["circle_id", "circleId"]) == Some(circle_id)
            }
            None => true,
        })
        .filter_map(|value| {
            let sort_key = profile_month_sort_key(value);
            let rank = profile_month_i64(value, &["circle_rank", "circleRank", "rank"]);
            let points = profile_month_i64(value, &["circle_points", "circlePoints", "points"]);
            if rank.is_none() && points.is_none() {
                None
            } else {
                Some((sort_key, ProfileCircleHistoryMetric { rank, points }))
            }
        })
        .max_by(
            |(left_key, _), (right_key, _)| match (left_key, right_key) {
                (Some(left), Some(right)) => left.cmp(right),
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => std::cmp::Ordering::Equal,
            },
        )
        .map(|(_, metric)| metric)
        .unwrap_or_default()
}

fn profile_fan_month_metric(
    original_index: usize,
    value: &Value,
    fallback_circle: Option<&str>,
) -> Option<ProfileFanMonthMetric> {
    let sort_key = profile_month_sort_key(value);
    let period = profile_month_period(value, sort_key, original_index);
    let fans = profile_month_i64(
        value,
        &[
            "total_fans",
            "totalFans",
            "fans",
            "fan_count",
            "fanCount",
            "total",
        ],
    )
    .map(compact_number)
    .unwrap_or_else(|| "-".to_string());
    let gain = profile_month_i64(
        value,
        &[
            "monthly_gain",
            "monthlyGain",
            "gain",
            "fan_gain",
            "fanGain",
            "total_gain",
            "totalGain",
        ],
    )
    .map(signed_compact_number)
    .unwrap_or_else(|| "-".to_string());
    let days = profile_month_i64(
        value,
        &[
            "active_days",
            "activeDays",
            "days",
            "active_day",
            "activeDay",
        ],
    )
    .map(format_number)
    .unwrap_or_else(|| "-".to_string());
    let avg_day = profile_month_f64(
        value,
        &[
            "avg_daily",
            "avgDaily",
            "avg_day",
            "avgDay",
            "average_daily",
            "averageDaily",
        ],
    )
    .map(|value| format_optional_rate(Some(value)))
    .unwrap_or_else(|| "-".to_string());
    let rank = profile_month_i64(
        value,
        &[
            "rank",
            "monthly_rank",
            "monthlyRank",
            "rank_total_fans",
            "rankTotalFans",
        ],
    )
    .map(|rank| format!("#{rank}"))
    .or_else(|| profile_month_string(value, &["rank"]).map(str::to_string))
    .unwrap_or_else(|| "-".to_string());
    let circle = profile_month_string(
        value,
        &[
            "circle_name",
            "circleName",
            "circle",
            "club",
            "club_name",
            "clubName",
        ],
    )
    .or(fallback_circle)
    .unwrap_or("-")
    .to_string();

    if fans == "-" && gain == "-" && days == "-" && avg_day == "-" && rank == "-" {
        return None;
    }

    Some(ProfileFanMonthMetric {
        original_index,
        sort_key,
        period,
        fans,
        gain,
        days,
        avg_day,
        rank,
        circle,
    })
}

fn profile_month_period(
    value: &Value,
    sort_key: Option<(i64, i64)>,
    original_index: usize,
) -> String {
    if let Some(period) = profile_month_string(
        value,
        &[
            "period",
            "month_label",
            "monthLabel",
            "label",
            "display_month",
            "displayMonth",
        ],
    ) {
        return truncate_chars(period, 22);
    }

    if let Some((year, month)) = sort_key {
        return format!("{} {year}", month_label(month));
    }

    format!("Month {}", original_index + 1)
}

fn profile_month_sort_key(value: &Value) -> Option<(i64, i64)> {
    let year = profile_month_i64(value, &["year", "ranking_year", "rankingYear"]);
    let month = profile_month_i64(value, &["month", "ranking_month", "rankingMonth"]);
    match (year, month) {
        (Some(year), Some(month)) if (1..=12).contains(&month) => return Some((year, month)),
        _ => {}
    }

    profile_month_string(
        value,
        &[
            "date",
            "month_start",
            "monthStart",
            "period_start",
            "periodStart",
            "created_at",
            "createdAt",
        ],
    )
    .and_then(parse_year_month)
}

fn profile_month_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .find_map(value_as_i64)
}

fn profile_month_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .find_map(value_as_f64)
}

fn profile_month_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
}

fn parse_year_month(value: &str) -> Option<(i64, i64)> {
    let value = value.trim();
    if value.len() < 7 || value.as_bytes().get(4) != Some(&b'-') {
        return None;
    }

    let year = value.get(0..4)?.parse::<i64>().ok()?;
    let month = value.get(5..7)?.parse::<i64>().ok()?;
    if (1..=12).contains(&month) {
        Some((year, month))
    } else {
        None
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

    let circle_comment = circle
        .comment
        .clone()
        .map(|comment| comment.trim().to_string())
        .filter(|comment| !comment.is_empty());

    let mut description_parts = vec![format!(
        "Club profile for {name}, including ranking, member activity, and fan progress."
    )];
    if let Some(comment) = &circle_comment {
        description_parts.push(comment.clone());
    }

    let mut metrics = vec![EmbedMetric {
        label: "Club ID".to_string(),
        value: circle
            .circle_id
            .map_or_else(|| circle_id.to_string(), |id| id.to_string()),
    }];
    let displayed_rank = circle.monthly_rank.or(circle.live_rank);
    let displayed_points = circle.monthly_point.or(circle.live_points);

    if let Some(rank) = displayed_rank {
        metrics.push(EmbedMetric {
            label: "Rank".to_string(),
            value: format!("#{rank}"),
        });
    }
    if let Some(points) = displayed_points {
        metrics.push(EmbedMetric {
            label: "Points".to_string(),
            value: format_number(points),
        });
    }
    if let Some(live_points) = circle.live_points {
        metrics.push(EmbedMetric {
            label: "Live Points".to_string(),
            value: format_number(live_points),
        });
    }
    if let Some(yesterday_points) = circle.yesterday_points {
        metrics.push(EmbedMetric {
            label: "Yesterday Points".to_string(),
            value: format_number(yesterday_points),
        });
    }
    let today_points = circle.live_points.or(displayed_points);
    if let (Some(points), Some(yesterday_points)) = (today_points, circle.yesterday_points) {
        metrics.push(EmbedMetric {
            label: "Today Gain".to_string(),
            value: signed_compact_number(points - yesterday_points),
        });
    }
    if let Some(yesterday_rank) = circle.yesterday_rank {
        metrics.push(EmbedMetric {
            label: "Yesterday Rank".to_string(),
            value: format!("#{yesterday_rank}"),
        });
    }
    if let Some(last_month_point) = circle.last_month_point {
        metrics.push(EmbedMetric {
            label: "Last Month Points".to_string(),
            value: format_number(last_month_point),
        });
    }
    if let Some(last_month_rank) = circle.last_month_rank {
        metrics.push(EmbedMetric {
            label: "Last Month Rank".to_string(),
            value: format!("#{last_month_rank}"),
        });
    }
    if let Some(members) = circle.member_count {
        metrics.push(EmbedMetric {
            label: "Members".to_string(),
            value: format_number(members),
        });
    }
    if let Some(join_style) = circle.join_style {
        metrics.push(EmbedMetric {
            label: "Join".to_string(),
            value: join_style_label(join_style).to_string(),
        });
    }
    if let Some(policy) = circle.policy {
        metrics.push(EmbedMetric {
            label: "Policy".to_string(),
            value: policy_label(policy).to_string(),
        });
    }
    if let Some(comment) = circle_comment {
        metrics.push(EmbedMetric {
            label: "Comment".to_string(),
            value: comment,
        });
    }
    if let Some(club_rank) = circle.club_rank {
        metrics.push(EmbedMetric {
            label: "Club Rank".to_string(),
            value: club_rank_label(club_rank).to_string(),
        });
        metrics.push(EmbedMetric {
            label: "Club Rank Id".to_string(),
            value: club_rank.to_string(),
        });
    }
    if let Some(max_rank) = circle.max_rank {
        metrics.push(EmbedMetric {
            label: "Lower Cutoff Rank".to_string(),
            value: format!("#{max_rank}"),
        });
    }
    if let Some(min_rank) = circle.min_rank {
        metrics.push(EmbedMetric {
            label: "Upper Cutoff Rank".to_string(),
            value: format!("#{min_rank}"),
        });
    }
    if let Some(needed) = circle.fans_to_next_tier {
        metrics.push(EmbedMetric {
            label: "Needed".to_string(),
            value: format_number(needed),
        });
    }
    if let (Some(current), Some(previous)) =
        (circle.fans_to_next_tier, circle.yesterday_fans_to_next_tier)
    {
        metrics.push(EmbedMetric {
            label: "Needed Delta".to_string(),
            value: signed_format_number(current - previous),
        });
    }
    if let Some(buffer) = circle.fans_to_lower_tier {
        metrics.push(EmbedMetric {
            label: "Buffer".to_string(),
            value: format_number(buffer),
        });
    }
    if let (Some(current), Some(previous)) = (
        circle.fans_to_lower_tier,
        circle.yesterday_fans_to_lower_tier,
    ) {
        metrics.push(EmbedMetric {
            label: "Buffer Delta".to_string(),
            value: signed_format_number(current - previous),
        });
    }
    if let Some(leader) = circle
        .leader_name
        .or_else(|| circle.leader_viewer_id.map(|id| id.to_string()))
    {
        metrics.push(EmbedMetric {
            label: "Leader".to_string(),
            value: leader,
        });
    }
    push_circle_member_gain_metrics(&mut metrics, &circle.members);
    metrics.push(metric("Asset Base", &config.asset_base_url));

    EmbedMetadata {
        title: format!("{name} | uma.moe"),
        description: description_parts.join(" "),
        canonical_url: absolute_url(config, &format!("/circles/{circle_id}")),
        image_url: image_url(config, "circle", circle_id),
        image_alt: format!("uma.moe club preview for {name}"),
        kind_label: "Club".to_string(),
        metrics,
        database: None,
        tierlist: None,
        resources: ResourceCatalog::default(),
    }
}

async fn circles_metadata(client: &Client, config: &Config, query: Option<&str>) -> EmbedMetadata {
    let clean_query = clean_query(query, &config.debug_query_key);
    let query_pairs = query_pairs(clean_query.as_deref());
    let metrics = fetch_circle_list(client, config, &query_pairs)
        .await
        .map(|response| circle_list_metrics(response, &query_pairs, config))
        .unwrap_or_else(|| fallback_circle_list_metrics(config));

    EmbedMetadata {
        title: "Club Leaderboard | uma.moe".to_string(),
        description: "Find and compare Umamusume clubs by rank, monthly points, tier gaps, members, and recruitment.".to_string(),
        canonical_url: absolute_url_with_query(config, "/circles", clean_query.as_deref()),
        image_url: image_url_with_query(config, "page", "circles", clean_query.as_deref()),
        image_alt: "uma.moe club leaderboard preview image".to_string(),
        kind_label: "Clubs".to_string(),
        metrics,
        database: None,
        tierlist: None,
        resources: ResourceCatalog::default(),
    }
}

async fn database_metadata(client: &Client, config: &Config, query: Option<&str>) -> EmbedMetadata {
    let Some(query) = clean_query(query, &config.debug_query_key) else {
        return page_metadata(config, "database", "/database", Some("Database"));
    };

    let resources = fetch_resource_catalog(client, config).await;
    let search_params = database_search_params_from_query_with_resources(&query, Some(&resources));
    if !has_meaningful_database_search_params(&search_params) {
        return page_metadata(config, "database", "/database", Some("Database"));
    }

    let preview = fetch_database_preview(client, config, &search_params).await;
    let canonical_url = absolute_url_with_query(config, "/database", Some(&query));
    let image_url = image_url_with_query(config, "database", "query", Some(&query));
    let query_label = database_query_label(&search_params, &resources);

    match preview.and_then(|preview| {
        preview
            .top_result
            .map(|top_result| (preview.total, top_result))
    }) {
        Some((total, result)) => database_result_metadata(
            config,
            result,
            total,
            query_label,
            &search_params,
            resources,
            canonical_url,
            image_url,
        ),
        None => EmbedMetadata {
            title: "Database search | uma.moe".to_string(),
            description: format!(
                "Shared Umamusume inheritance database search for {query_label}. No matching records were found yet."
            ),
            canonical_url,
            image_url,
            image_alt: "uma.moe database search preview".to_string(),
            kind_label: "Database".to_string(),
            metrics: vec![metric("Results", "0"), metric("Search", &query_label)],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        },
    }
}

fn database_result_metadata(
    config: &Config,
    result: DatabaseAccountRecord,
    total: i64,
    query_label: String,
    search_params: &[(String, String)],
    resources: ResourceCatalog,
    canonical_url: String,
    image_url: String,
) -> EmbedMetadata {
    let DatabaseAccountRecord {
        account_id,
        trainer_name,
        follower_num,
        last_updated,
        support_card,
        inheritance,
    } = result;

    let inheritance = inheritance.unwrap_or_default();
    let support_card = support_card.unwrap_or_default();
    let affinity_breakdown = resources.affinity_breakdown_from_params(
        inheritance.main_parent_id,
        inheritance.parent_left_id,
        inheritance.parent_right_id,
        &inheritance.main_win_saddles,
        &inheritance.left_win_saddles,
        &inheritance.right_win_saddles,
    );
    let computed_affinity = affinity_breakdown.and_then(|breakdown| {
        if breakdown.relation_available {
            positive_i64(breakdown.main_total)
        } else {
            None
        }
    });
    let backend_with_race_affinity = inheritance.affinity_score.and_then(|affinity| {
        affinity_breakdown
            .and_then(|breakdown| positive_i64(breakdown.race_total))
            .map(|race_affinity| affinity + race_affinity)
    });
    let effective_affinity = computed_affinity
        .or(backend_with_race_affinity)
        .or(inheritance.affinity_score)
        .or_else(|| affinity_breakdown.and_then(|breakdown| positive_i64(breakdown.main_total)));
    let left_affinity_score =
        affinity_breakdown.and_then(|breakdown| positive_i64(breakdown.left_total));
    let right_affinity_score =
        affinity_breakdown.and_then(|breakdown| positive_i64(breakdown.right_total));

    let trainer_name = trainer_name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| account_id.clone());
    let trainer_id = if account_id.is_empty() {
        "unknown".to_string()
    } else {
        account_id
    };

    let mut stat_parts = Vec::new();
    if let Some(affinity) = effective_affinity {
        stat_parts.push(format!("{affinity} affinity"));
    }
    if let Some(win_count) = inheritance.win_count {
        stat_parts.push(format!("{} G1 wins", format_number(win_count)));
    }
    if let Some(white_count) = inheritance.white_count {
        stat_parts.push(format!("{} white skills", format_number(white_count)));
    }

    let mut detail_parts = vec![
        format!("Trainer ID: {trainer_id}."),
        database_result_position_label(total),
    ];
    if !stat_parts.is_empty() {
        detail_parts.push(format!("{}.", stat_parts.join(", ")));
    }
    if let Some(main_parent_id) = inheritance.main_parent_id {
        detail_parts.push(format!(
            "Main: {}.",
            database_character_embed_label(&resources, main_parent_id)
        ));
    }
    if let (Some(left), Some(right)) = (inheritance.parent_left_id, inheritance.parent_right_id) {
        detail_parts.push(format!(
            "Parents: {} / {}.",
            database_character_embed_label(&resources, left),
            database_character_embed_label(&resources, right)
        ));
    }

    let mut metrics = vec![
        metric("Results", &compact_number(total)),
        metric("Trainer", &truncate_chars(&trainer_name, 18)),
    ];
    if let Some(inheritance_id) = inheritance.inheritance_id {
        metrics.push(metric("Record", &inheritance_id.to_string()));
    }
    if let Some(affinity) = effective_affinity {
        metrics.push(metric("Affinity", &format_number(affinity)));
    }
    if let Some(parent_rank) = inheritance.parent_rank {
        metrics.push(metric("Rank", &format_number(parent_rank)));
    }
    if let Some(win_count) = inheritance.win_count {
        metrics.push(metric("G1 Wins", &format_number(win_count)));
    }
    if let Some(white_count) = inheritance.white_count {
        metrics.push(metric("White", &format_number(white_count)));
    }
    if let Some(followers) = follower_num {
        metrics.push(metric("Followers", &format_number(followers)));
    }

    let title = if trainer_name == trainer_id {
        format!("{trainer_id} | uma.moe")
    } else {
        format!("{trainer_name} | {trainer_id} | uma.moe")
    };
    let highlights = database_query_highlights(search_params);

    EmbedMetadata {
        title,
        description: detail_parts.join(" "),
        canonical_url,
        image_url,
        image_alt: format!("uma.moe database search preview for {trainer_name}"),
        kind_label: "Database".to_string(),
        metrics,
        database: Some(DatabaseEmbedDetails {
            asset_base_url: config.asset_base_url.clone(),
            resources,
            query_label,
            result_total: total,
            matched_factor_ids: highlights.matched_factor_ids,
            matched_main_factor_ids: highlights.matched_main_factor_ids,
            matched_support_card_id: highlights.matched_support_card_id,
            matched_min_limit_break: highlights.matched_min_limit_break,
            trainer_name,
            trainer_id,
            record_id: inheritance.inheritance_id,
            main_parent_id: inheritance.main_parent_id,
            parent_left_id: inheritance.parent_left_id,
            parent_right_id: inheritance.parent_right_id,
            parent_rank: inheritance.parent_rank,
            parent_rarity: inheritance.parent_rarity,
            affinity_score: effective_affinity,
            left_affinity_score,
            right_affinity_score,
            win_count: inheritance.win_count,
            white_count: inheritance.white_count,
            follower_num,
            support_card_id: inheritance.support_card_id.or(support_card.support_card_id),
            limit_break_count: inheritance
                .limit_break_count
                .or(support_card.limit_break_count),
            last_updated: inheritance.last_updated.or(last_updated),
            blue_sparks: inheritance.blue_sparks,
            pink_sparks: inheritance.pink_sparks,
            green_sparks: inheritance.green_sparks,
            white_sparks: inheritance.white_sparks,
            main_blue_factors: inheritance.main_blue_factors,
            main_pink_factors: inheritance.main_pink_factors,
            main_green_factors: inheritance.main_green_factors,
            main_white_factors: inheritance.main_white_factors,
            left_blue_factors: inheritance.left_blue_factors,
            left_pink_factors: inheritance.left_pink_factors,
            left_green_factors: inheritance.left_green_factors,
            left_white_factors: inheritance.left_white_factors,
            right_blue_factors: inheritance.right_blue_factors,
            right_pink_factors: inheritance.right_pink_factors,
            right_green_factors: inheritance.right_green_factors,
            right_white_factors: inheritance.right_white_factors,
            main_win_saddles: inheritance.main_win_saddles,
            left_win_saddles: inheritance.left_win_saddles,
            right_win_saddles: inheritance.right_win_saddles,
        }),
        tierlist: None,
        resources: ResourceCatalog::default(),
    }
}

async fn page_metadata_by_slug(
    client: &Client,
    config: &Config,
    slug: &str,
    query: Option<&str>,
) -> EmbedMetadata {
    match slug {
        "home" => home_metadata(client, config).await,
        "database" => page_metadata(config, "database", "/database", Some("Database")),
        "timeline" => timeline_metadata(client, config).await,
        "tierlist" => tierlist_metadata(client, config, query).await,
        "rankings" => rankings_metadata(client, config, query).await,
        "activity" => activity_metadata(client, config, query).await,
        "circles" => circles_metadata(client, config, query).await,
        "tools" => tools_metadata(client, config).await,
        "statistics" => statistics_metadata(client, config).await,
        "lineage-planner" => lineage_planner_metadata(client, config, query).await,
        _ => generic_metadata(config, "/"),
    }
}

#[derive(Debug, Deserialize, Default)]
struct PrecomputedTierlistResponse {
    #[serde(default)]
    metadata: PrecomputedTierlistMetadata,
    #[serde(default)]
    cards: BTreeMap<String, PrecomputedTierlistCard>,
}

#[derive(Debug, Deserialize, Default)]
struct PrecomputedTierlistMetadata {
    #[serde(default, rename = "generatedAt")]
    generated_at: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PrecomputedTierlistCard {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    card_type: i64,
    #[serde(default)]
    rarity: i64,
    #[serde(default)]
    scores: Vec<i64>,
    #[serde(default)]
    tiers: Vec<String>,
}

async fn timeline_metadata(client: &Client, config: &Config) -> EmbedMetadata {
    let mut meta = page_metadata(config, "timeline", "/timeline", Some("Timeline"));
    meta.resources.timeline = fetch_banner_timeline_details(client, config).await;
    if let Some(timeline) = meta.resources.timeline() {
        let upcoming_count = timeline.events.len();
        upsert_metric(&mut meta.metrics, "Events", &upcoming_count.to_string());
        upsert_metric(&mut meta.metrics, "Source", "banner_timeline");
    }
    meta
}

async fn tierlist_metadata(client: &Client, config: &Config, query: Option<&str>) -> EmbedMetadata {
    let mut meta = tierlist_page_metadata(config, query);
    meta.tierlist = fetch_tierlist_details(client, config).await;
    if let Some(details) = &meta.tierlist {
        let card_count = details
            .rows
            .iter()
            .map(|row| row.cards.len())
            .sum::<usize>();
        upsert_metric(&mut meta.metrics, "Cards", &card_count.to_string());
        if let Some(generated_at) = details.generated_at.as_deref() {
            upsert_metric(&mut meta.metrics, "Generated", generated_at);
        }
    }
    meta
}

fn tierlist_page_metadata(config: &Config, query: Option<&str>) -> EmbedMetadata {
    let clean_query = clean_query(query, &config.debug_query_key);
    let mut meta = page_metadata(config, "tierlist", "/tierlist", Some("Tierlist"));
    meta.canonical_url = absolute_url_with_query(config, "/tierlist", clean_query.as_deref());
    meta.image_url = image_url_with_query(config, "page", "tierlist", clean_query.as_deref());
    meta
}

async fn fetch_tierlist_details(client: &Client, config: &Config) -> Option<TierlistEmbedDetails> {
    let cache = TIERLIST_CACHE.get_or_init(|| Mutex::new(None));
    if let Some(details) = cache.lock().ok().and_then(|guard| {
        guard.as_ref().and_then(|entry| {
            if entry.asset_base_url == config.asset_base_url {
                Some(entry.details.clone())
            } else {
                None
            }
        })
    }) {
        return Some(details);
    }

    let url = format!("{}/data/precomputed-tierlist.json", config.asset_base_url);
    let started_at = Instant::now();
    let response = match client.get(url.clone()).send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "precomputed tierlist request failed"
            );
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!(
            %status,
            %url,
            elapsed_ms = started_at.elapsed().as_millis(),
            "precomputed tierlist request returned non-success status"
        );
        return None;
    }

    let response = match response.json::<PrecomputedTierlistResponse>().await {
        Ok(response) => response,
        Err(error) => {
            warn!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "precomputed tierlist response did not match expected schema"
            );
            return None;
        }
    };

    let details = build_tierlist_details(&config.asset_base_url, response)?;
    debug!(
        %url,
        rows = details.rows.len(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "precomputed tierlist request completed"
    );
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(TierlistCacheEntry {
            asset_base_url: config.asset_base_url.clone(),
            details: details.clone(),
        });
    }

    Some(details)
}

fn build_tierlist_details(
    asset_base_url: &str,
    response: PrecomputedTierlistResponse,
) -> Option<TierlistEmbedDetails> {
    const LB4_INDEX: usize = 4;
    const ROWS: [(&str, &str); 5] = [
        ("S+", "Top percentile"),
        ("S", "High percentile"),
        ("A", "Strong picks"),
        ("B", "Strong picks"),
        ("C", "Strong picks"),
    ];
    const TYPES: [(i64, &str); 5] = [
        (0, "Speed"),
        (1, "Stamina"),
        (2, "Power"),
        (3, "Guts"),
        (4, "Intelligence"),
    ];

    let rows = ROWS
        .into_iter()
        .map(|(tier, range)| {
            let cards = TYPES
                .into_iter()
                .filter_map(|(card_type, stat_type)| {
                    response
                        .cards
                        .values()
                        .filter(|card| {
                            card.rarity == 3
                                && card.card_type == card_type
                                && card
                                    .tiers
                                    .get(LB4_INDEX)
                                    .is_some_and(|card_tier| card_tier == tier)
                                && card.scores.get(LB4_INDEX).is_some()
                                && !card.name.trim().is_empty()
                        })
                        .max_by(|a, b| {
                            a.scores[LB4_INDEX]
                                .cmp(&b.scores[LB4_INDEX])
                                .then_with(|| b.id.cmp(&a.id))
                        })
                        .map(|card| TierlistCardDetails {
                            id: card.id,
                            name: card.name.clone(),
                            stat_type: stat_type.to_string(),
                            score: card.scores[LB4_INDEX],
                        })
                })
                .collect::<Vec<_>>();

            TierlistRowDetails {
                tier: tier.to_string(),
                range: range.to_string(),
                cards,
            }
        })
        .filter(|row| !row.cards.is_empty())
        .collect::<Vec<_>>();

    if rows.is_empty() {
        return None;
    }

    Some(TierlistEmbedDetails {
        asset_base_url: asset_base_url.to_string(),
        generated_at: response.metadata.generated_at,
        rows,
    })
}

async fn home_metadata(client: &Client, config: &Config) -> EmbedMetadata {
    let stats = fetch_site_stats(client, config).await;
    page_metadata_with_home_stats(config, stats)
}

fn page_metadata_with_home_stats(
    config: &Config,
    stats: Option<SiteStatsResponse>,
) -> EmbedMetadata {
    let metrics = match stats {
        Some(stats) => vec![
            metric(
                "Tasks Today",
                &stats
                    .today
                    .tasks_24h
                    .map(format_number)
                    .unwrap_or_else(|| "0".to_string()),
            ),
            metric(
                "Updated Today",
                &stats
                    .freshness
                    .accounts_24h
                    .map(format_number)
                    .unwrap_or_else(|| "0".to_string()),
            ),
            metric(
                "Active 7d",
                &stats
                    .freshness
                    .accounts_7d
                    .map(format_number)
                    .unwrap_or_else(|| "0".to_string()),
            ),
            metric(
                "Umas Tracked",
                &stats
                    .freshness
                    .umas_tracked
                    .map(format_number)
                    .unwrap_or_else(|| "0".to_string()),
            ),
        ],
        None => vec![
            metric("Tasks Today", "Live"),
            metric("Updated Today", "Live"),
            metric("Active 7d", "Tracked"),
            metric("Umas Tracked", "Global"),
        ],
    };

    EmbedMetadata {
        title: "uma.moe - Umamusume Database & Tools".to_string(),
        description: "A practical Umamusume companion site for inheritance search, release tracking, rankings, clubs, profiles, and planning tools.".to_string(),
        canonical_url: absolute_url(config, "/"),
        image_url: image_url(config, "page", "home"),
        image_alt: "uma.moe - Umamusume Database & Tools preview image".to_string(),
        kind_label: "Home".to_string(),
        metrics,
        database: None,
        tierlist: None,
        resources: ResourceCatalog::default(),
    }
}

async fn tools_metadata(client: &Client, config: &Config) -> EmbedMetadata {
    let stats = fetch_site_stats(client, config).await;
    let mut meta = page_metadata(config, "tools", "/tools", Some("Tools"));

    meta.metrics = match stats {
        Some(stats) => vec![
            metric("Tools", "4"),
            metric("Live Tools", "2"),
            metric(
                "Tasks Today",
                &stats
                    .today
                    .tasks_24h
                    .map(format_number)
                    .unwrap_or_else(|| "0".to_string()),
            ),
            metric(
                "Updated Today",
                &stats
                    .freshness
                    .accounts_24h
                    .map(format_number)
                    .unwrap_or_else(|| "0".to_string()),
            ),
            metric(
                "Active 7d",
                &stats
                    .freshness
                    .accounts_7d
                    .map(format_number)
                    .unwrap_or_else(|| "0".to_string()),
            ),
            metric(
                "Umas Tracked",
                &stats
                    .freshness
                    .umas_tracked
                    .map(format_number)
                    .unwrap_or_else(|| "0".to_string()),
            ),
        ],
        None => vec![
            metric("Tools", "4"),
            metric("Live Tools", "2"),
            metric("Tasks Today", "Live"),
            metric("Updated Today", "Live"),
            metric("Active 7d", "Tracked"),
            metric("Umas Tracked", "Global"),
        ],
    };

    meta
}

async fn statistics_metadata(client: &Client, config: &Config) -> EmbedMetadata {
    let mut meta = page_metadata(
        config,
        "statistics",
        "/tools/statistics",
        Some("Statistics"),
    );
    if let Some(metrics) = fetch_statistics_preview_metrics(client, config).await {
        meta.metrics = metrics;
    }
    upsert_metric(&mut meta.metrics, "Asset Base", &config.asset_base_url);
    meta
}

async fn lineage_planner_metadata(
    client: &Client,
    config: &Config,
    query: Option<&str>,
) -> EmbedMetadata {
    let clean_query = clean_query(query, &config.debug_query_key);
    let resources = if has_lineage_planner_tree_query(clean_query.as_deref()) {
        fetch_resource_catalog(client, config).await
    } else {
        ResourceCatalog::default()
    };

    lineage_planner_metadata_from_query(config, clean_query.as_deref(), resources)
}

fn lineage_planner_metadata_from_query(
    config: &Config,
    query: Option<&str>,
    resources: ResourceCatalog,
) -> EmbedMetadata {
    let mut meta = page_metadata(
        config,
        "lineage-planner",
        "/tools/lineage-planner",
        Some("Lineage Planner"),
    );

    meta.canonical_url = absolute_url_with_query(
        config,
        "/tools/lineage-planner",
        query.filter(|query| !query.trim().is_empty()),
    );
    meta.image_url = image_url_with_query(
        config,
        "page",
        "lineage-planner",
        query.filter(|query| !query.trim().is_empty()),
    );

    if has_lineage_planner_tree_query(query) {
        meta.title = "Shared Lineage Planner | uma.moe".to_string();
        meta.description =
            "Shared Umamusume lineage planner tree with saved characters, sparks, and race wins."
                .to_string();
        upsert_metric(&mut meta.metrics, "Mode", "Shared Tree");
    }

    meta.resources = resources;
    meta
}

fn has_lineage_planner_tree_query(query: Option<&str>) -> bool {
    query_pairs(query)
        .iter()
        .any(|(key, value)| key == "tree" && !value.trim().is_empty())
}

async fn fetch_statistics_preview_metrics(
    client: &Client,
    config: &Config,
) -> Option<Vec<EmbedMetric>> {
    let datasets_url = format!("{}/statistics/datasets.json", config.asset_base_url);
    let datasets =
        fetch_json::<StatisticsDatasetsResponse>(client, reqwest::Url::parse(&datasets_url).ok()?)
            .await?;
    let support_lookup = fetch_statistics_support_lookup(client, config).await;
    let mut datasets = datasets.datasets;
    datasets.sort_by(|a, b| b.date.cmp(&a.date));

    for dataset in datasets {
        let Some(base_path) = dataset.base_path.as_deref() else {
            continue;
        };
        let compressed = dataset
            .format_version
            .or_else(|| {
                dataset
                    .index
                    .as_ref()
                    .and_then(|index| index.format_version)
            })
            .is_some_and(|version| version >= 4);
        let global_path = if compressed {
            "global/global.json.gz"
        } else {
            "global/global.json"
        };
        let global_url = statistics_asset_url(config, base_path, global_path);
        let Some(global) =
            fetch_json_payload::<Value>(client, reqwest::Url::parse(&global_url).ok()?, compressed)
                .await
        else {
            if compressed {
                let fallback_url = statistics_asset_url(config, base_path, "global/global.json");
                if let Some(global) =
                    fetch_json::<Value>(client, reqwest::Url::parse(&fallback_url).ok()?).await
                {
                    return Some(statistics_preview_metrics(
                        &dataset,
                        &global,
                        &support_lookup,
                    ));
                }
            }
            continue;
        };
        return Some(statistics_preview_metrics(
            &dataset,
            &global,
            &support_lookup,
        ));
    }

    None
}

fn statistics_asset_url(config: &Config, base_path: &str, path: &str) -> String {
    let base_path = base_path.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if base_path.starts_with("http://") || base_path.starts_with("https://") {
        return format!("{base_path}/{path}");
    }
    if base_path.starts_with("/assets/") {
        let origin = config
            .asset_base_url
            .strip_suffix("/assets")
            .unwrap_or(&config.public_base_url)
            .trim_end_matches('/');
        return format!("{origin}{base_path}/{path}");
    }
    if base_path.starts_with('/') {
        return format!("{}{base_path}/{path}", config.public_base_url);
    }
    format!("{}/{base_path}/{path}", config.asset_base_url)
}

#[derive(Clone, Debug)]
struct StatisticsSupportInfo {
    name: String,
    stat_type: String,
}

async fn fetch_statistics_support_lookup(
    client: &Client,
    config: &Config,
) -> BTreeMap<String, StatisticsSupportInfo> {
    let url = format!("{}/data/precomputed-tierlist.json", config.asset_base_url);
    let Some(response) = fetch_json::<PrecomputedTierlistResponse>(
        client,
        match reqwest::Url::parse(&url) {
            Ok(url) => url,
            Err(_) => return default_statistics_support_lookup(),
        },
    )
    .await
    else {
        return default_statistics_support_lookup();
    };

    let mut lookup = response
        .cards
        .into_iter()
        .filter_map(|(key, card)| {
            let id = if card.id > 0 {
                card.id.to_string()
            } else {
                key
            };
            let name = card.name.trim();
            if id.trim().is_empty() || name.is_empty() {
                return None;
            }

            Some((
                id,
                StatisticsSupportInfo {
                    name: name.to_string(),
                    stat_type: support_type_label(card.card_type).to_string(),
                },
            ))
        })
        .collect::<BTreeMap<_, _>>();

    for (id, name, stat_type) in [
        ("30036", "Riko Kashimoto", "Friend"),
        ("20023", "Sweep Tosho", "Speed"),
        ("30010", "Fine Motion", "Intelligence"),
    ] {
        lookup
            .entry(id.to_string())
            .or_insert(StatisticsSupportInfo {
                name: name.to_string(),
                stat_type: stat_type.to_string(),
            });
    }

    lookup
}

fn default_statistics_support_lookup() -> BTreeMap<String, StatisticsSupportInfo> {
    [
        ("30028", "Kitasan Black", "Speed"),
        ("30016", "Super Creek", "Stamina"),
        ("20023", "Sweep Tosho", "Speed"),
        ("30036", "Riko Kashimoto", "Friend"),
        ("30010", "Fine Motion", "Intelligence"),
    ]
    .into_iter()
    .map(|(id, name, stat_type)| {
        (
            id.to_string(),
            StatisticsSupportInfo {
                name: name.to_string(),
                stat_type: stat_type.to_string(),
            },
        )
    })
    .collect()
}

fn statistics_preview_metrics(
    dataset: &StatisticsDatasetInfo,
    global: &Value,
    support_lookup: &BTreeMap<String, StatisticsSupportInfo>,
) -> Vec<EmbedMetric> {
    let metadata = &global["metadata"];
    let total_entries = value_i64(metadata, "total_entries")
        .or_else(|| dataset.index.as_ref().and_then(|index| index.total_entries))
        .unwrap_or_default();
    let total_trainers = value_i64(metadata, "total_trainers")
        .or_else(|| {
            dataset
                .index
                .as_ref()
                .and_then(|index| index.total_trainers)
        })
        .unwrap_or_default();
    let generated = value_str(metadata, "generated_at")
        .or_else(|| {
            dataset
                .index
                .as_ref()
                .and_then(|index| index.generated_at.as_deref())
        })
        .or(dataset.date.as_deref())
        .unwrap_or("dataset");
    let scenario_scope = latest_statistics_scenario(global);
    let scenario_id = scenario_scope.map(|(id, _)| id);
    let scenario_label = scenario_scope.map(|(_, label)| label).unwrap_or("Overall");
    let scenario_scope_label = scenario_scope
        .map(|(_, label)| format!("{label} only"))
        .unwrap_or_else(|| "Overall".to_string());
    let scoped_total_entries = scenario_id
        .and_then(|id| global["scenario_distribution"][id]["count"].as_i64())
        .unwrap_or(total_entries);
    let class_source = scenario_id
        .map(|id| &global["team_class_distribution"]["by_scenario"][id])
        .filter(|value| value.is_object())
        .unwrap_or(&global["team_class_distribution"]);
    let scoped_total_trainers = value_i64(class_source, "total_trainers").unwrap_or(total_trainers);
    let mut metrics = vec![
        metric(
            "Dataset",
            dataset
                .id
                .as_deref()
                .or(dataset.date.as_deref())
                .unwrap_or("statistics"),
        ),
        metric("Generated", &format_updated_label(generated)),
        metric("Trained Umas", &format_number(scoped_total_entries)),
        metric("Trainers", &format_number(scoped_total_trainers)),
        metric("Statistics Scope", &scenario_scope_label),
        metric("Statistics Scope Short", scenario_label),
    ];

    for class in 1..=6 {
        if let Some(value) = class_source[class.to_string()]["percentage"].as_f64() {
            metrics.push(metric(&format!("Class {class}"), &format!("{value:.1}%")));
        }
    }

    for (key, label) in [
        ("speed", "Speed Mean"),
        ("stamina", "Stamina Mean"),
        ("power", "Power Mean"),
        ("guts", "Guts Mean"),
        ("wiz", "Wisdom Mean"),
    ] {
        let stat_source = scenario_id
            .map(|id| &global["stat_averages"]["by_scenario"][id])
            .filter(|value| value.is_object())
            .unwrap_or(&global["stat_averages"]["overall"]);
        if let Some(value) = stat_source[key]["mean"].as_f64() {
            metrics.push(metric(label, &format!("{value:.0}")));
        }
    }

    for (id, label) in [("1", "URA"), ("2", "Aoharu"), ("4", "MANT")] {
        if let Some(value) = global["scenario_distribution"][id]["percentage"].as_f64() {
            metrics.push(metric(
                &format!("Scenario {label}"),
                &format!("{value:.1}%"),
            ));
        }
    }

    let deck_source = scenario_id
        .map(|id| &global["support_card_combinations"]["by_scenario"][id])
        .filter(|value| value.is_object())
        .unwrap_or(&global["support_card_combinations"]["overall"]);
    push_deck_composition_metrics(&mut metrics, deck_source);

    let pushed_scoped_umas = scenario_id
        .map(|id| push_scenario_uma_distribution_metrics(&mut metrics, global, id))
        .unwrap_or(false);
    if !pushed_scoped_umas {
        push_top_distribution_metrics(
            &mut metrics,
            "Uma",
            &global["uma_distribution"],
            |id| statistics_uma_name(id),
            |_, _| None,
            true,
        );
    }

    let support_source = scenario_id
        .map(|id| &global["support_cards"]["by_scenario"][id])
        .filter(|value| value.is_object())
        .unwrap_or(&global["support_cards"]["overall"]);
    push_support_card_distribution_metrics(
        &mut metrics,
        support_source,
        support_lookup,
        scoped_total_entries,
    );

    metrics
}

fn latest_statistics_scenario(global: &Value) -> Option<(&'static str, &'static str)> {
    [("4", "MANT"), ("2", "Aoharu"), ("1", "URA")]
        .into_iter()
        .find(|(id, _)| {
            global["scenario_distribution"][*id]["count"]
                .as_i64()
                .is_some_and(|count| count > 0)
        })
}

fn push_deck_composition_metrics(metrics: &mut Vec<EmbedMetric>, value: &Value) {
    let Some(object) = value.as_object() else {
        return;
    };
    let mut rows = object
        .iter()
        .filter_map(|(id, row)| {
            let count = row["count"].as_i64()?;
            Some((id.as_str(), row, count))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| b.2.cmp(&a.2));

    for (index, (id, row, count)) in rows.into_iter().take(7).enumerate() {
        let composition = deck_composition_label(&row["composition"]);
        if composition.is_empty() {
            continue;
        }
        let percentage = row["percentage"]
            .as_f64()
            .map(|value| format!("{value:.2}%"))
            .unwrap_or_else(|| compact_number(count));
        metrics.push(metric(&format!("Deck {}", index + 1), &composition));
        metrics.push(metric(&format!("Deck Id {}", index + 1), id));
        metrics.push(metric(&format!("Deck Value {}", index + 1), &percentage));
        metrics.push(metric(
            &format!("Deck Count {}", index + 1),
            &compact_number(count),
        ));
    }
}

fn deck_composition_label(value: &Value) -> String {
    let Some(object) = value.as_object() else {
        return String::new();
    };
    [
        ("speed", "SPD"),
        ("stamina", "STA"),
        ("power", "POW"),
        ("guts", "GUT"),
        ("wisdom", "WIT"),
        ("friend", "FRD"),
        ("group", "GRP"),
    ]
    .into_iter()
    .filter_map(|(key, label)| {
        let count = object.get(key)?.as_i64()?;
        (count > 0).then(|| format!("{count} {label}"))
    })
    .collect::<Vec<_>>()
    .join(" / ")
}

fn push_top_distribution_metrics<F, G>(
    metrics: &mut Vec<EmbedMetric>,
    prefix: &str,
    value: &Value,
    name_for_id: F,
    detail_for_id: G,
    use_percentage: bool,
) where
    F: Fn(&str) -> String,
    G: Fn(&str, &Value) -> Option<String>,
{
    let Some(object) = value.as_object() else {
        return;
    };
    let mut rows = object
        .iter()
        .filter_map(|(id, row)| {
            let sort_value = if use_percentage {
                row["percentage"].as_f64()
            } else {
                row["total"].as_i64().map(|value| value as f64)
            }?;
            Some((id.as_str(), row, sort_value))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    for (index, (id, row, _)) in rows.into_iter().take(5).enumerate() {
        let label = name_for_id(id);
        let value = if use_percentage {
            row["percentage"]
                .as_f64()
                .map(|value| format!("{value:.2}%"))
                .unwrap_or_else(|| "tracked".to_string())
        } else {
            row["total"]
                .as_i64()
                .map(format_number)
                .unwrap_or_else(|| "tracked".to_string())
        };
        metrics.push(metric(&format!("{prefix} {}", index + 1), &label));
        metrics.push(metric(&format!("{prefix} Value {}", index + 1), &value));
        metrics.push(metric(&format!("{prefix} Id {}", index + 1), id));
        if let Some(detail) = detail_for_id(id, row) {
            metrics.push(metric(&format!("{prefix} Detail {}", index + 1), &detail));
        }
        if let Some(count) = row["count"].as_i64() {
            metrics.push(metric(
                &format!("{prefix} Count {}", index + 1),
                &format!("{} runs", compact_number(count)),
            ));
        } else if let Some(total) = row["total"].as_i64() {
            metrics.push(metric(
                &format!("{prefix} Count {}", index + 1),
                &compact_number(total),
            ));
        }
    }
}

fn push_scenario_uma_distribution_metrics(
    metrics: &mut Vec<EmbedMetric>,
    global: &Value,
    scenario_id: &str,
) -> bool {
    let Some(distances) = global["by_distance"].as_object() else {
        return false;
    };
    let mut counts = BTreeMap::<String, i64>::new();
    let mut total = 0_i64;

    for distance in distances.values() {
        let scenario = &distance["by_scenario"][scenario_id];
        if !scenario.is_object() {
            continue;
        }
        total += scenario["total_trained_umas"]
            .as_i64()
            .or_else(|| scenario["total_entries"].as_i64())
            .unwrap_or_default();

        let Some(umas) = scenario["uma_distribution"].as_object() else {
            continue;
        };
        for (id, row) in umas {
            let count = row["count"].as_i64().unwrap_or_default();
            if count > 0 {
                *counts.entry(id.to_string()).or_default() += count;
            }
        }
    }

    if counts.is_empty() || total <= 0 {
        return false;
    }

    let mut rows = counts.into_iter().collect::<Vec<_>>();
    rows.sort_by(|a, b| b.1.cmp(&a.1));

    for (index, (id, count)) in rows.into_iter().take(5).enumerate() {
        let percentage = (count as f64 / total as f64) * 100.0;
        metrics.push(metric(
            &format!("Uma {}", index + 1),
            &statistics_uma_name(&id),
        ));
        metrics.push(metric(
            &format!("Uma Value {}", index + 1),
            &format!("{percentage:.2}%"),
        ));
        metrics.push(metric(&format!("Uma Id {}", index + 1), &id));
        metrics.push(metric(
            &format!("Uma Count {}", index + 1),
            &format!("{} runs", compact_number(count)),
        ));
    }

    true
}

fn push_support_card_distribution_metrics(
    metrics: &mut Vec<EmbedMetric>,
    value: &Value,
    support_lookup: &BTreeMap<String, StatisticsSupportInfo>,
    total_entries: i64,
) {
    let Some(object) = value.as_object() else {
        return;
    };
    let mut rows = object
        .iter()
        .filter_map(|(id, row)| {
            let total = row["total"].as_i64()?;
            Some((id.as_str(), row, total))
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| b.2.cmp(&a.2));

    for (index, (id, row, total)) in rows.into_iter().take(5).enumerate() {
        let percentage = row["percentage"]
            .as_f64()
            .or_else(|| (total_entries > 0).then(|| (total as f64 / total_entries as f64) * 100.0));
        let value = percentage
            .map(|value| format!("{value:.2}%"))
            .unwrap_or_else(|| format_number(total));
        let support_type = statistics_support_type(id, support_lookup);

        metrics.push(metric(
            &format!("Support {}", index + 1),
            &statistics_support_name(id, support_lookup),
        ));
        metrics.push(metric(&format!("Support Value {}", index + 1), &value));
        metrics.push(metric(&format!("Support Id {}", index + 1), id));
        metrics.push(metric(
            &format!("Support Detail {}", index + 1),
            &format!("{support_type} support"),
        ));
        metrics.push(metric(
            &format!("Support Count {}", index + 1),
            &compact_number(total),
        ));
    }
}

fn value_i64<'a>(value: &'a Value, key: &str) -> Option<i64> {
    value[key].as_i64()
}

fn value_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value[key].as_str()
}

fn statistics_uma_name(id: &str) -> String {
    match id {
        "104101" => "Sakura Bakushin O",
        "105201" => "Haru Urara",
        "100901" => "Daiwa Scarlet",
        "100701" => "Gold Ship",
        "100801" => "Vodka",
        "101401" => "Oguri Cap",
        "100601" => "Mejiro McQueen",
        "103201" => "Rice Shower",
        _ if id.chars().any(char::is_alphabetic) => id,
        _ => return format!("Uma #{id}"),
    }
    .to_string()
}

fn statistics_support_name(id: &str, lookup: &BTreeMap<String, StatisticsSupportInfo>) -> String {
    lookup
        .get(id)
        .map(|support| support.name.clone())
        .unwrap_or_else(|| fallback_statistics_support_name(id).to_string())
}

fn statistics_support_type(id: &str, lookup: &BTreeMap<String, StatisticsSupportInfo>) -> String {
    lookup
        .get(id)
        .map(|support| support.stat_type.clone())
        .unwrap_or_else(|| "Support".to_string())
}

fn fallback_statistics_support_name(id: &str) -> &'static str {
    match id {
        "30028" => "Kitasan Black",
        "30016" => "Super Creek",
        "20023" => "Sweep Tosho",
        "30036" => "Riko Kashimoto",
        "30010" => "Fine Motion",
        "20013" => "Eishin Flash",
        "30025" => "Special Week",
        "30020" => "Biko Pegasus",
        "20020" => "King Halo",
        _ => "Support Card",
    }
}

fn support_type_label(value: i64) -> &'static str {
    match value {
        0 => "Speed",
        1 => "Stamina",
        2 => "Power",
        3 => "Guts",
        4 => "Intelligence",
        5 => "Friend",
        6 => "Group",
        _ => "Support",
    }
}

async fn rankings_metadata(client: &Client, config: &Config, query: Option<&str>) -> EmbedMetadata {
    let clean_query = clean_query(query, &config.debug_query_key);
    let query_pairs = query_pairs(clean_query.as_deref());
    let tab = ranking_tab(&query_pairs);
    let metrics = match tab {
        "alltime" => fetch_alltime_rankings(client, config, &query_pairs)
            .await
            .map(|response| alltime_rankings_metrics(response, &query_pairs)),
        "gains" => fetch_gains_rankings(client, config, &query_pairs)
            .await
            .map(|response| gains_rankings_metrics(response, &query_pairs)),
        _ => fetch_monthly_rankings(client, config, &query_pairs)
            .await
            .map(monthly_rankings_metrics),
    }
    .unwrap_or_else(|| fallback_rankings_metrics(tab, &query_pairs));

    EmbedMetadata {
        title: "Trainer Rankings | uma.moe".to_string(),
        description: "Global trainer fan rankings monthly, all-time, and recent gains.".to_string(),
        canonical_url: absolute_url_with_query(config, "/rankings", clean_query.as_deref()),
        image_url: image_url_with_query(config, "page", "rankings", clean_query.as_deref()),
        image_alt: "uma.moe trainer rankings preview image".to_string(),
        kind_label: "Rankings".to_string(),
        metrics,
        database: None,
        tierlist: None,
        resources: ResourceCatalog::default(),
    }
}

fn query_pairs(query: Option<&str>) -> Vec<(String, String)> {
    query
        .map(|query| {
            form_urlencoded::parse(query.as_bytes())
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect()
        })
        .unwrap_or_default()
}

fn query_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(candidate, value)| candidate == key && !value.trim().is_empty())
        .map(|(_, value)| value.as_str())
}

fn ranking_tab(pairs: &[(String, String)]) -> &'static str {
    match query_value(pairs, "tab") {
        Some("alltime") | Some("all-time") => "alltime",
        Some("gains") => "gains",
        _ => "monthly",
    }
}

async fn fetch_monthly_rankings(
    client: &Client,
    config: &Config,
    pairs: &[(String, String)],
) -> Option<MonthlyRankingsResponse> {
    let response: MonthlyRankingsResponse =
        fetch_json(client, monthly_rankings_url(config, pairs, None)?).await?;
    if !response.rankings.is_empty() || has_explicit_monthly_ranking_period(pairs) {
        return Some(response);
    }

    let (Some(mut year), Some(month)) = (response.year, response.month) else {
        return Some(response);
    };
    let previous_month = if month <= 1 {
        year -= 1;
        12
    } else {
        month - 1
    };

    let Some(url) = monthly_rankings_url(config, pairs, Some((year, previous_month))) else {
        return Some(response);
    };
    if let Some(candidate) = fetch_json::<MonthlyRankingsResponse>(client, url).await {
        if !candidate.rankings.is_empty() {
            return Some(candidate);
        }
    }

    Some(response)
}

fn monthly_rankings_url(
    config: &Config,
    pairs: &[(String, String)],
    period: Option<(i64, i64)>,
) -> Option<reqwest::Url> {
    let mut url =
        reqwest::Url::parse(&format!("{}/api/v4/rankings/monthly", config.api_base_url)).ok()?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("page", query_value(pairs, "page").unwrap_or("0"));
        query.append_pair("limit", "10");
        if let Some((year, month)) = period {
            query.append_pair("year", &year.to_string());
            query.append_pair("month", &month.to_string());
        } else {
            for key in ["month", "year"] {
                if let Some(value) = query_value(pairs, key) {
                    query.append_pair(key, value);
                }
            }
        }
        for key in ["query", "circle_name"] {
            if let Some(value) = query_value(pairs, key) {
                query.append_pair(key, value);
            }
        }
    }

    Some(url)
}

fn has_explicit_monthly_ranking_period(pairs: &[(String, String)]) -> bool {
    ["month", "year"]
        .iter()
        .any(|key| query_value(pairs, key).is_some())
}

async fn fetch_alltime_rankings(
    client: &Client,
    config: &Config,
    pairs: &[(String, String)],
) -> Option<AlltimeRankingsResponse> {
    let mut url =
        reqwest::Url::parse(&format!("{}/api/v4/rankings/alltime", config.api_base_url)).ok()?;
    {
        let sort = alltime_sort(pairs);
        let mut query = url.query_pairs_mut();
        query.append_pair("page", query_value(pairs, "page").unwrap_or("0"));
        query.append_pair("limit", "10");
        query.append_pair("sort_by", sort);
        for key in ["query", "circle_name"] {
            if let Some(value) = query_value(pairs, key) {
                query.append_pair(key, value);
            }
        }
    }

    fetch_json(client, url).await
}

async fn fetch_gains_rankings(
    client: &Client,
    config: &Config,
    pairs: &[(String, String)],
) -> Option<GainsRankingsResponse> {
    let mut url =
        reqwest::Url::parse(&format!("{}/api/v4/rankings/gains", config.api_base_url)).ok()?;
    {
        let sort = gains_sort(pairs);
        let mut query = url.query_pairs_mut();
        query.append_pair("page", query_value(pairs, "page").unwrap_or("0"));
        query.append_pair("limit", "10");
        query.append_pair("sort_by", sort);
        for key in ["query", "circle_name"] {
            if let Some(value) = query_value(pairs, key) {
                query.append_pair(key, value);
            }
        }
    }

    fetch_json(client, url).await
}

async fn fetch_json<T: DeserializeOwned>(client: &Client, url: reqwest::Url) -> Option<T> {
    let started_at = Instant::now();
    let response = match client.get(url.clone()).send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "embed preview request failed"
            );
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!(
            %status,
            %url,
            elapsed_ms = started_at.elapsed().as_millis(),
            "embed preview request returned non-success status"
        );
        return None;
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            warn!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "embed preview response body could not be read"
            );
            return None;
        }
    };

    match serde_json::from_str::<T>(&body) {
        Ok(value) => {
            debug!(
                %url,
                bytes = body.len(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "embed preview request completed"
            );
            Some(value)
        }
        Err(error) => {
            warn!(
                %error,
                %url,
                body_preview = %truncate_chars(&body, 500),
                elapsed_ms = started_at.elapsed().as_millis(),
                "embed preview response did not match expected schema"
            );
            None
        }
    }
}

async fn fetch_json_payload<T: DeserializeOwned>(
    client: &Client,
    url: reqwest::Url,
    gzipped: bool,
) -> Option<T> {
    let started_at = Instant::now();
    let response = match client.get(url.clone()).send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "embed preview request failed"
            );
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!(
            %status,
            %url,
            elapsed_ms = started_at.elapsed().as_millis(),
            "embed preview request returned non-success status"
        );
        return None;
    }

    let bytes = response.bytes().await.ok()?;
    let byte_count = bytes.len();
    if gzipped {
        let mut decoder = GzDecoder::new(bytes.as_ref());
        let mut json = String::new();
        if let Err(error) = decoder.read_to_string(&mut json) {
            warn!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "failed to decode gzipped statistics payload"
            );
            return None;
        }

        match serde_json::from_str(&json) {
            Ok(value) => {
                debug!(
                    %url,
                    compressed_bytes = byte_count,
                    decoded_bytes = json.len(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "embed preview payload request completed"
                );
                Some(value)
            }
            Err(error) => {
                warn!(
                    %error,
                    %url,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "embed preview payload response did not match expected schema"
                );
                None
            }
        }
    } else {
        match serde_json::from_slice(&bytes) {
            Ok(value) => {
                debug!(
                    %url,
                    bytes = byte_count,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "embed preview payload request completed"
                );
                Some(value)
            }
            Err(error) => {
                warn!(
                    %error,
                    %url,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "embed preview payload response did not match expected schema"
                );
                None
            }
        }
    }
}

fn monthly_rankings_metrics(response: MonthlyRankingsResponse) -> Vec<EmbedMetric> {
    let period = match (response.month, response.year) {
        (Some(month), Some(year)) => format!("{} {year}", month_label(month)),
        _ => "Current Month".to_string(),
    };
    let mut metrics = ranking_base_metrics(
        "Monthly",
        &period,
        response.total,
        "Monthly Gain",
        "Fans",
        "Avg/Day",
    );

    for (index, entry) in response.rankings.into_iter().take(10).enumerate() {
        let row = index + 1;
        metrics.push(metric(
            &format!("Rank {row}"),
            &format!("#{}", entry.rank.unwrap_or(row as i64)),
        ));
        metrics.push(metric(
            &format!("Trainer {row}"),
            &trainer_label(entry.trainer_name.clone(), entry.viewer_id),
        ));
        metrics.push(metric(
            &format!("Club {row}"),
            entry.circle_name.as_deref().unwrap_or("No club"),
        ));
        if let Some(club_rank) = entry.club_rank.or(entry.circle_rank) {
            metrics.push(metric(
                &format!("Club Rank Id {row}"),
                &club_rank.to_string(),
            ));
        }
        metrics.push(metric(
            &format!("Primary {row}"),
            &signed_compact(entry.monthly_gain.unwrap_or_default()),
        ));
        metrics.push(metric(
            &format!("Secondary {row}"),
            &compact_number(entry.total_fans.unwrap_or_default()),
        ));
        metrics.push(metric(
            &format!("Tertiary {row}"),
            &format_optional_rate(entry.avg_daily),
        ));
        if let Some(days) = entry.active_days {
            metrics.push(metric(&format!("Active Days {row}"), &format!("{days}d")));
        }
    }

    metrics
}

fn alltime_rankings_metrics(
    response: AlltimeRankingsResponse,
    pairs: &[(String, String)],
) -> Vec<EmbedMetric> {
    let sort = alltime_sort(pairs);
    let mut metrics = ranking_base_metrics(
        "All-Time",
        alltime_sort_label(sort),
        response.total,
        alltime_sort_label(sort),
        "Total Fans",
        "Total Gain",
    );

    for (index, entry) in response.rankings.into_iter().take(10).enumerate() {
        let row = index + 1;
        metrics.push(metric(
            &format!("Rank {row}"),
            &format!("#{}", alltime_rank(&entry, sort).unwrap_or(row as i64)),
        ));
        metrics.push(metric(
            &format!("Trainer {row}"),
            &trainer_label(entry.trainer_name.clone(), entry.viewer_id),
        ));
        metrics.push(metric(
            &format!("Club {row}"),
            entry.circle_name.as_deref().unwrap_or("No club"),
        ));
        if let Some(club_rank) = entry.club_rank.or(entry.circle_rank) {
            metrics.push(metric(
                &format!("Club Rank Id {row}"),
                &club_rank.to_string(),
            ));
        }
        metrics.push(metric(
            &format!("Primary {row}"),
            &alltime_value(&entry, sort),
        ));
        metrics.push(metric(
            &format!("Secondary {row}"),
            &compact_number(entry.total_fans.unwrap_or_default()),
        ));
        metrics.push(metric(
            &format!("Tertiary {row}"),
            &signed_compact(entry.total_gain.unwrap_or_default()),
        ));
    }

    metrics
}

fn gains_rankings_metrics(
    response: GainsRankingsResponse,
    pairs: &[(String, String)],
) -> Vec<EmbedMetric> {
    let sort_owned = response
        .sort_by
        .as_deref()
        .filter(|sort| is_gains_sort(sort))
        .unwrap_or_else(|| gains_sort(pairs))
        .to_string();
    let sort = sort_owned.as_str();
    let (primary, secondary, tertiary) = gains_labels(sort);
    let mut metrics = ranking_base_metrics(
        "Gains",
        gains_sort_label(sort),
        response.total,
        primary,
        secondary,
        tertiary,
    );

    for (index, entry) in response.rankings.into_iter().take(10).enumerate() {
        let row = index + 1;
        metrics.push(metric(
            &format!("Rank {row}"),
            &format!("#{}", gains_rank(&entry, sort).unwrap_or(row as i64)),
        ));
        metrics.push(metric(
            &format!("Trainer {row}"),
            &trainer_label(entry.trainer_name.clone(), entry.viewer_id),
        ));
        metrics.push(metric(
            &format!("Club {row}"),
            entry.circle_name.as_deref().unwrap_or("No club"),
        ));
        if let Some(club_rank) = entry.club_rank.or(entry.circle_rank) {
            metrics.push(metric(
                &format!("Club Rank Id {row}"),
                &club_rank.to_string(),
            ));
        }
        metrics.push(metric(
            &format!("Primary {row}"),
            &signed_compact(gains_value(&entry, sort).unwrap_or_default()),
        ));
        metrics.push(metric(
            &format!("Secondary {row}"),
            &signed_compact(gains_value(&entry, secondary_gain_key(sort)).unwrap_or_default()),
        ));
        metrics.push(metric(
            &format!("Tertiary {row}"),
            &signed_compact(gains_value(&entry, tertiary_gain_key(sort)).unwrap_or_default()),
        ));
    }

    metrics
}

fn fallback_rankings_metrics(tab: &str, pairs: &[(String, String)]) -> Vec<EmbedMetric> {
    match tab {
        "alltime" => ranking_base_metrics(
            "All-Time",
            alltime_sort_label(alltime_sort(pairs)),
            0,
            alltime_sort_label(alltime_sort(pairs)),
            "Total Fans",
            "Total Gain",
        ),
        "gains" => {
            let sort = gains_sort(pairs);
            let (primary, secondary, tertiary) = gains_labels(sort);
            ranking_base_metrics(
                "Gains",
                gains_sort_label(sort),
                0,
                primary,
                secondary,
                tertiary,
            )
        }
        _ => ranking_base_metrics(
            "Monthly",
            "Current Month",
            0,
            "Monthly Gain",
            "Fans",
            "Avg/Day",
        ),
    }
}

fn ranking_base_metrics(
    tab: &str,
    period: &str,
    total: i64,
    primary_label: &str,
    secondary_label: &str,
    tertiary_label: &str,
) -> Vec<EmbedMetric> {
    let total_value = if total > 0 {
        format_number(total)
    } else {
        "Live".to_string()
    };

    vec![
        metric("Tab", tab),
        metric("Period", period),
        metric("Total", &total_value),
        metric("Primary Label", primary_label),
        metric("Secondary Label", secondary_label),
        metric("Tertiary Label", tertiary_label),
    ]
}

fn alltime_sort(pairs: &[(String, String)]) -> &'static str {
    match query_value(pairs, "sortBy").or_else(|| query_value(pairs, "sort_by")) {
        Some("total_fans") => "total_fans",
        Some("total_gain") => "total_gain",
        Some("avg_day") => "avg_day",
        Some("avg_week") => "avg_week",
        _ => "avg_month",
    }
}

fn gains_sort(pairs: &[(String, String)]) -> &'static str {
    match query_value(pairs, "sortBy").or_else(|| query_value(pairs, "sort_by")) {
        Some("gain_3d") => "gain_3d",
        Some("gain_7d") => "gain_7d",
        _ => "gain_30d",
    }
}

fn is_gains_sort(sort: &str) -> bool {
    matches!(sort, "gain_3d" | "gain_7d" | "gain_30d")
}

fn alltime_sort_label(sort: &str) -> &'static str {
    match sort {
        "total_fans" => "Total Fans",
        "total_gain" => "Total Gain",
        "avg_day" => "Avg/Day",
        "avg_week" => "Avg/Week",
        _ => "Avg/Month",
    }
}

fn gains_sort_label(sort: &str) -> &'static str {
    match sort {
        "gain_3d" => "3-Day Gain",
        "gain_7d" => "7-Day Gain",
        _ => "30-Day Gain",
    }
}

fn gains_labels(sort: &str) -> (&'static str, &'static str, &'static str) {
    match sort {
        "gain_3d" => ("3d", "7d", "30d"),
        "gain_7d" => ("7d", "3d", "30d"),
        _ => ("30d", "7d", "3d"),
    }
}

fn secondary_gain_key(sort: &str) -> &'static str {
    match sort {
        "gain_3d" => "gain_7d",
        "gain_7d" => "gain_3d",
        _ => "gain_7d",
    }
}

fn tertiary_gain_key(sort: &str) -> &'static str {
    match sort {
        "gain_3d" | "gain_7d" => "gain_30d",
        _ => "gain_3d",
    }
}

fn alltime_rank(entry: &UserFanRankingAlltime, sort: &str) -> Option<i64> {
    match sort {
        "total_fans" => entry.rank_total_fans,
        "total_gain" => entry.rank_total_gain,
        "avg_day" => entry.rank_avg_day,
        "avg_week" => entry.rank_avg_week,
        "avg_month" => entry.rank_avg_month,
        _ => entry.rank,
    }
}

fn alltime_value(entry: &UserFanRankingAlltime, sort: &str) -> String {
    match sort {
        "total_fans" => compact_number(entry.total_fans.unwrap_or_default()),
        "total_gain" => signed_compact(entry.total_gain.unwrap_or_default()),
        "avg_day" => format_optional_rate(entry.avg_day),
        "avg_week" => format_optional_rate(entry.avg_week),
        _ => format_optional_rate(entry.avg_month),
    }
}

fn gains_rank(entry: &UserFanRankingGains, sort: &str) -> Option<i64> {
    match sort {
        "gain_3d" => entry.rank_3d,
        "gain_7d" => entry.rank_7d,
        _ => entry.rank_30d,
    }
}

fn gains_value(entry: &UserFanRankingGains, sort: &str) -> Option<i64> {
    match sort {
        "gain_3d" => entry.gain_3d,
        "gain_7d" => entry.gain_7d,
        _ => entry.gain_30d,
    }
}

fn trainer_label(name: Option<String>, viewer_id: Option<i64>) -> String {
    name.filter(|name| !name.trim().is_empty())
        .or_else(|| viewer_id.map(|id| format!("Trainer {id}")))
        .unwrap_or_else(|| "Unknown Trainer".to_string())
}

fn signed_compact(value: i64) -> String {
    if value > 0 {
        format!("+{}", compact_number(value))
    } else {
        compact_number(value)
    }
}

fn format_optional_rate(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format_number(value.round() as i64))
        .unwrap_or_else(|| "-".to_string())
}

fn month_label(month: i64) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Current Month",
    }
}

async fn activity_metadata(client: &Client, config: &Config, query: Option<&str>) -> EmbedMetadata {
    let clean_query = clean_query(query, &config.debug_query_key);
    let query_pairs = query_pairs(clean_query.as_deref());
    let metrics = fetch_activity_hall(client, config, &query_pairs)
        .await
        .map(|response| activity_hall_metrics(response, &query_pairs))
        .unwrap_or_else(|| fallback_activity_metrics(&query_pairs));

    EmbedMetadata {
        title: "Top 100 Club Activity Reports | uma.moe".to_string(),
        description: "Every observed account in a Top 100 club is listed by default, including 0 scores. The suspicion score ranks how unusual the reconstructed activity looks; it is not proof of botting.".to_string(),
        canonical_url: absolute_url_with_query(config, "/activity", clean_query.as_deref()),
        image_url: image_url_with_query(config, "page", "activity", clean_query.as_deref()),
        image_alt: "uma.moe Top 100 club activity reports preview image".to_string(),
        kind_label: "Activity".to_string(),
        metrics,
        database: None,
        tierlist: None,
        resources: ResourceCatalog::default(),
    }
}

async fn activity_detail_metadata(
    client: &Client,
    config: &Config,
    viewer_id: &str,
) -> EmbedMetadata {
    let report = fetch_activity_viewer_report(client, config, viewer_id).await;
    let (title, description, metrics) = match report {
        Some(report) => activity_detail_report_metadata(viewer_id, report),
        None => (
            format!("Activity Report {viewer_id} | uma.moe"),
            "Snapshot-based Top 100 club activity report. Suspicion scores are context, not proof."
                .to_string(),
            fallback_activity_detail_metrics(viewer_id),
        ),
    };

    EmbedMetadata {
        title,
        description,
        canonical_url: absolute_url(config, &format!("/activity/{viewer_id}")),
        image_url: image_url(config, "activity", viewer_id),
        image_alt: format!("uma.moe activity report preview for viewer {viewer_id}"),
        kind_label: "Activity".to_string(),
        metrics,
        database: None,
        tierlist: None,
        resources: ResourceCatalog::default(),
    }
}

async fn fetch_activity_viewer_report(
    client: &Client,
    config: &Config,
    viewer_id: &str,
) -> Option<ActivityViewerReport> {
    let mut url = reqwest::Url::parse(&format!(
        "{}/api/v4/shame/viewer/{}",
        config.api_base_url,
        urlencoding::encode(viewer_id)
    ))
    .ok()?;
    url.query_pairs_mut().append_pair("days", "60");

    fetch_json(client, url).await
}

fn activity_detail_report_metadata(
    viewer_id: &str,
    report: ActivityViewerReport,
) -> (String, String, Vec<EmbedMetric>) {
    let Some(score) = report.score else {
        return (
            format!("Activity Report {viewer_id} | uma.moe"),
            "Snapshot-based Top 100 club activity report. Suspicion scores are context, not proof."
                .to_string(),
            fallback_activity_detail_metrics(viewer_id),
        );
    };

    let trainer = trainer_label(score.trainer_name.clone(), score.viewer_id);
    let score_value = score.suspicion_score.unwrap_or_default();
    let verdict = score
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.verdict.as_deref())
        .map(activity_verdict_label)
        .unwrap_or("Activity pattern");
    let summary = score
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.summary.clone())
        .unwrap_or_else(|| {
            "Snapshot-based report from Top 100 club observations. This is context, not proof."
                .to_string()
        });
    let top_reason = score
        .evidence
        .as_ref()
        .and_then(|evidence| evidence.reasons.first())
        .and_then(|reason| reason.label.as_deref())
        .unwrap_or(verdict);
    let last_seen = score
        .last_seen
        .as_deref()
        .map(format_updated_label)
        .unwrap_or_else(|| "Last seen recently".to_string());
    let daily_points = report
        .daily
        .iter()
        .rev()
        .take(14)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let daily_labels = daily_points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            point
                .day
                .as_deref()
                .map(format_short_day_label)
                .unwrap_or_else(|| format!("D{}", index + 1))
        })
        .collect::<Vec<_>>()
        .join(",");
    let daily_fan_gains = daily_points
        .iter()
        .map(|point| point.fan_gain.unwrap_or_default().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let daily_active_seconds = daily_points
        .iter()
        .map(|point| point.active_seconds.unwrap_or_default().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let career_length_buckets = score
        .career_length_buckets
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let short_fan_gain_score_buckets = score
        .short_fan_gain_score_buckets
        .iter()
        .map(|value| format!("{value:.2}"))
        .collect::<Vec<_>>()
        .join(",");

    let mut metrics = vec![
        metric("Report Mode", "Detail"),
        metric("Trainer", &trainer),
        metric(
            "Viewer",
            &score
                .viewer_id
                .map(|id| format!("ID {id}"))
                .unwrap_or_else(|| format!("ID {viewer_id}")),
        ),
        metric(
            "Club",
            score.circle_name.as_deref().unwrap_or("Club context"),
        ),
        metric("Score", &score_value.to_string()),
        metric("Score Band", activity_score_band(score_value)),
        metric("Score Class", activity_score_class(score_value)),
        metric("Verdict", verdict),
        metric("Summary", &summary),
        metric("Primary Reason", top_reason),
        metric(
            "Days",
            &format!(
                "{} observed days",
                score.days_observed.unwrap_or(report.daily.len() as i64)
            ),
        ),
        metric("Last Seen", &last_seen),
        metric(
            "Total Fan Gain",
            &compact_number(score.total_fan_gain.unwrap_or_default()),
        ),
        metric(
            "Total Active",
            &format_duration_compact(score.total_active_seconds.unwrap_or_default()),
        ),
        metric(
            "Total Careers",
            &format_number(score.total_careers.unwrap_or_default()),
        ),
        metric(
            "Careers/hr",
            &format_optional_decimal(score.careers_per_active_hour),
        ),
        metric(
            "Peak Fans/min",
            &format_optional_rate(score.peak_fans_per_minute),
        ),
        metric(
            "Recent 3d",
            &signed_compact(score.recent_fan_gain_3d.unwrap_or_default()),
        ),
        metric(
            "Under-15m",
            &score.short_high_fan_careers.unwrap_or_default().to_string(),
        ),
        metric(
            "Short Score",
            &format_optional_decimal(score.short_fan_gain_score),
        ),
        metric(
            "Short Avg",
            &compact_number(score.short_career_avg_fan_gain.unwrap_or_default().round() as i64),
        ),
        metric(
            "P95 Short",
            &compact_number(score.short_career_p95_fan_gain.unwrap_or_default().round() as i64),
        ),
        metric(
            "Last 20 Avg",
            &format_duration_compact(
                score
                    .avg_career_length_last20_seconds
                    .unwrap_or_default()
                    .round() as i64,
            ),
        ),
        metric(
            "Weekly Buckets",
            &format!(
                "{} / 168",
                score.distinct_weekly_hour_buckets.unwrap_or_default()
            ),
        ),
        metric("Daily Days", &report.daily.len().to_string()),
        metric("Peak Daily", &signed_compact(max_daily_gain(&report.daily))),
        metric(
            "Peak Active Day",
            &format_duration_compact(max_daily_active_seconds(&report.daily)),
        ),
        metric("Daily Labels", &daily_labels),
        metric("Daily Fan Gains", &daily_fan_gains),
        metric("Daily Active Seconds", &daily_active_seconds),
        metric("Career Length Buckets", &career_length_buckets),
        metric(
            "Short Fan Gain Score Buckets",
            &short_fan_gain_score_buckets,
        ),
        metric("Heatmap Pattern", &heatmap_pattern(&report.heatmap)),
    ];

    if let Some(updated) = report.last_refreshed_at.as_deref() {
        metrics.push(metric("Updated", &format_updated_label(updated)));
    }

    (
        format!("{trainer} Activity Report | uma.moe"),
        summary,
        metrics,
    )
}

fn fallback_activity_detail_metrics(viewer_id: &str) -> Vec<EmbedMetric> {
    vec![
        metric("Report Mode", "Detail"),
        metric("Trainer", "Activity report"),
        metric("Viewer", &format!("ID {viewer_id}")),
        metric("Club", "Club context"),
        metric("Score", "-"),
        metric("Score Band", "Review"),
        metric("Score Class", "score-watch"),
        metric("Verdict", "Activity pattern"),
        metric(
            "Summary",
            "Snapshot-based report from Top 100 club observations. This is context, not proof.",
        ),
        metric("Primary Reason", "Snapshot context"),
        metric("Days", "60 day window"),
        metric("Last Seen", "Last seen recently"),
        metric("Total Fan Gain", "tracked"),
        metric("Total Active", "observed"),
        metric("Careers/hr", "rate"),
        metric("Peak Fans/min", "peak"),
        metric("Recent 3d", "recent"),
        metric("Under-15m", "0"),
        metric("Short Score", "0.0"),
        metric("Short Avg", "tracked"),
        metric("P95 Short", "tracked"),
        metric("Last 20 Avg", "length"),
        metric("Weekly Buckets", "0 / 168"),
        metric("Daily Days", "60"),
        metric("Peak Daily", "fan gain"),
        metric("Peak Active Day", "active time"),
        metric("Heatmap Pattern", &fallback_heatmap_pattern()),
    ]
}

async fn fetch_activity_hall(
    client: &Client,
    config: &Config,
    pairs: &[(String, String)],
) -> Option<ActivityHallResponse> {
    let mut url =
        reqwest::Url::parse(&format!("{}/api/v4/shame/hall", config.api_base_url)).ok()?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("page", query_value(pairs, "page").unwrap_or("0"));
        query.append_pair("limit", "8");
        if let Some(sort) = activity_sort_param(pairs) {
            query.append_pair("sort_by", sort);
        }
        query.append_pair(
            "min_score",
            query_value(pairs, "minScore")
                .or_else(|| query_value(pairs, "min_score"))
                .unwrap_or("0"),
        );
        if let Some(min_days) =
            query_value(pairs, "minDays").or_else(|| query_value(pairs, "min_days"))
        {
            query.append_pair("min_days", min_days);
        }
        if let Some(search_query) = query_value(pairs, "query") {
            query.append_pair("query", search_query);
        }
    }

    fetch_json(client, url).await
}

fn activity_hall_metrics(
    response: ActivityHallResponse,
    pairs: &[(String, String)],
) -> Vec<EmbedMetric> {
    let mut metrics = fallback_activity_metrics(pairs);
    upsert_metric(&mut metrics, "Total", &format_number(response.total));
    if let Some(updated) = response.last_refreshed_at.as_deref() {
        upsert_metric(&mut metrics, "Updated", &format_updated_label(updated));
    }

    for (index, entry) in response.entries.into_iter().take(8).enumerate() {
        let row = index + 1;
        let score = entry.suspicion_score.unwrap_or_default();
        let top_reason = entry
            .evidence
            .as_ref()
            .and_then(|evidence| evidence.reasons.first());
        let reason_label = top_reason
            .and_then(|reason| reason.label.as_deref())
            .or_else(|| {
                entry
                    .evidence
                    .as_ref()
                    .and_then(|evidence| evidence.verdict.as_deref())
                    .map(activity_verdict_label)
            })
            .unwrap_or("Activity pattern");
        let reason_severity = top_reason
            .and_then(|reason| reason.severity.as_deref())
            .unwrap_or("medium");

        metrics.push(metric(&format!("Rank {row}"), &format!("#{row}")));
        metrics.push(metric(
            &format!("Trainer {row}"),
            &trainer_label(entry.trainer_name.clone(), entry.viewer_id),
        ));
        metrics.push(metric(
            &format!("Viewer {row}"),
            &entry
                .viewer_id
                .map(|id| format!("ID {id}"))
                .unwrap_or_else(|| "ID tracked".to_string()),
        ));
        metrics.push(metric(&format!("Club {row}"), &activity_club_label(&entry)));
        metrics.push(metric(
            &format!("Facts {row}"),
            &activity_facts_label(&entry),
        ));
        metrics.push(metric(&format!("Reason {row}"), reason_label));
        metrics.push(metric(&format!("Reason Severity {row}"), reason_severity));
        metrics.push(metric(
            &format!("Fan Gain {row}"),
            &compact_number(entry.total_fan_gain.unwrap_or_default()),
        ));
        metrics.push(metric(
            &format!("Active {row}"),
            &format_duration_compact(entry.total_active_seconds.unwrap_or_default()),
        ));
        metrics.push(metric(
            &format!("Careers/hr {row}"),
            &format_optional_decimal(entry.careers_per_active_hour),
        ));
        metrics.push(metric(&format!("Score {row}"), &score.to_string()));
        metrics.push(metric(
            &format!("Score Band {row}"),
            activity_score_band(score),
        ));
        metrics.push(metric(
            &format!("Score Class {row}"),
            activity_score_class(score),
        ));
    }

    metrics
}

fn fallback_activity_metrics(pairs: &[(String, String)]) -> Vec<EmbedMetric> {
    vec![
        metric("Total", "Live"),
        metric("Sort", activity_sort_label(activity_sort(pairs))),
        metric("Min Days", &activity_min_days_label(pairs)),
        metric("Min Score", &activity_min_score_label(pairs)),
        metric("Updated", "Snapshot data"),
    ]
}

fn activity_sort(pairs: &[(String, String)]) -> &'static str {
    match query_value(pairs, "sortBy").or_else(|| query_value(pairs, "sort_by")) {
        Some("behavior_change") => "behavior_change",
        Some("short_fan_gain") => "short_fan_gain",
        Some("short_high_fan") => "short_high_fan",
        Some("online_streak") => "online_streak",
        Some("max_session") => "max_session",
        Some("careers_per_hour") => "careers_per_hour",
        Some("avg_career_length") => "avg_career_length",
        Some("careers") => "careers",
        Some("active_time") => "active_time",
        Some("fans_per_minute") => "fans_per_minute",
        Some("peak_fans_per_minute") => "peak_fans_per_minute",
        _ => "score",
    }
}

fn activity_sort_param(pairs: &[(String, String)]) -> Option<&'static str> {
    let sort = activity_sort(pairs);
    (sort != "score").then_some(sort)
}

fn activity_sort_label(sort: &str) -> &'static str {
    match sort {
        "behavior_change" => "Behavior spike",
        "short_fan_gain" => "Short high-fan score",
        "short_high_fan" => "Short high-fan careers",
        "online_streak" => "Online streak",
        "active_time" => "Active time",
        "careers_per_hour" => "Careers/hour",
        "fans_per_minute" => "Fans/minute",
        "peak_fans_per_minute" => "Peak fans/minute",
        "avg_career_length" => "Shortest avg career",
        "careers" => "Total careers",
        "max_session" => "Longest session",
        _ => "Suspicion score",
    }
}

fn activity_min_days_label(pairs: &[(String, String)]) -> String {
    query_value(pairs, "minDays")
        .or_else(|| query_value(pairs, "min_days"))
        .filter(|days| !days.trim().is_empty())
        .map(|days| format!("{days}+ days"))
        .unwrap_or_else(|| "1+ days".to_string())
}

fn activity_min_score_label(pairs: &[(String, String)]) -> String {
    query_value(pairs, "minScore")
        .or_else(|| query_value(pairs, "min_score"))
        .filter(|score| !score.trim().is_empty() && *score != "0")
        .map(|score| format!("{score}+"))
        .unwrap_or_else(|| "All scores".to_string())
}

fn activity_club_label(entry: &ActivityHallEntry) -> String {
    let club = entry.circle_name.as_deref().unwrap_or("Club context");
    match entry.circle_monthly_rank {
        Some(rank) => format!("{club} #{rank}"),
        None => club.to_string(),
    }
}

fn activity_facts_label(entry: &ActivityHallEntry) -> String {
    let days = entry.days_observed.unwrap_or_default();
    let careers = entry.total_careers.unwrap_or_default();
    format!("{days}d observed | {} careers", compact_number(careers))
}

fn activity_verdict_label(verdict: &str) -> &'static str {
    match verdict {
        "strong_automation_signal" => "Automation-like pattern",
        "very_high_suspicion" => "Rate anomaly",
        "schedule_suspicion" => "Schedule pattern",
        "below_threshold" => "Below threshold",
        _ => "Activity pattern",
    }
}

fn activity_score_band(score: i64) -> &'static str {
    if score >= 90 {
        "Critical"
    } else if score >= 75 {
        "High"
    } else if score >= 60 {
        "Elevated"
    } else if score >= 40 {
        "Watch"
    } else {
        "Low"
    }
}

fn activity_score_class(score: i64) -> &'static str {
    if score >= 90 {
        "score-critical"
    } else if score >= 75 {
        "score-high"
    } else if score >= 60 {
        "score-elevated"
    } else if score >= 40 {
        "score-watch"
    } else {
        "score-low"
    }
}

fn format_duration_compact(seconds: i64) -> String {
    if seconds <= 0 {
        return "0m".to_string();
    }

    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;

    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{}m", minutes.max(1))
    }
}

fn format_optional_decimal(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "-".to_string())
}

fn format_updated_label(value: &str) -> String {
    value
        .split_once('T')
        .map(|(date, _)| format!("Updated {date}"))
        .unwrap_or_else(|| "Updated recently".to_string())
}

fn format_short_day_label(value: &str) -> String {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3 {
        return value.to_string();
    }

    let month = parts[1].parse::<i64>().ok().unwrap_or_default();
    let day = parts[2].trim_start_matches('0');
    let month = match month {
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
        _ => return value.to_string(),
    };

    format!("{month} {}", if day.is_empty() { "0" } else { day })
}

fn max_daily_gain(points: &[ActivityDailyPoint]) -> i64 {
    points
        .iter()
        .filter_map(|point| point.fan_gain)
        .max()
        .unwrap_or_default()
}

fn max_daily_active_seconds(points: &[ActivityDailyPoint]) -> i64 {
    points
        .iter()
        .filter_map(|point| point.active_seconds)
        .max()
        .unwrap_or_default()
}

fn heatmap_pattern(cells: &[ActivityHeatmapCell]) -> String {
    let max_active = cells
        .iter()
        .filter_map(|cell| cell.active_seconds)
        .max()
        .unwrap_or_default();
    if max_active <= 0 {
        return fallback_heatmap_pattern();
    }

    let mut levels = vec![0_u8; 7 * 24];
    for cell in cells {
        let Some(day) = cell.dow.filter(|day| *day < 7) else {
            continue;
        };
        let Some(hour) = cell.hour.filter(|hour| *hour < 24) else {
            continue;
        };
        let active = cell.active_seconds.unwrap_or_default();
        let level = if active <= 0 {
            0
        } else {
            (((active as f64 / max_active as f64) * 4.0).ceil() as u8).clamp(1, 4)
        };
        levels[day * 24 + hour] = level;
    }

    levels
        .into_iter()
        .map(|level| char::from(b'0' + level))
        .collect()
}

fn fallback_heatmap_pattern() -> String {
    let mut pattern = String::with_capacity(7 * 24);
    for day in 0..7 {
        for hour in 0..24 {
            let level = if (18..=23).contains(&hour) && day % 2 == 0 {
                3
            } else if (6..=9).contains(&hour) {
                1
            } else if (12..=16).contains(&hour) && day % 3 == 0 {
                2
            } else {
                0
            };
            pattern.push(char::from(b'0' + level));
        }
    }
    pattern
}

fn upsert_metric(metrics: &mut Vec<EmbedMetric>, label: &str, value: &str) {
    if let Some(metric) = metrics
        .iter_mut()
        .find(|metric| metric.label.eq_ignore_ascii_case(label))
    {
        metric.value = value.to_string();
    } else {
        metrics.push(metric(label, value));
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
            vec![
                metric("View", "Schedule"),
                metric("Server", "Global"),
                metric("Asset Base", &config.asset_base_url),
                metric("Frontend Origin", &config.frontend_origin),
            ],
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
            "Club Leaderboard | uma.moe",
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
            vec![
                metric("Tool", "Planner"),
                metric("Tree", "Inheritance"),
                metric("Asset Base", &config.asset_base_url),
            ],
        ),
        "privacy-policy" => (
            "Privacy Policy | uma.moe",
            "Read how uma.moe handles site data, stored preferences, account identifiers, and privacy-sensitive information.",
            vec![
                metric("Policy", "Privacy"),
                metric("Scope", "uma.moe"),
                metric("Data", "Site usage"),
            ],
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
        database: None,
        tierlist: None,
        resources: ResourceCatalog::default(),
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
        database: None,
        tierlist: None,
        resources: ResourceCatalog::default(),
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

    let response = match client.get(&url).send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(%error, %url, "failed to fetch profile metadata");
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!(%status, %url, "profile metadata API returned non-success status");
        return None;
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            warn!(%error, %url, "failed to read profile metadata response body");
            return None;
        }
    };

    match serde_json::from_str::<UserProfileResponse>(&body) {
        Ok(profile) => Some(profile),
        Err(error) => {
            warn!(%error, %url, "failed to parse profile metadata response");
            None
        }
    }
}

async fn fetch_circle(client: &Client, config: &Config, circle_id: &str) -> Option<CircleDetails> {
    let response = fetch_circle_response(client, config, circle_id, None).await?;
    let fallback_year_month = if response.members.is_empty() {
        response
            .circle
            .last_updated
            .as_deref()
            .and_then(parse_year_month)
    } else {
        None
    };

    if let Some(year_month) = fallback_year_month {
        if let Some(monthly_response) =
            fetch_circle_response(client, config, circle_id, Some(year_month)).await
        {
            if !monthly_response.members.is_empty() {
                return Some(circle_from_response(monthly_response));
            }
        }
    }

    Some(circle_from_response(response))
}

async fn fetch_circle_response(
    client: &Client,
    config: &Config,
    circle_id: &str,
    year_month: Option<(i64, i64)>,
) -> Option<CircleDetailsResponse> {
    let mut url = reqwest::Url::parse(&format!("{}/api/v4/circles", config.api_base_url)).ok()?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("circle_id", circle_id);
        if let Some((year, month)) = year_month {
            query.append_pair("year", &year.to_string());
            query.append_pair("month", &month.to_string());
        }
    }

    fetch_json(client, url).await
}

fn circle_from_response(response: CircleDetailsResponse) -> CircleDetails {
    let mut circle = response.circle;
    circle.members = response.members;
    circle.club_rank = response.club_rank.or(circle.club_rank);
    circle.min_rank = response.min_rank.or(circle.min_rank);
    circle.max_rank = response.max_rank.or(circle.max_rank);
    circle.fans_to_next_tier = response.fans_to_next_tier.or(circle.fans_to_next_tier);
    circle.fans_to_lower_tier = response.fans_to_lower_tier.or(circle.fans_to_lower_tier);
    circle.yesterday_fans_to_next_tier = response
        .yesterday_fans_to_next_tier
        .or(circle.yesterday_fans_to_next_tier);
    circle.yesterday_fans_to_lower_tier = response
        .yesterday_fans_to_lower_tier
        .or(circle.yesterday_fans_to_lower_tier);
    circle
}

#[derive(Serialize)]
struct CircleMemberGainDataset {
    name: String,
    data: Vec<Option<i64>>,
    total: i64,
}

struct CircleMemberGainChart {
    labels: Vec<String>,
    datasets: Vec<CircleMemberGainDataset>,
    period: String,
}

fn push_circle_member_gain_metrics(
    metrics: &mut Vec<EmbedMetric>,
    members: &[CircleMemberMonthlyData],
) {
    let Some(chart) = circle_member_gain_chart(members) else {
        metrics.push(metric("Member Gain Count", "0"));
        return;
    };

    if let Ok(labels) = serde_json::to_string(&chart.labels) {
        metrics.push(metric("Member Gain Labels", &labels));
    }
    if let Ok(datasets) = serde_json::to_string(&chart.datasets) {
        metrics.push(metric("Member Gain Series", &datasets));
    }
    if let Some((day_gain, week_gain)) = circle_member_gain_summary(&chart) {
        metrics.push(metric("Member Day Gain", &signed_compact_number(day_gain)));
        metrics.push(metric(
            "Member Week Gain",
            &signed_compact_number(week_gain),
        ));
    }
    metrics.push(metric(
        "Member Gain Count",
        &chart.datasets.len().to_string(),
    ));
    metrics.push(metric("Member Gain Period", &chart.period));
}

fn circle_member_gain_summary(chart: &CircleMemberGainChart) -> Option<(i64, i64)> {
    let latest_index = chart.labels.len().checked_sub(1)?;
    let latest = circle_member_gain_total_at(&chart.datasets, latest_index);
    if latest <= 0 {
        return None;
    }

    let day_start = latest_index.saturating_sub(1);
    let week_start = latest_index.saturating_sub(7);
    let day_gain = latest.saturating_sub(circle_member_gain_total_at(&chart.datasets, day_start));
    let week_gain = latest.saturating_sub(circle_member_gain_total_at(&chart.datasets, week_start));

    Some((day_gain, week_gain))
}

fn circle_member_gain_total_at(datasets: &[CircleMemberGainDataset], index: usize) -> i64 {
    datasets
        .iter()
        .filter_map(|dataset| member_gain_value_at_or_before(&dataset.data, index))
        .sum()
}

fn member_gain_value_at_or_before(values: &[Option<i64>], index: usize) -> Option<i64> {
    let capped = index.min(values.len().saturating_sub(1));
    (0..=capped)
        .rev()
        .find_map(|index| values.get(index).copied().flatten())
}

fn circle_member_gain_chart(members: &[CircleMemberMonthlyData]) -> Option<CircleMemberGainChart> {
    let (year, month) = latest_member_year_month(members)?;
    let month_members = members
        .iter()
        .filter(|member| member.year == Some(year) && member.month == Some(month))
        .collect::<Vec<_>>();
    let month_members = if month_members.is_empty() {
        members.iter().collect::<Vec<_>>()
    } else {
        month_members
    };
    let days_in_month = days_in_month(year, month)?;
    let max_index_with_data = month_members
        .iter()
        .filter_map(|member| last_non_zero_index(&member.daily_fans))
        .max()
        .unwrap_or_default();
    let has_next_month_fallback = month_members.iter().any(|member| {
        can_use_next_month_start_fallback(
            &member.daily_fans,
            member.next_month_start,
            days_in_month,
        )
    });
    let days_to_show = if has_next_month_fallback {
        days_in_month
    } else {
        max_index_with_data
    };
    if days_to_show <= 0 {
        return None;
    }

    let labels = (1..=days_to_show)
        .map(|day| format!("{day:02}.{month:02}"))
        .collect::<Vec<_>>();

    let mut datasets = month_members
        .into_iter()
        .filter_map(|member| member_gain_dataset(member, days_to_show, days_in_month))
        .collect::<Vec<_>>();
    datasets.sort_by(|left, right| {
        right
            .total
            .cmp(&left.total)
            .then_with(|| left.name.cmp(&right.name))
    });
    datasets.truncate(30);

    if datasets.is_empty() {
        return None;
    }

    Some(CircleMemberGainChart {
        labels,
        datasets,
        period: format!("{} {year}", month_label(month)),
    })
}

fn member_gain_dataset(
    member: &CircleMemberMonthlyData,
    days_to_show: i64,
    days_in_month: i64,
) -> Option<CircleMemberGainDataset> {
    let name = member
        .trainer_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(str::trim)
        .map(str::to_string)
        .or_else(|| member.viewer_id.map(|id| id.to_string()))?;
    if member.daily_fans.iter().all(|value| *value == 0) {
        return None;
    }

    let data = (0..days_to_show)
        .map(|index| member_cumulative_gain(member, index as usize, days_in_month))
        .collect::<Vec<_>>();
    let total = data
        .iter()
        .rev()
        .find_map(|value| *value)
        .unwrap_or_default();
    if total <= 0 && data.iter().all(Option::is_none) {
        return None;
    }

    Some(CircleMemberGainDataset { name, data, total })
}

fn member_cumulative_gain(
    member: &CircleMemberMonthlyData,
    index: usize,
    days_in_month: i64,
) -> Option<i64> {
    let baseline =
        member
            .daily_fans
            .iter()
            .find_map(|value| if *value > 0 { Some(*value) } else { None })?;
    let data_index = index + 1;
    let has_future = has_future_current_club_fans(&member.daily_fans, data_index);
    let value = member
        .daily_fans
        .get(data_index)
        .copied()
        .filter(|value| *value > 0)
        .or_else(|| {
            if has_future {
                latest_current_club_fans_at_or_before(&member.daily_fans, data_index)
            } else {
                None
            }
        })
        .or_else(|| {
            if can_use_next_month_start_fallback(
                &member.daily_fans,
                member.next_month_start,
                days_in_month,
            ) && !has_future
            {
                member.next_month_start
            } else {
                None
            }
        })?;

    Some(value.saturating_sub(baseline).max(0))
}

fn has_future_current_club_fans(values: &[i64], index: usize) -> bool {
    values.iter().skip(index + 1).any(|value| *value > 0)
}

fn latest_current_club_fans_at_or_before(values: &[i64], index: usize) -> Option<i64> {
    values
        .iter()
        .take(index + 1)
        .rev()
        .find_map(|value| if *value > 0 { Some(*value) } else { None })
}

fn latest_member_year_month(members: &[CircleMemberMonthlyData]) -> Option<(i64, i64)> {
    members
        .iter()
        .filter_map(|member| Some((member.year?, member.month?)))
        .max()
}

fn last_non_zero_index(values: &[i64]) -> Option<i64> {
    values
        .iter()
        .rposition(|value| *value != 0)
        .map(|index| index as i64)
}

fn can_use_next_month_start_fallback(
    daily_fans: &[i64],
    next_month_start: Option<i64>,
    days_in_month: i64,
) -> bool {
    next_month_start.is_some_and(|value| value > 0)
        && !daily_fans
            .get(days_in_month as usize)
            .is_some_and(|value| *value > 0)
}

fn days_in_month(year: i64, month: i64) -> Option<i64> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

async fn fetch_circle_list(
    client: &Client,
    config: &Config,
    pairs: &[(String, String)],
) -> Option<CircleListResponse> {
    let mut url =
        reqwest::Url::parse(&format!("{}/api/v4/circles/list", config.api_base_url)).ok()?;
    {
        let sort = query_value(pairs, "sortBy")
            .or_else(|| query_value(pairs, "sort_by"))
            .unwrap_or("rank");
        let sort_dir = query_value(pairs, "sortOrder")
            .or_else(|| query_value(pairs, "sort_dir"))
            .unwrap_or("asc");
        let mut query = url.query_pairs_mut();
        query.append_pair("page", query_value(pairs, "page").unwrap_or("0"));
        query.append_pair("limit", "10");
        query.append_pair("sort_by", sort);
        query.append_pair("sort_dir", sort_dir);

        for key in ["name", "query"] {
            if let Some(value) = query_value(pairs, key) {
                query.append_pair(key, value);
            }
        }
    }

    fetch_json(client, url).await
}

fn circle_list_metrics(
    response: CircleListResponse,
    pairs: &[(String, String)],
    config: &Config,
) -> Vec<EmbedMetric> {
    let CircleListResponse {
        circles,
        list,
        total,
        total_count,
    } = response;
    let rows = if circles.is_empty() { list } else { circles };
    let total = total
        .as_ref()
        .and_then(value_as_i64)
        .or_else(|| total_count.as_ref().and_then(value_as_i64))
        .unwrap_or(rows.len() as i64);

    let mut metrics = circle_list_base_metrics(pairs, config, total, rows.len());
    for (index, circle) in rows.into_iter().take(10).enumerate() {
        push_circle_row_metrics(&mut metrics, index + 1, &circle);
    }

    metrics
}

fn fallback_circle_list_metrics(config: &Config) -> Vec<EmbedMetric> {
    let mut metrics = circle_list_base_metrics(&[], config, 100, 10);
    let fallback_rows = [
        CircleDetails {
            circle_id: Some(772781438),
            name: Some("Uma Utopia".to_string()),
            leader_name: Some("ItsJustWDSam".to_string()),
            member_count: Some(30),
            join_style: Some(2),
            policy: Some(3),
            monthly_rank: Some(1),
            monthly_point: Some(1_620_000_000),
            yesterday_rank: Some(2),
            yesterday_points: Some(1_582_000_000),
            live_rank: Some(1),
            live_points: Some(1_648_000_000),
            club_rank: Some(1),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(418820337),
            name: Some("Sprint Stars".to_string()),
            leader_name: Some("Bakushin!".to_string()),
            member_count: Some(29),
            join_style: Some(1),
            policy: Some(12),
            monthly_rank: Some(2),
            monthly_point: Some(1_514_000_000),
            yesterday_rank: Some(3),
            yesterday_points: Some(1_488_000_000),
            live_rank: Some(2),
            live_points: Some(1_531_000_000),
            club_rank: Some(2),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(620114882),
            name: Some("Blue Roses".to_string()),
            leader_name: Some("RiceFan".to_string()),
            member_count: Some(30),
            join_style: Some(3),
            policy: Some(8),
            monthly_rank: Some(3),
            monthly_point: Some(1_442_000_000),
            yesterday_rank: Some(1),
            yesterday_points: Some(1_431_000_000),
            live_rank: Some(3),
            live_points: Some(1_448_000_000),
            club_rank: Some(2),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(889244120),
            name: Some("Dream Gate".to_string()),
            leader_name: Some("TeioStep".to_string()),
            member_count: Some(27),
            join_style: Some(1),
            policy: Some(4),
            monthly_rank: Some(4),
            monthly_point: Some(1_308_000_000),
            yesterday_rank: Some(4),
            yesterday_points: Some(1_289_000_000),
            live_rank: Some(5),
            live_points: Some(1_318_000_000),
            club_rank: Some(3),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(735120448),
            name: Some("Morning Run".to_string()),
            leader_name: Some("Maya".to_string()),
            member_count: Some(25),
            join_style: Some(2),
            policy: Some(17),
            monthly_rank: Some(5),
            monthly_point: Some(1_255_000_000),
            yesterday_rank: Some(7),
            yesterday_points: Some(1_241_000_000),
            live_rank: Some(4),
            live_points: Some(1_269_000_000),
            club_rank: Some(3),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(540903147493),
            name: Some("Green Sprint".to_string()),
            leader_name: Some("Falcon".to_string()),
            member_count: Some(22),
            join_style: Some(1),
            policy: Some(5),
            monthly_rank: Some(6),
            monthly_point: Some(1_196_000_000),
            yesterday_rank: Some(6),
            yesterday_points: Some(1_187_000_000),
            live_rank: Some(6),
            live_points: Some(1_202_000_000),
            club_rank: Some(4),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(990411782),
            name: Some("Training Camp".to_string()),
            leader_name: Some("McQueen".to_string()),
            member_count: Some(30),
            join_style: Some(2),
            policy: Some(10),
            monthly_rank: Some(7),
            monthly_point: Some(1_150_000_000),
            yesterday_rank: Some(5),
            yesterday_points: Some(1_145_000_000),
            live_rank: Some(7),
            live_points: Some(1_156_000_000),
            club_rank: Some(4),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(310884520),
            name: Some("Victory Road".to_string()),
            leader_name: Some("Oguri".to_string()),
            member_count: Some(28),
            join_style: Some(1),
            policy: Some(6),
            monthly_rank: Some(8),
            monthly_point: Some(1_104_000_000),
            yesterday_rank: Some(8),
            yesterday_points: Some(1_099_000_000),
            live_rank: Some(8),
            live_points: Some(1_110_000_000),
            club_rank: Some(5),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(774200118),
            name: Some("Starlight Derby".to_string()),
            leader_name: Some("Opera".to_string()),
            member_count: Some(26),
            join_style: Some(2),
            policy: Some(13),
            monthly_rank: Some(9),
            monthly_point: Some(1_072_000_000),
            yesterday_rank: Some(10),
            yesterday_points: Some(1_064_000_000),
            live_rank: Some(9),
            live_points: Some(1_079_000_000),
            club_rank: Some(5),
            ..CircleDetails::default()
        },
        CircleDetails {
            circle_id: Some(681190405),
            name: Some("Meadow Bells".to_string()),
            leader_name: Some("Urara".to_string()),
            member_count: Some(24),
            join_style: Some(1),
            policy: Some(2),
            monthly_rank: Some(10),
            monthly_point: Some(1_041_000_000),
            yesterday_rank: Some(9),
            yesterday_points: Some(1_038_000_000),
            live_rank: Some(10),
            live_points: Some(1_045_000_000),
            club_rank: Some(6),
            ..CircleDetails::default()
        },
    ];

    for (index, circle) in fallback_rows.into_iter().enumerate() {
        push_circle_row_metrics(&mut metrics, index + 1, &circle);
    }

    metrics
}

fn circle_list_base_metrics(
    pairs: &[(String, String)],
    config: &Config,
    total: i64,
    row_count: usize,
) -> Vec<EmbedMetric> {
    let mode = if query_value(pairs, "query")
        .or_else(|| query_value(pairs, "name"))
        .is_some()
    {
        "Search"
    } else {
        "Top 100"
    };
    let sort = query_value(pairs, "sortBy")
        .or_else(|| query_value(pairs, "sort_by"))
        .map(circle_sort_label)
        .unwrap_or("Rank");

    vec![
        metric("Mode", mode),
        metric("Total", &format_number(total)),
        metric("Rows", &row_count.to_string()),
        metric("Sort", sort),
        metric("Asset Base", &config.asset_base_url),
    ]
}

fn push_circle_row_metrics(metrics: &mut Vec<EmbedMetric>, row: usize, circle: &CircleDetails) {
    let rank = circle
        .monthly_rank
        .or(circle.live_rank)
        .map(|rank| format!("#{rank}"))
        .unwrap_or_else(|| format!("#{row}"));
    let name = circle
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            circle
                .circle_id
                .map(|id| format!("Club {id}"))
                .unwrap_or_else(|| format!("Club {row}"))
        });
    let leader = circle
        .leader_name
        .as_deref()
        .filter(|leader| !leader.trim().is_empty())
        .map(str::to_string)
        .or_else(|| circle.leader_viewer_id.map(|id| id.to_string()))
        .unwrap_or_else(|| "Leader".to_string());
    let members = circle
        .member_count
        .map(|members| format!("{members}/30"))
        .unwrap_or_else(|| "--/30".to_string());
    let join = circle
        .join_style
        .map(join_style_label)
        .unwrap_or("Unknown")
        .to_string();
    let policy = circle
        .policy
        .map(policy_label)
        .unwrap_or("Playstyle")
        .to_string();
    let club_rank = circle
        .club_rank
        .map(club_rank_label)
        .unwrap_or_else(|| "Rank".to_string());
    let club_rank_id = circle
        .club_rank
        .map(|rank| rank.to_string())
        .unwrap_or_default();
    let points = circle
        .last_month_point
        .or(circle.monthly_point)
        .or(circle.live_points)
        .unwrap_or_default();
    let daily = match (circle.monthly_point, circle.yesterday_points) {
        (Some(current), Some(previous)) => signed_compact_number(current - previous),
        _ => "Daily".to_string(),
    };
    let today = match (circle.live_points, circle.monthly_point) {
        (Some(live), Some(monthly)) if live >= monthly => signed_compact_number(live - monthly),
        _ => "Live".to_string(),
    };
    let delta = match (circle.monthly_rank, circle.yesterday_rank) {
        (Some(current), Some(previous)) if previous != current => {
            let movement = previous - current;
            if movement > 0 {
                format!("+{movement}")
            } else {
                movement.to_string()
            }
        }
        _ => "0".to_string(),
    };

    metrics.push(metric(&format!("Rank {row}"), &rank));
    if let Some(yesterday_rank) = circle.yesterday_rank {
        metrics.push(metric(
            &format!("Yesterday Rank {row}"),
            &format!("#{yesterday_rank}"),
        ));
    }
    metrics.push(metric(&format!("Delta {row}"), &delta));
    metrics.push(metric(&format!("Club {row}"), &name));
    if let Some(comment) = circle
        .comment
        .as_deref()
        .filter(|comment| !comment.trim().is_empty())
    {
        metrics.push(metric(&format!("Comment {row}"), comment));
    }
    metrics.push(metric(&format!("Leader {row}"), &leader));
    metrics.push(metric(&format!("Members {row}"), &members));
    metrics.push(metric(&format!("Join {row}"), &join));
    metrics.push(metric(&format!("Policy {row}"), &policy));
    metrics.push(metric(&format!("Club Rank {row}"), &club_rank));
    metrics.push(metric(&format!("Club Rank Id {row}"), &club_rank_id));
    metrics.push(metric(&format!("Points {row}"), &compact_number(points)));
    if let Some(max_rank) = circle.max_rank {
        metrics.push(metric(
            &format!("Lower Cutoff Rank {row}"),
            &format!("#{max_rank}"),
        ));
    }
    if let Some(min_rank) = circle.min_rank {
        metrics.push(metric(
            &format!("Upper Cutoff Rank {row}"),
            &format!("#{min_rank}"),
        ));
    }
    if let Some(lower_gap) = circle.fans_to_lower_tier {
        metrics.push(metric(
            &format!("Lower Gap {row}"),
            &compact_number(lower_gap),
        ));
    }
    if let (Some(current), Some(previous)) = (
        circle.fans_to_lower_tier,
        circle.yesterday_fans_to_lower_tier,
    ) {
        metrics.push(metric(
            &format!("Lower Gap Delta {row}"),
            &signed_compact_number(current - previous),
        ));
    }
    if let Some(upper_gap) = circle.fans_to_next_tier {
        metrics.push(metric(
            &format!("Upper Gap {row}"),
            &compact_number(upper_gap),
        ));
    }
    if let (Some(current), Some(previous)) =
        (circle.fans_to_next_tier, circle.yesterday_fans_to_next_tier)
    {
        metrics.push(metric(
            &format!("Upper Gap Delta {row}"),
            &signed_compact_number(current - previous),
        ));
    }
    metrics.push(metric(&format!("Daily {row}"), &daily));
    metrics.push(metric(&format!("Today {row}"), &today));
}

fn circle_sort_label(value: &str) -> &'static str {
    match value {
        "fan_count" | "monthly_point" | "points" => "Fans",
        "name" => "Name",
        "members" | "member_count" => "Members",
        "live_rank" => "Live Rank",
        _ => "Rank",
    }
}

async fn fetch_site_stats(client: &Client, config: &Config) -> Option<SiteStatsResponse> {
    let cache = SITE_STATS_CACHE.get_or_init(|| Mutex::new(None));
    if let Some(stats) = cache.lock().ok().and_then(|guard| {
        guard.as_ref().and_then(|entry| {
            if entry.api_base_url == config.api_base_url
                && entry.fetched_at.elapsed() < SITE_STATS_CACHE_TTL
            {
                Some(entry.stats.clone())
            } else {
                None
            }
        })
    }) {
        return Some(stats);
    }

    let url = format!("{}/api/stats?days=30", config.api_base_url);
    let started_at = Instant::now();

    let response = match client.get(url.clone()).send().await {
        Ok(response) => response,
        Err(error) => {
            debug!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "site stats request failed"
            );
            return None;
        }
    };
    let status = response.status();
    if !status.is_success() {
        debug!(
            %status,
            %url,
            elapsed_ms = started_at.elapsed().as_millis(),
            "site stats request returned non-success status"
        );
        return None;
    }

    let stats = match response.json::<SiteStatsResponse>().await {
        Ok(stats) => stats,
        Err(error) => {
            debug!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "site stats response did not match expected schema"
            );
            return None;
        }
    };

    debug!(
        %url,
        elapsed_ms = started_at.elapsed().as_millis(),
        "site stats request completed"
    );

    if let Ok(mut guard) = cache.lock() {
        *guard = Some(SiteStatsCacheEntry {
            api_base_url: config.api_base_url.clone(),
            fetched_at: Instant::now(),
            stats: stats.clone(),
        });
    }

    Some(stats)
}

async fn fetch_database_preview(
    client: &Client,
    config: &Config,
    params: &[(String, String)],
) -> Option<DatabaseSearchPreview> {
    let mut url = reqwest::Url::parse(&format!("{}/search/query", config.search_base_url)).ok()?;
    for (key, value) in params {
        url.query_pairs_mut().append_pair(key, value);
    }

    let started_at = Instant::now();
    let response = match client.get(url.clone()).send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "database embed preview request failed"
            );
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!(
            %status,
            %url,
            elapsed_ms = started_at.elapsed().as_millis(),
            "database embed preview request returned non-success status"
        );
        return None;
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            warn!(
                %error,
                %url,
                elapsed_ms = started_at.elapsed().as_millis(),
                "database embed preview response body failed"
            );
            return None;
        }
    };

    let response = match serde_json::from_str::<DatabaseSearchResponse>(&body) {
        Ok(response) => response,
        Err(error) => {
            let body_preview = truncate_chars(&body, 400);
            warn!(
                %error,
                %url,
                %body_preview,
                elapsed_ms = started_at.elapsed().as_millis(),
                "database embed preview response did not match expected schema"
            );
            return None;
        }
    };

    debug!(
        %url,
        total = response.total,
        elapsed_ms = started_at.elapsed().as_millis(),
        "database embed preview request completed"
    );

    if response.items.is_empty() {
        warn!(%url, total = response.total, "database embed preview returned no items");
        return None;
    }

    let top_result = response
        .items
        .into_iter()
        .find(|item| item.inheritance.is_some());

    if top_result.is_none() {
        warn!(%url, total = response.total, "database embed preview returned no inheritance result");
    }

    Some(DatabaseSearchPreview {
        total: response.total,
        top_result,
    })
}

async fn fetch_resource_catalog(client: &Client, config: &Config) -> ResourceCatalog {
    let cache = RESOURCE_CACHE.get_or_init(|| Mutex::new(None));
    if let Some(catalog) = cache.lock().ok().and_then(|guard| {
        guard.as_ref().and_then(|entry| {
            let cache_matches = entry.base_url == config.resources_base_url
                && entry.token == config.resources_api_token;
            if cache_matches {
                Some(entry.catalog.clone())
            } else {
                None
            }
        })
    }) {
        return catalog;
    }

    let mut catalog = ResourceCatalog::default();
    let (characters, factors, skills, support_cards, affinity, race_program) = tokio::join!(
        fetch_resource_json::<Vec<ResourceCharacterRaw>>(client, config, "character"),
        fetch_resource_json::<Vec<ResourceFactorRaw>>(client, config, "factors"),
        fetch_resource_json::<Vec<ResourceSkillRaw>>(client, config, "skills"),
        fetch_resource_json::<Vec<ResourceSupportCardRaw>>(client, config, "support-cards-db"),
        fetch_resource_json::<ResourceAffinityRaw>(client, config, "affinity"),
        fetch_resource_json::<ResourceRaceProgramRaw>(client, config, "race_program"),
    );

    if let Some(characters) = characters {
        for character in characters {
            let Some(card_id) = value_as_i64(&character.id) else {
                continue;
            };
            if card_id <= 0 {
                continue;
            }

            let name = character
                .name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| format!("Uma #{card_id}"));
            let image = character
                .image
                .filter(|image| !image.trim().is_empty())
                .unwrap_or_else(|| format!("chara_stand_{card_id}.webp"));

            catalog
                .characters
                .insert(card_id, ResourceCharacter { name, image });
        }
    }

    if let Some(factors) = factors {
        for factor in factors {
            let Ok(factor_id) = factor.id.parse::<i64>() else {
                continue;
            };
            if factor_id <= 0 || factor.text.trim().is_empty() {
                continue;
            }

            catalog.factors.insert(
                factor_id,
                ResourceFactor {
                    text: factor.text,
                    factor_type: factor
                        .factor_type
                        .as_ref()
                        .and_then(value_as_i64)
                        .unwrap_or_else(|| infer_resource_factor_type(factor_id)),
                },
            );
        }
    }

    if let Some(skills) = skills {
        for skill in skills {
            let Some(skill_id) = skill.skill_id.as_ref().and_then(value_as_i64) else {
                continue;
            };
            let Some(name) = skill.name.filter(|name| !name.trim().is_empty()) else {
                continue;
            };

            catalog.skills.insert(skill_id, ResourceSkill { name });
        }
    }

    if let Some(support_cards) = support_cards {
        for support_card in support_cards {
            let Some(support_card_id) = value_as_i64(&support_card.id) else {
                continue;
            };
            let Some(name) = support_card.name.filter(|name| !name.trim().is_empty()) else {
                continue;
            };

            catalog
                .support_cards
                .insert(support_card_id, ResourceSupportCard { name });
        }
    }

    if let Some(affinity) = affinity {
        if !affinity.chars.is_empty() && !affinity.aff2.is_empty() {
            catalog.affinity = Some(AffinityMatrix::new(
                affinity.chars,
                affinity.aff2,
                affinity.aff3,
            ));
        }
    }

    if let Some(race_program) = race_program {
        for entry in race_program.races.into_values() {
            let Some(saddle_id) = entry.id.as_ref().and_then(value_as_i64) else {
                continue;
            };
            let Some(race_instance_id) = entry.race_instance_id.as_ref().and_then(value_as_i64)
            else {
                continue;
            };
            if saddle_id <= 0 || race_instance_id <= 0 {
                continue;
            }

            catalog
                .race_instance_saddles
                .entry(race_instance_id)
                .or_default()
                .push(saddle_id);
        }

        for saddle_ids in catalog.race_instance_saddles.values_mut() {
            saddle_ids.sort_unstable();
            saddle_ids.dedup();
        }
    }

    if catalog.has_any_data() {
        if let Ok(mut guard) = cache.lock() {
            *guard = Some(ResourceCacheEntry {
                base_url: config.resources_base_url.clone(),
                token: config.resources_api_token.clone(),
                catalog: catalog.clone(),
            });
        }
    } else {
        warn!(
            base_url = %config.resources_base_url,
            "resource catalog warm/fetch returned no data; leaving cache empty so a later request can retry"
        );
    }

    catalog
}

async fn fetch_banner_timeline_details(
    client: &Client,
    config: &Config,
) -> Option<TimelineEmbedDetails> {
    let cache = BANNER_TIMELINE_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some(entry) = guard.as_ref() {
            let cache_matches = entry.base_url == config.resources_base_url
                && entry.token == config.resources_api_token;
            if cache_matches {
                return entry.details.clone();
            }
        }
    }

    let details = fetch_resource_json::<Value>(client, config, "banner_timeline")
        .await
        .and_then(timeline_details_from_value);

    if details.is_some() {
        if let Ok(mut guard) = cache.lock() {
            *guard = Some(BannerTimelineCacheEntry {
                base_url: config.resources_base_url.clone(),
                token: config.resources_api_token.clone(),
                details: details.clone(),
            });
        }
    }

    details
}

#[cfg(test)]
fn timeline_details_from_raw(raw: BannerTimelineRaw) -> Option<TimelineEmbedDetails> {
    let mut events = raw
        .events
        .into_iter()
        .filter_map(timeline_event_from_raw)
        .collect::<Vec<_>>();

    events.sort_by(|left, right| {
        left.global_release_date
            .cmp(&right.global_release_date)
            .then_with(|| left.title.cmp(&right.title))
    });

    if events.is_empty() {
        None
    } else {
        Some(TimelineEmbedDetails { events })
    }
}

#[cfg(test)]
fn timeline_event_from_raw(raw: BannerTimelineEventRaw) -> Option<TimelineEventDetails> {
    let event_type = raw.event_type.trim();
    if event_type.is_empty() {
        return None;
    }

    let title = raw.title.trim();
    let global_release_date = raw.global_release_date.as_deref()?.trim();
    if title.is_empty() || global_release_date.len() < 10 {
        return None;
    }

    Some(TimelineEventDetails {
        event_type: event_type.to_string(),
        title: title.to_string(),
        description: raw
            .description
            .map(|description| description.trim().to_string())
            .filter(|description| !description.is_empty()),
        image_path: raw
            .image_path
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty()),
        global_release_date: global_release_date.to_string(),
        estimated_end_date: raw
            .estimated_end_date
            .map(|date| date.trim().to_string())
            .filter(|date| date.len() >= 10),
        is_confirmed: raw.is_confirmed,
        pickup_card_ids: raw.pickup_card_ids,
        related_characters: raw.related_characters,
        related_support_cards: raw.related_support_cards,
        prediction_kind: raw
            .prediction
            .as_ref()
            .and_then(|prediction| prediction.kind.clone())
            .filter(|kind| !kind.trim().is_empty()),
        prediction_likelihood: raw
            .prediction
            .and_then(|prediction| prediction.calendar_likelihood)
            .and_then(|likelihood| likelihood.score),
    })
}

fn timeline_details_from_value(value: Value) -> Option<TimelineEmbedDetails> {
    let events = match value {
        Value::Array(events) => events,
        Value::Object(mut object) => object.remove("events")?.as_array()?.clone(),
        _ => return None,
    };

    let mut events = events
        .into_iter()
        .filter_map(timeline_event_from_value)
        .collect::<Vec<_>>();

    events.sort_by(|left, right| {
        left.global_release_date
            .cmp(&right.global_release_date)
            .then_with(|| left.title.cmp(&right.title))
    });

    if events.is_empty() {
        None
    } else {
        Some(TimelineEmbedDetails { events })
    }
}

fn timeline_event_from_value(value: Value) -> Option<TimelineEventDetails> {
    let object = value.as_object()?;
    let event_type = string_field(object, &["type", "event_type", "eventType"])?;
    let title = string_field(object, &["title", "name"])?;
    let global_release_date = string_field(
        object,
        &[
            "global_release_date",
            "globalReleaseDate",
            "global_date",
            "globalDate",
        ],
    )?;

    if event_type.trim().is_empty()
        || title.trim().is_empty()
        || global_release_date.trim().len() < 10
    {
        return None;
    }

    let prediction = object.get("prediction").and_then(Value::as_object);
    let calendar_likelihood = prediction
        .and_then(|prediction| prediction.get("calendar_likelihood"))
        .or_else(|| prediction.and_then(|prediction| prediction.get("calendarLikelihood")))
        .and_then(Value::as_object);

    Some(TimelineEventDetails {
        event_type: event_type.trim().to_string(),
        title: title.trim().to_string(),
        description: timeline_description_from_value(object),
        image_path: timeline_image_path_from_value(object),
        global_release_date: global_release_date.trim().to_string(),
        estimated_end_date: string_field(
            object,
            &[
                "estimated_end_date",
                "estimatedEndDate",
                "end_date",
                "endDate",
            ],
        )
        .map(str::trim)
        .filter(|date| date.len() >= 10)
        .map(str::to_string),
        is_confirmed: bool_field(object, &["is_confirmed", "isConfirmed", "confirmed"])
            .unwrap_or_default(),
        pickup_card_ids: number_list_field(object, &["pickup_card_ids", "pickupCardIds"]),
        related_characters: string_list_field(object, &["related_characters", "relatedCharacters"]),
        related_support_cards: string_list_field(
            object,
            &["related_support_cards", "relatedSupportCards"],
        ),
        prediction_kind: prediction
            .and_then(|prediction| string_field(prediction, &["kind", "type"]))
            .map(str::trim)
            .filter(|kind| !kind.is_empty())
            .map(str::to_string),
        prediction_likelihood: calendar_likelihood
            .and_then(|likelihood| likelihood.get("score"))
            .and_then(value_as_f64),
    })
}

fn timeline_image_path_from_value(object: &serde_json::Map<String, Value>) -> Option<String> {
    const IMAGE_KEYS: &[&str] = &[
        "image_path",
        "imagePath",
        "image",
        "image_url",
        "imageUrl",
        "image_webp",
        "imageWebp",
        "webp",
        "banner",
        "banner_url",
        "bannerUrl",
        "banner_image",
        "bannerImage",
        "banner_image_path",
        "bannerImagePath",
        "story_banner",
        "storyBanner",
        "story_image",
        "storyImage",
        "event_image",
        "eventImage",
        "thumbnail",
        "thumbnail_url",
        "thumbnailUrl",
    ];
    const IMAGE_OBJECT_KEYS: &[&str] = &["images", "image", "assets", "asset", "banner", "media"];
    const NESTED_KEYS: &[&str] = &[
        "path",
        "url",
        "src",
        "href",
        "webp",
        "default",
        "banner",
        "banner_path",
        "bannerPath",
        "banner_url",
        "bannerUrl",
        "story",
        "story_banner",
        "storyBanner",
        "event",
    ];

    string_field(object, IMAGE_KEYS)
        .or_else(|| nested_string_field(object, IMAGE_OBJECT_KEYS, NESTED_KEYS))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
}

fn timeline_description_from_value(object: &serde_json::Map<String, Value>) -> Option<String> {
    string_or_string_list_field(
        object,
        &[
            "description",
            "content",
            "detail",
            "details",
            "race_description",
            "raceDescription",
            "race_conditions",
            "raceConditions",
            "conditions",
            "condition",
            "course_description",
            "courseDescription",
        ],
    )
    .map(|description| description.trim().to_string())
    .filter(|description| !description.is_empty())
}

fn string_field<'a>(object: &'a serde_json::Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|key| object.get(*key))
        .filter_map(Value::as_str)
        .find(|value| !value.trim().is_empty())
}

fn nested_string_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    object_keys: &[&str],
    value_keys: &[&str],
) -> Option<&'a str> {
    object_keys
        .iter()
        .filter_map(|key| object.get(*key))
        .filter_map(Value::as_object)
        .find_map(|nested| string_field(nested, value_keys))
}

fn string_or_string_list_field(
    object: &serde_json::Map<String, Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .filter_map(|key| object.get(*key))
        .find_map(|value| match value {
            Value::String(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
            Value::Array(values) => {
                let values = values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>();
                (!values.is_empty()).then(|| values.join("\n"))
            }
            _ => None,
        })
}

fn bool_field(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .filter_map(|key| object.get(*key))
        .find_map(|value| {
            value.as_bool().or_else(|| {
                value
                    .as_str()
                    .and_then(|value| value.trim().parse::<bool>().ok())
            })
        })
}

fn number_list_field(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Vec<i64> {
    keys.iter()
        .filter_map(|key| object.get(*key))
        .find_map(|value| match value {
            Value::Array(values) => Some(values.iter().filter_map(value_as_i64).collect()),
            Value::String(value) => Some(
                value
                    .split(',')
                    .filter_map(|part| part.trim().parse::<i64>().ok())
                    .collect(),
            ),
            _ => value_as_i64(value).map(|value| vec![value]),
        })
        .unwrap_or_default()
}

fn string_list_field(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .filter_map(|key| object.get(*key))
        .find_map(|value| match value {
            Value::Array(values) => Some(
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
            Value::String(value) => Some(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

async fn fetch_resource_json<T: DeserializeOwned>(
    client: &Client,
    config: &Config,
    resource_name: &str,
) -> Option<T> {
    let url = format!(
        "{}/current/{}.json.gz",
        config.resources_base_url, resource_name
    );
    let mut request = client.get(url.clone());
    if let Some(token) = &config.resources_api_token {
        request = request.header("X-API-Key", token).bearer_auth(token);
    }

    let started_at = Instant::now();
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            warn!(
                %error,
                %url,
                resource = resource_name,
                elapsed_ms = started_at.elapsed().as_millis(),
                "resource catalog request failed"
            );
            return None;
        }
    };

    let status = response.status();
    if !status.is_success() {
        warn!(
            %status,
            %url,
            resource = resource_name,
            elapsed_ms = started_at.elapsed().as_millis(),
            "resource catalog request returned non-success status"
        );
        return None;
    }

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => {
            warn!(
                %error,
                %url,
                resource = resource_name,
                elapsed_ms = started_at.elapsed().as_millis(),
                "failed to read resource catalog response body"
            );
            return None;
        }
    };
    let byte_count = bytes.len();

    match decode_resource_json::<T>(&bytes) {
        Ok(resource) => {
            debug!(
                %url,
                resource = resource_name,
                bytes = byte_count,
                elapsed_ms = started_at.elapsed().as_millis(),
                "resource catalog request completed"
            );
            Some(resource)
        }
        Err(error) => {
            warn!(
                %error,
                %url,
                resource = resource_name,
                elapsed_ms = started_at.elapsed().as_millis(),
                "resource catalog response did not match expected schema"
            );
            None
        }
    }
}

fn decode_resource_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(bytes);
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .context("failed to decompress gzipped resource JSON")?;
        serde_json::from_slice(&decoded).context("failed to parse decompressed resource JSON")
    } else {
        serde_json::from_slice(bytes).context("failed to parse resource JSON")
    }
}

#[cfg(test)]
fn database_search_params_from_query(query: &str) -> Vec<(String, String)> {
    database_search_params_from_query_with_resources(query, None)
}

fn database_search_params_from_query_with_resources(
    query: &str,
    resources: Option<&ResourceCatalog>,
) -> Vec<(String, String)> {
    let query_pairs: Vec<(String, String)> = form_urlencoded::parse(query.as_bytes())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();

    let mut params = vec![
        ("page".to_string(), "0".to_string()),
        ("limit".to_string(), "1".to_string()),
        ("search_type".to_string(), "inheritance".to_string()),
        ("sort_by".to_string(), "affinity_score".to_string()),
        ("sort_order".to_string(), "desc".to_string()),
        ("max_follower_num".to_string(), "999".to_string()),
    ];

    for (key, value) in &query_pairs {
        if key == "filters" || key == "page" || key == "limit" || key.starts_with("__") {
            continue;
        }

        if (key == "query" || key == "name") && !value.trim().is_empty() {
            let target = if value.chars().all(|ch| ch.is_ascii_digit()) {
                "trainer_id"
            } else {
                "trainer_name"
            };
            set_param(&mut params, target, value);
            continue;
        }

        if is_direct_database_query_param(key) && !value.trim().is_empty() {
            set_param(&mut params, key, value);
        }
    }

    if let Some(filter_state) = query_pairs
        .iter()
        .find(|(key, _)| key == "filters")
        .and_then(|(_, value)| decode_database_filter_state(value))
    {
        apply_database_filter_state(&mut params, &filter_state, resources);
    }

    params
}

fn apply_database_filter_state(
    params: &mut Vec<(String, String)>,
    state: &Value,
    resources: Option<&ResourceCatalog>,
) {
    if let Some(uql) = state
        .get("uql")
        .and_then(Value::as_str)
        .filter(|uql| !uql.trim().is_empty())
    {
        set_param(params, "uql", uql);
    }

    append_factor_groups(params, state, resources, "b", "blue_sparks", 0, 9);
    append_factor_groups(params, state, resources, "p", "pink_sparks", 1, 9);
    append_factor_groups(params, state, resources, "g", "green_sparks", 5, 9);
    append_factor_groups(params, state, resources, "w", "white_sparks", 2, 9);

    append_flat_factor_ids(
        params,
        state,
        resources,
        "mb",
        "main_parent_blue_sparks",
        0,
        3,
    );
    append_flat_factor_ids(
        params,
        state,
        resources,
        "mp",
        "main_parent_pink_sparks",
        1,
        3,
    );
    append_flat_factor_ids(
        params,
        state,
        resources,
        "mg",
        "main_parent_green_sparks",
        5,
        3,
    );
    append_factor_groups(
        params,
        state,
        resources,
        "mw",
        "main_parent_white_sparks",
        2,
        3,
    );

    set_param_array(params, state, "ow", "optional_white_sparks");
    set_param_array(params, state, "omw", "optional_main_white_sparks");
    set_param_array(params, state, "lw", "lineage_white");
    set_param_array(params, state, "ip", "parent_id");
    set_param_array(params, state, "ep", "exclude_parent_id");
    set_param_array(params, state, "emp", "exclude_main_parent_id");

    if let Some(tree) = state.get("t").and_then(Value::as_array) {
        if let Some(player_chara_id) = number_at(tree, 0) {
            set_param(params, "player_chara_id", &player_chara_id.to_string());
        }
        if let Some(main_parent_id) = number_at(tree, 1) {
            set_param(params, "main_parent_id", &main_parent_id.to_string());
        }
        if let Some(parent_left_id) = number_at(tree, 2) {
            set_param(params, "parent_left_id", &parent_left_id.to_string());
        }
        if let Some(parent_right_id) = number_at(tree, 3) {
            set_param(params, "parent_right_id", &parent_right_id.to_string());
        }
    }

    if let Some(include_main_ids) = number_array(state.get("imp")) {
        append_unique_csv_values(params, "main_parent_id", &include_main_ids);
    }

    append_race_schedule_saddles(params, state, resources);

    set_param_string(params, state, "sc", "support_card_id");
    set_param_number(params, state, "lb", "min_limit_break");
    set_param_string(params, state, "uid", "trainer_id");
    set_param_string(params, state, "un", "trainer_name");
    set_param_number(params, state, "mwc", "min_win_count");
    set_param_number(params, state, "mwh", "min_white_count");
    set_param_number(params, state, "pr", "parent_rank");
    set_param_number(params, state, "mf", "max_follower_num");
    set_param_number(params, state, "mmwc", "min_main_white_count");
    set_clamped_star_sum(params, state, "bss", "min_blue_stars_sum", Some(9));
    set_clamped_star_sum(params, state, "pss", "min_pink_stars_sum", Some(9));
    set_clamped_star_sum(params, state, "gss", "min_green_stars_sum", Some(9));
    set_clamped_star_sum(params, state, "wss", "min_white_stars_sum", None);
}

fn decode_database_filter_state(value: &str) -> Option<Value> {
    let normalized = value.replace(' ', "+");
    let bytes = BASE64_STANDARD.decode(normalized).ok()?;
    let decoded = String::from_utf8(bytes).ok()?;
    serde_json::from_str(&decoded).ok()
}

fn is_direct_database_query_param(key: &str) -> bool {
    matches!(
        key,
        "trainer_id"
            | "trainer_name"
            | "uql"
            | "player_chara_id"
            | "desired_main_chara_id"
            | "affinity_p2"
            | "main_parent_id"
            | "parent_left_id"
            | "parent_right_id"
            | "parent_id"
            | "exclude_parent_id"
            | "exclude_main_parent_id"
            | "parent_rank"
            | "parent_rarity"
            | "blue_sparks"
            | "pink_sparks"
            | "green_sparks"
            | "white_sparks"
            | "main_parent_blue_sparks"
            | "main_parent_pink_sparks"
            | "main_parent_green_sparks"
            | "main_parent_white_sparks"
            | "optional_white_sparks"
            | "optional_main_white_sparks"
            | "lineage_white"
            | "main_legacy_white"
            | "left_legacy_white"
            | "right_legacy_white"
            | "min_main_blue_factors"
            | "min_main_pink_factors"
            | "min_main_green_factors"
            | "min_main_white_count"
            | "min_win_count"
            | "min_white_count"
            | "support_card_id"
            | "min_limit_break"
            | "min_blue_stars_sum"
            | "min_pink_stars_sum"
            | "min_green_stars_sum"
            | "min_white_stars_sum"
            | "main_win_saddle"
            | "p2_main_chara_id"
            | "p2_win_saddle"
            | "max_follower_num"
            | "sort_by"
            | "sort_order"
    )
}

fn has_meaningful_database_search_params(params: &[(String, String)]) -> bool {
    params.iter().any(|(key, _)| {
        !matches!(
            key.as_str(),
            "page" | "limit" | "search_type" | "sort_by" | "sort_order" | "max_follower_num"
        )
    })
}

fn database_query_label(params: &[(String, String)], resources: &ResourceCatalog) -> String {
    if let Some(support_card_id) = param_value(params, "support_card_id") {
        return database_support_card_query_label(resources, support_card_id);
    }

    if let Some(trainer_id) = param_value(params, "trainer_id") {
        return format!("trainer {trainer_id}");
    }

    if let Some(trainer_name) = param_value(params, "trainer_name") {
        return format!("trainer {trainer_name}");
    }

    if param_value(params, "uql").is_some() {
        return "UQL search".to_string();
    }

    if let Some(main_parent_id) = param_value(params, "main_parent_id") {
        return database_character_query_label(resources, "main parent", main_parent_id);
    }

    if let Some(player_chara_id) = param_value(params, "player_chara_id") {
        return database_character_query_label(resources, "character", player_chara_id);
    }

    "shared filters".to_string()
}

fn database_support_card_query_label(resources: &ResourceCatalog, support_card_id: &str) -> String {
    parse_integer_label(support_card_id)
        .and_then(|id| resources.support_card_name(id))
        .map(|name| format!("support card {name}"))
        .unwrap_or_else(|| format!("support card {support_card_id}"))
}

fn database_character_query_label(
    resources: &ResourceCatalog,
    prefix: &str,
    character_id: &str,
) -> String {
    parse_integer_label(character_id)
        .and_then(|id| resources.character_name(id))
        .map(|name| format!("{prefix} {name}"))
        .unwrap_or_else(|| format!("{prefix} {character_id}"))
}

fn database_character_embed_label(resources: &ResourceCatalog, character_id: i64) -> String {
    resources
        .character_name(character_id)
        .map(str::to_string)
        .unwrap_or_else(|| format!("Uma #{character_id}"))
}

fn database_result_position_label(total: i64) -> String {
    if total <= 0 {
        "No matching results.".to_string()
    } else if total > 10_000 {
        "Result 1 of 10,000+.".to_string()
    } else {
        format!("Result 1 of {}.", format_number(total))
    }
}

fn database_query_highlights(params: &[(String, String)]) -> DatabaseQueryHighlights {
    let mut matched_factor_ids = Vec::new();
    let mut matched_main_factor_ids = Vec::new();
    let matched_support_card_id =
        param_value(params, "support_card_id").and_then(parse_integer_label);
    let matched_min_limit_break = param_value(params, "min_limit_break")
        .and_then(parse_integer_label)
        .map(|value| value.clamp(0, 4));

    for (key, value) in params {
        if !is_factor_filter_param(key) {
            continue;
        }

        let ids = parse_csv_numbers(value)
            .into_iter()
            .map(normalize_query_factor_id)
            .filter(|id| *id > 0)
            .collect::<Vec<_>>();

        matched_factor_ids.extend(ids.iter().copied());
        if is_main_factor_filter_param(key) {
            matched_main_factor_ids.extend(ids);
        }
    }

    matched_factor_ids.sort_unstable();
    matched_factor_ids.dedup();
    matched_main_factor_ids.sort_unstable();
    matched_main_factor_ids.dedup();

    DatabaseQueryHighlights {
        matched_factor_ids,
        matched_main_factor_ids,
        matched_support_card_id,
        matched_min_limit_break,
    }
}

fn is_factor_filter_param(key: &str) -> bool {
    matches!(
        key,
        "blue_sparks"
            | "pink_sparks"
            | "green_sparks"
            | "white_sparks"
            | "main_parent_blue_sparks"
            | "main_parent_pink_sparks"
            | "main_parent_green_sparks"
            | "main_parent_white_sparks"
            | "optional_white_sparks"
            | "optional_main_white_sparks"
            | "lineage_white"
            | "main_legacy_white"
            | "left_legacy_white"
            | "right_legacy_white"
    )
}

fn is_main_factor_filter_param(key: &str) -> bool {
    matches!(
        key,
        "main_parent_blue_sparks"
            | "main_parent_pink_sparks"
            | "main_parent_green_sparks"
            | "main_parent_white_sparks"
            | "optional_main_white_sparks"
            | "main_legacy_white"
    )
}

fn normalize_query_factor_id(id: i64) -> i64 {
    if id < 100 {
        return id;
    }

    let level = id.rem_euclid(10);
    if (1..=9).contains(&level) {
        id / 10
    } else {
        id
    }
}

fn parse_csv_numbers(value: &str) -> Vec<i64> {
    value
        .split(',')
        .filter_map(|part| part.trim().parse::<i64>().ok())
        .collect()
}

fn clean_query(query: Option<&str>, debug_query_key: &str) -> Option<String> {
    let query = query?.trim();
    if query.is_empty() {
        return None;
    }

    let pairs: Vec<(String, String)> = form_urlencoded::parse(query.as_bytes())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .filter(|(key, _)| key != debug_query_key && !key.starts_with("__"))
        .collect();

    if pairs.is_empty() {
        return None;
    }

    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        serializer.append_pair(&key, &value);
    }

    Some(serializer.finish())
}

fn absolute_url_with_query(config: &Config, path: &str, query: Option<&str>) -> String {
    let base = absolute_url(config, path);
    match query.filter(|query| !query.trim().is_empty()) {
        Some(query) => format!("{base}?{query}"),
        None => base,
    }
}

fn image_url_with_query(config: &Config, kind: &str, id: &str, query: Option<&str>) -> String {
    let base = image_url(config, kind, id);
    match query.filter(|query| !query.trim().is_empty()) {
        Some(query) => format!("{base}?{query}"),
        None => base,
    }
}

fn set_param(params: &mut Vec<(String, String)>, key: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }

    params.retain(|(existing_key, _)| existing_key != key);
    params.push((key.to_string(), value.to_string()));
}

fn append_param(params: &mut Vec<(String, String)>, key: &str, value: &str) {
    if !value.trim().is_empty() {
        params.push((key.to_string(), value.to_string()));
    }
}

fn set_param_string(params: &mut Vec<(String, String)>, state: &Value, compact: &str, key: &str) {
    if let Some(value) = state
        .get(compact)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        set_param(params, key, value);
    }
}

fn set_param_number(params: &mut Vec<(String, String)>, state: &Value, compact: &str, key: &str) {
    if let Some(value) = state.get(compact).and_then(value_as_i64) {
        set_param(params, key, &value.to_string());
    }
}

fn set_param_array(params: &mut Vec<(String, String)>, state: &Value, compact: &str, key: &str) {
    let Some(values) = number_array(state.get(compact)) else {
        return;
    };

    if !values.is_empty() {
        set_param(params, key, &csv_numbers(&values));
    }
}

fn append_race_schedule_saddles(
    params: &mut Vec<(String, String)>,
    state: &Value,
    resources: Option<&ResourceCatalog>,
) {
    let Some(resources) = resources else {
        return;
    };
    let Some(schedule) = state.get("rs").and_then(Value::as_array) else {
        return;
    };

    let mut saddle_ids = Vec::new();
    for entry in schedule {
        let Some(entry) = entry.as_array() else {
            continue;
        };
        let Some(race_instance_id) = number_at(entry, 3) else {
            continue;
        };

        saddle_ids.extend(resources.saddle_ids_for_race_instance(race_instance_id));
    }

    if !saddle_ids.is_empty() {
        append_unique_csv_values(params, "main_win_saddle", &saddle_ids);
    }
}

fn set_clamped_star_sum(
    params: &mut Vec<(String, String)>,
    state: &Value,
    compact: &str,
    key: &str,
    max_value: Option<i64>,
) {
    let Some(mut value) = state.get(compact).and_then(value_as_i64) else {
        return;
    };

    if value <= 0 {
        return;
    }

    if let Some(max_value) = max_value {
        value = value.min(max_value);
    }

    set_param(params, key, &value.to_string());
}

fn append_factor_groups(
    params: &mut Vec<(String, String)>,
    state: &Value,
    resources: Option<&ResourceCatalog>,
    compact: &str,
    key: &str,
    factor_type: i64,
    max_level: i64,
) {
    let Some(groups) = state.get(compact).and_then(Value::as_array) else {
        return;
    };

    for group in groups {
        if let Some(ids) = factor_entry_ids(group, max_level, factor_type, resources) {
            append_param(params, key, &csv_numbers(&ids));
        }
    }
}

fn append_flat_factor_ids(
    params: &mut Vec<(String, String)>,
    state: &Value,
    resources: Option<&ResourceCatalog>,
    compact: &str,
    key: &str,
    factor_type: i64,
    max_level: i64,
) {
    let Some(groups) = state.get(compact).and_then(Value::as_array) else {
        return;
    };

    let mut ids = Vec::new();
    let mut min_factor_count: Option<i64> = None;

    for group in groups {
        if let Some(minimum) = factor_entry_min(group) {
            let minimum = minimum.min(max_level);
            min_factor_count =
                Some(min_factor_count.map_or(minimum, |current| current.max(minimum)));
        }

        if let Some(group_ids) = factor_entry_ids(group, max_level, factor_type, resources) {
            ids.extend(group_ids);
        }
    }

    ids.sort_unstable();
    ids.dedup();

    if !ids.is_empty() {
        set_param(params, key, &csv_numbers(&ids));
    }

    if let Some(min_factor_count) = min_factor_count {
        if let Some(min_key) = match key {
            "main_parent_blue_sparks" => Some("min_main_blue_factors"),
            "main_parent_pink_sparks" => Some("min_main_pink_factors"),
            "main_parent_green_sparks" => Some("min_main_green_factors"),
            _ => None,
        } {
            set_param(params, min_key, &min_factor_count.to_string());
        }
    }
}

fn factor_entry_ids(
    entry: &Value,
    max_level: i64,
    factor_type: i64,
    resources: Option<&ResourceCatalog>,
) -> Option<Vec<i64>> {
    let entry = entry.as_array()?;
    let factor_id = number_at(entry, 0)?;

    let min_level = number_at(entry, 1).unwrap_or(1).max(1);
    if min_level > max_level {
        return None;
    }

    let max_level = number_at(entry, 2)
        .unwrap_or(max_level)
        .min(max_level)
        .max(1);

    if min_level > max_level {
        return None;
    }

    let factor_ids = if factor_id > 0 {
        vec![factor_id]
    } else {
        factor_ids_for_type(factor_type, resources)
    };

    if factor_ids.is_empty() {
        return None;
    }

    let mut ids = factor_ids
        .into_iter()
        .flat_map(|factor_id| (min_level..=max_level).map(move |level| factor_id * 10 + level))
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    Some(ids)
}

fn factor_entry_min(entry: &Value) -> Option<i64> {
    let entry = entry.as_array()?;
    number_at(entry, 1).map(|value| value.max(1))
}

fn factor_ids_for_type(factor_type: i64, resources: Option<&ResourceCatalog>) -> Vec<i64> {
    let mut ids = resources
        .map(|resources| {
            resources
                .factors
                .iter()
                .filter_map(|(factor_id, factor)| {
                    let matches_type = if factor_type == 2 {
                        matches!(factor.factor_type, 2 | 3 | 4)
                    } else {
                        factor.factor_type == factor_type
                    };
                    matches_type.then_some(*factor_id)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if ids.is_empty() {
        ids = match factor_type {
            0 => vec![10, 20, 30, 40, 50],
            1 => vec![110, 120, 210, 220, 230, 240, 310, 320, 330, 340],
            _ => Vec::new(),
        };
    }

    ids.sort_unstable();
    ids.dedup();
    ids
}

fn number_at(values: &[Value], index: usize) -> Option<i64> {
    values.get(index).and_then(value_as_i64)
}

fn number_array(value: Option<&Value>) -> Option<Vec<i64>> {
    let value = value?;

    match value {
        Value::Array(items) => Some(items.iter().filter_map(value_as_i64).collect()),
        Value::String(value) => Some(
            value
                .split(',')
                .filter_map(|part| part.trim().parse::<i64>().ok())
                .collect(),
        ),
        _ => value_as_i64(value).map(|value| vec![value]),
    }
}

fn value_as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_u64().and_then(|value| i64::try_from(value).ok()))
            .or_else(|| {
                number.as_f64().and_then(|value| {
                    if value.is_finite() {
                        Some(value.trunc() as i64)
                    } else {
                        None
                    }
                })
            }),
        Value::String(value) => parse_integer_label(value),
        _ => None,
    }
}

fn parse_integer_label(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = trimmed.parse::<i64>() {
        return Some(value);
    }

    let normalized = trimmed.replace([',', '_'], "");
    if let Ok(value) = normalized.parse::<i64>() {
        return Some(value);
    }

    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("over ")
        || lower.starts_with("more than ")
        || lower.starts_with("at least ")
        || lower.starts_with(">=")
        || lower.starts_with('>')
    {
        return first_integer_in_label(&normalized);
    }

    None
}

fn first_integer_in_label(value: &str) -> Option<i64> {
    let mut start = None;
    let mut end = 0;

    for (index, character) in value.char_indices() {
        let is_sign = character == '-' && start.is_none();
        if character.is_ascii_digit() || is_sign {
            if start.is_none() {
                start = Some(index);
            }
            end = index + character.len_utf8();
        } else if start.is_some() {
            break;
        }
    }

    value.get(start?..end)?.parse::<i64>().ok()
}

fn value_as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64().filter(|value| value.is_finite()),
        Value::String(value) => value
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite()),
        _ => None,
    }
}

fn deserialize_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    value_as_i64(&value).ok_or_else(|| {
        serde::de::Error::custom(format!("expected integer-compatible value, got {value}"))
    })
}

fn deserialize_optional_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.as_ref().and_then(value_as_i64))
}

fn append_unique_csv_values(params: &mut Vec<(String, String)>, key: &str, values: &[i64]) {
    let mut combined = param_value(params, key)
        .map(|existing| {
            existing
                .split(',')
                .filter_map(|part| part.trim().parse::<i64>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    combined.extend(values.iter().copied());
    combined.sort_unstable();
    combined.dedup();

    if !combined.is_empty() {
        set_param(params, key, &csv_numbers(&combined));
    }
}

fn csv_numbers(values: &[i64]) -> String {
    values
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn param_value<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .rev()
        .find(|(existing_key, value)| existing_key == key && !value.trim().is_empty())
        .map(|(_, value)| value.as_str())
}

fn shared_win_count(primary: &[i64], secondary: &[i64]) -> i64 {
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

fn normalize_affinity_chara_id(chara_id: i64) -> i64 {
    if chara_id >= 10_000 {
        chara_id / 100
    } else {
        chara_id
    }
}

fn infer_resource_factor_type(factor_id: i64) -> i64 {
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

fn should_never_embed(path: &str) -> bool {
    let path = path.to_ascii_lowercase();

    path.starts_with("/api/")
        || path.starts_with("/assets/")
        || path.starts_with("/resources/")
        || path.starts_with("/ingest/")
        || path.starts_with("/status-api/")
        || path.starts_with("/__embeds/")
        || path == "/login"
        || path == "/settings"
        || path == "/wip"
        || path == "/signin"
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

fn signed_compact_number(value: i64) -> String {
    if value > 0 {
        format!("+{}", compact_number(value))
    } else {
        compact_number(value)
    }
}

fn signed_format_number(value: i64) -> String {
    if value > 0 {
        format!("+{}", format_number(value))
    } else {
        format_number(value)
    }
}

fn compact_float(value: f64) -> String {
    if value.is_finite() {
        compact_number(value.round() as i64)
    } else {
        "0".to_string()
    }
}

fn join_style_label(value: i64) -> &'static str {
    match value {
        1 => "Open",
        2 => "Approval",
        3 => "Closed",
        _ => "Unknown",
    }
}

fn club_rank_label(value: i64) -> String {
    if value <= 0 {
        return "Rank".to_string();
    }

    match value {
        1 => "D".to_string(),
        2 => "D+".to_string(),
        3 => "C".to_string(),
        4 => "C+".to_string(),
        5 => "B".to_string(),
        6 => "B+".to_string(),
        7 => "A".to_string(),
        8 => "A+".to_string(),
        9 => "S".to_string(),
        10 => "S+".to_string(),
        11 => "SS".to_string(),
        _ => format!("R{value}"),
    }
}

fn policy_label(value: i64) -> &'static str {
    match value {
        1 => "You Do You",
        2 => "Laid-back",
        3 => "Going for Gold",
        4 => "Beginners Welcome",
        5 => "Let's Party!",
        6 => "Rank 2000+",
        7 => "Rank 1000+",
        8 => "Rank 500+",
        9 => "Rank 250+",
        10 => "Rank 100+",
        11 => "Rank 20+",
        12 => "Log in Daily",
        13 => "Log in Every 3 Days",
        14 => "Active in the Morning",
        15 => "Active in the Afternoon",
        16 => "Active in the Evening",
        17 => "Active at Night",
        _ => "Playstyle",
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    truncated.push_str("...");
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

fn js_string_literal(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');

    for ch in value.chars() {
        match ch {
            '\\' => literal.push_str("\\\\"),
            '"' => literal.push_str("\\\""),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '<' => literal.push_str("\\u003C"),
            '>' => literal.push_str("\\u003E"),
            '&' => literal.push_str("\\u0026"),
            '\u{2028}' => literal.push_str("\\u2028"),
            '\u{2029}' => literal.push_str("\\u2029"),
            ch if ch.is_control() => literal.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => literal.push(ch),
        }
    }

    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            public_base_url: "https://uma.moe".to_string(),
            frontend_origin: "http://127.0.0.1:4200".to_string(),
            asset_base_url: "https://uma.moe/assets".to_string(),
            api_base_url: "http://umamoe-backend:3201".to_string(),
            search_base_url: "http://umamoe-search:3202".to_string(),
            resources_base_url: "http://umamoe-resources:3204/resources".to_string(),
            resources_api_token: None,
            bot_user_agent_tokens: vec![],
            debug_query_key: "__embed".to_string(),
            image_cache_max_age: std::time::Duration::from_secs(300),
            image_cache_stale_while_revalidate: std::time::Duration::from_secs(86_400),
            image_cache_max_entries: 256,
            render_max_concurrency: 1,
        }
    }

    fn test_meta(kind_label: &str, canonical_url: &str) -> EmbedMetadata {
        EmbedMetadata {
            title: format!("{kind_label} | uma.moe"),
            description: format!("{kind_label} preview"),
            canonical_url: canonical_url.to_string(),
            image_url: "https://uma.moe/__embeds/images/page/home.png".to_string(),
            image_alt: format!("{kind_label} preview image"),
            kind_label: kind_label.to_string(),
            metrics: vec![metric("View", kind_label)],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        }
    }

    #[test]
    fn profile_team_stadium_member_ignores_non_numeric_alias_values() {
        let json = r#"{
            "trainer": {
                "account_id": "203895087945",
                "name": "Tunnelblick",
                "leader_chara_dress_id": 101301
            },
            "team_stadium": [
                {
                    "cardId": {"unexpected": true},
                    "distanceType": "1",
                    "rankScore": "11700"
                },
                {
                    "trained_chara_id": 184,
                    "card_id": 103801,
                    "distance_type": 2,
                    "rank_score": 10800
                }
            ]
        }"#;

        let profile = serde_json::from_str::<UserProfileResponse>(json).unwrap();

        assert_eq!(profile.trainer.name.as_deref(), Some("Tunnelblick"));
        assert_eq!(profile.trainer.leader_chara_dress_id, Some(101301));
        assert_eq!(profile.team_stadium.len(), 2);
        assert_eq!(profile.team_stadium[0].character_id, None);
        assert_eq!(profile.team_stadium[0].distance_type, Some(1));
        assert_eq!(profile.team_stadium[0].rank_score, Some(11700));
        assert_eq!(profile.team_stadium[1].trained_chara_id, Some(184));
        assert_eq!(profile.team_stadium[1].card_id, Some(103801));
        assert_eq!(
            profile.team_stadium[1].stadium_character_asset_id(),
            Some(103801)
        );
    }

    #[test]
    fn embed_html_redirects_humans_to_canonical_url() {
        let meta = test_meta(
            "Database",
            "https://uma.moe/database?trainer_id=540903147493&blue_sparks=103",
        );

        let html = render_embed_html(&meta, true);
        assert!(html.contains(
            r#"window.location.replace("https://uma.moe/database?trainer_id=540903147493\u0026blue_sparks=103");"#
        ));
        assert!(html.contains(
            r#"<meta property="og:url" content="https://uma.moe/database?trainer_id=540903147493&amp;blue_sparks=103">"#
        ));
        assert!(html.contains(
            r#"<a class="open-link" href="https://uma.moe/database?trainer_id=540903147493&amp;blue_sparks=103">"#
        ));
        assert!(html.contains(
            r#"<body class="embed-redirect-page embed-kind-database embed-type-database embed-route-database">"#
        ));
    }

    #[test]
    fn embed_html_debug_mode_does_not_redirect() {
        let meta = test_meta("Club", "https://uma.moe/circles/772781438");

        let html = render_embed_html(&meta, false);
        assert!(!html.contains("window.location.replace"));
        assert!(html
            .contains(r#"<meta property="og:url" content="https://uma.moe/circles/772781438">"#));
        assert!(html.contains(r#"<a class="open-link" href="https://uma.moe/circles/772781438">"#));
        assert!(html.contains(
            r#"<body class="embed-redirect-page embed-kind-club embed-type-club embed-route-club">"#
        ));
    }

    #[test]
    fn embed_html_exposes_kind_classes_for_known_embeds() {
        let cases = [
            ("Home", "embed-kind-home"),
            ("Profile", "embed-kind-profile"),
            ("Veterans", "embed-kind-veterans"),
            ("Career Menu", "embed-kind-career-menu"),
            ("Achievements", "embed-kind-achievements"),
            ("Titles", "embed-kind-titles"),
            ("Club", "embed-kind-club"),
            ("Clubs", "embed-kind-clubs"),
            ("Database", "embed-kind-database"),
            ("Timeline", "embed-kind-timeline"),
            ("Tierlist", "embed-kind-tierlist"),
            ("Rankings", "embed-kind-rankings"),
            ("Activity", "embed-kind-activity"),
            ("Tools", "embed-kind-tools"),
            ("Statistics", "embed-kind-statistics"),
            ("Lineage Planner", "embed-kind-lineage-planner"),
            ("Privacy Policy", "embed-kind-privacy-policy"),
            ("uma.moe", "embed-kind-uma-moe"),
        ];

        for (kind_label, class_name) in cases {
            let html = render_embed_html(&test_meta(kind_label, "https://uma.moe/test"), false);
            assert!(
                html.contains(class_name),
                "{kind_label} should render {class_name} on the body"
            );
            assert!(
                html.contains("embed-type-page"),
                "{kind_label} should render a route type class"
            );
            assert!(
                html.contains("embed-route-test"),
                "{kind_label} should render a route class"
            );
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
    fn tierlist_query_preserves_image_cache_bust() {
        let meta = tierlist_page_metadata(&config(), Some("sfds&__embed=1"));

        assert_eq!(meta.canonical_url, "https://uma.moe/tierlist?sfds=");
        assert_eq!(
            meta.image_url,
            "https://uma.moe/__embeds/images/page/tierlist.png?sfds="
        );
        assert_eq!(meta.kind_label, "Tierlist");
    }

    #[test]
    fn recognizes_all_supported_public_embed_routes() {
        let cases = [
            ("/", "Home"),
            ("/profile/540903147493", "Profile"),
            ("/profile/540903147493/veterans", "Veterans"),
            ("/profile/540903147493/cm", "Career Menu"),
            ("/profile/540903147493/achievements", "Achievements"),
            ("/profile/540903147493/titles", "Titles"),
            ("/circles", "Clubs"),
            ("/circles/", "Clubs"),
            ("/circles/114701329", "Club"),
            ("/circles/114701329/members", "Club"),
            ("/database", "Database"),
            ("/database/", "Database"),
            ("/inheritance", "Database"),
            ("/support-cards", "Database"),
            ("/timeline", "Timeline"),
            ("/tierlist", "Tierlist"),
            ("/rankings", "Rankings"),
            ("/activity", "Activity"),
            ("/activity/540903147493", "Activity"),
            ("/shame", "Activity"),
            ("/shame/540903147493", "Activity"),
            ("/tools", "Tools"),
            ("/tools/statistics", "Statistics"),
            ("/tools/lineage-planner", "Lineage Planner"),
            ("/privacy-policy", "Privacy Policy"),
        ];

        for (path, kind_label) in cases {
            let route =
                metadata_route_for_path(path).unwrap_or_else(|| panic!("{path} should embed"));
            assert_eq!(
                route.kind_label(),
                kind_label,
                "{path} should resolve to {kind_label}"
            );
        }
    }

    #[test]
    fn ignores_assets_and_api() {
        assert!(should_never_embed("/assets/app.js"));
        assert!(should_never_embed("/api/v4/circles"));
        assert!(should_never_embed("/login"));
        assert!(should_never_embed("/settings"));
        assert!(should_never_embed("/wip"));
        assert!(should_never_embed("/signin"));
        assert!(!should_never_embed("/privacy-policy"));
        assert!(!should_never_embed("/circles/772781438"));
    }

    #[test]
    fn precomputed_tierlist_keeps_cards_in_their_lb4_tier() {
        let mut cards = BTreeMap::new();
        cards.insert(
            "30020".to_string(),
            PrecomputedTierlistCard {
                id: 30020,
                name: "Biko Pegasus".to_string(),
                card_type: 0,
                rarity: 3,
                scores: vec![0, 0, 0, 0, 3276],
                tiers: vec![
                    "A".to_string(),
                    "A".to_string(),
                    "A".to_string(),
                    "A".to_string(),
                    "S".to_string(),
                ],
            },
        );
        cards.insert(
            "30065".to_string(),
            PrecomputedTierlistCard {
                id: 30065,
                name: "Zenno Rob Roy".to_string(),
                card_type: 0,
                rarity: 3,
                scores: vec![0, 0, 0, 0, 3068],
                tiers: vec![
                    "A".to_string(),
                    "A".to_string(),
                    "A".to_string(),
                    "A".to_string(),
                    "A".to_string(),
                ],
            },
        );

        let details = build_tierlist_details(
            "https://uma.moe/assets",
            PrecomputedTierlistResponse {
                metadata: PrecomputedTierlistMetadata::default(),
                cards,
            },
        )
        .expect("tierlist details render from valid card data");

        let s_row = details
            .rows
            .iter()
            .find(|row| row.tier == "S")
            .expect("S row exists");
        let a_row = details
            .rows
            .iter()
            .find(|row| row.tier == "A")
            .expect("A row exists");

        assert_eq!(s_row.cards[0].name, "Biko Pegasus");
        assert_eq!(a_row.cards[0].name, "Zenno Rob Roy");
    }

    #[test]
    fn circle_list_metrics_prefer_last_month_points_and_tier_gap_deltas() {
        let metrics = circle_list_metrics(
            CircleListResponse {
                circles: vec![CircleDetails {
                    circle_id: Some(717148109),
                    name: Some("NFlight".to_string()),
                    member_count: Some(30),
                    monthly_rank: Some(1),
                    monthly_point: Some(6_200_000_000),
                    live_points: Some(6_300_000_000),
                    last_month_point: Some(5_900_000_000),
                    club_rank: Some(11),
                    min_rank: Some(1),
                    max_rank: Some(100),
                    fans_to_lower_tier: Some(100_000_000),
                    yesterday_fans_to_lower_tier: Some(75_000_000),
                    fans_to_next_tier: Some(0),
                    yesterday_fans_to_next_tier: Some(4_000_000),
                    ..CircleDetails::default()
                }],
                list: Vec::new(),
                total: None,
                total_count: None,
            },
            &[],
            &config(),
        );

        let metric = |label: &str| {
            metrics
                .iter()
                .find(|metric| metric.label == label)
                .map(|metric| metric.value.as_str())
        };

        assert_eq!(metric("Points 1"), Some("5.9B"));
        assert_eq!(metric("Lower Cutoff Rank 1"), Some("#100"));
        assert_eq!(metric("Upper Cutoff Rank 1"), Some("#1"));
        assert_eq!(metric("Lower Gap 1"), Some("100.0M"));
        assert_eq!(metric("Lower Gap Delta 1"), Some("+25.0M"));
        assert_eq!(metric("Upper Gap 1"), Some("0"));
        assert_eq!(metric("Upper Gap Delta 1"), Some("-4.0M"));
    }

    #[test]
    fn database_direct_query_builds_top_result_search() {
        let params =
            database_search_params_from_query("trainer_id=540903147493&page=4&limit=50&__embed=1");

        assert_eq!(param_value(&params, "page"), Some("0"));
        assert_eq!(param_value(&params, "limit"), Some("1"));
        assert_eq!(param_value(&params, "search_type"), Some("inheritance"));
        assert_eq!(param_value(&params, "trainer_id"), Some("540903147493"));
        assert_eq!(param_value(&params, "__embed"), None);
    }

    #[test]
    fn database_preview_accepts_over_count_labels() {
        let response: DatabaseSearchResponse = serde_json::from_value(serde_json::json!({
            "items": [
                {
                    "account_id": "540903147493",
                    "trainer_name": "UUC｜FishPineApl",
                    "follower_num": "855",
                    "inheritance": {
                        "inheritance_id": 46622776,
                        "main_parent_id": 106801,
                        "parent_left_id": 102401,
                        "parent_right_id": 106401,
                        "parent_rank": "14,616",
                        "parent_rarity": 15,
                        "win_count": "over 10000",
                        "white_count": "over 10,000",
                        "affinity_score": 72,
                        "blue_sparks": [203, 402],
                        "pink_sparks": [3202, 1103, 3302],
                        "green_sparks": [10680102],
                        "white_sparks": [2012701],
                        "main_win_saddles": [10],
                        "left_win_saddles": [10],
                        "right_win_saddles": [20],
                        "support_card_id": "30036",
                        "limit_break_count": "4"
                    }
                }
            ],
            "total": "over 10000"
        }))
        .expect("database search preview should accept human count labels");

        assert_eq!(response.total, 10_000);
        let item = response.items.first().expect("top item should deserialize");
        assert_eq!(item.follower_num, Some(855));
        let inheritance = item.inheritance.as_ref().expect("inheritance should parse");
        assert_eq!(inheritance.parent_rank, Some(14_616));
        assert_eq!(inheritance.win_count, Some(10_000));
        assert_eq!(inheritance.white_count, Some(10_000));
        assert_eq!(inheritance.support_card_id, Some(30036));
        assert_eq!(inheritance.limit_break_count, Some(4));
    }

    #[test]
    fn database_query_alias_targets_search_field() {
        let name_params = database_search_params_from_query("query=ItsJustWDSam");
        assert_eq!(
            param_value(&name_params, "trainer_name"),
            Some("ItsJustWDSam")
        );

        let id_params = database_search_params_from_query("query=540903147493");
        assert_eq!(param_value(&id_params, "trainer_id"), Some("540903147493"));
    }

    #[test]
    fn database_compact_filters_decode_to_backend_params() {
        let state = r#"{"uid":"540903147493","un":"Arumi","sc":"30039","lb":4,"mwc":18,"b":[[10,3,3]],"mb":[[20,2,3]],"ow":[1001,1002],"t":[9001,1001,2001,3001,null,null,null],"bss":12}"#;
        let encoded = BASE64_STANDARD.encode(state);
        let query = format!("filters={}", urlencoding::encode(&encoded));
        let params = database_search_params_from_query(&query);

        assert_eq!(param_value(&params, "trainer_id"), Some("540903147493"));
        assert_eq!(param_value(&params, "trainer_name"), Some("Arumi"));
        assert_eq!(param_value(&params, "support_card_id"), Some("30039"));
        assert_eq!(param_value(&params, "min_limit_break"), Some("4"));
        assert_eq!(param_value(&params, "min_win_count"), Some("18"));
        assert_eq!(param_value(&params, "blue_sparks"), Some("103"));
        assert_eq!(
            param_value(&params, "main_parent_blue_sparks"),
            Some("202,203")
        );
        assert_eq!(param_value(&params, "min_main_blue_factors"), Some("2"));
        assert_eq!(
            param_value(&params, "optional_white_sparks"),
            Some("1001,1002")
        );
        assert_eq!(param_value(&params, "player_chara_id"), Some("9001"));
        assert_eq!(param_value(&params, "main_parent_id"), Some("1001"));
        assert_eq!(param_value(&params, "parent_left_id"), Some("2001"));
        assert_eq!(param_value(&params, "parent_right_id"), Some("3001"));
        assert_eq!(param_value(&params, "min_blue_stars_sum"), Some("9"));
    }

    #[test]
    fn database_compact_any_factor_group_expands_and_highlights() {
        let state = r#"{"b":[[0,8,9]],"lb":4}"#;
        let encoded = BASE64_STANDARD.encode(state);
        let query = format!("filters={}", urlencoding::encode(&encoded));
        let params = database_search_params_from_query(&query);

        assert_eq!(
            param_value(&params, "blue_sparks"),
            Some("108,109,208,209,308,309,408,409,508,509")
        );
        assert_eq!(param_value(&params, "min_limit_break"), Some("4"));

        let highlights = database_query_highlights(&params);
        assert_eq!(highlights.matched_factor_ids, vec![10, 20, 30, 40, 50]);
        assert_eq!(highlights.matched_min_limit_break, Some(4));
    }

    #[test]
    fn database_compact_race_schedule_maps_to_saddles() {
        let mut resources = ResourceCatalog::default();
        resources.race_instance_saddles.insert(7001, vec![42, 43]);
        resources.race_instance_saddles.insert(7002, vec![44]);

        let state = r#"{"rs":[[0,1,1,7001],[0,2,2,7002],[0,3,1,9999]]}"#;
        let encoded = BASE64_STANDARD.encode(state);
        let query = format!(
            "main_win_saddle=40&filters={}",
            urlencoding::encode(&encoded)
        );
        let params = database_search_params_from_query_with_resources(&query, Some(&resources));

        assert_eq!(param_value(&params, "main_win_saddle"), Some("40,42,43,44"));
    }

    #[test]
    fn banner_timeline_resource_normalizes_events() {
        let raw = BannerTimelineRaw {
            events: vec![
                BannerTimelineEventRaw {
                    event_type: "character_banner".to_string(),
                    title: "Taiki Shuttle + Mejiro Dober".to_string(),
                    description: None,
                    image_path: Some("assets/images/character/banner/2022_30098.png".to_string()),
                    global_release_date: Some("2026-06-12T22:00:00Z".to_string()),
                    estimated_end_date: Some("2026-06-22T21:59:59Z".to_string()),
                    is_confirmed: true,
                    pickup_card_ids: vec![101002, 105902],
                    related_characters: vec![
                        "Taiki Shuttle".to_string(),
                        "Mejiro Dober".to_string(),
                    ],
                    related_support_cards: Vec::new(),
                    prediction: Some(BannerTimelinePredictionRaw {
                        kind: Some("confirmed".to_string()),
                        calendar_likelihood: Some(BannerTimelineLikelihoodRaw {
                            score: Some(0.82),
                        }),
                    }),
                },
                BannerTimelineEventRaw {
                    event_type: "story_event".to_string(),
                    title: "Seek, Solve, Summer Walk!".to_string(),
                    description: None,
                    image_path: Some(
                        "assets/images/story/06_seek_solve_summer_walk_banner.png".to_string(),
                    ),
                    global_release_date: Some("2026-06-12T22:00:00Z".to_string()),
                    estimated_end_date: Some("2026-06-23T21:59:59Z".to_string()),
                    is_confirmed: true,
                    pickup_card_ids: Vec::new(),
                    related_characters: Vec::new(),
                    related_support_cards: Vec::new(),
                    prediction: None,
                },
            ],
        };

        let details = timeline_details_from_raw(raw).expect("timeline events should normalize");

        assert_eq!(details.events.len(), 2);
        let character_event = details
            .events
            .iter()
            .find(|event| event.event_type == "character_banner")
            .expect("character banner should be retained");
        let story_event = details
            .events
            .iter()
            .find(|event| event.event_type == "story_event")
            .expect("story event should be retained");
        assert_eq!(
            character_event.image_path.as_deref(),
            Some("assets/images/character/banner/2022_30098.png")
        );
        assert_eq!(character_event.pickup_card_ids, vec![101002, 105902]);
        assert_eq!(character_event.prediction_likelihood, Some(0.82));
        assert_eq!(
            story_event.image_path.as_deref(),
            Some("assets/images/story/06_seek_solve_summer_walk_banner.png")
        );
    }

    #[test]
    fn banner_timeline_accepts_image_aliases() {
        let raw: BannerTimelineRaw = serde_json::from_value(serde_json::json!({
            "events": [
                {
                    "type": "story_event",
                    "title": "Seek, Solve, Summer Walk!",
                    "image": "assets/images/story/06_seek_solve_summer_walk_banner.webp",
                    "global_release_date": "2026-06-12T22:00:00Z"
                }
            ]
        }))
        .expect("timeline JSON should deserialize");

        let details = timeline_details_from_raw(raw).expect("timeline event should normalize");
        assert_eq!(
            details.events[0].image_path.as_deref(),
            Some("assets/images/story/06_seek_solve_summer_walk_banner.webp")
        );
    }

    #[test]
    fn banner_timeline_accepts_story_filename_and_champions_description() {
        let details = timeline_details_from_value(serde_json::json!({
            "events": [
                {
                    "type": "story_event",
                    "title": "Seek, Solve, Summer Walk!",
                    "story_banner": "06_seek_solve_summer_walk_banner.webp",
                    "global_release_date": "2026-06-11T22:00:00Z"
                },
                {
                    "type": "champions_meeting",
                    "title": "Champions Meeting: Cancer Cup",
                    "description": "Hanshin - Turf<br>2200m - Medium - Clockwise<br>Good - Summer - Cloudy</div>",
                    "global_release_date": "2026-06-21T22:00:00Z"
                }
            ]
        }))
        .expect("timeline events should parse");

        let story = details
            .events
            .iter()
            .find(|event| event.event_type == "story_event")
            .expect("story event should parse");
        let champions = details
            .events
            .iter()
            .find(|event| event.event_type == "champions_meeting")
            .expect("champions meeting should parse");

        assert_eq!(
            story.image_path.as_deref(),
            Some("06_seek_solve_summer_walk_banner.webp")
        );
        assert_eq!(
            champions.description.as_deref(),
            Some("Hanshin - Turf<br>2200m - Medium - Clockwise<br>Good - Summer - Cloudy</div>")
        );
    }

    #[test]
    fn banner_timeline_accepts_generated_resource_shape() {
        let details = timeline_details_from_value(serde_json::json!({
            "version": "test",
            "calculation": {
                "confirmed_anchor_count": 1
            },
            "events": [
                {
                    "type": "paid_banner",
                    "title": "Special Week + 8 more",
                    "image": "50001.png",
                    "image_path": "assets/images/paid/banner/50001.png",
                    "global_release_date": "2021-02-24T03:00:00Z",
                    "estimated_end_date": "2023-03-29T22:00:00Z",
                    "is_confirmed": true,
                    "pickup_card_ids": [100101, "100201"],
                    "related_characters": ["Special Week", "Silence Suzuka"],
                    "prediction": {
                        "kind": "confirmed",
                        "calendar_likelihood": {
                            "score": "0.82"
                        }
                    }
                }
            ]
        }))
        .expect("generated banner timeline shape should parse");

        assert_eq!(details.events.len(), 1);
        assert_eq!(details.events[0].event_type, "paid_banner");
        assert_eq!(
            details.events[0].image_path.as_deref(),
            Some("assets/images/paid/banner/50001.png")
        );
        assert_eq!(details.events[0].pickup_card_ids, vec![100101, 100201]);
        assert_eq!(details.events[0].prediction_likelihood, Some(0.82));
    }

    #[test]
    fn resource_json_decoder_accepts_raw_gzip_bytes() {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(br#"{"events":[{"type":"story_event","title":"Story","global_release_date":"2026-06-12T22:00:00Z"}]}"#)
            .expect("gzip test payload should write");
        let bytes = encoder.finish().expect("gzip test payload should finish");
        let decoded: Value = decode_resource_json(&bytes).expect("gzip resource should decode");

        assert!(decoded.get("events").and_then(Value::as_array).is_some());
    }

    #[test]
    fn resource_affinity_uses_parent_gp_pairs_and_race_overlap() {
        let n = 4;
        let mut aff2 = vec![0; n * n];
        let mut aff3 = vec![0; n * n * n];
        aff2[1] = 99;
        aff2[6] = 7;
        aff2[7] = 8;
        aff3[6] = 33;
        aff3[7] = 44;

        let resources = ResourceCatalog {
            affinity: Some(AffinityMatrix::new(vec![1, 2, 3, 4], aff2, aff3)),
            ..ResourceCatalog::default()
        };
        let breakdown = resources
            .affinity_breakdown_from_params(
                Some(2),
                Some(3),
                Some(4),
                &[10, 20, 30],
                &[10, 99],
                &[20],
            )
            .unwrap();

        assert_eq!(breakdown.main_total, 17);
        assert_eq!(breakdown.left_total, 8);
        assert_eq!(breakdown.right_total, 9);
        assert_eq!(breakdown.race_total, 2);
    }

    #[test]
    fn resource_affinity_without_target_uses_breeding_base_plus_race() {
        let n = 3;
        let mut aff2 = vec![0; n * n];
        aff2[1] = 7;
        aff2[2] = 8;

        let resources = ResourceCatalog {
            affinity: Some(AffinityMatrix::new(vec![2, 3, 4], aff2, vec![])),
            ..ResourceCatalog::default()
        };
        let breakdown = resources
            .affinity_breakdown_from_params(Some(2), Some(3), Some(4), &[10, 20], &[10], &[20])
            .unwrap();

        assert_eq!(breakdown.main_total, 17);
        assert_eq!(breakdown.left_total, 8);
        assert_eq!(breakdown.right_total, 9);
        assert_eq!(breakdown.race_total, 2);
    }

    #[test]
    fn database_result_prefers_race_inclusive_resource_affinity() {
        let n = 4;
        let mut aff2 = vec![0; n * n];
        let mut aff3 = vec![0; n * n * n];
        aff2[1] = 99;
        aff2[6] = 7;
        aff2[7] = 8;
        aff3[6] = 33;
        aff3[7] = 44;

        let resources = ResourceCatalog {
            characters: BTreeMap::from([
                (
                    2,
                    ResourceCharacter {
                        name: "Main Uma".to_string(),
                        image: "main.webp".to_string(),
                    },
                ),
                (
                    3,
                    ResourceCharacter {
                        name: "Left Uma".to_string(),
                        image: "left.webp".to_string(),
                    },
                ),
                (
                    4,
                    ResourceCharacter {
                        name: "Right Uma".to_string(),
                        image: "right.webp".to_string(),
                    },
                ),
            ]),
            affinity: Some(AffinityMatrix::new(vec![1, 2, 3, 4], aff2, aff3)),
            ..ResourceCatalog::default()
        };
        let params = vec![("player_chara_id".to_string(), "1".to_string())];
        let result = DatabaseAccountRecord {
            account_id: "540903147493".to_string(),
            trainer_name: Some("UUC".to_string()),
            follower_num: None,
            last_updated: None,
            support_card: None,
            inheritance: Some(DatabaseInheritanceRecord {
                main_parent_id: Some(2),
                parent_left_id: Some(3),
                parent_right_id: Some(4),
                affinity_score: Some(9),
                main_win_saddles: vec![10, 20],
                left_win_saddles: vec![10],
                right_win_saddles: vec![20],
                ..DatabaseInheritanceRecord::default()
            }),
        };

        let meta = database_result_metadata(
            &config(),
            result,
            1,
            "character 1".to_string(),
            &params,
            resources,
            "https://uma.moe/database".to_string(),
            "https://uma.moe/__embeds/images/database/query.png".to_string(),
        );
        assert_eq!(meta.title, "UUC | 540903147493 | uma.moe");
        assert!(meta
            .description
            .starts_with("Trainer ID: 540903147493. Result 1 of 1."));
        assert!(!meta.description.contains("character 1"));
        assert!(meta.description.contains("17 affinity"));
        assert!(meta.description.contains("Main: Main Uma."));
        assert!(meta.description.contains("Parents: Left Uma / Right Uma."));
        let database = meta.database.unwrap();

        assert_eq!(database.affinity_score, Some(17));
        assert_eq!(database.left_affinity_score, Some(8));
        assert_eq!(database.right_affinity_score, Some(9));
    }

    #[test]
    fn database_query_highlights_factor_filters() {
        let params = database_search_params_from_query(
            "blue_sparks=202&white_sparks=2012701&main_parent_pink_sparks=2103&support_card_id=30039&min_limit_break=4",
        );
        let highlights = database_query_highlights(&params);

        assert_eq!(highlights.matched_factor_ids, vec![20, 210, 201270]);
        assert_eq!(highlights.matched_main_factor_ids, vec![210]);
        assert_eq!(highlights.matched_support_card_id, Some(30039));
        assert_eq!(highlights.matched_min_limit_break, Some(4));
    }

    #[test]
    fn database_query_label_uses_resource_names() {
        let mut resources = ResourceCatalog::default();
        resources.support_cards.insert(
            30039,
            ResourceSupportCard {
                name: "Fine Motion".to_string(),
            },
        );
        resources.characters.insert(
            105001,
            ResourceCharacter {
                name: "Curren Chan".to_string(),
                image: "chara_stand_105001.webp".to_string(),
            },
        );

        let support_params =
            database_search_params_from_query("support_card_id=30039&trainer_name=Arumi");
        assert_eq!(
            database_query_label(&support_params, &resources),
            "support card Fine Motion"
        );

        let parent_params = database_search_params_from_query("main_parent_id=105001");
        assert_eq!(
            database_query_label(&parent_params, &resources),
            "main parent Curren Chan"
        );
    }

    #[test]
    fn clean_query_removes_embed_debug_param() {
        assert_eq!(
            clean_query(Some("filters=abc&__embed=1"), "__embed").as_deref(),
            Some("filters=abc")
        );
    }

    #[test]
    fn lineage_planner_shared_query_preserves_tree_url_state() {
        let meta = lineage_planner_metadata_from_query(
            &config(),
            Some("tree=abc-def_123"),
            ResourceCatalog::default(),
        );

        assert_eq!(
            meta.canonical_url,
            "https://uma.moe/tools/lineage-planner?tree=abc-def_123"
        );
        assert_eq!(
            meta.image_url,
            "https://uma.moe/__embeds/images/page/lineage-planner.png?tree=abc-def_123"
        );
        assert_eq!(meta.title, "Shared Lineage Planner | uma.moe");
        assert!(meta
            .metrics
            .iter()
            .any(|metric| metric.label == "Mode" && metric.value == "Shared Tree"));
    }
}
