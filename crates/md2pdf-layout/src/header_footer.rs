use md2pdf_style::NumberingFormat;

const ROMAN_NUMERALS: [(u32, &str); 13] = [
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];

fn to_roman(mut n: u32) -> String {
    let mut result = String::new();
    for &(value, numeral) in &ROMAN_NUMERALS {
        while n >= value {
            result.push_str(numeral);
            n -= value;
        }
    }
    result
}

/// Formats `n` per `format`. Roman numerals have no representation for 0 and become unwieldy
/// past the conventional range of 1-3999, so both fall back to the plain arabic form -- silently
/// for 0 (an unusual but harmless `start_at = 0` choice), with a warning above 3999.
pub fn format_page_number(n: u32, format: NumberingFormat) -> String {
    match format {
        NumberingFormat::Arabic => n.to_string(),
        NumberingFormat::RomanLower | NumberingFormat::RomanUpper => {
            if n == 0 {
                return n.to_string();
            }
            if n > 3999 {
                eprintln!(
                    "warning: page number {n} exceeds the conventional roman numeral range (1-3999); \
                     falling back to arabic for this value"
                );
                return n.to_string();
            }
            let roman = to_roman(n);
            match format {
                NumberingFormat::RomanLower => roman.to_lowercase(),
                _ => roman,
            }
        }
    }
}
