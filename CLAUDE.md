# Datadog CLI - AI Agent Developer Guide

Quick reference for AI agents maintaining this Rust CLI tool.

## Quick Reference

**Stack**: Rust 2024 (1.91.1+), clap, tokio, reqwest
**Commands**: metrics, logs, monitors, events, hosts, dashboards, spans, services, rum, config
**Config**: `~/.config/datadog-cli/config.toml` (TOML only, no env vars)

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
├── handlers/            # Command handlers (11 handlers + common traits)
│   ├── mod.rs           # Handler module
│   ├── common.rs        # Shared traits (TimeHandler, ParameterParser, etc.)
│   ├── metrics.rs       # metrics command
│   ├── logs.rs          # logs search
│   ├── logs_aggregate.rs  # logs aggregate
│   ├── logs_timeseries.rs # logs timeseries
│   ├── monitors.rs      # monitors list/get
│   ├── events.rs        # events command
│   ├── hosts.rs         # hosts command
│   ├── dashboards.rs    # dashboards list/get
│   ├── spans.rs         # spans command
│   ├── services.rs      # services command
│   └── rum.rs           # rum command
├── error.rs             # DatadogError (thiserror)
└── utils.rs             # parse_time() (interim + chrono)
```

---

## Architecture

**Data Flow**: Terminal → CLI Parser → `Config::load()` → Handler → DatadogClient → Datadog API → Format output

**Key Patterns**:
- **Config** (config.rs): TOML-only from `~/.config/datadog-cli/config.toml`, methods: `load()`, `init()`, `edit()`
- **Handlers** (handlers/*.rs): Implement traits from `common.rs` (TimeHandler, ParameterParser, TagFilter, ResponseFormatter)
- **Retry** (datadog/retry.rs): 3 retries with exponential backoff (2s, 4s, 8s)
- **Time** (utils.rs): Natural language ("1 hour ago"), ISO8601, Unix timestamps
- **Output** (cli/output.rs): JSON (default), JSONL (Unix tools), Table

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

## Development

**Build & Test**:
```bash
cargo build                              # Development build
cargo build --release                    # Optimized release
cargo test                               # All tests
cargo test handlers                      # Specific module
```

**Code Quality**:
```bash
cargo fmt                                # Format
cargo clippy -- -D warnings              # Lint
RUST_LOG=debug cargo run -- <command>    # Debug
```

**Key Dependencies**: tokio (async), clap (CLI), reqwest (HTTP), serde/serde_json (serialization), toml (config), interim (natural time), chrono (ISO8601)

---

**For users**: See [README.md](README.md)
