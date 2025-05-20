use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // Phone number patterns for various formats
    static ref PHONE_PATTERN: Regex = Regex::new(
        r"^(?:(?:\+\d{1,3}[-. ]?)?(?:\(\d{1,4}\)|\d{1,4})[-. ]?\d{1,4}[-. ]?\d{1,4}(?:[-. ]?\d{1,4})?|\d{3,7}|\+\d{1,3}[-. ]?\d{3,12})$"
    ).unwrap();
    
    // IP pattern to exclude from phone numbers
    static ref IP_PATTERN: Regex = Regex::new(
        r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    // Return false if it looks like an IP address
    if IP_PATTERN.is_match(value) {
        return false;
    }
    
    PHONE_PATTERN.is_match(value)
}

pub fn extract_phonenumbers(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // Phone pattern for extraction (more permissive than validation pattern)
    let phone_pattern = Regex::new(r"\b(?:\+\d{1,3}[- ]?)?\(?\d{3}\)?[- ]?\d{3}[- ]?\d{4}\b").unwrap();
    for cap in phone_pattern.captures_iter(text) {
        let number = cap[0].to_string();
        // Skip if it looks like an IP address
        if !number.contains(".") {
            results.push(number);
        }
    }
    
    // Simple numeric sequence that might be phone numbers
    let simple_pattern = Regex::new(r"\b\d{10,12}\b").unwrap();
    for cap in simple_pattern.captures_iter(text) {
        let num = cap[0].to_string();
        // Skip if it looks like a timestamp, port, IP address or other numeric identifier
        if !text.contains(&format!(":{}", &num)) && 
           !text.contains(&format!(".{}", &num)) &&
           !num.contains(".") {
            results.push(num);
        }
    }
    
    results
}

pub struct PhoneNumberMatcher {}

// Comment out the PatternMatcher implementation
// impl crate::patterns::PatternMatcher for PhoneNumberMatcher {
//     fn matches(&self, value: &str) -> bool {
//         is_match(value)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_phone_numbers() {
        let valid_phones = vec![
            "123-456-7890",
            "(123) 456-7890",
            "+1 123-456-7890",
            "123.456.7890",
            "1234567890",
            "+12345678901",
        ];
        
        for phone in valid_phones {
            assert!(is_match(phone), "Should match: {}", phone);
        }
    }

    #[test]
    fn test_invalid_phone_numbers() {
        let invalid_phones = vec![
            "123-45-678",
            "1234",
            "abcd",
            "8.8.8.8",         // IP address
            "192.168.1.1",     // IP address
            "10.0.0.1",        // IP address
        ];
        
        for phone in invalid_phones {
            assert!(!is_match(phone), "Should not match: {}", phone);
        }
    }
}