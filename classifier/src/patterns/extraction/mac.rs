use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    // MAC address with various separator formats
    static ref MAC_PATTERN: Regex = Regex::new(
        r"^(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$"
    ).unwrap();
}

pub fn is_match(value: &str) -> bool {
    MAC_PATTERN.is_match(value)
}

pub fn extract_macs(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    
    // Extract MACs with separators
    let mac_with_sep = Regex::new(r"\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b").unwrap();
    for cap in mac_with_sep.captures_iter(text) {
        results.push(cap[0].to_string());
    }
    
    // Extract MACs without separators
    let mac_no_sep = Regex::new(r"[^0-9A-Za-z]([0-9a-f]{12})[^0-9A-Za-z]").unwrap();
    for cap in mac_no_sep.captures_iter(text) {
        if let Some(mac) = cap.get(1) {
            results.push(mac.as_str().to_string());
        }
    }
    
    results
}

pub struct MacMatcher {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_macs() {
        let valid_macs = vec![
            "00:1A:2B:3C:4D:5E",
            "00-1A-2B-3C-4D-5E",
            "00:1a:2b:3c:4d:5e",
            "FF:FF:FF:FF:FF:FF",
            "ff:ff:ff:ff:ff:ff",
        ];
        
        for mac in valid_macs {
            assert!(is_match(mac), "MAC should be valid: {}", mac);
        }
    }
    
    #[test]
    fn test_invalid_macs() {
        let invalid_macs = vec![
            "00:1A:2B:3C:4D",         // too short
            "00:1A:2B:3C:4D:5E:6F",   // too long
            "00:1A:2B:3C:4D:5G",      // invalid hex
            "001A2B3C4D5E",           // no separators
            "00 1A 2B 3C 4D 5E",      // wrong separator
        ];
        
        for mac in invalid_macs {
            assert!(!is_match(mac), "MAC should be invalid: {}", mac);
        }
    }
}