use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FileMetrics {
    pub path: String,
    #[serde(serialize_with = "round_to_3")]
    pub hotspot_score: f64,
    pub revisions: u32,
    pub author_count: u32,
    #[serde(serialize_with = "round_to_3")]
    pub main_contributor_percentage: f64,
    #[serde(serialize_with = "round_to_3")]
    pub knowledge_distribution: f64,
}

fn round_to_3<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64((*value * 1000.0).round() / 1000.0)
}