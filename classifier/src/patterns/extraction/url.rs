use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // URL pattern without lookahead assertions
    static ref URL_PATTERN: Regex = Regex::new(
        r"^(?:https?|ftp)://(?:(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?\.)+[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?|\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})(?::\d{1,5})?(?:/[^\s]*)?$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    URL_PATTERN.is_match(value)
}

pub fn extract_urls(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // Extract URLs in common formats
    let url_pattern = Regex::new(r"\bhttps?://(?:[-\w]+\.)+[\w]{2,}(?:/[%\w\.-]*)*(?:\?\S*)?\b").unwrap();
    for cap in url_pattern.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    
    // Extract www URLs
    let www_pattern = Regex::new(r"\bwww\.(?:[-\w]+\.)+[\w]{2,}(?:/[%\w\.-]*)*(?:\?\S*)?\b").unwrap();
    for cap in www_pattern.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    
    results
}

pub struct UrlMatcher {}

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