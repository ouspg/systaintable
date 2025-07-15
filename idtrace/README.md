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
