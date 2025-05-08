use regex::Regex;
use lazy_static::lazy_static;
use super::PatternMatcher;

lazy_static! {
    // IPv4 pattern
    static ref IPV4_PATTERN: Regex = Regex::new(
        r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$"
    ).unwrap();
    
    // Simplified IPv6 pattern
    static ref IPV6_PATTERN: Regex = Regex::new(
        r"^(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|(?:[0-9a-fA-F]{1,4}:){1,7}:|(?:[0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|(?:[0-9a-fA-F]{1,4}:){1,5}(?::[0-9a-fA-F]{1,4}){1,2}|(?:[0-9a-fA-F]{1,4}:){1,4}(?::[0-9a-fA-F]{1,4}){1,3}|(?:[0-9a-fA-F]{1,4}:){1,3}(?::[0-9a-fA-F]{1,4}){1,4}|(?:[0-9a-fA-F]{1,4}:){1,2}(?::[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:(?::[0-9a-fA-F]{1,4}){1,6}|:(?:(?::[0-9a-fA-F]{1,4}){1,7}|:)$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    IPV4_PATTERN.is_match(value) || IPV6_PATTERN.is_match(value)
}

pub struct IpMatcher {}

impl PatternMatcher for IpMatcher {
    fn matches(&self, value: &str) -> bool {
        is_match(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ipv4() {
        let valid_ips = vec![
            "192.168.1.1",
            "10.0.0.255",
            "127.0.0.1",
            "255.255.255.255",
            "0.0.0.0",
        ];
        
        for ip in valid_ips {
            assert!(is_match(ip), "IP should be valid: {}", ip);
        }
    }
    
    #[test]
    fn test_invalid_ipv4() {
        let invalid_ips = vec![
            "192.168.1",
            "192.168.1.256",
            "300.168.1.1",
            "192.168.1.1.1",
            "192.168.1,1",
        ];
        
        for ip in invalid_ips {
            assert!(!is_match(ip), "IP should be invalid: {}", ip);
        }
    }
    
    #[test]
    fn test_valid_ipv6() {
        let valid_ips = vec![
            "2001:0db8:85a3:0000:0000:8a2e:0370:7334",
            "2001:db8:85a3::8a2e:370:7334",
            "::1",
            "::",
            "fe80::1ff:fe23:4567:890a",
        ];
        
        for ip in valid_ips {
            assert!(is_match(ip), "IPv6 should be valid: {}", ip);
        }
    }
}