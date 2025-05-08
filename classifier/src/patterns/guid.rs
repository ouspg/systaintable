use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // UUID/GUID pattern (both with and without braces)
    static ref GUID_PATTERN: Regex = Regex::new(
        r"^(?:\{)?[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}(?:\})?$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    GUID_PATTERN.is_match(value)
}

pub struct GuidMatcher {}

impl PatternMatcher for GuidMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_guids() {
        let valid_guids = vec![
            "123e4567-e89b-12d3-a456-426614174000",
            "{123e4567-e89b-12d3-a456-426614174000}",
            "A987FBC9-4BED-3078-CF07-9141BA07C9F3",
            "{A987FBC9-4BED-3078-CF07-9141BA07C9F3}",
        ];
        
        for guid in valid_guids {
            assert!(is_match(guid), "GUID should be valid: {}", guid);
        }
    }
    
    #[test]
    fn test_invalid_guids() {
        let invalid_guids = vec![
            "123e4567-e89b-12d3-a456",
            "123e4567e89b12d3a456426614174000",
            "123e4567-e89b-12d3-a456-42661417400Z", // non-hex character
            "{123e4567-e89b-12d3-a456-426614174000", // missing closing brace
            "123e4567-e89b-12d3-a456-426614174000}", // missing opening brace
        ];
        
        for guid in invalid_guids {
            assert!(!is_match(guid), "GUID should be invalid: {}", guid);
        }
    }
}