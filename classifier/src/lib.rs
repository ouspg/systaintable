pub mod patterns;

use patterns::PatternMatcher;
use std::collections::HashMap;

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
    if patterns::guid::is_match(value) {
        matches.push("guid".to_string());
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
    if patterns::realname::is_match(value) {
        matches.push("realname".to_string());
    }
    if patterns::time::is_match(value) {
        matches.push("time".to_string());
    }
    if patterns::tty::is_match(value) {
        matches.push("tty".to_string());
    }
    if patterns::uid::is_match(value) {
        matches.push("uid".to_string());
    }
    if patterns::url::is_match(value) {
        matches.push("url".to_string());
    }
    if patterns::username::is_match(value) {
        matches.push("username".to_string());
    }
    
    matches
}

pub fn get_all_matchers() -> HashMap<String, Box<dyn PatternMatcher>> {
    let mut matchers: HashMap<String, Box<dyn PatternMatcher>> = HashMap::new();
    
    matchers.insert("address".to_string(), Box::new(patterns::address::AddressMatcher {}));
    matchers.insert("dnsname".to_string(), Box::new(patterns::dnsname::DnsNameMatcher {}));
    matchers.insert("email".to_string(), Box::new(patterns::email::EmailMatcher {}));
    matchers.insert("guid".to_string(), Box::new(patterns::guid::GuidMatcher {}));
    matchers.insert("ip".to_string(), Box::new(patterns::ip::IpMatcher {}));
    matchers.insert("mac".to_string(), Box::new(patterns::mac::MacMatcher {}));
    matchers.insert("phonenumber".to_string(), Box::new(patterns::phonenumber::PhoneNumberMatcher {}));
    matchers.insert("pid".to_string(), Box::new(patterns::pid::PidMatcher {}));
    matchers.insert("realname".to_string(), Box::new(patterns::realname::RealNameMatcher {}));
    matchers.insert("time".to_string(), Box::new(patterns::time::TimeMatcher {}));
    matchers.insert("tty".to_string(), Box::new(patterns::tty::TtyMatcher {}));
    matchers.insert("uid".to_string(), Box::new(patterns::uid::UidMatcher {}));
    matchers.insert("url".to_string(), Box::new(patterns::url::UrlMatcher {}));
    matchers.insert("username".to_string(), Box::new(patterns::username::UsernameMatcher {}));
    
    matchers
}