use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref EMAIL_PATTERN: Regex = Regex::new(
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    ).unwrap();

    // Add extraction pattern without anchors
    static ref EMAIL_EXTRACTION_PATTERN: Regex = Regex::new(
        r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    EMAIL_PATTERN.is_match(value)
}

pub fn extract_emails(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    for cap in EMAIL_EXTRACTION_PATTERN.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    
    results
}

pub struct EmailMatcher {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emails() {
        let valid_emails = vec![
            "user@example.com",
            "user.name@example.com",
            "user+tag@example.com",
            "user123@example.co.uk",
            "user-name@example-domain.com",
        ];
        
        for email in valid_emails {
            assert!(is_match(email), "Email should be valid: {}", email);
        }
    }
    
    #[test]
    fn test_invalid_emails() {
        let invalid_emails = vec![
            "user@",
            "@example.com",
            "user@example",
            "user@.com",
            "user@example..com",
            "user name@example.com",
            "user@exam_ple.com",
        ];
        
        for email in invalid_emails {
            assert!(!is_match(email), "Email should be invalid: {}", email);
        }
    }
}