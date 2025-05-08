use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // Process ID pattern (numeric, typically 1-5 digits)
    static ref PID_PATTERN: Regex = Regex::new(
        r"^(?:\[)?(?:PID:?)?\s*\d{1,7}(?:\])?$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    PID_PATTERN.is_match(value)
}

pub struct PidMatcher {}

impl PatternMatcher for PidMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pids() {
        let valid_pids = vec![
            "1234",
            "PID: 1234",
            "[1234]",
            "PID:1234",
            "1",
            "9999999",
        ];
        
        for pid in valid_pids {
            assert!(is_match(pid), "PID should be valid: {}", pid);
        }
    }
    
    #[test]
    fn test_invalid_pids() {
        let invalid_pids = vec![
            "12a34",        // non-numeric
            "PID 1234",     // no separator
            "pid: 1234",    // lowercase
            "99999999",     // too long
            "[1234",        // unbalanced brackets
        ];
        
        for pid in invalid_pids {
            assert!(!is_match(pid), "PID should be invalid: {}", pid);
        }
    }
}