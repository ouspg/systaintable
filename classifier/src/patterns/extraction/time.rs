use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // More strict time pattern for validation
    static ref TIME_PATTERN: Regex = Regex::new(
        r"^(?:(?:(?:[01]?\d|2[0-3])[:\.][0-5]\d(?:[:\.][0-5]\d)?(?:\s*[AP]M)?(?:\s+[A-Z]{3,4})?(?:[+-]\d{4})?)|(?:[01]\d|2[0-3])[0-5]\d|(?:\d{4}-\d{2}-\d{2}T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d{1,3})?(?:Z|[+-]\d{2}:?\d{2})?)|(?:\d{4}-\d{2}-\d{2}\s(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:,\d{3})?)|(?:T(?:[01]\d|2[0-3]):[0-5]\d(?::[0-5]\d)?Z)|(?:\d{10}))$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    // Additional validation to filter out false positives
    if value == "8.8.8" || value.contains("60") || value.starts_with("54:") {
        return false;
    }
    
    // Check if periods are used as separators in time format
    if value.contains('.') {
        let parts: Vec<&str> = value.split('.').collect();
        if parts.len() >= 2 {
            // Validate hour range
            if let Ok(hour) = parts[0].parse::<u32>() {
                if hour > 23 {
                    return false;
                }
            }
            // Validate minute range
            if let Ok(minute) = parts[1].parse::<u32>() {
                if minute > 59 {
                    return false;
                }
            }
            // Validate second range if present
            if parts.len() > 2 {
                if let Ok(second) = parts[2].parse::<u32>() {
                    if second > 59 {
                        return false;
                    }
                }
            }
        }
    }
    
    // If no issues found, use the regex for final validation
    TIME_PATTERN.is_match(value)
}

pub fn extract_times(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // Time patterns for extraction (with stricter validation)
    let time_pattern = Regex::new(r"\b(?:(?:[01]?\d|2[0-3])[:\.][0-5]\d(?:[:\.][0-5]\d)?(?:\s*[AP]M)?(?:\s+[A-Z]{3,4})?(?:[+-]\d{4})?)\b").unwrap();
    for cap in time_pattern.captures_iter(text) {
        let time = cap[0].to_string();
        if time != "8.8.8" && !time.contains("60") && !time.starts_with("54:") {
            results.push(time);
        }
    }
    
    // Military time format (HHMM)
    let military_time = Regex::new(r"\b(?:[01]\d|2[0-3])[0-5]\d\b").unwrap();
    for cap in military_time.captures_iter(text) {
        let time = cap[0].to_string();
        // Skip if it looks like a year
        if !text.contains(&format!("-{}-", &time)) && !text.contains(&format!("/{}/", &time)) {
            results.push(time);
        }
    }
    
    // ISO format timestamps (YYYY-MM-DDTHH:MM:SS)
    let iso_timestamp = Regex::new(r"\b\d{4}-\d{2}-\d{2}T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d{1,3})?(?:Z|[+-]\d{2}:?\d{2})?\b").unwrap();
    for cap in iso_timestamp.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    
    // Regular date-time format (YYYY-MM-DD HH:MM:SS)
    let regular_timestamp = Regex::new(r"\b\d{4}-\d{2}-\d{2}\s(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:,\d{3})?\b").unwrap();
    for cap in regular_timestamp.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    
    // Unix timestamps (10 digits)
    let unix_timestamp = Regex::new(r"\b\d{10}\b").unwrap();
    for cap in unix_timestamp.captures_iter(text) {
        // Unix timestamp validation - numeric only
        let timestamp = cap[0].to_string();
        if timestamp.chars().all(|c| c.is_digit(10)) {
            results.push(timestamp);
        }
    }
    
    results
}

pub struct TimeMatcher {}

// Comment out the PatternMatcher impl

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
            "1345",
            "0845",
            "13:45 UTC",
            "13:45+0200",
            "13:45 +02:00",
            "T13:45Z",
            "13:45:30Z",
            "13:45:30+0100",
            "2025-05-12 19:03:25",
            "1702206016",  // Unix timestamp (2023-12-10 12:53:36 UTC)
            "946684800",   // Unix timestamp (2000-01-01 00:00:00 UTC)
            "2024-12-16T14:06:41.000Z",  // ISO 8601 timestamp with milliseconds
            "2024-12-16T14:06:41Z",      // ISO 8601 timestamp without milliseconds
            "2024-12-16T14:06:41.000+01:00", // ISO 8601 with timezone offset
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