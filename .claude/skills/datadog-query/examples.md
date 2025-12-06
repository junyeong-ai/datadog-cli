# Datadog CLI - Advanced Workflows

Complex multi-step investigation patterns. For basic usage, see [SKILL.md](SKILL.md).

---

## Cross-Service Error Correlation

Find errors in one service that caused errors in downstream services:

```bash
# Get trace IDs from service A errors
datadog-cli --format jsonl logs search "status:error service:auth-service" \
  --from "10 minutes ago" --limit 100 | \
  jq -r '.trace_id // empty' | \
  sort -u > trace_ids.txt

# Find correlated errors in service B for each trace
while read trace_id; do
  datadog-cli --format jsonl spans "trace_id:$trace_id service:user-service error:true" \
    --from "10 minutes ago" --to "now" --limit 10 | \
    jq -r '[.trace_id, .service, .resource, .error.message // ""] | @tsv'
done < trace_ids.txt
```

---

## Complete Error Investigation Workflow

Multi-step investigation for a service incident:

```bash
QUERY="status:error service:api"
TIME_RANGE="1 hour ago"

echo "=== Error Investigation for service:api ==="

# 1. Total error count
echo -e "\n1. Total Errors:"
datadog-cli --format json logs aggregate "$QUERY" \
  --from "$TIME_RANGE" --to "now" | \
  jq -r '.data.buckets[0].computes.c0 // 0'

# 2. Hourly trend
echo -e "\n2. Error Trend (hourly):"
datadog-cli --format json logs timeseries "$QUERY" \
  --from "6 hours ago" --to "now" \
  --interval "1h" --aggregation "count" | \
  jq -r '.data.buckets[] | "\(.timestamp | strftime("%H:%M")): \(.computes.c0)"'

# 3. Sample error messages
echo -e "\n3. Sample Errors:"
datadog-cli --format jsonl logs search "$QUERY" \
  --from "$TIME_RANGE" --limit 5 | \
  jq -r '[.timestamp, .message] | @tsv'

# 4. Related APM traces
echo -e "\n4. Related Traces:"
datadog-cli --format jsonl spans "service:api error:true" \
  --from "$TIME_RANGE" --to "now" --limit 3 | \
  jq -r '[.resource, .error.message // ""] | @tsv'
```

---

## Batch Processing with Pagination

Process large datasets using cursor-based pagination:

```bash
QUERY="service:checkout error:true"
FROM="1 hour ago"
TO="now"
CURSOR=""
PAGE=1

while true; do
  echo "Processing page $PAGE..."

  if [[ -z "$CURSOR" ]]; then
    RESULT=$(datadog-cli --format json spans "$QUERY" \
      --from "$FROM" --to "$TO" --limit 100)
  else
    RESULT=$(datadog-cli --format json spans "$QUERY" \
      --from "$FROM" --to "$TO" --limit 100 --cursor "$CURSOR")
  fi

  # Process data
  echo "$RESULT" | jq -r '.data[] | [.trace_id, .resource] | @tsv'

  # Get next cursor
  CURSOR=$(echo "$RESULT" | jq -r '.meta.page.after // empty')

  # Exit if no more pages
  [[ -z "$CURSOR" ]] && break

  PAGE=$((PAGE + 1))
done
```

---

## Service Dependency Discovery

Analyze service relationships from span data:

```bash
SERVICE="api-gateway"

# Find downstream services (called by this service)
echo "Services called by $SERVICE:"
datadog-cli --format jsonl spans "service:$SERVICE" \
  --from "1 hour ago" --to "now" --limit 1000 | \
  jq -r '.meta."peer.service" // empty' | \
  sort -u

# Find upstream services (calling this service)
echo "Services calling $SERVICE:"
datadog-cli --format jsonl spans "@peer.service:$SERVICE" \
  --from "1 hour ago" --to "now" --limit 1000 | \
  jq -r '.service' | \
  sort -u
```
