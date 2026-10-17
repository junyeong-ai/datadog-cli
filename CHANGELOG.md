# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-10-17

### Added

- Explicit Pup OAuth integration through `--auth-provider pup --pup-org <session>`. The selected site and named session are fixed, static credentials are ignored, provider execution is bounded, and failures never fall back to another identity. Pup owns login, storage, and refresh.

### Changed

- Dashboard models retain optional configuration in `extra` maps. `DashboardSummary` is a JSON Map alias; `AuthorInfo`, `TemplateVariable`, and `WidgetLayout` were removed.

### Fixed

- Dashboard output preserves author fields, multi-value template defaults, variable types, column breaks, and extension settings while retaining existing timestamp and author aliases.
- Configuration warnings and final errors use fallible stderr writers. Closed diagnostic pipes preserve valid results and original failure exit codes.

## [0.6.0] - 2026-10-17

### Changed

- RUM records preserve the full event details map at `attributes`, including application, view, session, resource, error, and custom fields. Stack truncation and `--full-stack-trace` apply to `attributes.error.stack`.
- Rust RUM models follow the official nested API schema. The separate detail models were removed; cursor metadata and page models now retain extension fields in `extra` maps.

### Fixed

- Cursor searches and log aggregations retain server metadata at `meta.api`. Server timeouts and response warnings are also written to stderr independently of logging filters, including JSONL and table output. Nonfatal warnings retain successful partial results and exit code 0.
- Incident detail queries preserve requested related resources at `meta.included`, consistently with incident lists.

## [0.5.0] - 2026-10-17

### Changed

- Scalar JSON preserves ordered API columns at `data.columns`, including duplicate or absent names, types, values, unit metadata, and extension fields. JSONL emits one columnar response body; table output projects columns into rows with indexed headers.
- Rust timeseries and scalar client methods return validated typed attributes instead of raw response envelopes.

### Fixed

- Metric processing errors, including HTTP 200 formula errors, return exit code 4 rather than successful empty data. Invalid response shapes and misaligned arrays return decode errors; explicit empty arrays remain valid.
- Foreground editor interrupts are registered before launch and handled outside API cancellation. The parent waits for the editor, preserves editors that continue after Ctrl+C, and returns 130 when the editor exits interrupted.

## [0.4.1] - 2026-10-17

### Fixed

- Configuration editing supports editor arguments and quoted executable paths using platform-specific argument parsing, without invoking a shell. Empty commands and invalid POSIX quoting fail explicitly.
- Public timeseries and scalar query methods validate 1–26 metric queries before making requests. CLI and client share the query limit, and query names no longer use unchecked integer conversion.
- Environment examples correctly describe inherited tag filters when `DD_TAG_FILTER` is unset.

## [0.4.0] - 2026-10-17

### Removed

- `metrics --max-points` and automatic query rewriting. Metric expressions now pass through unchanged. Use explicit `.rollup(aggregation, seconds)` or `timeseries --interval seconds` instead; neither option promises an exact maximum point count.

### Fixed

- Source installation delegates artifact discovery to Cargo and stages the native executable, honoring custom target directories while overriding configured cross-compilation targets.
- Retryable HTTP responses honor `Retry-After`, including 503. Conflicting rate-limit headers use the longer wait; waits beyond 30 seconds return the original error and exit code.
- Installation documentation separates script-managed binaries from Cargo-managed binaries and pairs each method with its matching uninstall command.

## [0.3.0] - 2026-10-17

### Changed

- Development toolchain pinned to Rust 1.99.1; Rust 1.97 remains the minimum supported version.
- Runtime and development dependencies updated against the October 17, 2026 registry; unused test dependencies and benchmark features removed.
- Global and project config fields now merge before environment and CLI overrides. Explicit credentials in a higher source select the authentication method; unknown config fields and zero timeouts fail validation.
- HTTP transport separated from endpoint definitions, with sensitive authentication headers, disabled redirects, one retry policy, full-jitter backoff, `Retry-After` handling, and a configurable total timeout.
- JSONL consistently emits data without envelopes for detail and aggregate results; use JSON to retain metadata. Configuration display fully redacts credentials.
- Tables support detail objects and propagate output errors. Configuration output/edit/path consistently use the nearest project file when present.
- Query skill narrowed to supported read operations; command reference generated from Clap and checked in CI. English/Korean documentation and environment examples aligned with behavior.
- CI actions and installed tools updated and pinned; tests include Windows and release builds include Intel macOS. Duplicate Codecov configuration removed.
- Installer fails on binary download/checksum errors, keeps release-matched skill files, and compares skills by content rather than a duplicated version field.

### Fixed

- Reversed time ranges, integer timestamp overflow, empty tag-filter prefixes matching every tag, and `RUST_LOG` directives being overridden by the default level.
- Broken-pipe errors in table/config output and stale jq/cursor paths in skill examples.
- Overflow from extreme timeseries intervals, existing config format aliases, and latest-alias checksums.
- Binary installs stage replacements before overwriting an existing executable. Source installs honor the repository toolchain and lockfile. Configuration initialization creates files exclusively with private Unix permissions.

## [0.2.0] - 2026-07-10

### Added

- Personal access token authentication (`--token`, `DD_TOKEN`, config `token`) sent as `Authorization: Bearer`; takes precedence over the api/app key pair
- `timeseries` and `scalar` commands: v2 cross-product metric queries with multi-query formulas (`--formula "a / b * 100"`)
- New read commands: `slo list|get`, `incidents list|get`, `error-tracking search|get`, `downtimes`, `audit`, `teams`
- `llm-obs` command: LLM Observability span search (preview API) with `--ml-app` / `--span-kind` filters
- `logs search --storage-tier` (indexes, online-archives, flex)
- Cursor pagination now returns the next cursor at `pagination.next_cursor` (logs, events, spans, rum, audit, llm-obs)
- Differentiated process exit codes: 3 auth, 4 API, 5 rate limit, 6 network/timeout, 7 decode
- Integration test suite (wiremock) covering the HTTP layer, retry policy, and per-endpoint request shapes; criterion benchmarks

### Changed

- `[defaults]` config keys `time_range`, `limit`, and `page_size` removed: they were parsed but never applied to any command (command flags carry the defaults)
- Rust 1.97 (pinned via `rust-toolchain.toml`); reqwest 0.13, toml 1.x, and all dependencies updated
- Events migrated from the deprecated v1 API to `POST /api/v2/events/search` (query-based CLI surface)
- Spans search migrated to the canonical `POST /api/v2/spans/events/search` envelope
- Services now list Software Catalog entities (`GET /api/v2/catalog/entity`) with `--kind/--name/--owner/--include`
- Retry policy redesigned: only transient failures retry (transport errors, 408/429/5xx); 429 honors `x-ratelimit-reset`; capped exponential backoff
- Typed clap argument structs flow end-to-end (validation at the CLI boundary; ranges and enums enforced)
- Config split into on-disk optional schema and resolved validated `Config`; malformed config files now fail loudly
- Site validation uses the complete whitelist of 9 Datadog regional sites (adds ap2, uk1, us2-fed)
- JSON output is compact when piped, pretty on a terminal; table output renders the union of row fields
- `--format` accepted after subcommands (global flag)
- Install script is safe for `curl | bash`: non-interactive defaults, required checksum verification, temp-dir downloads, skill fetched from the release tag

### Fixed

- Multibyte UTF-8 input no longer aborts the process in token masking (`config show`) and span field truncation
- Bare-integer timestamps outside the Unix-seconds range (e.g. millisecond timestamps) are rejected with a clear error instead of querying a garbage time window
- Transport errors (connection failures, timeouts) are now retried; 4xx and decode errors fail immediately instead of retrying
- Dashboards list pagination reported `has_next: false` on every page
- `--max-points 0` and `--count 0` caused divide-by-zero aborts
- Broken pipe on stdout (e.g. piping into `head`) exits quietly instead of reporting an IO error
- Tracing output moved to stderr so piped JSON output stays clean

## [0.1.0] - 2025-12-01

Initial release.
