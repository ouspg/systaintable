use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // Basic street address pattern
    static ref ADDRESS_PATTERN: Regex = Regex::new(
        r"^(?:\d+\s)(?:[A-Za-z0-9.-]+\s)+(?:Avenue|Lane|Road|Boulevard|Drive|Street|Ave|Dr|Rd|Blvd|Ln|St)\.?(?:\s[A-Za-z]+)?$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    ADDRESS_PATTERN.is_match(value)
}

pub fn extract_addresses(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    for cap in ADDRESS_PATTERN.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    results
}

pub struct AddressMatcher {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_addresses() {
        let valid_addresses = vec![
            "123 Main Street",
            "456 Elm St.",
            "789 Oak Avenue",
            "1011 Pine Rd",
            "2468 Maple Boulevard",
        ];
        
        for address in valid_addresses {
            assert!(is_match(address), "Address should be valid: {}", address);
        }
    }
    
    #[test]
    fn test_invalid_addresses() {
        let invalid_addresses = vec![
            "123",
            "Main Street",
            "123 @#$%",
            "123 Main abc",
        ];
        
        for address in invalid_addresses {
            assert!(!is_match(address), "Address should be invalid: {}", address);
        }
    }
}