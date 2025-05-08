use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    static ref DNSNAME_PATTERN: Regex = Regex::new(
        r"^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    DNSNAME_PATTERN.is_match(value)
}

pub struct DnsNameMatcher {}

impl PatternMatcher for DnsNameMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

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
        ];
        
        for name in invalid_names {
            assert!(!is_match(name), "DNS name should be invalid: {}", name);
        }
    }
