# Datadog CLI

[![CI](https://github.com/junyeong-ai/datadog-cli/workflows/CI/badge.svg)](https://github.com/junyeong-ai/datadog-cli/actions)
[![Lint](https://github.com/junyeong-ai/datadog-cli/workflows/Lint/badge.svg)](https://github.com/junyeong-ai/datadog-cli/actions)
[![codecov](https://codecov.io/gh/junyeong-ai/datadog-cli/branch/main/graph/badge.svg)](https://codecov.io/gh/junyeong-ai/datadog-cli)
[![Rust](https://img.shields.io/badge/rust-1.91.1%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.1.0-blue?style=flat-square)](https://github.com/junyeong-ai/datadog-cli/releases)

> **[한국어](README.md)** | **🌐 English**

---

> **⚡ Fast and Powerful Datadog API Query Tool**
>
> - 🚀 **High Performance** (Rust-based, 10x faster than Python SDK)
> - 🕐 **Natural Time** ("1 hour ago", "30 minutes ago")
> - 📊 **Multiple Outputs** (JSON, JSONL, Table)
> - 🔒 **Secure** (rustls-based TLS 1.3)

---

## ⚡ Quick Start (1 minute)

```bash
# 1. Install
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh | bash

# 2. Initialize config
datadog-cli config init

# 3. Set API keys
datadog-cli config edit

# 4. Start using! 🎉
datadog-cli monitors list
datadog-cli logs search "status:error" --from "1 hour ago"
datadog-cli metrics "avg:system.cpu.user{*}"
```

---

## 🎯 Key Features

### Logs
```bash
# Search logs (natural time)
datadog-cli logs search "service:web status:error" --from "1 hour ago"

# Aggregate logs (count)
datadog-cli logs aggregate "service:api" --from "6 hours ago"

# Timeseries analysis
datadog-cli logs timeseries "status:error" \
  --from "24 hours ago" \
  --interval "1h" \
  --aggregation "count"
```

### Metrics
```bash
# Query metrics
datadog-cli metrics "avg:system.cpu.user{*}"

# Filter by tags
datadog-cli metrics "avg:system.cpu.user{service:web}"

# Group by
datadog-cli metrics "avg:system.cpu.user{*} by {service}"
```

### APM & RUM
```bash
# Search spans (errors only)
datadog-cli spans "service:api error:true" --from "30 minutes ago"

# RUM events
datadog-cli rum "@type:error" --from "1 hour ago"

# List services
datadog-cli services --env production
```

### Monitoring
```bash
# List monitors
datadog-cli monitors list --tags "env:prod"

# Get monitor details
datadog-cli monitors get 12345678

# Query events
datadog-cli events --from "1 day ago" --priority "normal"
```

### Infrastructure
```bash
# List hosts
datadog-cli hosts --filter "env:production"

# List dashboards
datadog-cli dashboards list
```

---

## 📦 Installation

### Method 1: Prebuilt Binary (Recommended) ⭐

**Automatic install**:
```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh | bash
```

**Manual install**:
1. Download binary from [Releases](https://github.com/junyeong-ai/datadog-cli/releases)
2. Extract: `tar -xzf datadog-*.tar.gz`
3. Move to PATH: `mv datadog-cli ~/.local/bin/`

### Method 2: Cargo

```bash
cargo install datadog-cli
```

### Method 3: Build from Source

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
./scripts/install.sh
```

**Requirements**: Rust 1.91.1+

### 🤖 Claude Code Skill (Optional)

When running `./scripts/install.sh`, you can choose to install the Claude Code skill:

- **User-level** (recommended): Available in all projects
- **Project-level**: Team auto-deployment via Git
- **Skip**: Manual installation later

Installing the skill enables natural language Datadog queries in Claude Code.

---

## ⚙️ Configuration

### Priority

```
1. CLI arguments     --api-key, --app-key (highest)
2. Environment vars  DD_API_KEY, DD_APP_KEY, DD_SITE
3. Project config    ./.datadog.toml
4. Global config     ~/.config/datadog-cli/config.toml
```

### Config Files

**Global config** (`~/.config/datadog-cli/config.toml`):

```toml
api_key = "your-api-key-here"
app_key = "your-app-key-here"
site = "datadoghq.com"  # or datadoghq.eu, ddog-gov.com, etc.

[defaults]
format = "json"           # Output format: json, jsonl, table
time_range = "1 hour ago" # Default time range
limit = 10                # Default result limit
page_size = 100           # Items per page
# tag_filter = "env:,service:"  # Tag filter (optional)

[network]
timeout_secs = 30         # Request timeout (seconds)
max_retries = 3           # Max retry attempts
```

**Project config** (`.datadog.toml`):

```toml
# Use different keys per project
api_key = "project-specific-key"
app_key = "project-specific-app-key"
site = "datadoghq.eu"
```

### Config Management

```bash
# Initialize config
datadog-cli config init

# Show config (tokens masked)
datadog-cli config show

# Config file path
datadog-cli config path

# Edit with $EDITOR
datadog-cli config edit
```

### Environment Variables

```bash
export DD_API_KEY="your-api-key"
export DD_APP_KEY="your-app-key"
export DD_SITE="datadoghq.com"
```

---

## 💡 Usage Tips

### Natural Time Parsing

```bash
# Natural language (recommended)
datadog-cli logs search "query" --from "1 hour ago" --to "now"
datadog-cli metrics "query" --from "30 minutes ago"

# ISO8601
datadog-cli logs search "query" --from "2024-01-01T00:00:00Z"

# Unix timestamp
datadog-cli metrics "query" --from "1704067200"
```

### Unix Pipeline Integration

```bash
# Extract metric points with jq
datadog-cli metrics "system.cpu.user" --format jsonl | jq '.series[].pointlist'

# Extract log messages only
datadog-cli logs search "query" --format jsonl | jq -r '.logs[].message'

# Count errors
datadog-cli logs search "status:error" --format jsonl | jq '.logs | length'
```

### Table Output

```bash
# Human-readable table format
datadog-cli monitors list --format table
datadog-cli hosts --format table
```

### Tag Filtering

```bash
# 30-70% response size reduction
datadog-cli logs search "query" --tag-filter "env:,service:"

# Exclude all tags
datadog-cli logs search "query" --tag-filter ""

# Include all tags (default)
datadog-cli logs search "query" --tag-filter "*"
```

**Environment variable**:
```bash
export DD_TAG_FILTER="env:,service:"
```

**Applies to**: logs search, spans, rum, hosts

---

## 📖 Commands

| Command | Description | Example |
|---------|-------------|---------|
| `metrics` | Query metrics | `datadog-cli metrics "avg:system.cpu.user{*}"` |
| `logs search` | Search logs | `datadog-cli logs search "query" --from "1h ago"` |
| `logs aggregate` | Aggregate logs | `datadog-cli logs aggregate "query" --from "6h ago"` |
| `logs timeseries` | Logs timeseries | `datadog-cli logs timeseries "query" --interval "1h"` |
| `monitors list` | List monitors | `datadog-cli monitors list --tags "env:prod"` |
| `monitors get` | Get monitor | `datadog-cli monitors get 12345678` |
| `events` | Query events | `datadog-cli events --from "1 day ago"` |
| `hosts` | List hosts | `datadog-cli hosts --filter "env:production"` |
| `dashboards list` | List dashboards | `datadog-cli dashboards list` |
| `dashboards get` | Get dashboard | `datadog-cli dashboards get abc-def-ghi` |
| `spans` | Search APM spans | `datadog-cli spans "service:api" --from "..."` |
| `services` | List services | `datadog-cli services --env prod` |
| `rum` | Search RUM events | `datadog-cli rum "@type:error"` |
| `config` | Config management | `datadog-cli config show` |

---

## 🛠️ Troubleshooting

### Config Not Found

**Symptom**: `Config not found` error

**Solution**:
```bash
# 1. Create config file
datadog-cli config init

# 2. Check config path
datadog-cli config path

# 3. Set API keys
datadog-cli config edit
```

### Auth Failure

**Symptom**: `AuthError` or 403 error

**Solution**:
1. Check API keys: `datadog-cli config show`
2. Regenerate API keys in Datadog
3. Test with environment variables:
   ```bash
   DD_API_KEY="new-key" DD_APP_KEY="new-app-key" datadog-cli monitors list
   ```

### Invalid Site

**Symptom**: `Invalid site` error

**Solution**:
```bash
# Check and edit site
datadog-cli config edit
# Set site to one of:
# - datadoghq.com (US1)
# - datadoghq.eu (EU)
# - ddog-gov.com (US1-FED)
# - us3.datadoghq.com (US3)
# - us5.datadoghq.com (US5)
# - ap1.datadoghq.com (AP1)
```

---

## 🔧 Development

### Build

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run
cargo run -- metrics "system.cpu.user"
```

### Test

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With debug logs
RUST_LOG=debug cargo test
```

### Code Quality

```bash
# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Run all
cargo fmt && cargo clippy -- -D warnings && cargo test
```

---

## 🤝 Contributing

Issues and PRs are welcome!

1. Fork
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit (`git commit -m 'Add amazing feature'`)
4. Push (`git push origin feature/amazing-feature`)
5. Pull Request

---

## 📄 License

MIT License - See [LICENSE](LICENSE)

---

## 🔗 Links

- [Datadog API Documentation](https://docs.datadoghq.com/api/)
- [GitHub Repository](https://github.com/junyeong-ai/datadog-cli)
- [Issue Tracker](https://github.com/junyeong-ai/datadog-cli/issues)

---

**For AI Agents**: See [CLAUDE.md](CLAUDE.md)
