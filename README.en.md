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
DATADOG_CLI_VERSION=0.7.0 bash install.sh
```

Source builds require Rustup and C build tools. Development uses Rust 1.99.1 from `rust-toolchain.toml`; the minimum supported Rust version is 1.97.

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
./scripts/install.sh --source
```

The script installs the binary to `~/.local/bin`. Use `./scripts/uninstall.sh` to remove it. Configuration and skill removal are separate choices; skill removal creates a backup.
To manage only the binary with Cargo, install with `cargo install --path . --locked --target host-tuple` and remove it with `cargo uninstall datadog-cli`. Cargo defaults to `~/.cargo/bin`. Choose one installation method to avoid competing binaries on PATH.

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

## OAuth authentication

Use [Pup 2.0+](https://github.com/DataDog/pup) for OAuth login and refresh. Install Pup on PATH, then create a named session with the read scopes you need. This example covers logs and metrics:

```bash
pup auth login --site datadoghq.com --org cli-read --scopes timeseries_query,logs_read_data
datadog-cli --auth-provider pup --pup-org cli-read --site datadoghq.com logs search "service:api"
```

`--pup-org` is a saved Pup session name; select the actual Datadog organization during browser consent. `--site` must match that session's site. Other commands require their API scopes and user permissions.
Each CLI command invokes `pup auth token --org=<session>` once. Pup owns storage and refresh; this CLI does not persist the token. Provider execution is limited to 60 seconds. Failure, empty output, or invalid output returns authentication error 3 without falling back to keys or another token.
Provider selection is CLI-only; config files cannot enable external command execution. It conflicts with `--api-key`, `--app-key`, and `--token`, and does not apply to `config` commands. Ambient static credentials are ignored when the provider is selected.
Personal PATs and service account SATs can be supplied directly through `DD_TOKEN`. Use SATs or service account keys for unattended execution. Manage OAuth consent and sessions through `pup auth`.

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
Retryable responses honor `Retry-After`; 429 also honors `x-ratelimit-reset`. When both are present, the longer wait applies. Waits over 30 seconds return the original error immediately.
Redirects are not followed. HTTP(S) environment proxies and OS proxy settings are supported.

```bash
datadog-cli config show   # Effective configuration with masked secrets
datadog-cli config path   # Nearest project file, otherwise global path
datadog-cli config edit   # Open that path in $EDITOR; defaults to vim
```

`EDITOR` can include arguments, such as `code --wait`. Double-quote executable paths containing spaces. Linux/macOS use POSIX quoting rules; Windows uses Windows command-line quoting rules. Variables, pipelines, and command substitution are not expanded. During editing, the editor handles Ctrl+C while the parent waits for it to exit. An interrupted editor makes the CLI exit with code 130.

`config init` creates the global file. `DD_TAG_FILTER` sets the default tag filter; `RUST_LOG` sets the log level.
See the [command reference](.claude/skills/datadog-query/reference.md) for supported sites and options.

## Upgrading to 0.7

Dashboard list and detail output preserves optional API settings and extension fields. Detail authors are available through the original `author_handle` and `author_name` fields and the existing `author` alias; `created` and `modified` aliases remain available. Template variable `defaults` and `type`, widget `is_column_break`, and unknown settings are retained.
Rust Dashboard models keep fields outside the required core in `extra` maps. `DashboardSummary` is a JSON Map alias; `AuthorInfo`, `TemplateVariable`, and `WidgetLayout` were removed.
Configuration warnings and final error messages no longer abort on closed stderr pipes. The original exit code is preserved even if the final error message cannot be written.

## Upgrading to 0.6

RUM event details are preserved in each record's `attributes`: read `.data[].attributes.error.stack` for error stacks and `.data[].attributes.view` for page details. `--full-stack-trace` applies to that error stack.
Server query metadata, including status, warnings, and request IDs, is preserved at `.meta.api`. Server timeouts and response warnings also appear on stderr in every output format. Partial results with nonfatal warnings retain exit code 0; do not interpret a warned aggregate as a complete count.
Incident list and detail `--include` resources are available at `.meta.included`. Use `--format json` when scripts need metadata.
Rust `RumAttributes` follows the API schema, with event details in its `attributes` map. The separate detail models such as `RumApplication` were removed; `CursorMeta` and `CursorPage` preserve extension fields in an `extra` map.

## Upgrading to 0.5

`scalar` preserves the ordered API column array at `data.columns`, including `type`, optional `name`, `values`, and unit metadata. Identical column names no longer overwrite one another.
JSONL emits one response body containing `columns`. Table headers use `[index] name (type)` to distinguish columns.
Extract numbers with `jq '.data.columns[] | select(.type == "number") | .values[]'`; omit the `.data` prefix for JSONL.
Reported metric-processing errors exit with code 4; malformed response structures exit with code 7.
The Rust client's `query_scalar` and `query_timeseries` now return validated `ScalarAttributes` and `TimeseriesAttributes` directly.

## Upgrading to 0.4

`metrics --max-points` and automatic rollup have been removed. `metrics` passes query expressions unchanged.
Use `metrics "avg:system.cpu.user{*}.rollup(avg, 3600)"` to choose aggregation and interval explicitly.
For multiple queries, use `timeseries "avg:system.cpu.user{*}" "avg:system.cpu.system{*}" --interval 3600`. Intervals are in seconds and do not guarantee a maximum number of returned points.

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
