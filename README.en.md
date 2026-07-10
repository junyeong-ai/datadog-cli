# Datadog CLI

[![CI](https://github.com/junyeong-ai/datadog-cli/workflows/CI/badge.svg)](https://github.com/junyeong-ai/datadog-cli/actions)
[![Lint](https://github.com/junyeong-ai/datadog-cli/workflows/Lint/badge.svg)](https://github.com/junyeong-ai/datadog-cli/actions)
[![codecov](https://codecov.io/gh/junyeong-ai/datadog-cli/branch/main/graph/badge.svg)](https://codecov.io/gh/junyeong-ai/datadog-cli)
[![Rust](https://img.shields.io/badge/rust-1.97%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/version-0.2.0-blue?style=flat-square)](https://github.com/junyeong-ai/datadog-cli/releases)
[![DeepWiki](https://img.shields.io/badge/DeepWiki-junyeong--ai%2Fdatadog--cli-blue.svg?logo=data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACwAAAAyCAYAAAAnWDnqAAAAAXNSR0IArs4c6QAAA05JREFUaEPtmUtyEzEQhtWTQyQLHNak2AB7ZnyXZMEjXMGeK/AIi+QuHrMnbChYY7MIh8g01fJoopFb0uhhEqqcbWTp06/uv1saEDv4O3n3dV60RfP947Mm9/SQc0ICFQgzfc4CYZoTPAswgSJCCUJUnAAoRHOAUOcATwbmVLWdGoH//PB8mnKqScAhsD0kYP3j/Yt5LPQe2KvcXmGvRHcDnpxfL2zOYJ1mFwrryWTz0advv1Ut4CJgf5uhDuDj5eUcAUoahrdY/56ebRWeraTjMt/00Sh3UDtjgHtQNHwcRGOC98BJEAEymycmYcWwOprTgcB6VZ5JK5TAJ+fXGLBm3FDAmn6oPPjR4rKCAoJCal2eAiQp2x0vxTPB3ALO2CRkwmDy5WohzBDwSEFKRwPbknEggCPB/imwrycgxX2NzoMCHhPkDwqYMr9tRcP5qNrMZHkVnOjRMWwLCcr8ohBVb1OMjxLwGCvjTikrsBOiA6fNyCrm8V1rP93iVPpwaE+gO0SsWmPiXB+jikdf6SizrT5qKasx5j8ABbHpFTx+vFXp9EnYQmLx02h1QTTrl6eDqxLnGjporxl3NL3agEvXdT0WmEost648sQOYAeJS9Q7bfUVoMGnjo4AZdUMQku50McDcMWcBPvr0SzbTAFDfvJqwLzgxwATnCgnp4wDl6Aa+Ax283gghmj+vj7feE2KBBRMW3FzOpLOADl0Isb5587h/U4gGvkt5v60Z1VLG8BhYjbzRwyQZemwAd6cCR5/XFWLYZRIMpX39AR0tjaGGiGzLVyhse5C9RKC6ai42ppWPKiBagOvaYk8lO7DajerabOZP46Lby5wKjw1HCRx7p9sVMOWGzb/vA1hwiWc6jm3MvQDTogQkiqIhJV0nBQBTU+3okKCFDy9WwferkHjtxib7t3xIUQtHxnIwtx4mpg26/HfwVNVDb4oI9RHmx5WGelRVlrtiw43zboCLaxv46AZeB3IlTkwouebTr1y2NjSpHz68WNFjHvupy3q8TFn3Hos2IAk4Ju5dCo8B3wP7VPr/FGaKiG+T+v+TQqIrOqMTL1VdWV1DdmcbO8KXBz6esmYWYKPwDL5b5FA1a0hwapHiom0r/cKaoqr+27/XcrS5UwSMbQAAAABJRU5ErkJggg==)](https://deepwiki.com/junyeong-ai/datadog-cli)

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

# Formulas across multiple queries (v2 API)
datadog-cli timeseries "sum:errors{*}" "sum:hits{*}" --formula "a / b * 100"

# Single aggregate values over the window (v2 API)
datadog-cli scalar "avg:system.cpu.user{*} by {host}" --aggregator avg
```

### APM & RUM
```bash
# Search spans (errors only)
datadog-cli spans "service:api error:true" --from "30 minutes ago"

# RUM events
datadog-cli rum "@type:error" --from "1 hour ago"

# List services from Software Catalog
datadog-cli services --kind service --owner platform-team
```

### Monitoring
```bash
# List monitors
datadog-cli monitors list --tags "env:prod"

# Get monitor details
datadog-cli monitors get 12345678

# Search events (v2 query syntax)
datadog-cli events "source:alert status:error" --from "1 day ago"

# List SLOs / downtimes
datadog-cli slo list --query "checkout"
datadog-cli downtimes --current-only

# Incidents & Error Tracking
datadog-cli incidents list
datadog-cli error-tracking search "service:api" --track trace
```

### Infrastructure
```bash
# List hosts
datadog-cli hosts --filter "env:production"

# List dashboards
datadog-cli dashboards list

# Teams & audit trail
datadog-cli teams --keyword platform
datadog-cli audit "@action:login" --from "1 day ago"
```

### LLM Observability (preview API)
```bash
# Search LLM spans (the underlying Datadog API is in preview)
datadog-cli llm-obs "@ml_app:chatbot" --span-kind llm --from "1 hour ago"
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

**Requirements**: Rust 1.97+

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
# Or authenticate with a personal access token instead of the key pair
# (takes precedence when both are set):
# token = "ddpat_..."
site = "datadoghq.com"  # or datadoghq.eu, ddog-gov.com, etc.

[defaults]
format = "json"           # Output format: json, jsonl, table
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

# Or a personal access token instead of the key pair
export DD_TOKEN="ddpat_..."
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
# Extract metric values with jq
datadog-cli metrics "system.cpu.user" | jq '.data[].points'

# Extract log messages only
datadog-cli logs search "query" --format jsonl | jq -r '.message'

# Count errors
datadog-cli logs search "status:error" | jq '.data | length'
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

### Cursor Pagination

```bash
# Page through results with the cursor from the previous response
datadog-cli logs search "query" --limit 100
# → read .pagination.next_cursor from the output
datadog-cli logs search "query" --limit 100 --cursor "<next_cursor>"
```

**Applies to**: logs search, events, spans, rum, audit, llm-obs

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (invalid input, IO) |
| 2 | Usage error (clap) |
| 3 | Authentication failure |
| 4 | Datadog API error |
| 5 | Rate limit exceeded |
| 6 | Network error / timeout |
| 7 | Unexpected response format |

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
| `events` | Search events (v2) | `datadog-cli events "source:alert" --from "1 day ago"` |
| `hosts` | List hosts | `datadog-cli hosts --filter "env:production"` |
| `dashboards list` | List dashboards | `datadog-cli dashboards list` |
| `dashboards get` | Get dashboard | `datadog-cli dashboards get abc-def-ghi` |
| `spans` | Search APM spans | `datadog-cli spans "service:api" --from "..."` |
| `services` | Software Catalog entities | `datadog-cli services --kind service` |
| `rum` | Search RUM events | `datadog-cli rum "@type:error"` |
| `timeseries` | v2 formula queries | `datadog-cli timeseries "sum:a{*}" "sum:b{*}" --formula "a/b"` |
| `scalar` | v2 scalar queries | `datadog-cli scalar "avg:cpu{*}" --aggregator avg` |
| `slo list` / `slo get` | SLOs | `datadog-cli slo list --query "checkout"` |
| `incidents list` / `incidents get` | Incidents | `datadog-cli incidents list` |
| `error-tracking search` / `get` | Error Tracking issues | `datadog-cli error-tracking search --track trace` |
| `downtimes` | List downtimes | `datadog-cli downtimes --current-only` |
| `audit` | Audit trail search | `datadog-cli audit "@action:login"` |
| `teams` | List teams | `datadog-cli teams --keyword platform` |
| `llm-obs` | LLM Observability spans (preview) | `datadog-cli llm-obs "@ml_app:bot"` |
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
# - us3.datadoghq.com (US3)
# - us5.datadoghq.com (US5)
# - datadoghq.eu (EU1)
# - ap1.datadoghq.com (AP1)
# - ap2.datadoghq.com (AP2)
# - uk1.datadoghq.com (UK1)
# - ddog-gov.com (US1-FED)
# - us2.ddog-gov.com (US2-FED)
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
