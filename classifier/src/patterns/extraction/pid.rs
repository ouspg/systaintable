use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // Process ID pattern (numeric, typically 1-5 digits)
    static ref PID_PATTERN: Regex = Regex::new(
        r"^(?:\[)?(?:PID:?)?\s*\d{1,7}(?:\])?$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    PID_PATTERN.is_match(value)
}

pub fn extract_pids(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // Extract explicit PID references
    let pid_pattern = Regex::new(r"\b(?:PID|pid)(?::|\s+)(\d{1,7})\b").unwrap();
    for cap in pid_pattern.captures_iter(text) {
        if let Some(pid) = cap.get(1) {
            results.push(pid.as_str().to_string());
        }
    }
    
    // Extract PIDs in brackets
    let bracketed_pid = Regex::new(r"\[(?:PID:?)?\s*(\d{1,7})\]").unwrap();
    for cap in bracketed_pid.captures_iter(text) {
        if let Some(pid) = cap.get(1) {
            results.push(pid.as_str().to_string());
        }
    }
    
    results
}

pub struct PidMatcher {}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pids() {
        let valid_pids = vec![
            "1234",
            "24839",
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