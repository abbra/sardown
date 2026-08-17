#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct DocumentStyle {
    pub title: String,
    pub author: String,
}
