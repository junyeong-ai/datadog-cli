# Datadog CLI - AI Agent Developer Guide

Essential knowledge for implementing features and debugging this Rust CLI tool.

---

## Core Patterns

### 4-Tier Configuration System

**Implementation** (`src/config.rs:23-56`):
```rust
pub fn load(
    cli_api_key: Option<String>,
    cli_app_key: Option<String>,
    cli_site: Option<String>,
) -> Result<Self> {
    // 1. Load file (project or global)
    // 2. Override with env vars (DD_API_KEY, DD_APP_KEY, DD_SITE)
    // 3. Override with CLI args
    // 4. Validate
}
```

**Why**: Flexible configuration for different environments. Project config (`.datadog.toml`) discovered by walking up directories.

**Priority**: CLI args → ENV vars → Project config → Global config

**Location**: Global config at `~/.config/datadog-cli/config.toml`

---

### Handler Trait System

**Location**: `src/handlers/common.rs`

**Traits**:
- `TimeHandler`: Parse natural language, ISO8601, Unix timestamps
- `ParameterParser`: Build API query parameters
- `TagFilter`: Apply tag filters from env (`DD_TAG_FILTER`)
- `ResponseFormatter`: Format API responses (JSON, JSONL, Table)
- `Paginator`: Handle pagination for list operations
- `ResponseFilter`: Filter response data

**Why**: Shared logic across 11 handlers without duplication. Each handler implements only the traits it needs.

**Example**:
```rust
impl TimeHandler for LogsHandler {}
impl TagFilter for LogsHandler {}
impl ResponseFormatter for LogsHandler {}
```

---

### Natural Language Time Parsing

**Implementation** (`src/utils.rs`):
```rust
pub fn parse_time(input: &str) -> Result<i64> {
    // "1 hour ago" -> Unix timestamp
    // ISO8601 -> Unix timestamp
    // Unix timestamp -> Unix timestamp
}
```

**Why**: User-friendly time input. Uses `interim` + `chrono` crates.

**Supported formats**:
- Natural: "1 hour ago", "30 minutes ago", "now"
- ISO8601: "2024-01-01T00:00:00Z"
- Unix: "1704067200"

---

### Exponential Backoff Retry

**Implementation** (`src/datadog/retry.rs`):
```rust
const MAX_RETRIES: u32 = 3;
const DELAYS: [u64; 3] = [2, 4, 8];  // seconds

pub async fn retry_with_backoff<F, Fut, T>(f: F) -> Result<T>
```

**Why**: Handles transient network errors, rate limits (429).

**Behavior**: 3 retries with 2s, 4s, 8s delays. Total max wait: 14s.

---

## Development Tasks

### Add New Command

1. **Add to `Command` enum** (`src/cli/mod.rs:36-177`)
   ```rust
   #[derive(Subcommand)]
   pub enum Command {
       NewCommand {
           #[arg(long)]
           param: String,
       },
   }
   ```

2. **Add handler** (`src/handlers/new_command.rs`)
   ```rust
   pub async fn handle(param: String) -> Result<()> {
       // Implementation
   }
   ```

3. **Add to dispatcher** (`src/cli/commands.rs`)
   ```rust
   Command::NewCommand { param } => {
       handlers::new_command::handle(param).await?;
   }
   ```

4. **Implement traits** if using shared patterns
   ```rust
   impl TimeHandler for NewCommandHandler {}
   impl ResponseFormatter for NewCommandHandler {}
   ```

---

### Add Handler Trait Method

1. **Define in `common.rs`** (`src/handlers/common.rs`)
   ```rust
   pub trait NewTrait {
       fn new_method(&self) -> Result<String>;
   }
   ```

2. **Provide default impl** if possible
   ```rust
   impl<T> NewTrait for T {
       fn new_method(&self) -> Result<String> {
           Ok("default".to_string())
       }
   }
   ```

3. **Use in handlers** where needed

---

### Modify Config

1. **Update `Config` struct** (`src/config.rs:7-16`)
   ```rust
   pub struct Config {
       pub new_field: Option<String>,
   }
   ```

2. **Update `merge()` logic** if needed (`src/config.rs:113-124`)
   ```rust
   if other.new_field.is_some() {
       self.new_field = other.new_field;
   }
   ```

3. **Update `validate()`** for new constraints (`src/config.rs:126-147`)

4. **Update `init()` template** (`src/config.rs:149-180`, template at 164-167)

---

## Common Issues

### Time Parsing Fails

**Symptom**: `DateParseError`

**Cause**: Invalid time format

**Fix**: Check supported formats in `src/utils.rs`:
- Natural: "1 hour ago", "30 minutes ago", "now"
- ISO8601: "2024-01-01T00:00:00Z"
- Unix: "1704067200"

**Example**:
```bash
# Good
datadog logs search "query" --from "1 hour ago"

# Bad
datadog logs search "query" --from "yesterday"  # Not supported
```

---

### Config Not Found

**Symptom**: `Config not found` error

**Check**:
```bash
datadog config path
```

**Fix**:
```bash
datadog config init
```

**Note**: Searches for `.datadog.toml` in current directory and parents, then falls back to `~/.config/datadog-cli/config.toml`.

---

### Handler Not Using Traits

**Symptom**: Code duplication across handlers

**Fix**: Implement traits from `common.rs`:
```rust
impl TimeHandler for MyHandler {}
impl ParameterParser for MyHandler {}
impl ResponseFormatter for MyHandler {}
```

**Benefit**: Automatic access to shared functionality without reimplementation.

---

### Tag Filter Not Working

**Symptom**: Response includes all tags despite `--tag-filter`

**Cause**: Not implementing `TagFilter` trait or not calling it

**Fix**:
```rust
impl TagFilter for MyHandler {}

// In handler
let params = self.apply_tag_filter(base_params, tag_filter);
```

**Note**: Tag filtering reduces response size by 30-70% by excluding unwanted tag prefixes.

---

## Key Constants

**Locations**:
- `src/config.rs`: Default site (`datadoghq.com`), config paths
- `src/datadog/retry.rs`: Max retries (3), delays (2s, 4s, 8s)
- `src/utils.rs`: Time parsing logic
- `src/datadog/client.rs`: Tag filter env var (`DD_TAG_FILTER`)
- `src/handlers/logs.rs`: Default query (`*`), default limit (100)
- `src/handlers/metrics.rs`: Rollup interval calculation constants

**To modify**: Edit constant or add to `Config` struct for user configuration.

---

## Module Reference

### src/config.rs

**4-tier config system**: CLI args → ENV vars → Project → Global

**Methods**:
- `load()`: Load with priority system
- `find_project_config()`: Walk up directories for `.datadog.toml`
- `global_config_path()`: Returns `~/.config/datadog-cli/config.toml`
- `init()`: Create default config at global path
- `show()`: Display with masked secrets
- `edit()`: Open in `$EDITOR`
- `validate()`: Check required fields and site validity

---

### src/datadog/client.rs

**HTTP client**: reqwest with rustls, retry logic

**Methods**:
- `new()`: Create client with config
- `request()`: Generic request with retry
- Specific methods: `query_metrics()`, `search_logs()`, `list_monitors()`, etc.

**Features**:
- TLS 1.3 with rustls (no OpenSSL dependency)
- Exponential backoff retry (3 attempts)
- Rate limit handling (429 errors)
- Regional site support

---

### src/cli/commands.rs

**Command dispatcher**: Routes commands to handlers

**Pattern**: Each command calls appropriate handler from `src/handlers/`

**Example**:
```rust
Command::Logs { subcommand } => match subcommand {
    LogsSubcommand::Search { query, from, to, .. } => {
        handlers::logs::handle_search(query, from, to, ..).await?;
    }
}
```

---

### src/handlers/

**11 handlers**:
- metrics
- logs (search, aggregate, timeseries)
- monitors (list, get)
- events
- hosts
- dashboards (list, get)
- spans
- services
- rum

**Common traits**: `TimeHandler`, `ParameterParser`, `TagFilter`, `ResponseFormatter`, `Paginator`, `ResponseFilter`

**Pattern**: Each handler implements only the traits it needs, inheriting shared functionality automatically.

---

## Architecture Highlights

### Trait-Based Design

**Philosophy**: Composition over inheritance. Each handler is a thin wrapper that composes shared traits.

**Benefits**:
- Zero code duplication across handlers
- Easy to add new handlers (implement relevant traits)
- Type-safe shared functionality
- Compile-time verification of correct API usage

---

### Error Handling

**Strategy**: Centralized error types using `thiserror`

**Location**: `src/error.rs`

**Types**:
- `AuthError`: API key/app key issues
- `ApiError`: Datadog API errors (4xx, 5xx)
- `RateLimitError`: 429 errors
- `TimeoutError`: Request timeouts
- `DateParseError`: Time parsing failures
- `InvalidInput`: User input validation errors

**Why**: Type-safe error handling with automatic conversion and display formatting.

---

### Performance Optimizations

**Tokio**: Minimal features (`rt-multi-thread`, `macros`, `time`)

**Reqwest**: `rustls-tls` (no OpenSSL), `json` only

**Release profile** (`Cargo.toml`):
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

**Result**: ~3.6MB binary, 10x faster than Python SDK

---

This guide contains only implementation-critical knowledge. For user documentation, see [README.md](README.md).
