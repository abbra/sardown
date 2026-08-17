use md2pdf_layout::{format_page_number, resolve_template, PageContext};
use md2pdf_style::{DocumentStyle, NumberingFormat};

fn ctx(h1: Option<&str>, h2: Option<&str>) -> PageContext {
    PageContext { current_h1: h1.map(String::from), current_h2: h2.map(String::from), is_chapter_opener: false }
}

fn no_document() -> DocumentStyle {
    DocumentStyle::default()
}

#[test]
fn arabic_format_is_the_plain_number() {
    assert_eq!(format_page_number(42, NumberingFormat::Arabic), "42");
}

#[test]
fn roman_upper_formats_known_values_correctly() {
    assert_eq!(format_page_number(1, NumberingFormat::RomanUpper), "I");
    assert_eq!(format_page_number(4, NumberingFormat::RomanUpper), "IV");
    assert_eq!(format_page_number(9, NumberingFormat::RomanUpper), "IX");
    assert_eq!(format_page_number(49, NumberingFormat::RomanUpper), "XLIX");
    assert_eq!(format_page_number(3999, NumberingFormat::RomanUpper), "MMMCMXCIX");
}

#[test]
fn roman_lower_is_the_lowercased_roman_upper_form() {
    assert_eq!(format_page_number(49, NumberingFormat::RomanLower), "xlix");
}

#[test]
fn roman_format_falls_back_to_arabic_beyond_conventional_range() {
    assert_eq!(format_page_number(4000, NumberingFormat::RomanUpper), "4000");
}

#[test]
fn roman_format_of_zero_falls_back_to_arabic() {
    assert_eq!(format_page_number(0, NumberingFormat::RomanUpper), "0");
}

#[test]
fn substitutes_h1_and_h2() {
    let context = ctx(Some("Chapter One"), Some("Section A"));
    assert_eq!(resolve_template("{h1} / {h2}", &context, "3", "10", &no_document()), "Chapter One / Section A");
}

#[test]
fn missing_h1_or_h2_resolves_to_an_empty_string() {
    let context = ctx(None, None);
    assert_eq!(resolve_template("[{h1}]", &context, "3", "10", &no_document()), "[]");
}

#[test]
fn substitutes_page_and_total_pages() {
    let context = ctx(None, None);
    assert_eq!(resolve_template("Page {page} of {total_pages}", &context, "3", "10", &no_document()), "Page 3 of 10");
}

#[test]
fn a_template_with_no_placeholders_is_returned_unchanged() {
    let context = ctx(None, None);
    assert_eq!(resolve_template("My Book", &context, "3", "10", &no_document()), "My Book");
}

#[test]
fn mixes_literal_text_and_placeholders_in_one_template() {
    let context = ctx(Some("Intro"), None);
    assert_eq!(resolve_template("-- {h1} --", &context, "1", "1", &no_document()), "-- Intro --");
}

#[test]
fn substitutes_title_and_author() {
    let context = ctx(None, None);
    let document = DocumentStyle { title: "My Book".to_string(), author: "Jane Doe".to_string(), ..Default::default() };
    assert_eq!(resolve_template("{title} by {author}", &context, "1", "1", &document), "My Book by Jane Doe");
}

#[test]
fn substitutes_date() {
    let context = ctx(None, None);
    let document = DocumentStyle { date: "2026-08-17".to_string(), ..Default::default() };
    assert_eq!(resolve_template("Generated {date}", &context, "1", "1", &document), "Generated 2026-08-17");
}
