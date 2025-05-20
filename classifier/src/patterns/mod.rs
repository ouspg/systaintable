// Define the PatternMatcher trait
pub trait PatternMatcher {
    fn matches(&self, value: &str) -> bool;
}

// Make extraction module public
pub mod extraction;

// Re-export extraction modules that exist
pub use extraction::address;
pub use extraction::dnsname;
pub use extraction::email;
pub use extraction::ip;
pub use extraction::mac;
pub use extraction::phonenumber;
pub use extraction::pid;
pub use extraction::time;
pub use extraction::tty;
pub use extraction::url;

// Removed: guid, realname, uid, username