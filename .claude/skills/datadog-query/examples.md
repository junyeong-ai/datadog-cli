# Investigation examples

## Contents

- [Service errors](#service-errors)
- [Bounded cursor pagination](#bounded-cursor-pagination)
- [Metric formulas](#metric-formulas)
- [Monitor evaluation and metric units](#monitor-evaluation-and-metric-units)
- [Agent Observability spans](#agent-observability-spans)
- [Authentication and access](#authentication-and-access)

## Service errors

Use one window for log counts, samples, and spans. Counts and samples answer different questions.

```bash
set -eo pipefail
FROM="2026-10-17T00:00:00Z"
TO="2026-10-17T01:00:00Z"

datadog-cli logs aggregate "service:api status:error" --from "$FROM" --to "$TO"
datadog-cli logs aggregate "status:error" --group-by service --group-limit 10 --from "$FROM" --to "$TO"
datadog-cli logs search "service:api status:error" --from "$FROM" --to "$TO" --limit 20 --format jsonl |
  jq -r '[.timestamp, .service, .message] | @tsv'
datadog-cli spans "service:api error:true" --from "$FROM" --to "$TO" --limit 20 --format jsonl |
  jq -r '[.id, .attributes.service] | @tsv'
```

Inspect `.meta.api.status` and `.meta.api.warnings` before treating a count as complete. Server timeout and warning diagnostics also appear on stderr; exit code 0 alone does not guarantee completeness.

Log custom attributes remain at `.data[].attributes` in JSON or `.attributes` in JSONL; inspect them for the instrumentation's correlation fields before constructing a span query.

The log sample is not the total count. Span results keep API attributes nested; custom fields depend on instrumentation. Inspect those attributes before correlating by trace ID or extracting resource/error fields.

## Bounded cursor pagination

This Bash example reads at most ten pages with a fixed time window. It emits records to stdout, reports incomplete collection on stderr, and fails on API errors or repeated cursors.

```bash
set -eo pipefail
FROM="2026-10-17T00:00:00Z"
TO="2026-10-17T01:00:00Z"
CURSOR=""
SEEN_CURSORS=()
MAX_PAGES=10

for ((PAGE=1; PAGE<=MAX_PAGES; PAGE++)); do
  ARGS=(spans "service:checkout error:true" --from "$FROM" --to "$TO" --limit 100 --format json)
  if [[ -n "$CURSOR" ]]; then
    ARGS+=(--cursor "$CURSOR")
  fi
  RESULT=$(datadog-cli "${ARGS[@]}")
  jq -c '.data[]' <<<"$RESULT"
  CURSOR=$(jq -r '.pagination.next_cursor // empty' <<<"$RESULT")
  if [[ -z "$CURSOR" ]]; then
    break
  fi
  for SEEN in "${SEEN_CURSORS[@]}"; do
    if [[ "$SEEN" == "$CURSOR" ]]; then
      echo "Repeated pagination cursor; collection is incomplete" >&2
      exit 1
    fi
  done
  SEEN_CURSORS+=("$CURSOR")
done

if [[ -n "$CURSOR" ]]; then
  echo "Page budget reached; collection is incomplete" >&2
  exit 1
fi
```

## Metric formulas

```bash
datadog-cli timeseries "sum:trace.http.request.errors{service:api}" \
  "sum:trace.http.request.hits{service:api}" --formula "a / b * 100" --from "1 hour ago"
datadog-cli scalar "avg:system.cpu.user{env:prod} by {host}" --aggregator avg
```

Scalar JSON keeps the ordered API columns at `.data.columns`; use `.columns` for JSONL. Names can repeat, so use column type and position when selecting values. For example: `jq '.data.columns[] | select(.type == "number") | .values[]'`.

Metric names and tags depend on your instrumentation. Check the selected metric's units and aggregation before interpreting a formula as a rate or percentage.

## Monitor evaluation and metric units

After choosing a monitor ID from `monitors list`, inspect its full options with `monitors get <id>`. Evaluation delay, missing-data behavior, grouping delays, and recovery thresholds can explain behavior that the query alone cannot.

```bash
datadog-cli metrics "avg:system.net.bytes_rcvd{*}" --from "5 minutes ago" |
  jq '.data[] | {metric, units}'
```

Interpret both positions of `units` and the scale factor; a primary byte unit with a per-second unit describes a rate. Retain null positions. Do not infer units by matching the metric name.

## Agent Observability spans

Choose one filter mode. Structured filters are useful when the app and span kind are known; a query is passed as written when the investigation needs query syntax.

```bash
datadog-cli llm-obs --ml-app chatbot --span-kind llm --from "5 minutes ago" --limit 10
datadog-cli llm-obs '@ml_app:chatbot AND @meta.span.kind:llm' --from "5 minutes ago" --limit 10
```

The API is a preview. Missing product indexes are an API failure, not evidence that the application has no spans.

## Authentication and access

Use configured credentials. `datadog-cli config show` reports the selected method, source, site, and named Pup session with secrets masked; it never invokes Pup. Do not print tokens, read keychain contents into a transcript, or place secrets in command examples.

### Credential selection

Configuration merges global `~/.config/datadog-cli/config.toml` → nearest project `.datadog.toml` → environment → CLI. Export environment variables; `.env` is not loaded automatically. Supported keys are `DD_API_KEY`, `DD_APP_KEY`, `DD_TOKEN`, `DD_SITE`, and `DD_TAG_FILTER`.

The global `[auth].method` selects `api-keys`, `token`, or `oauth`. An explicit method ignores unrelated credential types, including environment values. Without it, a nonempty token wins within a source, and explicit keys in a higher source replace a lower token. Missing key fields inherit lower values; an empty token clears an inherited token.

OAuth uses global `[auth].pup_org` and Pup 2.0+ on PATH. Pup owns storage and refresh. The CLI serializes provider processes per site with a token-free OS file lock; waiting and execution share a 60-second budget. Direct Pup commands do not participate in that CLI lock. For a user-selected override, use `--auth-method oauth --pup-org <session> --site <site>`. The session name identifies stored credentials; the actual Datadog organization is selected during consent. Project `[auth]` is rejected, and a conflicting project site requires an explicit site override.

### Failures

- `permission_denied` / HTTP 403 means the current identity or scopes lack access. Check product availability, user permissions, and the endpoint's scope. Repeating login alone does not establish access. Downtime reads require `monitors_downtime`, which a generic read-only scope selection may omit.
- `authentication` means credentials or the OAuth provider failed. Confirm the selected site/session. Provider failure returns exit code 3 without switching to another key, token, or session.
- Do not switch identities or start a new consent flow unless requested by the user. For requested login/setup, inspect `pup auth login --help`, select the existing site and an explicit named session, and request the scopes needed for the chosen commands. Review the actual consent permissions.
- Product/index configuration and preview API availability can prevent an otherwise authenticated query. Report the API error rather than turning it into an empty result.

Use `--error-format json` when automation needs the final error's `.error.kind`, `.error.exit_code`, and optional `.error.http_status` or `.error.retry_after_seconds`. Earlier stderr lines may contain other diagnostics. No credential value is needed to explain an access failure.
