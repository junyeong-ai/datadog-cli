# Datadog CLI

Fast and powerful Datadog API query tool - Natural language time parsing support

## ✨ Features

- **⚡ High Performance**: Rust-based, 10x faster than Python SDK
- **🔒 Secure**: rustls-based TLS 1.3 secure communication
- **📊 Multiple Outputs**: JSON, JSONL, Table for Unix pipelines
- **🕐 Natural Time**: Intuitive time like "1 hour ago", "30 minutes ago"
- **⚙️ Flexible Config**: CLI args, env vars, project/global config files

## 🚀 Quick Start

### Installation

**Method 1: Prebuilt Binary (Recommended - Fast)**

```bash
curl -sSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh | bash
```

**Method 2: Cargo Install**

```bash
# Install from crates.io (coming soon)
cargo install datadog-cli

# Or local build
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
cargo install --path .
```

**Method 3: Source Build (For Developers)**

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
cargo build --release
# Binary: target/release/datadog
```

### Configuration

```bash
# 1. Create config file
datadog config init

# 2. Set API keys (choose one)
export DD_API_KEY="your-api-key"
export DD_APP_KEY="your-app-key"

# Or
datadog config edit

# Or
datadog --api-key "key" --app-key "key" metrics "..."
```

### Basic Usage

```bash
# Query metrics (last 1 hour)
datadog metrics "system.cpu.user"

# Search logs
datadog logs search "service:web status:error" --from "1 hour ago"

# List monitors
datadog monitors list
```

## 📖 Commands

| Command | Description | Example |
|---------|-------------|---------|
| `metrics` | Query metrics | `datadog metrics "avg:system.cpu.user{*}"` |
| `logs` | Search/analyze logs | `datadog logs search "query" --limit 100` |
| `monitors` | Manage monitors | `datadog monitors list --tags "env:prod"` |
| `events` | Query events | `datadog events --from "1 day ago"` |
| `hosts` | List hosts | `datadog hosts --filter "env:production"` |
| `dashboards` | Manage dashboards | `datadog dashboards list` |
| `spans` | Search APM spans | `datadog spans "service:api"` |
| `services` | List services | `datadog services --env prod` |
| `rum` | Search RUM events | `datadog rum "query"` |
| `config` | Config management | `datadog config show` |

## ⚙️ Configuration

### Priority

```
1. CLI arguments       --api-key, --app-key (highest)
2. Environment vars    DD_API_KEY, DD_APP_KEY, DD_SITE
3. Project config      ./.datadog.toml
4. Global config       ~/.config/datadog-cli/config.toml
```

### Config File Example

**Global** (`~/.config/datadog-cli/config.toml`):

```toml
api_key = "your-api-key-here"
app_key = "your-app-key-here"
site = "datadoghq.com"  # or datadoghq.eu, ddog-gov.com, etc.
```

**Project** (`.datadog.toml`):

```toml
# Project-specific keys
api_key = "project-key"
app_key = "project-app-key"
site = "datadoghq.eu"
```

## 💡 Tips

### With jq

```bash
# Extract metric points
datadog metrics "system.cpu.user" --format jsonl | jq '.series[].pointlist'

# Extract log messages
datadog logs search "query" --format jsonl | jq -r '.logs[].message'
```

### Time Parsing

```bash
# Natural language
datadog metrics "..." --from "1 hour ago" --to "now"

# ISO8601
datadog metrics "..." --from "2024-01-01T00:00:00Z"

# Unix timestamp
datadog metrics "..." --from "1704067200"
```

## 🛠️ Troubleshooting

### Config Not Found

```bash
datadog config init
datadog config path
datadog config edit
```

### Auth Failed

```bash
datadog config show
DD_API_KEY="new-key" DD_APP_KEY="new-app-key" datadog monitors list
```

## 🔧 Development

```bash
# Build
cargo build --release

# Test
cargo test

# Lint
cargo clippy -- -D warnings && cargo fmt
```

## 📄 License

MIT License

## 🔗 Links

- [Datadog API Docs](https://docs.datadoghq.com/api/)
- [GitHub Repository](https://github.com/junyeong-ai/datadog-cli)
