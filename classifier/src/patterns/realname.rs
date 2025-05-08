use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // Real name pattern - firstName lastName with possible middle names/initials
    static ref REALNAME_PATTERN: Regex = Regex::new(
        r"^[A-Z][a-z]+(?:[\s'-][A-Za-z][a-z]*)*(?:[\s][A-Z][a-z]+)$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    REALNAME_PATTERN.is_match(value)
}

pub struct RealNameMatcher {}

impl PatternMatcher for RealNameMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        let valid_names = vec![
            "John Smith",
            "Mary Jane Smith",
            "John O'Brien",
            "Jean-Claude Smith",
            "Robert J Smith",
        ];
        
        for name in valid_names {
            assert!(is_match(name), "Name should be valid: {}", name);
        }
    }
    
    #[test]
    fn test_invalid_names() {
        let invalid_names = vec![
            "john smith",          // lowercase first letter
            "John",                // missing last name
            "John123 Smith",       // numbers
            "John Smith123",       // numbers
            "J0hn Smith",          // numbers
        ];
        
        for name in invalid_names {
            assert!(!is_match(name), "Name should be invalid: {}", name);
        }
    }
}