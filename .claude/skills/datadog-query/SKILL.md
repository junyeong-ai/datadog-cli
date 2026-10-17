---
name: datadog-query
description: Query Datadog observability data with datadog-cli to investigate logs, metrics, traces, RUM, monitors, incidents, and related operational resources. Use when the user wants Datadog data or a Datadog-backed investigation.
allowed-tools: Bash, Read
---

# Datadog queries

Run `datadog-cli` for requested queries and report the observed results, time window, and scope. Commands query existing resources; dashboard creation and other remote mutations are not supported.

Use `datadog-cli <command> --help` or the relevant section in [reference.md](reference.md) for exact flags and limits. For multi-query investigations and bounded pagination, read [examples.md](examples.md).

## Query decisions

- Select the narrowest useful service, environment, and time window. Time inputs accept relative expressions, RFC3339, or Unix seconds.
- Use `--format json` when pagination or metadata matters. Use `--format jsonl` for record pipelines. Global flags work before or after subcommands; diagnostics go to stderr.
- Search commands return one page. State when results are sampled or incomplete; never interpret a limited sample as a total. Use `logs aggregate` for log counts.
- Cursor searches return `.pagination.next_cursor`; pass it back with `--cursor`. Fix absolute time bounds across pages, bound requests to the investigation, and stop on API errors or a repeated cursor. Offset/page-number commands have different flags; error-tracking search has no pagination.
- Logs expose fields such as `.service` and `.message` at record level. Spans preserve the API envelope: read `.attributes.service`, not `.service`. Inspect a record before assuming optional/custom field paths. Log output does not currently expose trace IDs.
- `--tag-filter` retains prefixes for logs search, spans, RUM, and hosts. Empty input excludes tags; `*` retains them. Filtering tags does not filter the records themselves.

## Authentication and failures

Use configured credentials without printing them. Configuration merges global file → nearest project file → environment → CLI. Environment keys are `DD_API_KEY`, `DD_APP_KEY`, `DD_TOKEN`, `DD_SITE`, and `DD_TAG_FILTER`; `.env` is not loaded automatically. `config show` masks secret values.

A higher source with explicit keys replaces a lower token; a nonempty token wins within a source. Use environment or config credentials rather than placing secrets in command examples.

The client retries transient failures within its request budget. Report exhausted rate limits/timeouts instead of adding an unbounded outer retry loop. Exit codes distinguish authentication (3), API (4), rate limits (5), network/timeouts (6), and decoding (7). Product permissions and preview API availability can restrict results.
