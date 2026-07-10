# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
