pub mod patterns;

use std::collections::HashMap;

// Define wrapper structs for pattern matchers
pub struct AddressMatcher {}
impl patterns::PatternMatcher for AddressMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::address::is_match(value)
    }
}

pub struct DnsNameMatcher {}
impl patterns::PatternMatcher for DnsNameMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::dnsname::is_match(value)
    }
}

pub struct EmailMatcher {}
impl patterns::PatternMatcher for EmailMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::email::is_match(value)
    }
}

pub struct IpMatcher {}
impl patterns::PatternMatcher for IpMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::ip::is_match(value)
    }
}

pub struct MacMatcher {}
impl patterns::PatternMatcher for MacMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::mac::is_match(value)
    }
}

pub struct PhoneNumberMatcher {}
impl patterns::PatternMatcher for PhoneNumberMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::phonenumber::is_match(value)
    }
}

pub struct PidMatcher {}
impl patterns::PatternMatcher for PidMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::pid::is_match(value)
    }
}

pub struct TimeMatcher {}
impl patterns::PatternMatcher for TimeMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::time::is_match(value)
    }
}

pub struct TtyMatcher {}
impl patterns::PatternMatcher for TtyMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::tty::is_match(value)
    }
}

pub struct UrlMatcher {}
impl patterns::PatternMatcher for UrlMatcher {
    fn matches(&self, value: &str) -> bool {
        patterns::url::is_match(value)
    }
}

/// Main classifier function that takes a string value and returns a list of matched categories
pub fn classify(value: &str) -> Vec<String> {
    if value.is_empty() {
        return vec![];
    }

    let mut matches = Vec::new();
    
    // Check each pattern
    if patterns::address::is_match(value) {
        matches.push("address".to_string());
    }
    if patterns::dnsname::is_match(value) {
        matches.push("dnsname".to_string());
    }
    if patterns::email::is_match(value) {
        matches.push("email".to_string());
    }
    if patterns::ip::is_match(value) {
        matches.push("ip".to_string());
    }
    if patterns::mac::is_match(value) {
        matches.push("mac".to_string());
    }
    if patterns::phonenumber::is_match(value) {
        matches.push("phonenumber".to_string());
    }
    if patterns::pid::is_match(value) {
        matches.push("pid".to_string());
    }
    if patterns::time::is_match(value) {
        matches.push("time".to_string());
    }
    if patterns::tty::is_match(value) {
        matches.push("tty".to_string());
    }
    if patterns::url::is_match(value) {
        matches.push("url".to_string());
    }
    
    matches
}

/// Returns a map of all pattern matchers
pub fn get_all_matchers() -> HashMap<String, Box<dyn patterns::PatternMatcher>> {
    let mut matchers: HashMap<String, Box<dyn patterns::PatternMatcher>> = HashMap::new();
    
    matchers.insert("address".to_string(), Box::new(AddressMatcher {}));
    matchers.insert("dnsname".to_string(), Box::new(DnsNameMatcher {}));
    matchers.insert("email".to_string(), Box::new(EmailMatcher {}));
    matchers.insert("ip".to_string(), Box::new(IpMatcher {}));
    matchers.insert("mac".to_string(), Box::new(MacMatcher {}));
    matchers.insert("phonenumber".to_string(), Box::new(PhoneNumberMatcher {}));
    matchers.insert("pid".to_string(), Box::new(PidMatcher {}));
    matchers.insert("time".to_string(), Box::new(TimeMatcher {}));
    matchers.insert("tty".to_string(), Box::new(TtyMatcher {}));
    matchers.insert("url".to_string(), Box::new(UrlMatcher {}));
    
    matchers
}

/// Checks if a value matches any of the known patterns
pub fn is_any_match(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    
    patterns::address::is_match(value) ||
    patterns::dnsname::is_match(value) ||
    patterns::email::is_match(value) ||
    patterns::ip::is_match(value) ||
    patterns::mac::is_match(value) ||
    patterns::phonenumber::is_match(value) ||
    patterns::pid::is_match(value) ||
    patterns::time::is_match(value) ||
    patterns::tty::is_match(value) ||
    patterns::url::is_match(value)
}