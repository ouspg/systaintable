# Identity Tracer

A Rust program for classified value identity tracing from large JSON output files. It combines classified values into identities based on line numbers and allows users to trace specific identities.

## Features

- Parse large JSON files containing classified values
- Group entries by line number to create identities
- Interactive interface for selecting specific identities to trace
- Output filtered results in JSON format

## Installation

### Prerequisites

- Rust and Cargo (install from [rustup.rs](https://rustup.rs/))

### Building from source

1. Clone the repository:
2. Build the project:
````
cargo build --release
````

3. The executable will be available at `./target/release/identity_tracer`


### Command Line Arguments

- `-i, --input <FILE>`: Input JSON file path
- `-l, --line`: Enable identity tracing by line number

## Usage

````
# Basic identity analysis
./target/release/identity_tracer -l -i input.json

# Analyze with specific data types only
./target/release/identity_tracer -l -t "IP,DNSname,Email" -i input.json

# Search for a specific value
./target/release/identity_tracer -l -s "example@email.com" -i input.json

# Save results to file
./target/release/identity_tracer -l -s "192.168.1.1" -o results.json -i input.json
````

## Advanced Options

````
# Analyze value statistics without merging
./target/release/identity_tracer -l -a -i input.json

# Fast mode (skip transitive closure)
./target/release/identity_tracer -l --fast -i input.json

# Adjust frequency threshold (exclude values appearing in >5% of lines)
./target/release/identity_tracer -l -f 5 -i input.json

# Use specific number of threads
./target/release/identity_tracer -l -j 8 -i input.json

# Debug mode with detailed merge information
./target/release/identity_tracer -l -d -i input.json
````