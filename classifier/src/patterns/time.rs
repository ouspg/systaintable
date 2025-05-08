use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // Time pattern in various formats (12h and 24h)
    static ref TIME_PATTERN: Regex = Regex::new(
        r"^(?:(?:(?:0?[0-9]|1[0-2])(?::|\.)(?:[0-5][0-9])(?:(?::|\.)(?:[0-5][0-9]))?\s*(?:[AaPp][Mm]))|(?:(?:[01]?[0-9]|2[0-3])(?::|\.)(?:[0-5][0-9])(?:(?::|\.)(?:[0-5][0-9])?)?))$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    TIME_PATTERN.is_match(value)
}

pub struct TimeMatcher {}

impl PatternMatcher for TimeMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_times() {
        let valid_times = vec![
            "12:34:56",
            "12:34",
            "1:23",
            "01:23:45",
            "13:45",
            "23:59:59",
            "12:34 AM",
            "1:23 PM",
            "01:23:45 PM",
            "12.34.56",
            "12.34 AM",
        ];
        
        for time in valid_times {
            assert!(is_match(time), "Time should be valid: {}", time);
        }
    }
    
    #[test]
    fn test_invalid_times() {
        let invalid_times = vec![
            "24:00:00",        // hours out of range
            "12:60:00",        // minutes out of range
            "12:34:60",        // seconds out of range
            "12:34 ZM",        // invalid AM/PM
            "12-34-56",        // invalid separator
            "1234",            // no separator
        ];
        
        for time in invalid_times {
            assert!(!is_match(time), "Time should be invalid: {}", time);
        }
    }
}