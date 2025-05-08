use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // Phone number patterns for various formats
    static ref PHONE_PATTERN: Regex = Regex::new(
        r"^(?:(?:\+\d{1,3}[-. ]?)?(?:\(\d{1,4}\)|\d{1,4})[-. ]?\d{1,4}[-. ]?\d{1,4}(?:[-. ]?\d{1,4})?)$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    PHONE_PATTERN.is_match(value)
}

pub struct PhoneNumberMatcher {}

impl PatternMatcher for PhoneNumberMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_phones() {
        let valid_phones = vec![
            "123-456-7890",
            "(123) 456-7890",
            "123.456.7890",
            "1234567890",
            "+1 123-456-7890",
            "+44 1234 567890",
            "+1 (123) 456-7890",
        ];
        
        for phone in valid_phones {
            assert!(is_match(phone), "Phone should be valid: {}", phone);
        }
    }
    
    #[test]
    fn test_invalid_phones() {
        let invalid_phones = vec![
            "123-456",                // too short
            "123-456-789a",           // non-numeric
            "(123)456-7890)",         // unbalanced parenthesis
            "123 - 456 - 7890",       // improper spacing
            "+a 123-456-7890",        // invalid country code
        ];
        
        for phone in invalid_phones {
            assert!(!is_match(phone), "Phone should be invalid: {}", phone);
        }
    }
}