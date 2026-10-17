# Development guide

Rust 2024 CLI. `rust-toolchain.toml` pins development Rust; `Cargo.toml` declares the independently tested MSRV. Keep `.tool-versions` aligned with the development toolchain.

## Architecture

- `src/cli/mod.rs`: Clap arguments, defaults, and ranges. Handlers consume typed arguments directly.
- `src/config.rs`: global → nearest project → environment → CLI field merging and resolved configuration. A higher source with keys clears an inherited token; a nonempty token wins within a source. Config files reject unknown fields. Never include secret values or raw config snippets in errors.
- `src/datadog/transport.rs`: pooled HTTP client, sensitive authentication headers, request deadlines, error mapping, and retries. Redirects and reqwest's automatic retries are disabled so one policy controls attempts.
- `src/datadog/client.rs`: endpoint paths, request bodies, and shared search parameters.
- `src/datadog/models.rs`: verified response fields. Preserve extensible API fields where callers depend on passthrough data.
- `src/handlers/`: endpoint-specific transformation and pagination. `common.rs` owns shared time, tag, pagination, and output-envelope behavior.
- `src/output.rs`: JSON/JSONL/table writers. Stdout contains results; tracing writes to stderr. Propagate I/O errors so closed pipelines exit quietly.

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
shellcheck scripts/install.sh scripts/uninstall.sh
actionlint
```

Use unit tests for merge/parse policies and Wiremock tests for observable HTTP behavior. Avoid process-wide environment changes in tests; use pure layer inputs or isolated child processes. Check request counts and returned errors rather than timing randomized backoff. Real network tests must not require production credentials.

The release profile aborts on panic: validate boundary inputs and avoid unchecked arithmetic on user or API data. Benchmark performance changes with `cargo bench --bench parsing`; do not make unsupported performance claims.

## Documentation and dependencies

Generate command syntax from Clap, not a second manual option list:

```bash
cargo run --locked --example command_reference > .claude/skills/datadog-query/reference.md
```

Update both READMEs, `.env.example`, and the query skill when observable behavior changes. Keep `CLAUDE.md` as the entry point to this guide instead of duplicating rules. Skills are installed from the binary's release tag and compared by content.

Commit `Cargo.lock`. Keep runtime features scoped to actual use; separate major upgrades from unrelated behavior changes when preparing commits. Verify licenses, advisories, MSRV, and release-target builds. CI actions use verified commit SHAs; the SLSA reusable workflow uses a version tag required by its provenance verification model. CI-installed tools have explicit versions.
