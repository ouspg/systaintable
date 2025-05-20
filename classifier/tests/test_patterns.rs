use regex_classifier::{classify, get_all_matchers};

#[test]
fn test_email_classification() {
    let results = classify("user@example.com");
    assert!(results.contains(&"email".to_string()));
}

#[test]
fn test_ip_classification() {
    let results = classify("192.168.1.1");
    assert!(results.contains(&"ip".to_string()));
}

#[test]
fn test_valid_phones() {
    let valid_phones = vec![
        "123-456-7890",
        "(123) 456-7890",
        "123.456.7890",
        "1234567890",
        "+1 123-456-7890",
        "+44 1234 567890",
        "+1 (123) 456-7890",
        "112",           // Emergency number
        "0200200",       // Medium length local number
        "+358444094030", // International without spaces
        "+358 44 409 4030", // International with spaces
    ];
    
    for phone in valid_phones {
        assert!(is_match(phone), "Phone should be valid: {}", phone);
    }
}

#[test]
fn test_multiple_matches() {
    // Some values might match multiple patterns
    let results = classify("1234");
    // Could match pid and uid depending on patterns
    assert!(results.len() >= 1);
}

#[test]
fn test_no_matches() {
    let results = classify("~~~~");
    assert_eq!(results.len(), 0);
}

#[test]
fn test_pattern_matchers() {
    let matchers = get_all_matchers();
    
    // Test email matcher
    let email_matcher = matchers.get("email").unwrap();
    assert!(email_matcher.matches("user@example.com"));
    assert!(!email_matcher.matches("not-an-email"));
    
    // Test IP matcher
    let ip_matcher = matchers.get("ip").unwrap();
    assert!(ip_matcher.matches("192.168.1.1"));
    assert!(!ip_matcher.matches("not-an-ip"));
}