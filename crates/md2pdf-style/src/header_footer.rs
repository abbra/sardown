#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct HeaderZones {
    pub left: String,
    pub center: String,
    pub right: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeaderFooterMode {
    #[default]
    Uniform,
    TwoSided,
}
