# Datadog CLI - AI Agent Development Guide

> For users: [README.md](README.md)

---

## Project Overview

Rust-based CLI tool for querying Datadog API with 13 commands and 3 output formats.

**Key Facts:**
- Language: Rust 2024 Edition (1.91.1+)
- Binary: 5.1MB (LTO optimized)
- Tests: 117 (100% passing)
- Commands: 13 (metrics, logs, monitors, events, hosts, dashboards, spans, services, rum, config)
- Output: JSON, JSONL, Table

---

## Architecture

### 3-Layer Design

```
┌─────────────────────────────────────┐
│      CLI Layer (clap)               │
│  - Command parsing                  │
│  - Output formatting                │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│    Handlers Layer (12 modules)      │
│  - Business logic                   │
│  - Parameter extraction             │
│  - Response formatting              │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│   DatadogClient (HTTP/2)            │
│  - API communication                │
│  - Retry logic                      │
│  - Error handling                   │
└─────────────────────────────────────┘
```

### Key Components

| Path | Purpose |
|------|---------|
| `src/cli/mod.rs` | Clap CLI definition (commands, args) |
| `src/cli/commands.rs` | Command dispatcher |
| `src/cli/output.rs` | Output formatting (json/jsonl/table) |
| `src/handlers/common.rs` | Shared traits (TimeHandler, etc.) |
| `src/handlers/*.rs` | 12 command handlers |
| `src/datadog/client.rs` | HTTP client + retry |
| `src/datadog/models.rs` | API response types |
| `src/error.rs` | Error types (thiserror) |
| `src/utils.rs` | Time parsing, formatting |

---

## Trait-Based Design

### Core Traits (src/handlers/common.rs)

**TimeHandler** - Natural language time → Unix timestamp
```rust
trait TimeHandler {
    fn parse_time(&self, params: &Value, api_version: u8) -> Result<TimeParams>;
    fn timestamp_to_iso8601(&self, timestamp: i64) -> Result<String>;
    fn parse_time_iso8601(&self, params: &Value) -> Result<(String, String)>;
}
```

**ParameterParser** - Extract parameters from JSON
```rust
trait ParameterParser {
    fn extract_string(&self, params: &Value, key: &str) -> Option<String>;
    fn extract_i32(&self, params: &Value, key: &str, default: i32) -> i32;
    fn extract_query(&self, params: &Value, default: &str) -> String;
}
```

**TagFilter** - Tag filtering logic
```rust
trait TagFilter {
    fn extract_tag_filter<'a>(...) -> &'a str;
    fn filter_tags(&self, tags: &[String], filter: &str) -> Vec<String>;
}
```

**ResponseFormatter** - Format API responses
```rust
trait ResponseFormatter {
    fn format_list(&self, data: Value, pagination: Option<Value>, meta: Option<Value>) -> Value;
    fn format_detail(&self, data: Value) -> Value;
}
```

**Paginator** - Pagination handling
```rust
trait Paginator {
    fn parse_pagination(&self, params: &Value) -> (usize, usize);
}
```

### Handler Pattern

```rust
pub struct LogsHandler;

impl TimeHandler for LogsHandler {}
impl TagFilter for LogsHandler {}
impl ParameterParser for LogsHandler {}
impl ResponseFormatter for LogsHandler {}

impl LogsHandler {
    pub async fn search(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = LogsHandler;

        // Use traits
        let (from_iso, to_iso) = handler.parse_time_iso8601(params)?;
        let tag_filter = handler.extract_tag_filter(params, &client);
        let limit = handler.extract_i32(params, "limit", 10);

        // API call
        let response = client.search_logs(query, &from_iso, &to_iso, Some(limit)).await?;

        // Format response
        Ok(handler.format_list(data, Some(pagination), None))
    }
}
```

---

## Code Organization

```
src/
├── cli/
│   ├── mod.rs              # Clap CLI definition
│   │   - GlobalOpts (api-key, app-key, format)
│   │   - Command enum (Metrics, Logs, Monitors, etc.)
│   │   - run() function (entry point)
│   ├── commands.rs         # Command dispatcher
│   │   - execute() - routes commands to handlers
│   │   - handle_config() - config management
│   └── output.rs           # Output formatting
│       - Format enum (Json, JsonLines, Table)
│       - print() - format selection
│
├── datadog/
│   ├── client.rs           # DatadogClient
│   │   - new() - client creation
│   │   - API methods (list_monitors, search_logs, etc.)
│   │   - retry logic integration
│   ├── models.rs           # API response types
│   │   - LogsResponse, Monitor, Event, etc.
│   └── retry.rs            # Exponential backoff
│       - RetryPolicy
│
├── handlers/               # 12 handlers (one per command type)
│   ├── common.rs           # Shared traits
│   ├── metrics.rs          # Metrics query
│   ├── logs.rs             # Logs search
│   ├── logs_aggregate.rs   # Logs aggregation
│   ├── logs_timeseries.rs  # Logs time series
│   ├── monitors.rs         # Monitors list/get
│   ├── events.rs           # Events query
│   ├── hosts.rs            # Hosts list
│   ├── dashboards.rs       # Dashboards list/get
│   ├── spans.rs            # APM spans
│   ├── services.rs         # Services catalog
│   └── rum.rs              # RUM events
│
├── error.rs                # DatadogError (thiserror)
├── utils.rs                # parse_time(), truncate_stack_trace()
├── lib.rs                  # Library entry
└── main.rs                 # Binary entry
```

---

## Important Patterns

### 1. Error Handling (thiserror)

```rust
// src/error.rs
#[derive(Error, Debug)]
pub enum DatadogError {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DatadogError>;
```

Usage: `return Err(DatadogError::InvalidInput("Missing query".into()))?`

### 2. Time Parsing (interim + chrono)

```rust
// src/utils.rs
pub fn parse_time(input: &str) -> Result<i64> {
    use interim::parse_datetime;

    // Natural language: "1 hour ago", "yesterday"
    if let Ok(dt) = parse_datetime(input) {
        return Ok(dt.timestamp());
    }

    // Unix timestamp: "1704067200"
    if let Ok(ts) = input.parse::<i64>() {
        return Ok(ts);
    }

    // ISO8601: "2025-01-15T10:30:00Z"
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(input) {
        return Ok(dt.timestamp());
    }

    Err(DatadogError::InvalidInput(format!("Invalid time: {}", input)))
}
```

### 3. Output Formatting

```rust
// src/cli/output.rs
pub enum Format {
    Json,       // Pretty JSON (default)
    JsonLines,  // One JSON per line (Unix tools)
    Table,      // comfy-table (human-readable)
}

pub fn print(data: &Value, format: &Format) -> io::Result<()> {
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut io::stdout().lock(), data)?;
        }
        Format::JsonLines => {
            if let Some(items) = data.get("data").and_then(|d| d.as_array()) {
                for item in items {
                    serde_json::to_writer(&mut io::stdout().lock(), item)?;
                    writeln!(io::stdout())?;
                }
            }
        }
        Format::Table => {
            // comfy-table rendering
        }
    }
    Ok(())
}
```

### 4. Retry Logic (exponential backoff)

```rust
// src/datadog/retry.rs
pub struct RetryPolicy {
    max_retries: u32,  // Default: 3
    base_delay_ms: u64, // Default: 1000
}

impl RetryPolicy {
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        for attempt in 0..=self.max_retries {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < self.max_retries => {
                    let delay = self.base_delay_ms * 2_u64.pow(attempt);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }
}
```

---

## Common Tasks

### Adding a New Command

**1. Define in CLI** (src/cli/mod.rs)
```rust
#[derive(Subcommand)]
pub enum Command {
    // ... existing commands

    NewCommand {
        #[arg(short, long)]
        param: String,
    },
}
```

**2. Create Handler** (src/handlers/new_command.rs)
```rust
use crate::handlers::common::*;

pub struct NewCommandHandler;

impl TimeHandler for NewCommandHandler {}
impl ParameterParser for NewCommandHandler {}

impl NewCommandHandler {
    pub async fn action(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = NewCommandHandler;

        // Extract parameters
        let param = handler.extract_string(params, "param")
            .ok_or_else(|| DatadogError::InvalidInput("Missing param".into()))?;

        // Call API
        let response = client.new_api_method(&param).await?;

        // Format response
        Ok(handler.format_detail(json!(response)))
    }
}
```

**3. Add to Dispatcher** (src/cli/commands.rs)
```rust
pub async fn execute(command: &Command, client: Arc<DatadogClient>) -> Result<Value> {
    match command {
        // ... existing commands

        Command::NewCommand { param } => {
            let params = json!({"param": param});
            handlers::new_command::NewCommandHandler::action(client, &params).await
        }
    }
}
```

**4. Add to Module** (src/handlers/mod.rs)
```rust
pub mod new_command;
```

**5. Add Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_extraction() {
        let params = json!({"param": "value"});
        assert_eq!(params["param"].as_str(), Some("value"));
    }
}
```

### Modifying Existing Handler

**Pattern:**
1. Read `src/handlers/common.rs` for available traits
2. Use trait methods instead of duplicating code
3. Follow existing handler patterns
4. Add tests for new behavior

**Example: Add new parameter**
```rust
// Before
pub async fn search(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
    let query = params["query"].as_str().unwrap_or("*").to_string();
    // ...
}

// After (using ParameterParser trait)
pub async fn search(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
    let handler = LogsHandler;
    let query = handler.extract_query(params, "*");
    let new_param = handler.extract_string(params, "new_param");
    // ...
}
```

### Adding New Output Format

**1. Add Format Variant** (src/cli/output.rs)
```rust
pub enum Format {
    Json,
    JsonLines,
    Table,
    NewFormat, // Add here
}
```

**2. Implement Formatting**
```rust
impl Format {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Format::Json),
            "jsonl" | "jsonlines" => Ok(Format::JsonLines),
            "table" => Ok(Format::Table),
            "newformat" => Ok(Format::NewFormat), // Add here
            _ => Err(format!("Invalid format: {}", s)),
        }
    }
}

pub fn print(data: &Value, format: &Format) -> io::Result<()> {
    match format {
        Format::Json => print_json(data),
        Format::JsonLines => print_jsonlines(data),
        Format::Table => print_table(data),
        Format::NewFormat => print_new_format(data), // Implement
    }
}

fn print_new_format(data: &Value) -> io::Result<()> {
    // Implementation
}
```

---

## Testing

### Structure
- Unit tests: Each module has `#[cfg(test)] mod tests`
- 117 tests total (all passing)
- No mocking - test logic, not API calls

### Running
```bash
cargo test                    # All tests
cargo test --lib handlers     # Handler tests only
cargo test -- --nocapture     # Verbose output
```

### Key Test Patterns

**1. Trait Testing**
```rust
#[test]
fn test_time_handler() {
    let handler = LogsHandler;
    let params = json!({"from": "1 hour ago", "to": "now"});
    let result = handler.parse_time(&params, 1);
    assert!(result.is_ok());
}
```

**2. Parameter Extraction**
```rust
#[test]
fn test_param_parser() {
    let handler = LogsHandler;
    let params = json!({"limit": 50});
    assert_eq!(handler.extract_i32(&params, "limit", 10), 50);
    assert_eq!(handler.extract_i32(&params, "missing", 10), 10);
}
```

**3. Error Cases**
```rust
#[test]
fn test_missing_query() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = Arc::new(DatadogClient::new("key".into(), "app".into(), None).unwrap());
        let params = json!({});
        let result = LogsHandler::search(client, &params).await;
        assert!(result.is_err());
    });
}
```

---

## Dependencies

### Production (12)
- **tokio** (1.48) - Async runtime
- **clap** (4.5.51) - CLI parsing
- **reqwest** (0.12) - HTTP client (rustls-tls)
- **serde / serde_json** (1.0) - Serialization
- **thiserror** (2.0) - Error types
- **chrono** (0.4) - Date/time
- **interim** (0.2) - Natural language time
- **dotenvy** (0.15) - .env files
- **comfy-table** (7.2.1) - Table formatting
- **log / env_logger** (0.4 / 0.11) - Logging

### Dev (5)
- **wiremock** (0.6) - HTTP mocking
- **criterion** (0.5) - Benchmarks
- **tokio-test** (0.4) - Async test utils
- **assert_matches** (1.5) - Pattern matching assertions
- **serial_test** (3.2) - Serial test execution

---

## Build & Release

### Development
```bash
cargo build
cargo run -- monitors list
RUST_LOG=debug cargo run -- monitors list
```

### Release
```bash
cargo build --release
# Result: target/release/datadog (5.1MB)
```

**Optimization (Cargo.toml):**
```toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit
opt-level = 3           # Maximum optimization
strip = true            # Strip symbols
```

---

## Quick Reference

### File to Edit for Common Changes

| Task | File |
|------|------|
| Add command | `src/cli/mod.rs` + new handler |
| Change API endpoint | `src/datadog/client.rs` |
| Add output format | `src/cli/output.rs` |
| Modify retry logic | `src/datadog/retry.rs` |
| Add new trait | `src/handlers/common.rs` |
| Fix parsing | `src/utils.rs` |
| Add response type | `src/datadog/models.rs` |

### Debug Commands
```bash
# See all logs
RUST_LOG=debug cargo run -- <command>

# Test specific module
cargo test handlers::logs::tests

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

---

**For users:** [README.md](README.md)
