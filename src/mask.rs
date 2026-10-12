//! Account-number masking. At most the last four characters are ever shown.

/// `883821` -> `****3821`; `*****1234` (masked at source) -> `****1234`;
/// label-like values (`Investor Checking ...449`) show only their trailing
/// digit run (max 4); a bare number of four or fewer characters is fully masked.
pub fn mask_number(number: &str) -> String {
    let s = number.trim();
    let chars: Vec<char> = s.chars().filter(|c| c.is_alphanumeric()).collect();
    if s.contains(char::is_whitespace) || s.contains("...") {
        let mut run: Vec<char> = s
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .take(4)
            .collect();
        run.reverse();
        return format!("****{}", run.into_iter().collect::<String>());
    }
    let premasked = s.len() > chars.len();
    if chars.len() > 4 || (chars.len() == 4 && premasked) {
        let tail: String = chars[chars.len() - 4..].iter().collect();
        return format!("****{tail}");
    }
    "****".to_string()
}

/// Last `n` alphanumeric characters (lowercased), for the account selector.
pub fn last_n(number: &str, n: usize) -> String {
    let chars: Vec<char> = number
        .chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let start = chars.len().saturating_sub(n);
    chars[start..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_long_numbers_to_last_four() {
        assert_eq!(mask_number("883821"), "****3821");
        assert_eq!(mask_number("Z1234-5678"), "****5678");
    }

    #[test]
    fn premasked_and_label_values() {
        assert_eq!(mask_number("*****1234"), "****1234");
        assert_eq!(mask_number("Investor Checking ...449"), "****449");
        assert_eq!(mask_number("Brokerage ...3821"), "****3821");
        assert_eq!(mask_number("Some Label"), "****");
    }

    #[test]
    fn short_numbers_fully_masked() {
        assert_eq!(mask_number("449"), "****");
        assert_eq!(mask_number(""), "****");
        assert_eq!(mask_number("1234"), "****");
    }

    #[test]
    fn masked_output_never_contains_prefix() {
        assert!(!mask_number("987654321").contains("9876"));
    }

    #[test]
    fn last_n_ignores_punctuation_and_case() {
        assert_eq!(last_n("AB-12cd", 4), "12cd");
        assert_eq!(last_n("12", 4), "12");
    }
}
