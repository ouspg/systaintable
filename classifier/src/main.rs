use clap::Parser;
use regex_classifier::classify;

/// Regex classifier for identifying data types
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Value to classify
    #[arg(index = 1)]
    value: String,
}

fn main() {
    let args = Args::parse();
    let value = args.value.trim();
    
    let categories = classify(value);
    
    if categories.is_empty() {
        println!("The value '{}' did not match any known categories", value);
    } else {
        println!("The value '{}' was classified as: {}", value, categories.join(", "));
    }
}