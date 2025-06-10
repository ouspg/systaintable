use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // DNS name pattern
    static ref DNS_PATTERN: Regex = Regex::new(
        r"^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(?:\.[a-zA-Z]{2,})*$"
    ).unwrap();
    
    // DNS extraction pattern without the problematic lookbehind
    static ref DNS_EXTRACT_PATTERN: Regex = Regex::new(
        r"(?:(?:[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)"
    ).unwrap();
    
    // Email pattern for filtering
    static ref EMAIL_PATTERN: Regex = Regex::new(
        r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    // Handle special cases with trailing characters
    let clean_value = value.trim_end_matches(|c| c == '.' || c == ']' || c == ')');
    DNS_PATTERN.is_match(clean_value)
}

pub fn extract_dnsnames(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // First, find all email addresses so we can exclude them
    let email_matches: Vec<_> = EMAIL_PATTERN.find_iter(text)
        .map(|m| (m.start(), m.end()))
        .collect();
    
    // Then extract domain names
    for cap in DNS_EXTRACT_PATTERN.captures_iter(text) {
        let domain = cap[0].to_string();
        let match_start = text.find(&domain).unwrap_or(0);
        let match_end = match_start + domain.len();
        
        // Skip if this match is part of an email
        if email_matches.iter().any(|&(start, end)| match_start >= start && match_end <= end) {
            continue;
        }
        
        // Rest of your validation logic remains the same
        let clean_domain = domain.trim_end_matches(|c| c == '.' || c == ']' || c == ')');
        
        if clean_domain.chars().all(|c| c.is_digit(10) || c == '.') {
            continue;
        }
        
        if clean_domain.contains('.') && !clean_domain.starts_with('.') && !clean_domain.contains("..") {
            results.push(clean_domain.to_string());
        }
    }
    
    results
}

pub struct DnsNameMatcher {}

// Comment out the impl block

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_dnsnames() {
        let valid_names = vec![
            "example.com",
            "sub.example.com",
            "sub-domain.example.co.uk",
            "a.b.c.d.e.f.g",
            "xn--80akhbyknj4f.xn--p1ai", // IDN domain
            "time.google.com.]",          // Special case with trailing characters
            "dns.google.com.",            // Trailing dot (valid in DNS syntax)
        ];
        
        for name in valid_names {
            assert!(is_match(name), "DNS name should be valid: {}", name);
        }
    }
    
    #[test]
    fn test_invalid_dnsnames() {
        let invalid_names = vec![
            "example",
            ".com",
            "example..com",
            "ex ample.com",
            "-example.com",
            "example-.com",
            "example.com-",
            "example.example@example.com"
        ];
        
        for name in invalid_names {
            assert!(!is_match(name), "DNS name should be invalid: {}", name);
        }
    }
    
    #[test]
    fn test_extract_dnsnames() {
        let text = "Connect to time.google.com.] and example.com for services.";
        let names = extract_dnsnames(text);
        assert!(names.contains(&"time.google.com".to_string()));
        assert!(names.contains(&"example.com".to_string()));
    }
}
