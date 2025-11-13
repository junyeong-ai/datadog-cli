# Datadog CLI - AI Agent Developer Guide

Quick reference for AI agents maintaining this Rust CLI tool.

## Quick Reference

**What**: High-performance Datadog CLI with TOML config
**Stack**: Rust 2024 (1.91.1+), clap, tokio, reqwest, toml, dirs
**Tests**: 122 passing (14s)
**Binary**: 5.1MB (LTO optimized)
**Commands**: 13 (logs, metrics, monitors, events, hosts, spans, services, rum, dashboards, config)
**Config**: TOML only (`~/.config/datadog-cli/config.toml`)

---

## Project Structure

```
src/
├── main.rs              # Entry: Tokio runtime, CLI routing
├── config.rs            # TOML config loader (~/.config/datadog-cli/config.toml)
├── cli/
│   ├── mod.rs           # clap: Cli struct, Config::load()
│   ├── commands.rs      # Command dispatcher
│   └── output.rs        # JSON/JSONL/Table formatting
├── datadog/
│   ├── client.rs        # HTTP client (reqwest + rustls)
│   ├── retry.rs         # Exponential backoff (3 retries)
│   └── models.rs        # API response types
├── handlers/            # 13 command handlers
│   ├── common.rs        # Shared traits (TimeHandler, ParameterParser, etc.)
│   ├── logs.rs          # logs search
│   ├── logs_aggregate.rs
│   ├── logs_timeseries.rs
│   ├── metrics.rs
│   ├── monitors.rs
│   ├── events.rs
│   ├── hosts.rs
│   ├── dashboards.rs
│   ├── spans.rs
│   ├── services.rs
│   └── rum.rs
├── error.rs             # DatadogError (thiserror)
└── utils.rs             # parse_time() (interim + chrono)
```

---

## Architecture

### Data Flow

```
Terminal Command
  ↓ clap::Parser
Cli struct (cli/mod.rs)
  ↓ Config::load()
TOML config (~/.config/datadog-cli/config.toml)
  ↓ match command
Handler (handlers/*.rs)
  ↓ API call
DatadogClient (datadog/client.rs)
  ↓ HTTP/2 + rustls
Datadog API
  ↓ format output
stdout (JSON/JSONL/Table)
```

**Key Points**:
- Config: TOML only (no env vars, no .env)
- API: HTTP/2 with rustls-tls
- Retry: Exponential backoff (1s, 2s, 4s)
- Time: Natural language via interim library

---

## Key Components

### Config (src/config.rs)

```rust
pub struct Config {
    pub api_key: String,
    pub app_key: String,
    pub site: String,  // default: "datadoghq.com"
}

impl Config {
    pub fn load() -> Result<Self>        // Load from TOML
    pub fn init() -> Result<PathBuf>     // Create template
    pub fn show() -> Result<String>      // Display (masked)
    pub fn edit() -> Result<()>          // Open $EDITOR
    pub fn config_path() -> Option<PathBuf>  // ~/.config/datadog-cli/config.toml
}
```

**File location**: `~/.config/datadog-cli/config.toml`
```toml
api_key = "your-api-key"
app_key = "your-app-key"
site = "datadoghq.com"
```

**Permissions**: 0o600 (Unix only, warning if too permissive)

### CLI (src/cli/mod.rs)

```rust
pub struct Cli {
    pub format: String,  // json|jsonl|table
    pub quiet: bool,
    pub verbose: bool,
    pub command: Command,
}

pub enum Command {
    Metrics { query, from, to, max_points },
    Logs { action: LogsAction },
    Monitors { action: MonitorsAction },
    // ... 10 more
    Config { action: ConfigAction },
}
```

**No GlobalOpts** - Config loaded separately via `Config::load()`

### Handler Pattern (src/handlers/*.rs)

All handlers implement common traits from `src/handlers/common.rs`:
- `TimeHandler`: Natural language time → Unix timestamp
- `ParameterParser`: Extract params from JSON
- `TagFilter`: Filter tags by prefix
- `ResponseFormatter`: Format API responses
- `Paginator`: Handle pagination

Example handler:
```rust
pub struct LogsHandler;

impl TimeHandler for LogsHandler {}
impl ParameterParser for LogsHandler {}
impl TagFilter for LogsHandler {}
impl ResponseFormatter for LogsHandler {}

impl LogsHandler {
    pub async fn search(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        // Use trait methods, call API, format response
    }
}
```

### Time Parsing (src/utils.rs)

```rust
pub fn parse_time(input: &str) -> Result<i64>
```

Supports:
- Natural language: "1 hour ago", "yesterday" (interim library)
- ISO8601: "2025-01-15T10:30:00Z" (chrono)
- Unix timestamp: "1704067200"

### Error Handling (src/error.rs)

```rust
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

### Output Formatting (src/cli/output.rs)

```rust
pub enum Format {
    Json,       // Pretty JSON (default)
    JsonLines,  // One JSON per line (Unix tools)
    Table,      // comfy-table
}

pub fn print(data: &Value, format: &Format) -> io::Result<()>
```

### Retry Logic (src/datadog/retry.rs)

```rust
pub struct RetryPolicy {
    max_retries: 3,
    base_delay_ms: 1000,
}

// Exponential backoff: 1s, 2s, 4s
```

---

## Common Tasks

### Adding a Command

1. Add variant to `Command` enum (cli/mod.rs)
2. Create handler in `handlers/` directory
3. Implement required traits from `common.rs`
4. Add to dispatcher in `cli/commands.rs`
5. Add to `handlers/mod.rs`

### Modifying Handler

- Read `handlers/common.rs` for available traits
- Use trait methods instead of duplicating code
- Follow existing handler patterns

### Adding Output Format

- Add variant to `Format` enum (cli/output.rs)
- Implement formatting in `print()` function

---

## Testing

```bash
cargo test                  # All 122 tests (14s)
cargo test handlers         # Handler tests only
cargo test --lib config     # Config tests only
```

**Test structure**:
- Unit tests in each module (`#[cfg(test)] mod tests`)
- No mocking - test logic, not API calls
- Handler tests focus on parameter extraction and formatting

---

## Build & Release

```bash
# Development
cargo build
cargo run -- monitors list

# Release (5.1MB optimized)
cargo build --release

# Install
cp target/release/datadog ~/.local/bin/
```

**Optimization** (Cargo.toml):
```toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1
opt-level = 3
strip = true
```

---

## Dependencies

**Production** (13):
- tokio: Async runtime
- clap: CLI parsing
- reqwest: HTTP client (rustls-tls)
- serde: Serialization framework
- serde_json: JSON support
- toml: Config parsing
- dirs: Config path
- thiserror: Error types
- chrono: Date/time
- interim: Natural language time
- comfy-table: Table formatting
- log: Logging facade
- env_logger: Logger implementation

**Dev** (5):
- wiremock: HTTP mocking
- criterion: Benchmarks
- tokio-test: Async test utils
- assert_matches: Pattern matching assertions
- serial_test: Serial test execution

---

## Quick Reference Table

| Task | File |
|------|------|
| Add command | cli/mod.rs + new handler |
| Change API endpoint | datadog/client.rs |
| Add output format | cli/output.rs |
| Modify retry logic | datadog/retry.rs |
| Add trait | handlers/common.rs |
| Fix time parsing | utils.rs |
| Add response type | datadog/models.rs |
| Config logic | config.rs |

---

## Debug Commands

```bash
# Debug logging
RUST_LOG=debug cargo run -- monitors list

# Test specific module
cargo test handlers::logs::tests

# Format check
cargo fmt --check

# Linting
cargo clippy -- -D warnings
```

---

**For users**: See [README.md](README.md)
