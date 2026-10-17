# Datadog CLI

[한국어](README.md) · [Command reference](.claude/skills/datadog-query/reference.md)

A Rust CLI for querying Datadog logs, metrics, APM, RUM, and operational resources.
Supports natural time expressions and JSON, JSONL, and table output. Query commands do not modify Datadog resources.

## Installation

Download Linux x86_64/ARM64, macOS Apple Silicon/Intel, and Windows x86_64 binaries from [Releases](https://github.com/junyeong-ai/datadog-cli/releases).
Install the ZIP manually on Windows. Linux and macOS also support the installer:

```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh | bash
```

Select a release with `DATADOG_CLI_VERSION`. Installation stops on checksum failure.
The accompanying skill comes from the same release tag. Non-interactive installation defaults to installing the binary and user skill.

```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh -o install.sh
DATADOG_CLI_VERSION=0.3.0 bash install.sh
```

Source builds require Rustup and C build tools. Development uses Rust 1.99.1 from `rust-toolchain.toml`; the minimum supported Rust version is 1.97.

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
cargo install --path . --locked
# To install the local skill with the installer:
# ./scripts/install.sh --source
```

Use `./scripts/uninstall.sh` to remove the binary. Configuration and skill removal are separate choices; skill removal creates a backup.

## Getting started

```bash
datadog-cli config init
datadog-cli config edit
datadog-cli logs search "service:api status:error" --from "1 hour ago"
datadog-cli metrics "avg:system.cpu.user{*}"
datadog-cli monitors list
```

Configure an API/Application key pair or a token. Export environment variables in your shell; `.env` files are not loaded automatically.

```bash
export DD_API_KEY="your-api-key"
export DD_APP_KEY="your-app-key"
export DD_SITE="datadoghq.com"
# Use a token instead of the key pair:
# export DD_TOKEN="your-token"
```

Set `INSTALL_DIR` for the binary destination and `DATADOG_CLI_SKILL_DIR` for the skill destination. Use the same paths when uninstalling.

## Configuration

Fields merge in ascending priority: **global file → nearest project file → environment → CLI arguments**.
Project configuration is `.datadog.toml`, searched from the current directory through its ancestors.
Global configuration is `~/.config/datadog-cli/config.toml`.

Within a source, a nonempty token takes precedence over keys. Explicit keys in a higher source replace a lower source's token.
Missing key fields inherit lower values. An empty token clears a lower token; an empty key clears that key.
Malformed TOML, unknown configuration fields, invalid output formats, and zero-second timeouts are errors.

```toml
api_key = "your-api-key"
app_key = "your-app-key"
# token = "your-token"
site = "datadoghq.com"

[defaults]
format = "json"
# tag_filter = "env:,service:"

[network]
timeout_secs = 30
total_timeout_secs = 120
max_retries = 3
```

`timeout_secs` limits each request; `total_timeout_secs` limits an API call including retries and waits.
Only transient network failures, 408, 429, and 5xx responses retry. Exponential backoff uses jitter and a 30-second cap.
For 429, `x-ratelimit-reset` or `Retry-After` supplies the wait; waits over 30 seconds return the error immediately.
Redirects are not followed. HTTP(S) environment proxies and OS proxy settings are supported.

```bash
datadog-cli config show   # Effective configuration with masked secrets
datadog-cli config path   # Nearest project file, otherwise global path
datadog-cli config edit   # Open that path in $EDITOR; defaults to vim
```

`config init` creates the global file. `DD_TAG_FILTER` sets the default tag filter; `RUST_LOG` sets the log level.
See the [command reference](.claude/skills/datadog-query/reference.md) for supported sites and options.

## Upgrading to 0.3

Remove obsolete `time_range`, `limit`, and `page_size` keys from `[defaults]`. Previous versions did not apply them to queries; unknown fields now fail validation. Set time ranges and counts with command options.
Detail and aggregate JSONL emit the `data` body only. Use `--format json` in scripts that require metadata.

## Query examples

```bash
datadog-cli logs aggregate "service:api status:error" --from "6 hours ago"
datadog-cli logs timeseries "status:error" --from "24 hours ago" --interval 1h
datadog-cli timeseries "sum:errors{*}" "sum:hits{*}" --formula "a / b * 100"
datadog-cli scalar "avg:system.cpu.user{*} by {host}" --aggregator avg
datadog-cli spans "service:api error:true" --from "30 minutes ago"
datadog-cli rum "@type:error" --from "1 hour ago"
datadog-cli services --kind service --owner platform-team
datadog-cli events "source:alert status:error"
datadog-cli slo list --query checkout
datadog-cli incidents list
datadog-cli error-tracking search "service:api" --track trace
datadog-cli downtimes --current-only
datadog-cli hosts --filter "env:production"
datadog-cli dashboards list
datadog-cli teams --keyword platform
datadog-cli audit "@action:login"
datadog-cli llm-obs "@ml_app:chatbot" --span-kind llm
```

Time inputs accept `now`, `1 hour ago`, `30m ago`, RFC3339, or Unix **seconds**. Start time must not exceed end time.
LLM Observability uses a preview API. Product-specific features such as incidents require the relevant product and permissions.

## Output and automation

`--format` works before or after subcommands.
JSON is indented on a terminal and compact in pipes. JSONL emits only `data` records, without pagination metadata.
Tables accept an object or an array of objects. Diagnostics go to stderr.

```bash
datadog-cli logs search "status:error" --format jsonl | jq -r '.service'
datadog-cli monitors list --format table
datadog-cli spans "service:api" --limit 100 --format json > page.json
jq -r '.pagination.next_cursor // empty' page.json
```

Search returns one page. Pass the returned cursor with `--cursor` to fetch another.
Fix absolute start/end times across pages to query the same window. Consult command help for endpoint-specific pagination.
`--tag-filter "env:,service:"` retains those prefixes; `""` excludes tags and `"*"` retains all tags.

Exit codes: `0` success/closed stdout pipe, `1` input/configuration/other errors, `2` CLI usage,
`3` authentication, `4` API, `5` rate limit, `6` network/timeout, `7` response decoding, `130` Ctrl+C.

## Agents and development

Install the query [skill](.claude/skills/datadog-query/SKILL.md) to `~/.claude/skills/datadog-query` with the installer, or copy the folder into your agent's skill directory.
See [AGENTS.md](AGENTS.md) for development rules and [examples.md](.claude/skills/datadog-query/examples.md) for investigation workflows.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo deny check
cargo run --locked --example command_reference > .claude/skills/datadog-query/reference.md
```

Run benchmarks with `cargo bench --bench parsing`. CI covers Linux/macOS/Windows tests, MSRV, coverage, and dependency checks.
