# Datadog CLI Examples and Patterns

Real-world examples for production monitoring, debugging, and automation.

## Unix Pipeline Patterns

Leverage `--format jsonl` for powerful Unix-style data processing:

### Count Errors by Service

```bash
datadog logs search "status:error" --from "1 hour ago" --format jsonl | \
  jq -r '.service' | \
  sort | \
  uniq -c | \
  sort -rn
```

**Output:**
```
  23 web-api
  15 auth-service
   8 payment-gateway
   4 notification-service
```

---

### High CPU Services

Find services with CPU usage above 80%:

```bash
datadog metrics "avg:system.cpu.user{*} by {service}" \
  --from "4 hours ago" \
  --format json | \
  jq '.series[] | select(.pointlist[-1][1] > 80) | {service: .tag_set[0], cpu: .pointlist[-1][1]}'
```

**Output:**
```json
{"service":"web-api","cpu":85.3}
{"service":"batch-processor","cpu":92.1}
```

---

### Failed Traces with Details

Extract resource and error from failed traces:

```bash
datadog spans "http.status_code:>=500" \
  --from "30 minutes ago" \
  --limit 100 \
  --format jsonl | \
  jq -r '[.resource, .error, .service] | @tsv'
```

**Output:**
```
POST /api/users	ConnectionTimeout	api-gateway
GET /checkout	DatabaseError	web-app
POST /payments	ValidationError	payment-service
```

---

### Alert Monitor Summary

List all monitors currently in Alert state:

```bash
datadog monitors list --format jsonl | \
  grep '"status":"Alert"' | \
  jq -r '.name' | \
  sort
```

**Output:**
```
API Response Time High
Database Connection Pool Exhausted
Error Rate Spike - Production
Memory Usage Critical
```

---

### Monitor Status Dashboard

Count monitors by status:

```bash
datadog monitors list --format jsonl | \
  jq -r '.overall_state' | \
  sort | \
  uniq -c
```

**Output:**
```
  45 OK
   3 Alert
   2 Warn
   1 No Data
```

---

## Monitoring Scripts

### Error Rate Alert Script

Monitor error rate and send alerts:

```bash
#!/bin/bash

ERROR_COUNT=$(datadog logs search "status:error" \
  --from "5 minutes ago" \
  --format json | \
  jq '.meta.page.total')

THRESHOLD=10

if [ "$ERROR_COUNT" -gt "$THRESHOLD" ]; then
  echo "⚠️  High error rate: $ERROR_COUNT errors in last 5 minutes"
  # Send alert (Slack, PagerDuty, etc.)
  # curl -X POST "$SLACK_WEBHOOK" -d "{\"text\":\"High error rate: $ERROR_COUNT\"}"
else
  echo "✅ Normal: $ERROR_COUNT errors"
fi
```

---

### Service Health Check

Verify service is responding and not throwing errors:

```bash
#!/bin/bash

SERVICE_NAME="api-gateway"

# Check for recent errors
ERROR_COUNT=$(datadog logs search "service:$SERVICE_NAME status:error" \
  --from "10 minutes ago" \
  --format json | \
  jq '.meta.page.total')

# Check for recent traces
TRACE_COUNT=$(datadog spans "service:$SERVICE_NAME" \
  --from "10 minutes ago" \
  --limit 1 \
  --format json | \
  jq '.meta.page.total')

if [ "$ERROR_COUNT" -gt 5 ]; then
  echo "❌ $SERVICE_NAME: High error rate ($ERROR_COUNT errors)"
  exit 1
elif [ "$TRACE_COUNT" -eq 0 ]; then
  echo "⚠️  $SERVICE_NAME: No traces (possible downtime)"
  exit 1
else
  echo "✅ $SERVICE_NAME: Healthy"
  exit 0
fi
```

---

### Daily Error Report

Generate daily error report by service:

```bash
#!/bin/bash

echo "Error Report - $(date)"
echo "================================"

datadog logs search "status:error" \
  --from "24 hours ago" \
  --format jsonl | \
  jq -r '.service' | \
  sort | \
  uniq -c | \
  sort -rn | \
  head -10 | \
  while read count service; do
    echo "[$count errors] $service"
  done

echo ""
echo "Total errors:"
datadog logs aggregate "status:error" \
  --from "24 hours ago" \
  --format json | \
  jq '.meta.aggregates[0].value'
```

---

### Performance Degradation Detector

Compare current performance vs baseline:

```bash
#!/bin/bash

SERVICE="web-api"

# Current average response time (last 15 minutes)
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

# Alert if 50% slower
THRESHOLD=$(echo "$BASELINE * 1.5" | bc)

if (( $(echo "$CURRENT > $THRESHOLD" | bc -l) )); then
  echo "⚠️  Performance degradation detected!"
  echo "Current: ${CURRENT}ms, Baseline: ${BASELINE}ms"
else
  echo "✅ Performance normal: ${CURRENT}ms"
fi
```

---

## Multi-Command Workflows

### Complete Error Investigation

```bash
#!/bin/bash

QUERY="status:error service:api"
TIME_RANGE="1 hour ago"

echo "=== Error Investigation ==="
echo ""

echo "1. Total error count:"
datadog logs aggregate "$QUERY" --from "$TIME_RANGE" --format json | \
  jq '.meta.aggregates[0].value'

echo ""
echo "2. Error trend (last 6 hours, hourly):"
datadog logs timeseries "$QUERY" \
  --from "6 hours ago" \
  --interval "1h" \
  --aggregation "count" \
  --format json | \
  jq -r '.data.buckets[] | "\(.timestamp | strftime("%H:%M")): \(.computes.c0)"'

echo ""
echo "3. Recent error samples:"
datadog logs search "$QUERY" \
  --from "$TIME_RANGE" \
  --limit 5 \
  --format jsonl | \
  jq -r '[.timestamp, .message] | @tsv'

echo ""
echo "4. Related failed traces:"
datadog spans "service:api error:true" \
  --from "$TIME_RANGE" \
  --limit 3 \
  --format jsonl | \
  jq -r '[.resource, .error] | @tsv'
```

---

### Service Dependency Analysis

```bash
#!/bin/bash

SERVICE="api-gateway"

echo "=== $SERVICE Dependencies ==="
echo ""

echo "Services called by $SERVICE:"
datadog spans "service:$SERVICE" \
  --from "1 hour ago" \
  --limit 1000 \
  --format jsonl | \
  jq -r '.meta."downstream_services"[]? // empty' | \
  sort -u

echo ""
echo "Services calling $SERVICE:"
datadog spans "service:* @downstream_services:$SERVICE" \
  --from "1 hour ago" \
  --limit 1000 \
  --format jsonl | \
  jq -r '.service' | \
  sort -u
```

---

### Dashboard Data Export

Export dashboard widgets to CSV:

```bash
#!/bin/bash

DASHBOARD_ID="abc-123-xyz"

datadog dashboards get "$DASHBOARD_ID" --format json | \
  jq -r '.widgets[] | [.definition.title, .definition.type, .id] | @csv' > dashboard_widgets.csv

echo "Exported to dashboard_widgets.csv"
```

---

## Advanced Filtering

### Error Logs Excluding Known Issues

```bash
# Exclude specific known error patterns
datadog logs search "status:error -message:*DeprecationWarning* -message:*timeout*" \
  --from "1 hour ago" \
  --format jsonl
```

---

### High-Value User Errors (RUM)

```bash
# Find errors for premium users only
datadog rum "@type:error @user.plan:premium" \
  --from "24 hours ago" \
  --limit 50 \
  --tag-filter "user_id:,session_id:" \
  --format jsonl | \
  jq -r '[.user.id, .error.message, .view.url] | @tsv'
```

---

### Cross-Service Error Correlation

```bash
# Find errors in service A followed by errors in service B
SERVICE_A_ERRORS=$(datadog logs search "status:error service:auth-service" \
  --from "10 minutes ago" \
  --format jsonl | \
  jq -r '.trace_id' | \
  sort -u)

echo "$SERVICE_A_ERRORS" | while read trace_id; do
  datadog spans "trace_id:$trace_id service:user-service error:true" \
    --from "10 minutes ago" \
    --format jsonl | \
    jq -r '[.trace_id, .service, .resource, .error] | @tsv'
done
```

---

## Performance Monitoring

### 95th Percentile Response Time

```bash
# Track P95 response time over time
datadog logs timeseries "service:api @http.status_code:200" \
  --from "24 hours ago" \
  --interval "1h" \
  --aggregation "pc95" \
  --metric "@duration" \
  --format json | \
  jq -r '.data.buckets[] | "\(.timestamp | strftime("%Y-%m-%d %H:%M")): \(.computes.c0)ms"'
```

---

### Slow Queries Detection

```bash
# Find database queries taking more than 1 second
datadog spans "resource:*SELECT* @duration:>1000000000" \
  --from "1 hour ago" \
  --limit 50 \
  --format jsonl | \
  jq -r '[(.duration / 1000000000), .resource] | @tsv' | \
  sort -rn
```

---

## Integration Examples

### Slack Alert Integration

```bash
#!/bin/bash

SLACK_WEBHOOK="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"

CRITICAL_MONITORS=$(datadog monitors list --format jsonl | \
  grep '"status":"Alert"' | \
  jq -r '.name')

if [ -n "$CRITICAL_MONITORS" ]; then
  MESSAGE="🚨 Critical Monitors:\n$CRITICAL_MONITORS"
  curl -X POST "$SLACK_WEBHOOK" \
    -H 'Content-Type: application/json' \
    -d "{\"text\":\"$MESSAGE\"}"
fi
```

---

### JSON to CSV Conversion

```bash
# Convert logs to CSV for spreadsheet analysis
datadog logs search "status:error" \
  --from "24 hours ago" \
  --limit 1000 \
  --format jsonl | \
  jq -r '[.timestamp, .service, .message, .host] | @csv' > errors.csv
```

---

## Tips for Production Use

1. **Always use --format jsonl for pipelines** - Easier to process line-by-line
2. **Apply --tag-filter early** - Reduce response size by 30-70%
3. **Use --limit judiciously** - Start small, paginate if needed
4. **Leverage jq for complex filtering** - More flexible than grep
5. **Cache monitor/dashboard lists** - They change infrequently
6. **Use environment variables** - DD_API_KEY, DD_APP_KEY for security
7. **Error handling** - Always check exit codes in production scripts
8. **Rate limiting** - Add delays between bulk queries to avoid throttling
