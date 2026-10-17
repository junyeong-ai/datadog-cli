# Development guide

Rust 2024 CLI. `rust-toolchain.toml` pins development Rust; `Cargo.toml` declares the independently tested MSRV. Keep `.tool-versions` aligned with the development toolchain.

## Architecture

- `src/cli/mod.rs`: Clap arguments, defaults, and ranges. Handlers consume typed arguments directly.
- `src/auth.rs`: fixed Pup token provider, bounded asynchronous process lifetime and output. Select OAuth through explicit CLI options or global `[auth]`; never allow project config to enable credential processes. Pin the selected site/session and never fall back to another identity on provider failure.
- `src/config.rs`: global → nearest project → environment → CLI field merging and resolved configuration. A higher source with keys clears an inherited token; a nonempty token wins within a source. Explicit auth methods filter unrelated credentials before merging. `[auth]` is global-only; project site changes cannot silently select another OAuth identity. Keep configuration planning separate from credential execution so `config show` never invokes Pup. Config files reject unknown fields. Never include secret values or raw config snippets in errors.
- `src/datadog/transport.rs`: pooled HTTP client, sensitive authentication headers, request deadlines, error mapping, and retries. Redirects and reqwest's automatic retries are disabled so one policy controls attempts.
- `src/datadog/client.rs`: endpoint paths, request bodies, and shared search parameters.
- `src/datadog/models.rs`: verified response fields and metric array alignment. Metric client methods reject processing errors before handlers transform results. Preserve extensible API fields where callers depend on passthrough data.
- `src/handlers/`: endpoint-specific transformation and pagination. `common.rs` owns shared time, tag, pagination, and output-envelope behavior.
- `src/diagnostics.rs`: shared fallible stderr writes. Ignore diagnostic BrokenPipe, propagate other warning I/O errors, and preserve the original exit code when a final error cannot be printed.
- `src/output.rs`: JSON/JSONL/table writers. Stdout contains results; tracing writes to stderr. Propagate I/O errors so closed pipelines exit quietly.

Preserve query API metadata under `meta.api`; server timeout and warning diagnostics must remain visible on stderr in every format, independently of logging filters. Keep pagination separate from query completeness. RUM event details live in the record's `attributes` map; only the explicit stack option changes `attributes.error.stack`.

Keep scalar column arrays ordered; column names are optional attributes, not unique identifiers. Scalar table projection is explicit in `src/output.rs`.

Pass metric query expressions unchanged. Use the structured timeseries interval for server-side rollup; do not infer query syntax or aggregation from string patterns.

Time ranges use one reference instant and Unix seconds internally. Convert to milliseconds only at endpoints that require them. Do not unify distinct Datadog request envelopes or pagination families merely because command names resemble one another.

## API changes

Verify endpoint contracts against https://docs.datadoghq.com/api/latest/ before changing bodies, authentication, limits, or pagination. Read-only search POSTs may retry; newly introduced mutations need an explicit idempotency policy. LLM Observability uses a preview endpoint.

Keep existing flags, exit codes, and JSON field paths compatible unless the task requires a documented change. Search commands return one page; JSONL does not include pagination metadata. Confirm output structure before writing jq examples.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo test --doc --locked
cargo deny check
shellcheck scripts/install.sh scripts/uninstall.sh tests/install_test.sh
bash tests/install_test.sh
actionlint
```

Use unit tests for merge/parse policies and Wiremock tests for observable HTTP behavior. Avoid process-wide environment changes in tests; use pure layer inputs or isolated child processes. Check request counts and returned errors rather than timing randomized backoff. Real network tests must not require production credentials.

The release profile aborts on panic: validate boundary inputs and avoid unchecked arithmetic on user or API data. Benchmark performance changes with `cargo bench --bench parsing`; do not make unsupported performance claims.

## Documentation and dependencies

Generate command syntax from Clap, not a second manual option list:

```bash
cargo run --locked --example command_reference > .claude/skills/datadog-query/reference.md
```

Update both READMEs, `.env.example`, and the query skill when observable behavior changes. Keep `CLAUDE.md` as the entry point to this guide instead of duplicating rules. Skills are installed from the binary's release tag. Replace the complete skill directory without backups and stage it outside discovery directories so copies cannot be discovered as additional skills.

Commit `Cargo.lock`. Keep runtime features scoped to actual use; separate major upgrades from unrelated behavior changes when preparing commits. Verify licenses, advisories, MSRV, and release-target builds. CI actions use verified commit SHAs; the SLSA reusable workflow uses a version tag required by its provenance verification model. CI-installed tools have explicit versions.
