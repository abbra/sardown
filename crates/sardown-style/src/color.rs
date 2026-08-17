/// An RGB color. Accepts either a 6-digit hex string (`"#1a1a1a"` or `"1a1a1a"`) or a 3-element
/// `[r, g, b]` array in a stylesheet TOML file, so users can write whichever is more natural for
/// a given value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub [u8; 3]);

impl<'de> serde::Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum ColorRepr {
            Hex(String),
            Rgb([u8; 3]),
        }
        match ColorRepr::deserialize(deserializer)? {
            ColorRepr::Rgb(rgb) => Ok(Color(rgb)),
            ColorRepr::Hex(hex) => parse_hex(&hex).map(Color).map_err(serde::de::Error::custom),
        }
    }
}

fn parse_hex(input: &str) -> Result<[u8; 3], String> {
    let digits = input.strip_prefix('#').unwrap_or(input);
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("invalid color {input:?}: expected a 6-digit hex string like \"#1a1a1a\" or a [r, g, b] array"));
    }
    // Safe to unwrap: the hex-digit check above guarantees every byte parses.
    let byte = |offset: usize| u8::from_str_radix(&digits[offset..offset + 2], 16).unwrap();
    Ok([byte(0), byte(2), byte(4)])
}
