# Investigation examples

## Service errors

Use one window for log counts, samples, and spans. Counts and samples answer different questions.

```bash
set -eo pipefail
FROM="2026-10-17T00:00:00Z"
TO="2026-10-17T01:00:00Z"

datadog-cli logs aggregate "service:api status:error" --from "$FROM" --to "$TO"
datadog-cli logs search "service:api status:error" --from "$FROM" --to "$TO" --limit 20 --format jsonl |
  jq -r '[.timestamp, .service, .message] | @tsv'
datadog-cli spans "service:api error:true" --from "$FROM" --to "$TO" --limit 20 --format jsonl |
  jq -r '[.id, .attributes.service] | @tsv'
```

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

Metric names and tags depend on your instrumentation. Check the selected metric's units and aggregation before interpreting a formula as a rate or percentage.
