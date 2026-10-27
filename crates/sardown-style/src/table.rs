use crate::Color;

/// Which rules a table draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableBorder {
    /// A vertical line between columns and one under the header row.
    #[default]
    Grid,
    /// Horizontal rules only: above the table, under the header, between rows and below the table.
    Horizontal,
    /// No rules; fills, if any, carry the structure.
    None,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct TableStyle {
    pub cell_padding_pt: f32,
    pub text_size_pt: f32,
    pub min_row_height_pt: f32,
    pub border: TableBorder,
    pub border_color: Color,
    pub border_width_pt: f32,
    /// Extra space above and below each row's text, on top of `min_row_height_pt`.
    pub row_padding_pt: f32,
    /// Fill behind the header row.
    pub header_background: Option<Color>,
    /// Text color of the header row; the body text color when unset.
    pub header_color: Option<Color>,
    pub header_bold: bool,
    /// Fill behind every second body row.
    pub stripe_background: Option<Color>,
    /// Space left under the table before the next block.
    pub space_after_pt: f32,
}

impl Default for TableStyle {
    fn default() -> Self {
        TableStyle {
            cell_padding_pt: 12.0,
            text_size_pt: 10.5,
            min_row_height_pt: 20.0,
            border: TableBorder::Grid,
            border_color: Color([180, 180, 180]),
            border_width_pt: 0.75,
            row_padding_pt: 0.0,
            header_background: None,
            header_color: None,
            header_bold: false,
            stripe_background: None,
            space_after_pt: 0.0,
        }
    }
}
