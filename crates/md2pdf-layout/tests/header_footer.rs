use md2pdf_layout::format_page_number;
use md2pdf_style::NumberingFormat;

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
