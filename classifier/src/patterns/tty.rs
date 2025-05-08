use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

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

impl PatternMatcher for TtyMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
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