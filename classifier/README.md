# Classifier

* Classifier is a simple tool for doing rexep classification of log values.

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
