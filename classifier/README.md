# Classifier

* Classifier is a simple tool for doing rexep classification of log values.

## Features

- **Multi-pattern detection**: Emails, IP addresses, DNS names, URLs, usernames, TTY references
- **Multiple interfaces**: CLI, JSON output, Lambda function, MCP server
- **High performance**: Parallel processing with Rayon for large files
- **Flexible output**: JSON, human-readable summaries, or raw classifications
- **Cloud integration**: S3 file processing and pre-signed URL generation
- **MCP protocol**: Integration with AI tools like Cursor and Claude Desktop

## Usage Guide
### Basic analysis
cargo run -- /path/to/logfile.log

### Process only first 1000 lines
cargo run -- /path/to/logfile.log --limit 1000

### Show statistics only
cargo run -- /path/to/logfile.log --stats

### Exclude specific pattern types
cargo run -- /path/to/logfile.log --exclude email,ip

### Verbose output with all matches
cargo run -- /path/to/logfile.log --verbose

### Sample every 10th line (for huge files)
cargo run -- /path/to/logfile.log --sample 10
## Project tree

```
classifier/
├── Cargo.toml            # Project metadata and dependencies
├── src/
│   ├── main.rs          # Main entry point
│   ├── lib.rs           # Library definitions
│   ├── patterns/        # Module for regex patterns
│   │   ├── mod.rs       # Module definition
│   │   ├── address.rs   # Address pattern
│   │   ├── dnsname.rs   # DNS name pattern  
│   │   ├── email.rs     # Email pattern
│   │   # ... and other pattern files
└── tests/                # Integration tests
    ├── test_patterns.rs  # Tests for patterns
└── data/                 # Safe path for data files (data/* in gitignore)
```

## Using the classifier

Create the project structure:

cargo new --lib regex-classifier
cd regex-classifier

## Create the directories and files as described above

### Build the project

```
cargo build --release
```

## Run tests

```
cargo test
````

## Using the classifier

```
cargo run -- "user@example.com"
Output: The value 'user@example.com' was classified as: email
```

## MCP Cursor config

running the server
```
./target/release/mcp_http_server
```

Config for connecting with cursor
Add this to Settings-> MCP Tools-> New MCP Server

```
{
  "mcpServers": {
    "regex-classifier": {
      "url": "http://localhost:8080/mcp"
    }
  }
}
```
