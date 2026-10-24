# Datadog CLI

[한국어](README.md) · [Command reference](.claude/skills/datadog-query/reference.md)

A Rust CLI for querying Datadog logs, metrics, APM, RUM, and operational resources.
Supports relative times such as `1h` or `now-1h`, RFC3339, and JSON, JSONL, and table output. Query commands do not modify Datadog resources.

## Installation

Download Linux x86_64/ARM64, macOS Apple Silicon/Intel, and Windows x86_64 binaries from [Releases](https://github.com/junyeong-ai/datadog-cli/releases).
Install the ZIP manually on Windows. Linux and macOS also support the installer:

```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh | bash
```

Select a release with `DATADOG_CLI_VERSION`. Installation stops on checksum failure.
The accompanying skill comes from the same release tag. Non-interactive installation defaults to installing the binary and user skill. Existing skills are replaced without a backup.

```bash
curl -fsSL https://raw.githubusercontent.com/junyeong-ai/datadog-cli/main/scripts/install.sh -o install.sh
DATADOG_CLI_VERSION=0.9.5 bash install.sh
```

Source builds require Rustup and C build tools. Development uses Rust 1.99.1 from `rust-toolchain.toml`; the minimum supported Rust version is 1.97.

```bash
git clone https://github.com/junyeong-ai/datadog-cli
cd datadog-cli
./scripts/install.sh --source
```

The script installs the binary to `~/.local/bin`. Use `./scripts/uninstall.sh` to remove it. Configuration and skill removal are separate choices; selecting skill removal deletes it without a backup.
To manage only the binary with Cargo, install with `cargo install --path . --locked --target host-tuple` and remove it with `cargo uninstall datadog-cli`. Cargo defaults to `~/.cargo/bin`. Choose one installation method to avoid competing binaries on PATH.

Set `INSTALL_DIR` for the binary destination and `DATADOG_CLI_SKILL_DIR` for the skill destination. Use the same paths when uninstalling.

See the [changelog](CHANGELOG.md) for changes and migration steps when upgrading.

## Getting started

```bash
datadog-cli config init
datadog-cli config edit
datadog-cli logs search "service:api status:error" --from 1h
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

## Query examples

```bash
datadog-cli logs aggregate "service:api status:error" --from 6h
datadog-cli logs timeseries "status:error" --from 24h --interval 1h
datadog-cli timeseries "sum:errors{*}" "sum:hits{*}" --formula "a / b * 100"
datadog-cli scalar "avg:system.cpu.user{*} by {host}" --aggregator avg
datadog-cli spans "service:api error:true" --from 30m
datadog-cli rum "@type:error" --from 1h
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
datadog-cli llm-obs --ml-app chatbot --span-kind llm
```

Time inputs accept `now`, past offsets (`15m`, `1h`, `7d`, `now-1h`, `2 hours ago`; units s, m, h, d, w), RFC3339, or Unix **seconds**. Offsets always point into the past; other forms are CLI usage errors (exit code 2). A start time after the end time fails with exit code 1 before credentials are used.
`metrics` passes query expressions unchanged. Choose aggregation and interval in the query, such as `metrics "avg:system.cpu.user{*}.rollup(avg, 3600)"`, or set the interval for multiple queries in seconds with `timeseries --interval 3600`. Intervals do not guarantee a maximum number of returned points.
LLM Observability uses the Agent Observability preview API. Supply a positional query or structured `--ml-app`/`--span-kind` filters; the two modes are mutually exclusive. Queries are passed unchanged.
Rust `search_llm_obs_spans` accepts `LlmObsSearchParams` and `LlmObsFilter` to distinguish these modes. Product-specific features such as incidents require the relevant product and permissions.

## API discovery and execution

Discover reviewed API operations and their input schemas without credentials:

```bash
datadog-cli capabilities domains
datadog-cli capabilities list --domain ci-visibility-pipelines
datadog-cli capabilities show v2.SearchCIAppPipelineEvents
```

The catalog pins an official contract snapshot. It does not probe your organization's product access or user permissions.
Operation lists default to 25 entries. Use `--search` for literal case-insensitive ID/summary matching and `--offset`/`--limit` for continuation.

Execute a reviewed operation with `query DOMAIN VERSION.OPERATION --input JSON`.
The request envelope uses `path`, `query`, `headers`, and `body` and is schema-validated before credential execution. Objects that declare properties reject undeclared fields, so a misspelled field cannot silently widen a query. Use `--input @request.json` or `--input -` for a file or stdin.

```bash
datadog-cli query containers v2.ListContainers
datadog-cli query metrics v1.GetMetricMetadata --input '{"path":{"metric_name":"system.cpu.user"}}'
```

For `query`, `.data` is the unmodified API response, distinct from the dedicated commands' projections. `.meta` retains source, HTTP status, API metadata, and execution accounting. JSONL emits the API response without CLI metadata, one line per element when the response is an array. A single response does not establish that all matching records were collected.

Defaults bound authentication/API execution to 60 seconds, four API attempts including retries, and 8 MiB of cumulative response bodies. Adjust `--deadline`, `--max-calls`, and `--max-bytes` explicitly. CSV/binary responses require `--output PATH`; existing files are never overwritten. Product access, permissions, and site availability vary by API. See [contract maintenance and policy](api/README.md). APIs returning credentials or complete authentication configurations are excluded. Webhook configuration reads are excluded because URLs can contain Basic authentication credentials. Synthetic investigations use dedicated result and history APIs.

Operations advertising `time_range` in `capabilities show` accept `--from 1h --to now`. Bounds use one reference instant and the operation's reviewed seconds, milliseconds, or RFC3339 fields. Range flags conflict with native input time fields. Other operations retain their native input structure. `.meta.context.time_range` records the normalized requested bounds.

Repeat `--select /data` with exact JSON Pointers to return a pointer-keyed projection. Missing paths are errors; API metadata remains available. `.meta.result.processing` is `complete`, `partial`, or `unknown` from explicitly mapped server states, independently of pagination.

Multiple pages use `--max-pages N` only where the catalog declares a cursor contract. Time-based searches require fixed bounds. They return the original responses in `.data.pages`, with exact continuation metadata and separate processing status. `--wait` polls a DDSQL job within the same deadline, byte and call budgets; submission is never retried.

## MCP specialist tools

Discover official MCP specialist tools offline with `mcp list --toolset profiling`, inspect the current server schema with `mcp show TOOL`, then use `mcp call TOOL --input JSON`. An explicit reviewed policy covers read/analysis tools including DBM, profiling, Kubernetes and data lineage. MCP uses the same identity/site configuration with no automatic REST/MCP substitution. Deadline, HTTP-call and response-byte budgets include handshake, discovery, execution and cleanup. Actual tool exposure and product permissions vary by organization. Vendor analysis remains in the original tool result and should be distinguished from independently verified observations.

## OAuth authentication

Use [Pup 2.0+](https://github.com/DataDog/pup) for OAuth login and refresh. Install Pup on PATH, then create a named session with the read scopes you need. This example covers logs and metrics:

```bash
pup auth login --site datadoghq.com --org cli-read --scopes timeseries_query,logs_read_data
```

Choose the default authentication method in the global `~/.config/datadog-cli/config.toml`. `site` is a top-level field and must match the login site.

```toml
site = "datadoghq.com"

[auth]
method = "oauth"
pup_org = "cli-read"
```

Subsequent commands need no authentication options:

```bash
datadog-cli logs search "service:api"
datadog-cli config show
```

`method` accepts `api-keys`, `token`, or `oauth`. `api-keys` uses the existing `api_key`/`app_key`; `token` uses `token` or `DD_TOKEN`. `oauth` uses the Pup session and ignores static keys/tokens. Environment variables override credential values only within the selected method. Without `[auth]`, existing token/key precedence is preserved.
`pup_org` is a saved Pup session name; select the actual Datadog organization during browser consent. Other commands require their API scopes and user permissions.
Use `--auth-method api-keys|token|oauth` to select a method for one invocation. Explicit CLI credentials also override the global default method. Credential options that conflict with `--auth-method` are errors. Override the session with `--pup-org` and the site with `--site` or `DD_SITE`. The existing `--auth-provider pup` remains equivalent to `--auth-method oauth`.
`[auth]` is allowed only in the global file. A project `.datadog.toml` selecting a different OAuth site is an error unless the site is explicitly overridden with `--site` or `DD_SITE`. Arbitrary credential commands are not configurable.
Each OAuth API command invokes `pup auth token --org=<session>` once. Pup owns storage and refresh; this CLI does not persist tokens. CLI credential processes for the same site are serialized with an OS file lock to prevent concurrent refresh writes. The lock stores no token. Provider execution, including lock wait, is limited to 60 seconds. Failure, empty output, or invalid output returns authentication error 3 without falling back to another key, token, or session.
`config show` displays the method, selection source, session, and masked configuration without invoking Pup. Authentication options apply to API commands and `config show`. Use SATs or service account keys for unattended execution.

## Configuration

Fields merge in ascending priority: **global file → nearest project file → environment → CLI arguments**.
Project configuration is `.datadog.toml`, searched from the current directory through its ancestors.
Global configuration is `~/.config/datadog-cli/config.toml`.
Home resolution honors `HOME` on Linux/macOS and `USERPROFILE` on Windows.

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

## Output and automation

`--format` works before or after API and discovery subcommands. Config commands print text only and fail with exit code 1 when `--format` is given.
JSON is indented on a terminal and compact in pipes. JSONL emits only `data` records, without pagination metadata.
Tables accept an object or an array of objects. Diagnostics go to stderr.
Log custom fields are preserved at `.data[].attributes`; actual trace ID keys depend on instrumentation.
Dashboard lists and details preserve optional API settings and extension fields. Detail authors are available through the original `author_handle` and `author_name` fields and the `author` alias, `created` and `modified` aliases are provided, and template variable `defaults` and `type` and widget `is_column_break` are retained.
Monitor details preserve evaluation settings, recovery thresholds, and extension fields. Hosts retain IDs, aliases, and server-supplied metadata and metrics. Spans retain HTTP and messaging attributes; `--full-stack-trace` affects only error stacks.
Metric `.data[].units` preserves primary/per-unit order, null positions, and scaling. The existing `.unit` summarizes the first non-null unit. Metric-processing errors reported by the API exit with code 4, including in HTTP 200 responses; malformed response structures exit with code 7.
`scalar` preserves the ordered API column array at `.data.columns`; identical column names do not overwrite one another. Extract numbers with `jq '.data.columns[] | select(.type == "number") | .values[]'` and omit the `.data` prefix for JSONL. Table headers use `[index] name (type)` to distinguish columns.
RUM event details are at `.data[].attributes`; incident `--include` resources are at `.meta.included`.
Server query status, warnings, and request IDs are preserved at `.meta.api`. Server timeouts and response warnings also appear on stderr in every output format. Partial results with nonfatal warnings still exit with code 0, so do not interpret a warned aggregate as a complete count.

```bash
datadog-cli logs search "status:error" --format jsonl | jq -r '.service'
datadog-cli logs aggregate "status:error" --group-by service --group-limit 10
datadog-cli monitors list --format table
datadog-cli spans "service:api" --limit 100 --format json > page.json
jq -r '.pagination.next_cursor // empty' page.json
```

`logs aggregate` and `logs timeseries` accept `--group-by` and `--group-limit` to compute the most frequent groups on the server. This is a bounded ranking, not an exhaustive list of groups; inspect API warnings alongside the result.

Search returns one page. Pass the returned cursor with `--cursor` to fetch another.
`pagination.returned` counts records in this response; `total_count` is the server-reported total or `null` when unknown.
`has_next_is_estimate: true` identifies continuation inferred from page size. The existing `total` retains the returned count when no total is known; do not use it for aggregation.
Fix absolute start/end times across pages to query the same window. Consult command help for endpoint-specific pagination.
`--tag-filter "env:,service:"` retains those prefixes; `""` excludes tags and `"*"` retains all tags.

`--error-format json` emits the final runtime error as one JSON line on stderr. `.error.kind` distinguishes `authentication` from `permission_denied`; both retain exit code 3. Other diagnostics such as retry logs may precede it, and CLI usage errors remain text. When stderr is closed, results and the original exit code are preserved even if warnings or the final error cannot be written.

Exit codes: `0` success/closed stdout pipe, `1` input/configuration/other errors, `2` CLI usage,
`3` authentication, `4` API, `5` rate limit, `6` network/timeout, `7` response decoding, `130` Ctrl+C.

## Agents and development

Install the query [skill](.claude/skills/datadog-query/SKILL.md) to `~/.claude/skills/datadog-query` with the installer, or copy the folder into your agent's skill directory. In Claude Code, the skill pre-approves only `datadog-cli` and `jq` commands during the turn that invokes it.
See [AGENTS.md](AGENTS.md) for development rules and [examples.md](.claude/skills/datadog-query/examples.md) for investigation workflows.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo deny check
cargo run --locked --example command_reference > .claude/skills/datadog-query/reference.md
```

CI covers Linux/macOS/Windows tests, MSRV, workflow security audits, and dependency checks.
