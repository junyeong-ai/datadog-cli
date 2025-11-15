# Datadog CLI - AI Agent Developer Guide

Essential knowledge for maintaining and extending this Rust CLI tool.

---

## Core Patterns

### 4-Tier Config System

**Implementation** (`src/config.rs` lines 23-56):
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

**Priority**: CLI > ENV > Project > Global

---

### Handler Trait System

**Location**: `src/handlers/common.rs`

**Traits**:
- `TimeHandler`: Parse natural language, ISO8601, Unix timestamps
- `ParameterParser`: Build API query parameters
- `TagFilter`: Apply tag filters from env (`DD_FILTER_TAGS`)
- `ResponseFormatter`: Format API responses

**Why**: Shared logic across 11 handlers without duplication.

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

**Location**: `src/utils.rs` lines 1-50

---

### Exponential Backoff Retry

**Implementation** (`src/datadog/retry.rs`):
```rust
const MAX_RETRIES: u32 = 3;
const DELAYS: [u64; 3] = [2, 4, 8];  // seconds

pub async fn retry_with_backoff<F, Fut, T>(f: F) -> Result<T>
```

**Why**: Handles transient network errors, rate limits (429).

**Location**: `src/datadog/retry.rs` lines 1-50

---

## Development Tasks

### Add New Command

1. **Add to `Command` enum** (`src/cli/mod.rs` lines 30-171)
2. **Add handler** (`src/handlers/`)
3. **Add to dispatcher** (`src/cli/commands.rs`)
4. **Implement handler trait** if using shared patterns

### Add Handler Trait Method

1. **Define in `common.rs`** (`src/handlers/common.rs`)
2. **Provide default impl** if possible
3. **Use in handlers** where needed

### Modify Config

1. **Update `Config` struct** (`src/config.rs` lines 6-16)
2. **Update `merge()` logic** if needed
3. **Update `validate()`** for new constraints
4. **Update `init()` template**

---

## Common Issues

### Time Parsing Fails

**Symptom**: `DateParseError`

**Cause**: Invalid time format

**Fix**: Check supported formats in `src/utils.rs`. Supports:
- Natural: "1 hour ago", "30 minutes ago", "now"
- ISO8601: "2024-01-01T00:00:00Z"
- Unix: "1704067200"

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
```

---

## Key Constants

**Locations**:
- `config.rs`: Default site (`datadoghq.com`)
- `datadog/retry.rs`: Max retries (3), delays (2s, 4s, 8s)
- `utils.rs`: Time parsing logic
- `handlers/common.rs`: Tag filter env var (`DD_FILTER_TAGS`)

**To modify**: Edit constant or add to `Config` struct for user configuration.

---

## Module Reference

### src/config.rs

**4-tier config system**: CLI > ENV > Project > Global

**Methods**:
- `load()`: Load with priority
- `find_project_config()`: Walk up directories
- `global_config_path()`: XDG config dir
- `init()`: Create default config
- `show()`: Display with masked secrets
- `edit()`: Open in $EDITOR

### src/datadog/client.rs

**HTTP client**: reqwest with rustls, retry logic

**Methods**:
- `new()`: Create client
- `request()`: Generic request with retry
- Specific methods: `query_metrics()`, `search_logs()`, etc.

### src/cli/commands.rs

**Command dispatcher**: Routes commands to handlers

**Pattern**: Each command calls appropriate handler from `src/handlers/`

### src/handlers/

**11 handlers**: metrics, logs, logs_aggregate, logs_timeseries, monitors, events, hosts, dashboards, spans, services, rum

**Common traits**: `TimeHandler`, `ParameterParser`, `TagFilter`, `ResponseFormatter`

---

**For users**: See [README.md](README.md)
