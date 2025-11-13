# Datadog CLI

[![Rust](https://img.shields.io/badge/rust-1.91.1%2B%20(2024%20edition)-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-117%20passing-green?style=flat-square)](https://github.com/junyeong-ai/datadog-cli)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

> High-performance CLI tool for querying Datadog from the command line

[한국어](README.md) | English

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
mkdir -p ~/.config/datadog-cli
cat > ~/.config/datadog-cli/config << EOF
DD_API_KEY=your_api_key
DD_APP_KEY=your_app_key
DD_SITE=datadoghq.com
EOF
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

### vs Datadog Web UI
| Item | Web UI | Datadog CLI |
|------|--------|-------------|
| Query Speed | Browser loading | ✅ Instant (<1s) |
| Automation | ❌ Impossible | ✅ Scriptable |
| Data Processing | Manual copy | ✅ Unix tools |

### vs Python SDK
| Item | Python SDK | Datadog CLI |
|------|-----------|-------------|
| Installation | pip, dependencies | ✅ Single binary |
| Start Time | 10min+ | ✅ 3 minutes |
| Memory | Python runtime | ✅ Native (low) |

### vs curl
| Item | curl | Datadog CLI |
|------|------|-------------|
| Authentication | Manual headers | ✅ Automatic |
| Error Handling | Manual parsing | ✅ Clear messages |
| Output | Raw JSON | ✅ Format selection |

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
datadog config show                  # Show current config (masked)
datadog config path [--global]       # Show config file path
datadog config list                  # List all config sources
datadog config edit [--global]       # Edit configuration
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

### Example 4: Script Automation
```bash
#!/bin/bash
# Error rate monitoring script

ERROR_COUNT=$(dd logs search "status:error" \
  --from "5 minutes ago" \
  --format json | \
  jq '.pagination.total')

if [ $ERROR_COUNT -gt 10 ]; then
  echo "⚠️  High error rate: $ERROR_COUNT errors"
  # Send Slack notification
  curl -X POST $SLACK_WEBHOOK -d "{\"text\":\"High error rate: $ERROR_COUNT\"}"
else
  echo "✅ Error rate normal: $ERROR_COUNT errors"
fi
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
# Set via environment variable
DD_TAG_FILTER="env:,service:" datadog logs search "status:error"

# Or pass as parameter
datadog logs search "status:error" --tag-filter "env:,service:"

# Strategies
DD_TAG_FILTER="*"                    # All tags (default)
DD_TAG_FILTER=""                     # Exclude all tags
DD_TAG_FILTER="env:,service:"        # Specific prefixes (recommended!)
DD_TAG_FILTER="env:production"       # Specific values only
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

### Priority
Configuration is applied in this order:

1. **Environment Variables** (highest priority)
   ```bash
   DD_API_KEY=xxx DD_APP_KEY=yyy datadog monitors list
   ```

2. **Local .env** (project-specific)
   ```bash
   # .env file
   DD_API_KEY=xxx
   DD_APP_KEY=yyy
   DD_SITE=datadoghq.com
   ```

3. **Global Config** (user default)
   ```bash
   # ~/.config/datadog-cli/config
   DD_API_KEY=xxx
   DD_APP_KEY=yyy
   DD_SITE=datadoghq.com
   ```

### Available Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `DD_API_KEY` | Datadog API key | - | ✅ |
| `DD_APP_KEY` | Datadog Application key | - | ✅ |
| `DD_SITE` | Datadog site | `datadoghq.com` | ❌ |
| `DD_TAG_FILTER` | Tag filter (optimize response size) | `*` (all) | ❌ |
| `LOG_LEVEL` | Log level (error/warn/info/debug) | `warn` | ❌ |

**Examples:**
```bash
# Include all tags (default)
DD_TAG_FILTER="*" datadog logs search "status:error"

# Include specific tags only (recommended)
DD_TAG_FILTER="env:,service:" datadog logs search "status:error"

# Enable debug logging
LOG_LEVEL=debug datadog monitors list
```

### Configuration Commands
```bash
# Show current config (API keys masked)
datadog config show

# Show config file paths
datadog config path              # Local .env
datadog config path --global     # Global config

# List all config sources
datadog config list

# Edit configuration
datadog config edit --global     # Edit global config
```

### Configuration File Locations

**Global Config (recommended):**
```
~/.config/datadog-cli/config
```

**Local Config:**
```
.env (project root)
```

**Template:** See `.env.example`

### Datadog Site Configuration

Use `DD_SITE` environment variable to specify your Datadog site:

| Site | Value | Region |
|------|-------|--------|
| US1 (default) | `datadoghq.com` | United States |
| EU | `datadoghq.eu` | Europe |
| US3 | `us3.datadoghq.com` | United States |
| US5 | `us5.datadoghq.com` | United States |
| US1-FED | `ddog-gov.com` | US Government |

```bash
DD_SITE=datadoghq.eu datadog monitors list
```

### ⚠️ Important: .env File
`.env` is a **shared project file** (also used by Node.js, Docker, etc.).

**Safe Approach:**
- ✅ **Use global config** (`~/.config/datadog-cli/config`) - datadog-cli only
- ⚠️ **Use .env** - For project-specific overrides only
- ❌ **Never delete .env** - May contain other tool settings

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
- ❌ Local .env - Manual removal required

---

## 🛠️ Development

### Build
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release
# Result: target/release/dd (5.1MB)
```

### Testing
```bash
cargo test              # 117 tests
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
| **Tests** | 117 (100% passing) |
| **Dependencies** | 12 (production) |
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
