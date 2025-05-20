use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // TTY device pattern
    static ref TTY_PATTERN: Regex = Regex::new(
        r"^(?:/dev/)?(?:tty|pts|console|ttys|ttyS|ttyUSB|ttyACM)(?:\d+)$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    TTY_PATTERN.is_match(value)
}

pub struct TtyMatcher {}

pub fn extract_ttys(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // Extract TTY paths
    let tty_pattern = Regex::new(r"\b(?:/dev/)?(?:tty|pts)/\d+\b").unwrap();
    for cap in tty_pattern.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    
    // Extract TTY references
    let tty_ref = Regex::new(r"\b(?:tty|pts)[/:]\d+\b").unwrap();
    for cap in tty_ref.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ttys() {
        let valid_ttys = vec![
            "/dev/tty0",
            "/dev/tty1",
            "/dev/pts/0",
            "/dev/console0",
            "/dev/ttys000",
            "/dev/ttyS1",
            "/dev/ttyUSB0",
            "/dev/ttyACM0",
            "tty0",
            "pts/1",
        ];
        
        for tty in valid_ttys {
            assert!(is_match(tty), "TTY should be valid: {}", tty);
        }
    }
    
    #[test]
    fn test_invalid_ttys() {
        let invalid_ttys = vec![
            "/dev/tty",          // missing number
            "/dev/ttyx0",        // invalid type
            "dev/tty0",          // missing slash
            "/dev/tty0a",        // extra character
            "tty",               // missing number
        ];
        
        for tty in invalid_ttys {
            assert!(!is_match(tty), "TTY should be invalid: {}", tty);
        }
    }
}