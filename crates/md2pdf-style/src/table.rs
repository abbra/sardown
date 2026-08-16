#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct TableStyle {
    pub cell_padding_pt: f32,
    pub text_size_pt: f32,
    pub min_row_height_pt: f32,
}

impl Default for TableStyle {
    fn default() -> Self {
        TableStyle { cell_padding_pt: 12.0, text_size_pt: 10.5, min_row_height_pt: 20.0 }
    }
}
