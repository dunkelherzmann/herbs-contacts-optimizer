//! Countries and their dialling rules, and putting a country code in front of a phone number.

use crate::text::phone_digits;
pub use crate::countries_data::COUNTRIES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Country {
    pub name: &'static str,
    /// The two-letter code, such as "ES".
    pub iso: &'static str,
    /// The country code without the plus, such as "34".
    pub code: &'static str,
    /// What is dialled in this country before another country's code, such as "00"; may be empty.
    pub international_prefix: &'static str,
    /// What is dialled inside the country before a national number, such as "0"; may be empty.
    pub trunk_prefix: &'static str,
}

/// True when the number already starts with a country code (a plus).
pub fn has_country_code(number: &str) -> bool {
    number.trim_start().starts_with('+')
}

/// True for a number a country code can be put in front of: no plus yet, at least five digits,
/// and not a service code (`*100#`).
pub fn lacks_country_code(number: &str) -> bool {
    let trimmed = number.trim();
    !has_country_code(trimmed) && !trimmed.starts_with(['*', '#']) && phone_digits(trimmed).len() >= 5
}

/// The number with a country code: `code` (digits, with or without a plus) goes in front, after the
/// dialling habits of `country` are undone: its international prefix becomes the plus, its trunk
/// prefix is dropped. `None` when the number has a code already or cannot take one.
pub fn with_country_code(number: &str, code: &str, country: Option<&Country>) -> Option<String> {
    let code: String = code.chars().filter(char::is_ascii_digit).collect();
    if !lacks_country_code(number) || code.is_empty() {
        return None;
    }
    let digits = phone_digits(number);
    if let Some(country) = country {
        if !country.international_prefix.is_empty() {
            if let Some(rest) = digits.strip_prefix(country.international_prefix) {
                return Some(format!("+{rest}"));
            }
        }
        if !country.trunk_prefix.is_empty() {
            if let Some(rest) = digits.strip_prefix(country.trunk_prefix) {
                return Some(format!("+{code} {rest}"));
            }
        }
    }
    Some(format!("+{code} {digits}"))
}

/// The first country with this code (several share +1 and +7), for its dialling rules.
pub fn by_code(code: &str) -> Option<&'static Country> {
    let code: String = code.chars().filter(char::is_ascii_digit).collect();
    // The biggest country of a shared code is the usual meaning.
    let usual = match code.as_str() {
        "1" => "US",
        "7" => "RU",
        "44" => "GB",
        "39" => "IT",
        "61" => "AU",
        "47" => "NO",
        "358" => "FI",
        "212" => "MA",
        "262" => "RE",
        "590" => "GP",
        "599" => "CW",
        _ => "",
    };
    COUNTRIES.iter().find(|c| c.iso == usual).or_else(|| COUNTRIES.iter().find(|c| c.code == code))
}

/// The country code a number starts with, as "+34"; the longest code that fits.
pub fn code_of(number: &str) -> Option<String> {
    let digits = phone_digits(number);
    let digits = digits.strip_prefix('+')?;
    (1..=3).rev().map(|len| digits.get(..len).unwrap_or_default()).find(|code| !code.is_empty() && COUNTRIES.iter().any(|c| c.code == *code)).map(|code| format!("+{code}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn country(iso: &str) -> &'static Country {
        COUNTRIES.iter().find(|c| c.iso == iso).unwrap()
    }

    #[test]
    fn the_table_has_the_countries_with_their_rules() {
        assert!(COUNTRIES.len() > 200);
        assert_eq!((country("ES").code, country("ES").trunk_prefix), ("34", ""));
        assert_eq!((country("DE").code, country("DE").international_prefix, country("DE").trunk_prefix), ("49", "00", "0"));
        assert_eq!(country("US").international_prefix, "011");
        assert_eq!(by_code("+1").unwrap().iso, "US");
        assert_eq!(by_code("34").unwrap().iso, "ES");
    }

    #[test]
    fn a_country_code_goes_in_front_the_right_way() {
        let (es, de, us) = (country("ES"), country("DE"), country("US"));
        assert_eq!(with_country_code("600 111 222", es.code, Some(es)).unwrap(), "+34 600111222");
        assert_eq!(with_country_code("0170 1234567", de.code, Some(de)).unwrap(), "+49 1701234567");
        assert_eq!(with_country_code("0034 600 111 222", de.code, Some(de)).unwrap(), "+34600111222");
        assert_eq!(with_country_code("011 34 600 111 222", us.code, Some(us)).unwrap(), "+34600111222");
        assert_eq!(with_country_code("(555) 010-2030", "+1", Some(us)).unwrap(), "+1 5550102030");
        assert_eq!(with_country_code("555 0100", "999", None).unwrap(), "+999 5550100");
        assert_eq!(with_country_code("+34 600 111 222", "49", Some(de)), None);
        assert_eq!(with_country_code("*100#", "34", Some(es)), None);
        assert_eq!(with_country_code("112", "34", Some(es)), None);
        assert_eq!(with_country_code("600 111 222", "", Some(es)), None);
    }

    #[test]
    fn the_code_a_number_starts_with_is_found() {
        assert_eq!(code_of("+34 600 111 222").unwrap(), "+34");
        assert_eq!(code_of("+1 555 010 2030").unwrap(), "+1");
        assert_eq!(code_of("+358 40 123").unwrap(), "+358");
        assert_eq!(code_of("600 111 222"), None);
        assert!(lacks_country_code("600 111 222") && !lacks_country_code("+34 600") && !lacks_country_code("112"));
    }
}
