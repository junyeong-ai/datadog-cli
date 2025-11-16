# Datadog CLI - Complex Query Patterns

Practical multi-step investigation workflows. All examples use correct `--format` placement (global option before command).

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

---

## Complete Error Investigation Workflow

Multi-command investigation for a service:

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

## Performance Baseline Comparison

Compare current performance vs historical baseline:

```bash
SERVICE="web-api"
METRIC="@duration"

echo "=== Performance Comparison for $SERVICE ==="

# Current avg duration (last 15 min)
CURRENT=$(datadog-cli --format json logs timeseries \
  "service:$SERVICE @http.status_code:200" \
  --from "15 minutes ago" --to "now" \
  --interval "15m" --aggregation "avg" --metric "$METRIC" | \
  jq -r '.data.buckets[-1].computes.c0 // "N/A"')

# Baseline avg duration (same time yesterday)
BASELINE=$(datadog-cli --format json logs timeseries \
  "service:$SERVICE @http.status_code:200" \
  --from "25 hours ago" --to "24 hours ago" \
  --interval "15m" --aggregation "avg" --metric "$METRIC" | \
  jq -r '.data.buckets[-1].computes.c0 // "N/A"')

echo "Current:  ${CURRENT}ms"
echo "Baseline: ${BASELINE}ms"

# Calculate percentage change if both values exist
if [[ "$CURRENT" != "N/A" && "$BASELINE" != "N/A" ]]; then
  CHANGE=$(echo "scale=1; (($CURRENT - $BASELINE) / $BASELINE) * 100" | bc)
  echo "Change:   ${CHANGE}%"
fi
```

---

## Advanced Filtering Patterns

### Exclude Known Noise

Filter out known issues to focus on new errors:

```bash
# Errors excluding deprecation warnings and timeouts
datadog-cli --format jsonl logs search \
  "status:error -message:*DeprecationWarning* -message:*timeout*" \
  --from "1 hour ago" --limit 50 | \
  jq -r '[.timestamp, .service, .message] | @tsv'
```

### High-Value User Errors

Filter RUM errors for premium users only:

```bash
# Premium user errors with relevant context
datadog-cli --format jsonl rum "@type:error @user.plan:premium" \
  --from "24 hours ago" --limit 50 \
  --tag-filter "user_id:,session_id:,view_id:" | \
  jq -r '[.user.id, .error.message, .view.url] | @tsv' | \
  column -t -s $'\t'
```

---

## Infrastructure Monitoring

### Host Health Check

List unhealthy hosts with key metrics:

```bash
# Hosts with issues in last hour
datadog-cli --format jsonl hosts \
  --from "1 hour ago" --count 100 \
  --tag-filter "env:,availability-zone:,instance-type:" | \
  jq -r 'select(.is_muted == false) | [.name, .tags_by_source.Datadog[] | select(startswith("env:"))] | @tsv' | \
  column -t -s $'\t'
```

### Active Monitors Summary

Check monitor status across environments:

```bash
# Group monitors by status
echo "=== Monitor Status Summary ==="
datadog-cli --format jsonl monitors list --tags "env:prod" | \
  jq -r '.overall_state' | \
  sort | uniq -c | \
  awk '{printf "%-10s: %s\n", $2, $1}'
```

---

## Data Pipeline Patterns

### Export to CSV

```bash
# Export logs to CSV
datadog-cli --format jsonl logs search "service:api" \
  --from "1 hour ago" --limit 1000 | \
  jq -r '[.timestamp, .service, .status, .message] | @csv' > logs.csv
```

### Real-time Monitoring

```bash
# Monitor error rate every 30 seconds
while true; do
  COUNT=$(datadog-cli --format json logs aggregate "status:error" \
    --from "5 minutes ago" --to "now" | \
    jq -r '.data.buckets[0].computes.c0 // 0')

  echo "$(date '+%H:%M:%S') - Errors in last 5min: $COUNT"
  sleep 30
done
```

### Batch Processing with Pagination

```bash
# Process all spans in batches
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

## Tips for Effective Queries

### Output Format Selection

```bash
# Use json for single queries
datadog-cli --format json logs search "query" --limit 1

# Use jsonl for pipelines (one object per line)
datadog-cli --format jsonl logs search "query" --limit 100 | \
  jq -r '.service' | sort | uniq -c

# Use table for human review
datadog-cli --format table monitors list
```

### Tag Filtering Best Practices

```bash
# Without tag filter (large response)
datadog-cli --format json logs search "service:api" --limit 10
# Response size: ~500KB with 100+ tags per log

# With tag filter (minimal response)
datadog-cli --format json logs search "service:api" --limit 10 \
  --tag-filter "env:,service:,version:"
# Response size: ~50KB with only 3 tag prefixes

# Size reduction: 90%
```

### Time Range Optimization

```bash
# Good: Specific short range
datadog-cli --format json logs search "query" --from "30 minutes ago"

# Better: Narrow range for high-traffic services
datadog-cli --format json logs search "query" \
  --from "5 minutes ago" --to "now"

# Best: Use timeseries for trends instead of raw logs
datadog-cli --format json logs timeseries "query" \
  --from "24 hours ago" --to "now" \
  --interval "1h" --aggregation "count"
```

---

## Debugging Failed Queries

```bash
# Enable verbose mode to see API requests
datadog-cli -v --format json logs search "query" --from "1 hour ago"

# Check config
datadog-cli config show

# Test connectivity
datadog-cli --format json monitors list --limit 1
```
