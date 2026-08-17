#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct DocumentStyle {
    pub title: String,
    pub author: String,
    /// A static date string, used as-is (no parsing/reformatting). If empty (the default), the
    /// CLI fills in today's date at render time unless `--date` overrides it -- see
    /// `{date}` in Headers and Footers.
    pub date: String,
}
