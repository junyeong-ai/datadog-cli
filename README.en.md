# Datadog CLI

[![Rust](https://img.shields.io/badge/rust-1.91.1%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-122%20passing-green?style=flat-square)](https://github.com/junyeong-ai/datadog-cli)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

> High-performance CLI tool for querying Datadog from the command line

[한국어](README.md) | [English](README.en.md)

---

## ✨ Key Features

- 🚀 **5.1MB Single Binary** - No dependencies, instant execution
- 📊 **13 Commands** - metrics, logs, monitors, events, hosts, spans, services, rum, dashboards
- 🌍 **Natural Language Time** - "1 hour ago", "yesterday", "last week"
- 🎯 **3 Output Formats** - JSON, JSONL, Table
- 🔧 **Unix Pipeline** - Perfect integration with grep, jq, etc.
- ⚡ **Optimized Performance** - HTTP/2 + rustls, async processing

---

## 🚀 Quick Start (3 Minutes)

### 1. Installation
```bash
./install.sh
```
Binary will be installed to `~/.local/bin/datadog`

### 2. Configuration
```bash
datadog config init
vim ~/.config/datadog-cli/config.toml
```

**config.toml:**
```toml
api_key = "your-api-key-here"
app_key = "your-app-key-here"
site = "datadoghq.com"
```

### 3. Usage
```bash
datadog monitors list
datadog metrics "avg:system.cpu.user{*}" --from "1 hour ago"
datadog logs search "status:error" --limit 10
```

Done! 🎉

---

## 💡 Why Datadog CLI?

| Feature | Web UI | Python SDK | curl | Datadog CLI |
|---------|--------|-----------|------|-------------|
| Query Speed | Browser loading | 10min+ setup | Manual headers | ✅ Instant (<1s) |
| Automation | ❌ Impossible | Available | Available | ✅ Scriptable |
| Installation | - | pip + dependencies | Built-in | ✅ Single binary |
| Data Processing | Manual copy | Python code | Raw JSON | ✅ Unix tools |

---

## 📋 Commands

### Metrics & Infrastructure
```bash
datadog metrics <query>              # Query metrics
datadog hosts [options]              # List hosts
```

### Logs & Analytics
```bash
datadog logs search <query>          # Search logs
datadog logs aggregate [options]     # Aggregate logs (count/sum/avg/min/max)
datadog logs timeseries [options]    # Time series analysis
```

### Monitoring & Events
```bash
datadog monitors list                # List monitors
datadog monitors get <id>            # Get monitor details
datadog events [options]             # Query events
```

### Dashboards
```bash
datadog dashboards list              # List dashboards
datadog dashboards get <id>          # Get dashboard details
```

### APM & Tracing
```bash
datadog spans [options]              # Search APM spans
datadog services [options]           # Service catalog
```

### RUM (Real User Monitoring)
```bash
datadog rum [options]                # User experience monitoring
```

### Configuration
```bash
datadog config init                  # Create config file
datadog config show                  # Show current config (masked)
datadog config path                  # Show config file path
datadog config edit                  # Edit config file
```

**All options:** `datadog --help` or `datadog <command> --help`

---

## 🎯 Usage Examples

### Example 1: Production Error Monitoring
```bash
# Search production errors from last hour
datadog logs search "status:error env:production" \
  --from "1 hour ago" \
  --limit 50 \
  --format table
```

**Output:**
```
┌────────────────────┬─────────────────────┬───────────────────┐
│ timestamp          ┆ service             ┆ message           │
├────────────────────┼─────────────────────┼───────────────────┤
│ 2025-11-13 06:00   ┆ payment-api         ┆ Connection timeout│
│ 2025-11-13 06:02   ┆ auth-service        ┆ Invalid token     │
└────────────────────┴─────────────────────┴───────────────────┘
```

### Example 2: CPU Usage Trend Analysis
```bash
# API server CPU usage for last 24 hours
datadog metrics "avg:system.cpu.user{service:api}" \
  --from "24 hours ago" \
  --to "now" \
  --format json
```

**Output:**
```json
{
  "data": [{
    "metric": "system.cpu.user",
    "points": [
      {"timestamp": "2025-11-12 06:00:00 UTC", "value": 45.2},
      {"timestamp": "2025-11-12 12:00:00 UTC", "value": 62.8}
    ]
  }]
}
```

### Example 3: Unix Pipeline Usage
```bash
# Count monitors in Alert status
datadog --format jsonl monitors list | \
  grep '"status":"Alert"' | \
  jq -s 'length'

# Output: 42
```

**Advanced Example:**
```bash
# Top 5 services by error count
datadog logs aggregate \
  --query "status:error" \
  --from "1 hour ago" \
  --compute '[{"aggregation":"count","type":"total"}]' \
  --group-by '[{"facet":"service"}]' \
  --format json | \
  jq '.data.buckets | sort_by(.count) | reverse | .[0:5]'
```

---

## 🌟 Advanced Features

### Natural Language Time
```bash
# Relative time
datadog logs search "..." --from "10 minutes ago"
datadog logs search "..." --from "2 hours ago"
datadog logs search "..." --from "3 days ago"

# Named times
datadog logs search "..." --from "yesterday"
datadog logs search "..." --from "last week"
datadog logs search "..." --from "last month"

# Absolute time
datadog logs search "..." --from "2025-01-15T10:30:00Z"
datadog logs search "..." --from "1704067200"  # Unix timestamp
```

### Tag Filtering
Tag filtering can significantly reduce response size:

```bash
# Pass as parameter
datadog logs search "status:error" --tag-filter "env:,service:"

# Strategies
--tag-filter "*"                    # All tags (default)
--tag-filter ""                     # Exclude all tags
--tag-filter "env:,service:"        # Specific prefixes (recommended!)
--tag-filter "env:production"       # Specific values only
```

### Output Formats
```bash
# JSON (default) - Raw API response
datadog monitors list --format json

# JSONL (JSON Lines) - Unix-friendly
datadog monitors list --format jsonl | grep "Alert" | jq -s '.'

# Table - Human-readable
datadog monitors list --format table
```

### Unix Pipeline Patterns
```bash
# Pattern 1: Filter + Aggregate
datadog --format jsonl monitors list | \
  grep "production" | \
  jq -s 'length'

# Pattern 2: Data Transformation
datadog monitors list --format json | \
  jq '.data[] | {id, name, status}'

# Pattern 3: Save to File
datadog monitors list > monitors.json
jq '.data | length' monitors.json
jq '.data[] | select(.status=="Alert")' monitors.json
```

---

## ⚙️ Configuration

### TOML Config File

**Location:** `~/.config/datadog-cli/config.toml`

```toml
api_key = "your-api-key"
app_key = "your-app-key"
site = "datadoghq.com"  # or datadoghq.eu, us3.datadoghq.com, etc.
```

**Permissions:** On Unix systems, automatically set to 600 (owner read/write only).

### Configuration Commands
```bash
# Create config file
datadog config init

# Show current config (API keys masked)
datadog config show

# Show config file path
datadog config path

# Edit config file (uses $EDITOR)
datadog config edit
```

### Datadog Site Configuration

Use `site` field to specify your Datadog site:

| Site | Value | Region |
|------|-------|--------|
| US1 (default) | `datadoghq.com` | United States |
| EU | `datadoghq.eu` | Europe |
| US3 | `us3.datadoghq.com` | United States |
| US5 | `us5.datadoghq.com` | United States |
| US1-FED | `ddog-gov.com` | US Government |

**config.toml example:**
```toml
api_key = "your-api-key"
app_key = "your-app-key"
site = "datadoghq.eu"
```

---

## 📦 Installation & Removal

### Installation
```bash
./install.sh
```
Binary will be installed to `~/.local/bin/datadog`

### Removal
```bash
./uninstall.sh
```

**Removal Scope:**
- ✅ Binary (`~/.local/bin/datadog`)
- ✅ Global config (`~/.config/datadog-cli/`) - Optional

---

## 🛠️ Development

### Build
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release
# Result: target/release/datadog (5.1MB)
```

### Testing
```bash
cargo test              # 122 tests
cargo fmt --check       # Format check
cargo clippy           # Linting
```

### Debugging
```bash
RUST_LOG=debug cargo run -- monitors list
```

---

## 📊 Performance

| Metric | Value |
|--------|-------|
| **Binary Size** | 5.1MB |
| **Tests** | 122 (100% passing) |
| **Dependencies** | 13 (production) |
| **Build Optimization** | LTO + strip + opt-level 3 |
| **Avg Response Time** | 0.6-1.2s (API server time) |

---

## 📄 License

MIT License - See [LICENSE](LICENSE) file

---

## 🤝 Contributing

Issues and Pull Requests are welcome!

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines
- `cargo fmt` - Code formatting
- `cargo clippy -- -D warnings` - Linting (0 warnings)
- `cargo test` - All tests must pass
- AI agent development: See [CLAUDE.md](CLAUDE.md)

---

<div align="center">

**Made with 🦀 Rust**

[⭐ Star this repo](https://github.com/junyeong-ai/datadog-cli) · [🐛 Report Bug](https://github.com/junyeong-ai/datadog-cli/issues) · [✨ Request Feature](https://github.com/junyeong-ai/datadog-cli/issues)

</div>
