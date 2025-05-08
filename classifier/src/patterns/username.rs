use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // Username pattern (common system username formats)
    static ref USERNAME_PATTERN: Regex = Regex::new(
        r"^[a-z_](?:[a-z0-9_-]{0,31}|[a-z0-9_-]{0,30}\$)$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    USERNAME_PATTERN.is_match(value)
}

pub struct UsernameMatcher {}

impl PatternMatcher for UsernameMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_usernames() {
        let valid_usernames = vec![
            "admin",
            "user123",
            "user-name",
            "user_name",
            "a",
            "_user",
            "user$",
            "very_long_username_that_is_valid",
        ];
        
        for username in valid_usernames {
            assert!(is_match(username), "Username should be valid: {}", username);
        }
    }
    
    #[test]
    fn test_invalid_usernames() {
        let invalid_usernames = vec![
            "Admin",                 // uppercase
            "123user",               // starts with digit
            "user name",             // contains space
            "-user",                 // starts with hyphen
            "very_long_username_that_is_invalid_due_to_length", // too long
            "user#name",             // invalid character
        ];
        
        for username in invalid_usernames {
            assert!(!is_match(username), "Username should be invalid: {}", username);
        }
    }
}