use md2pdf_style::Color;

#[test]
fn parses_a_hex_string_with_hash_prefix() {
    let toml_text = r##"color = "#1a2b3c""##;
    #[derive(serde::Deserialize)]
    struct Wrapper {
        color: Color,
    }
    let parsed: Wrapper = toml::from_str(toml_text).unwrap();
    assert_eq!(parsed.color, Color([0x1a, 0x2b, 0x3c]));
}

#[test]
fn parses_a_hex_string_without_hash_prefix() {
    let toml_text = r#"color = "1a2b3c""#;
    #[derive(serde::Deserialize)]
    struct Wrapper {
        color: Color,
    }
    let parsed: Wrapper = toml::from_str(toml_text).unwrap();
    assert_eq!(parsed.color, Color([0x1a, 0x2b, 0x3c]));
}

#[test]
fn parses_an_rgb_array() {
    let toml_text = "color = [26, 43, 60]";
    #[derive(serde::Deserialize)]
    struct Wrapper {
        color: Color,
    }
    let parsed: Wrapper = toml::from_str(toml_text).unwrap();
    assert_eq!(parsed.color, Color([26, 43, 60]));
}

#[test]
fn rejects_a_hex_string_of_the_wrong_length() {
    let toml_text = r##"color = "#1a2b""##;
    #[derive(Debug, serde::Deserialize)]
    struct Wrapper {
        // Never read: only deserialization itself (rejecting the bad value) is under test.
        #[allow(dead_code)]
        color: Color,
    }
    let err = toml::from_str::<Wrapper>(toml_text).unwrap_err();
    assert!(err.to_string().contains("1a2b"), "expected the bad value in the error, got: {err}");
}

#[test]
fn rejects_a_hex_string_with_non_hex_characters() {
    let toml_text = r##"color = "#gggggg""##;
    #[derive(serde::Deserialize)]
    struct Wrapper {
        // Never read: only deserialization itself (rejecting the bad value) is under test.
        #[allow(dead_code)]
        color: Color,
    }
    assert!(toml::from_str::<Wrapper>(toml_text).is_err());
}
