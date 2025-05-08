use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // URL pattern
    static ref URL_PATTERN: Regex = Regex::new(
        r"^(?:(?:https?|ftp):\/\/)(?:\S+(?::\S*)?@)?(?:(?!10(?:\.\d{1,3}){3})(?!127(?:\.\d{1,3}){3})(?!169\.254(?:\.\d{1,3}){2})(?!192\.168(?:\.\d{1,3}){2})(?!172\.(?:1[6-9]|2\d|3[0-1])(?:\.\d{1,3}){2})(?:[1-9]\d?|1\d\d|2[01]\d|22[0-3])(?:\.(?:1?\d{1,2}|2[0-4]\d|25[0-5])){2}(?:\.(?:[1-9]\d?|1\d\d|2[0-4]\d|25[0-4]))|(?:(?:[a-z\u00a1-\uffff0-9]+-?)*[a-z\u00a1-\uffff0-9]+)(?:\.(?:[a-z\u00a1-\uffff0-9]+-?)*[a-z\u00a1-\uffff0-9]+)*(?:\.(?:[a-z\u00a1-\uffff]{2,})))(?::\d{2,5})?(?:\/[^\s]*)?$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    URL_PATTERN.is_match(value)
}

pub struct UrlMatcher {}

impl PatternMatcher for UrlMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_urls() {
        let valid_urls = vec![
            "http://example.com",
            "https://example.com",
            "http://www.example.com",
            "http://example.com/path",
            "http://example.com/path?query=value",
            "http://example.com:8080",
            "http://user:pass@example.com",
            "http://192.168.1.1",
            "ftp://ftp.example.com",
        ];
        
        for url in valid_urls {
            assert!(is_match(url), "URL should be valid: {}", url);
        }
    }
    
    #[test]
    fn test_invalid_urls() {
        let invalid_urls = vec![
            "example.com",           // missing protocol
            "http://",               // missing domain
            "http:/example.com",     // missing slash
            "http:example.com",      // missing slashes
            "http://example",        // missing TLD
            "http://..com",          // invalid domain
        ];
        
        for url in invalid_urls {
            assert!(!is_match(url), "URL should be invalid: {}", url);
        }
    }
}