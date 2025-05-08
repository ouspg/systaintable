use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // User ID pattern (numeric UID or alphanumeric with possible hyphens)
    static ref UID_PATTERN: Regex = Regex::new(
        r"^(?:(?:uid=)?(?:\d+)|(?:[a-zA-Z][-a-zA-Z0-9]{0,31}))$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    UID_PATTERN.is_match(value)
}

pub struct UidMatcher {}

impl PatternMatcher for UidMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_uids() {
        let valid_uids = vec![
            "0",
            "1000",
            "uid=0",
            "uid=1000",
            "u1000",
            "user-123",
            "admin",
            "a-very-long-uid-that-is-valid-123",
        ];
        
        for uid in valid_uids {
            assert!(is_match(uid), "UID should be valid: {}", uid);
        }
    }
    
    #[test]
    fn test_invalid_uids() {
        let invalid_uids = vec![
            "uid=",             // missing value
            "_user",            // starts with non-letter
            "user_name",        // contains underscore
            "a-very-long-uid-that-is-invalid-due-to-length", // too long
        ];
        
        for uid in invalid_uids {
            assert!(!is_match(uid), "UID should be invalid: {}", uid);
        }
    }
}