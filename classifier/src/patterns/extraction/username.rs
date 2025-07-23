use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Match common SSH log patterns for usernames
    static ref SSH_INVALID_USER: Regex = Regex::new(r"Invalid user (\S+) from").unwrap();
    static ref SSH_FAILED_PASSWORD: Regex = Regex::new(r"Failed password for (?:invalid user )?(\S+) from").unwrap();
    static ref SSH_ACCEPTED: Regex = Regex::new(r"Accepted (?:password|publickey) for (\S+) from").unwrap();
    static ref SSH_USER_NOT_ALLOWED: Regex = Regex::new(r"User (\S+) from").unwrap();
    
    // Match JSON host fields as usernames
    static ref JSON_HOST_PATTERN: Regex = Regex::new(
        r#""host"\s*:\s*"([^"]+)""#
    ).unwrap();
}

pub fn extract_usernames(input: &str) -> Vec<String> {
    let mut usernames = Vec::new();
    
    // Try all the SSH patterns
    if let Some(caps) = SSH_INVALID_USER.captures(input) {
        if let Some(username) = caps.get(1) {
            usernames.push(username.as_str().to_string());
        }
    }
    
    if let Some(caps) = SSH_FAILED_PASSWORD.captures(input) {
        if let Some(username) = caps.get(1) {
            usernames.push(username.as_str().to_string());
        }
    }
    
    if let Some(caps) = SSH_ACCEPTED.captures(input) {
        if let Some(username) = caps.get(1) {
            usernames.push(username.as_str().to_string());
        }
    }
    
    if let Some(caps) = SSH_USER_NOT_ALLOWED.captures(input) {
        if let Some(username) = caps.get(1) {
            usernames.push(username.as_str().to_string());
        }
    }
    
    // Extract JSON host fields as usernames
    for cap in JSON_HOST_PATTERN.captures_iter(input) {
        if let Some(m) = cap.get(1) {
            usernames.push(m.as_str().to_string());
        }
    }

    usernames
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_usernames() {
        assert_eq!(
            extract_usernames("Dec 10 06:55:46 LabSZ sshd[24200]: Invalid user webmaster from 173.234.31.186"),
            vec!["webmaster"]
        );
        assert_eq!(
            extract_usernames("Dec 10 08:55:12 LabSZ sshd[25814]: Failed password for invalid user admin from 61.177.172.13"),
            vec!["admin"]
        );
    }
}