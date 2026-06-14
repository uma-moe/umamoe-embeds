#[derive(Clone, Copy)]
pub(super) enum SparkType {
    Stats,
    Aptitude,
    Unique,
    Race,
    Skill,
    Scenario,
}

#[derive(Clone, Copy)]
pub(super) struct SparkSource {
    pub level: u8,
    pub affinity: f32,
}

pub(super) struct SparkDisplayMetrics {
    pub proc_chance_pct: f32,
    pub expected_procs: f32,
}

impl SparkSource {
    pub(super) fn new(level: u8, affinity: f32) -> Self {
        Self { level, affinity }
    }
}

pub(super) fn spark_base_chance(spark_type: SparkType, level: u8) -> f32 {
    let chances = spark_base_chances(spark_type);
    let index = usize::from(level.min((chances.len() - 1) as u8));
    chances[index]
}

pub(super) fn spark_proc_chance(spark_type: SparkType, source: SparkSource) -> f32 {
    (spark_base_chance(spark_type, source.level) * (1.0 + source.affinity / 100.0)).min(100.0)
}

pub(super) fn combined_spark_metrics(
    spark_type: SparkType,
    sources: &[SparkSource],
    per_run: bool,
) -> SparkDisplayMetrics {
    let instances = if per_run { 2.0 } else { 1.0 };
    let mut expected_yield = 0.0_f32;
    let mut probability_of_none = 1.0_f32;

    for source in sources {
        let chance = spark_proc_chance(spark_type, *source) / 100.0;
        expected_yield += chance * instances;
        probability_of_none *= (1.0 - chance).powf(instances);
    }

    SparkDisplayMetrics {
        proc_chance_pct: ((1.0 - probability_of_none) * 100.0).clamp(0.0, 100.0),
        expected_procs: round_to(expected_yield, 2),
    }
}

pub(super) fn format_probability(value: f32) -> String {
    if value >= 99.95 {
        "100%".to_string()
    } else if value >= 10.0 {
        format!("{value:.1}%")
    } else {
        format!("{value:.2}%")
    }
}

fn spark_base_chances(spark_type: SparkType) -> &'static [f32] {
    match spark_type {
        SparkType::Stats => &[0.0, 70.0, 80.0, 90.0],
        SparkType::Aptitude => &[0.0, 1.0, 3.0, 5.0],
        SparkType::Unique => &[0.0, 5.0, 10.0, 15.0],
        SparkType::Race => &[0.0, 1.0, 2.0, 3.0],
        SparkType::Skill | SparkType::Scenario => &[0.0, 3.0, 6.0, 9.0],
    }
}

fn round_to(value: f32, places: i32) -> f32 {
    let scale = 10_f32.powi(places);
    (value * scale).round() / scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_matching_spark_sources_with_affinity_scaling() {
        let metrics = combined_spark_metrics(
            SparkType::Aptitude,
            &[SparkSource::new(3, 72.0), SparkSource::new(2, 68.0)],
            false,
        );

        assert_eq!(format_probability(metrics.proc_chance_pct), "13.2%");
        assert_eq!(metrics.expected_procs, 0.14);
    }
}
