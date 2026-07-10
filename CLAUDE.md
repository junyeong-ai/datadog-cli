# Datadog CLI - AI Agent Developer Guide

Essential knowledge for implementing features and debugging this Rust CLI tool.

Toolchain: Rust 1.97 (pinned in `rust-toolchain.toml`), edition 2024.

---

## Core Patterns

### Typed CLI Args Flow End-to-End

Clap arg structs (`src/cli/mod.rs`) are passed directly to handlers — there is
no intermediate parameter representation. Defaults, ranges, and enums live in
exactly one place: the clap attributes.

```rust
// src/cli/mod.rs — definition
#[derive(Args)]
pub struct LogsSearchArgs {
    #[arg(long, default_value = "10", value_parser = clap::value_parser!(i32).range(1..=1000))]
    pub limit: i32,
    ...
}

// src/handlers/logs.rs — consumption
pub async fn search(client: &DatadogClient, args: &LogsSearchArgs) -> Result<Value>
```

**Why**: input validation happens once at the clap boundary (invalid values are
rejected before any request is built); handlers trust their inputs.

### 4-Tier Configuration System

**Implementation** (`src/config.rs`): two types.

- `ConfigFile` (private, serde): every field `Option` so merging is
  presence-based — a tier overrides only fields it actually sets.
- `Config` (public, resolved): non-optional fields. If a `Config` exists,
  credentials are present and the site is valid.

**Priority**: CLI args → ENV vars (`DD_API_KEY`, `DD_APP_KEY`, `DD_TOKEN`,
`DD_SITE`, `DD_TAG_FILTER`) → Project config (`.datadog.toml`, found by
walking up directories) → Global config (`~/.config/datadog-cli/config.toml`).

**Auth** is a `Credentials` enum: classic `Keys {api_key, app_key}`
(`DD-API-KEY`/`DD-APPLICATION-KEY` headers) or `Token` — a personal access
token (`ddpat_…`) sent as `Authorization: Bearer`, verified working across
v1 and v2 endpoints. A configured token takes precedence over keys.

A missing config file is fine; a malformed one is a hard error (never
silently ignored). Valid sites are whitelisted in `config::VALID_SITES`
(9 regional sites incl. ap2/uk1/us2-fed).

### Handler Trait System

**Location**: `src/handlers/common.rs`

- `TimeHandler`: `parse_time_range(from, to)` / `parse_time_range_iso8601` —
  natural language, ISO8601, Unix timestamps (via `src/utils.rs::parse_time`)
- `TagFilter`: `resolve_tag_filter(arg, client)` + `filter_tags`/`filter_tags_map`
- `ResponseFilter`: stack-trace truncation, verbose-field stripping
- `ResponseFormatter`: `format_list` / `format_detail` (`{data, pagination?, meta?}`)
- `PaginationInfo`: one output schema, four constructors matching Datadog's
  pagination families — `from_offset` (server reports total), `from_offset_without_total`,
  `from_page_number`, `from_cursor` (exposes `next_cursor` in output)

Each handler is a unit struct implementing only the traits it needs.

### HTTP Client & Retry

**Location**: `src/datadog/client.rs` + `src/datadog/retry.rs`

- `DatadogClient::new(&Config)`; `with_base_url(&Config, url)` exists for
  integration tests against wiremock.
- `SearchParams` bundles the shared v2 search fields (query/from/to/limit/
  cursor/sort); `search_body()` builds the common `filter/page/sort` body.
- Retry policy (`retry::next_delay`): retries ONLY transient failures —
  transport errors, timeouts, HTTP 408/429/5xx. 4xx and decode errors fail
  immediately. 429 waits for the server's `x-ratelimit-reset` when present;
  a reset beyond 30s returns the error instead of blocking. Backoff is
  exponential (2s, 4s, 8s…) capped at 30s.
- Retries emit `tracing::warn!` events; the subscriber writes to **stderr**
  (stdout is reserved for data so pipes stay clean).

### Request Body Shapes (verified against Datadog docs, July 2026)

Not all v2 search endpoints share one shape — don't copy templates across:

| Endpoint | Body shape |
|---|---|
| `POST /api/v2/logs/events/search`, `events/search`, `audit/events/search` | **flat** `{filter, page, sort}` |
| `POST /api/v2/spans/events/search` | JSON:API envelope, `type: "search_request"` |
| `POST /api/v2/query/timeseries` / `scalar` | JSON:API envelope, `type: "timeseries_request"` / `"scalar_request"`, from/to in **epoch ms** |
| `POST /api/v2/llm-obs/v1/spans/events/search` | JSON:API envelope, `type: "spans"`, requires `Content-Type: application/vnd.api+json` (preview API) |
| `POST /api/v2/error-tracking/issues/search` | envelope `data.attributes`, from/to epoch ms, **no pagination** |

### Error Handling

**Location**: `src/error.rs` (`thiserror`)

Variants: `ApiError {status, message}`, `AuthError`, `DateParseError`,
`NetworkError` (transport only — no `#[from]`, mapped explicitly at the send
site), `DecodeError`, `JsonError`, `IoError`, `InvalidInput`,
`RateLimitError {reset_secs}`, `TimeoutError`.

`DatadogError::exit_code()` maps classes to process exit codes
(3 auth, 4 API, 5 rate-limit, 6 network/timeout, 7 decode, 1 other);
`main.rs` uses it. Code 2 is clap's.

---

## Development Tasks

### Add New Command

1. **Define args + enum variant** (`src/cli/mod.rs`)
   ```rust
   #[command(about = "...")]
   NewCommand(NewCommandArgs),

   #[derive(Args)]
   pub struct NewCommandArgs { ... }   // clap owns defaults/ranges/enums
   ```
2. **Add client method** (`src/datadog/client.rs`) — verify the endpoint's
   exact body shape/pagination against docs.datadoghq.com first; model
   responses loosely (`serde_json::Value` passthrough) unless fields are
   verified stable.
3. **Add handler** (`src/handlers/new_command.rs`) mirroring the nearest
   sibling; register in `src/handlers/mod.rs`.
4. **Dispatch** (`src/cli/commands.rs`).
5. **Integration test** (`tests/client_test.rs`) asserting the request shape
   with wiremock `body_partial_json`/`query_param` matchers.

### Modify Config

1. Add the field to `ConfigFile` (as `Option`) AND `Config` (resolved) in
   `src/config.rs`; resolve it in `Config::resolve()`.
2. Update the `init()` template and `show()`.

---

## API Endpoint Map (as of July 2026)

| Command | Endpoint | Pagination |
|---|---|---|
| metrics | `GET /api/v1/query` (canonical, not deprecated) | — |
| timeseries / scalar | `POST /api/v2/query/timeseries` / `scalar` | — |
| logs search | `POST /api/v2/logs/events/search` (`--storage-tier` indexes/online-archives/flex) | cursor |
| logs aggregate/timeseries | `POST /api/v2/logs/analytics/aggregate` | — |
| monitors | `GET /api/v1/monitor` (v1 is canonical; no v2 CRUD exists) | page |
| events | `POST /api/v2/events/search` (v1 events is deprecated) | cursor |
| hosts | `GET /api/v1/hosts` (canonical) | offset+total |
| dashboards | `GET /api/v1/dashboard` (canonical) | offset |
| spans | `POST /api/v2/spans/events/search` | cursor |
| services | `GET /api/v2/catalog/entity` (Software Catalog; successor to services/definitions) | offset |
| rum | `POST /api/v2/rum/events/search` | cursor |
| slo | `GET /api/v1/slo` | offset + `metadata.page.total_filtered_count` |
| incidents | `GET /api/v2/incidents` (requires Incident Management product) | offset, max 100 |
| error-tracking | `POST /api/v2/error-tracking/issues/search` | **none** |
| downtimes | `GET /api/v2/downtime` | offset |
| audit | `POST /api/v2/audit/events/search` | cursor, max 1000 |
| teams | `GET /api/v2/team` | page number, max 100 |
| llm-obs | `POST /api/v2/llm-obs/v1/spans/events/search` (**preview**) | cursor, max 5000 |

Rate limits: logs/spans search are 300 req/hour per org; the client reads
`x-ratelimit-reset` on 429 rather than hardcoding limits.

---

## Testing

- **Unit tests**: colocated `#[cfg(test)]` modules (retry policy, config
  resolution, pagination math, time parsing, tag filtering).
- **Integration tests**: `tests/client_test.rs` — wiremock-backed; cover the
  HTTP layer (auth headers, status→error mapping, retry/no-retry behavior,
  rate-limit reset handling, request body shapes per endpoint). Retry tests
  use `max_retries: 1` so real backoff stays ~2s; nextest runs them in parallel.
- **Benches**: `benches/parsing.rs` (criterion).

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

## CI

`.github/workflows/`: ci.yml (nextest on ubuntu/macos, MSRV 1.97 check,
coverage→Codecov, release build, cargo-audit, cargo-deny with `deny.toml`),
lint.yml (fmt + clippy), release.yml (tag-triggered multi-target build).
Warnings are denied via `CARGO_BUILD_WARNINGS: deny` (cache-friendly, Rust 1.97+).

---

## Performance

- Tokio minimal features; reqwest 0.13 with `rustls` (no OpenSSL), `json`,
  `query` features only.
- Release profile: `opt-level=3, lto=true, codegen-units=1, strip=true, panic="abort"`.
  `panic="abort"` means any panic is a hard abort — validate at boundaries,
  never index/divide on unvalidated input.

This guide contains only implementation-critical knowledge. For user documentation, see [README.md](README.md).
