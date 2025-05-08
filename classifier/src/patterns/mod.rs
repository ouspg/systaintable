pub mod address;
pub mod dnsname;
pub mod email;
pub mod guid;
pub mod ip;
pub mod mac;
pub mod phonenumber;
pub mod pid;
pub mod realname;
pub mod time;
pub mod tty;
pub mod uid;
pub mod url;
pub mod username;

pub trait PatternMatcher {
    fn matches(&self, value: &str) -> bool;
}