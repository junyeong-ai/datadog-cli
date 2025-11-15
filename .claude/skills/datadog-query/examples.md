# Datadog CLI - Complex Query Patterns

Advanced patterns for multi-step investigations and cross-service analysis.

---

## Cross-Service Error Correlation

Find errors in one service that caused errors in downstream services:

```bash
# Get trace IDs from service A errors
SERVICE_A_ERRORS=$(datadog logs search "status:error service:auth-service" \
  --from "10 minutes ago" \
  --format jsonl | \
  jq -r '.trace_id' | \
  sort -u)

# Find correlated errors in service B
echo "$SERVICE_A_ERRORS" | while read trace_id; do
  datadog spans "trace_id:$trace_id service:user-service error:true" \
    --from "10 minutes ago" \
    --format jsonl | \
    jq -r '[.trace_id, .service, .resource, .error] | @tsv'
done
```

---

## Service Dependency Analysis

Discover upstream and downstream dependencies:

```bash
SERVICE="api-gateway"

# Services called by this service
datadog spans "service:$SERVICE" \
  --from "1 hour ago" \
  --limit 1000 \
  --format jsonl | \
  jq -r '.meta."downstream_services"[]? // empty' | \
  sort -u

# Services calling this service
datadog spans "service:* @downstream_services:$SERVICE" \
  --from "1 hour ago" \
  --limit 1000 \
  --format jsonl | \
  jq -r '.service' | \
  sort -u
```

---

## Multi-Command Investigation

Complete error investigation workflow:

```bash
QUERY="status:error service:api"
TIME_RANGE="1 hour ago"

# Total count
datadog logs aggregate "$QUERY" --from "$TIME_RANGE" --format json | \
  jq '.meta.aggregates[0].value'

# Hourly trend
datadog logs timeseries "$QUERY" \
  --from "6 hours ago" \
  --interval "1h" \
  --aggregation "count" \
  --format json | \
  jq -r '.data.buckets[] | "\(.timestamp | strftime("%H:%M")): \(.computes.c0)"'

# Sample errors
datadog logs search "$QUERY" \
  --from "$TIME_RANGE" \
  --limit 5 \
  --format jsonl | \
  jq -r '[.timestamp, .message] | @tsv'

# Related traces
datadog spans "service:api error:true" \
  --from "$TIME_RANGE" \
  --limit 3 \
  --format jsonl | \
  jq -r '[.resource, .error] | @tsv'
```

---

## Performance Comparison

Compare current performance vs baseline:

```bash
SERVICE="web-api"

# Current (last 15 minutes)
CURRENT=$(datadog logs timeseries "service:$SERVICE @http.status_code:200" \
  --from "15 minutes ago" \
  --interval "15m" \
  --aggregation "avg" \
  --metric "@duration" \
  --format json | \
  jq '.data.buckets[-1].computes.c0')

# Baseline (yesterday same time)
BASELINE=$(datadog logs timeseries "service:$SERVICE @http.status_code:200" \
  --from "25 hours ago" \
  --to "24 hours ago" \
  --interval "15m" \
  --aggregation "avg" \
  --metric "@duration" \
  --format json | \
  jq '.data.buckets[-1].computes.c0')

echo "Current: ${CURRENT}ms, Baseline: ${BASELINE}ms"
```

---

## Complex Filtering

Exclude known issues and focus on new errors:

```bash
# Errors excluding known patterns
datadog logs search "status:error -message:*DeprecationWarning* -message:*timeout*" \
  --from "1 hour ago" \
  --format jsonl

# High-value user errors only
datadog rum "@type:error @user.plan:premium" \
  --from "24 hours ago" \
  --limit 50 \
  --tag-filter "user_id:,session_id:" \
  --format jsonl | \
  jq -r '[.user.id, .error.message, .view.url] | @tsv'
```

---

## Tips

- Use `--format jsonl` for Unix pipelines (jq, grep, awk)
- Apply `--tag-filter` early to reduce response size
- Use `jq -r` for TSV output when piping to other tools
- Combine multiple queries for comprehensive analysis
