---
name: datadog-query
description: Query Datadog observability data via CLI with natural language time parsing, automatic pagination, and tag filtering (30-70% response reduction). Use for investigating errors, analyzing metrics, checking monitors, searching APM traces, or building Unix pipelines with grep/jq. Supports logs, metrics, monitors, events, hosts, dashboards, spans, services, and RUM.
---

# EXECUTION DIRECTIVE

**When user requests Datadog data: Execute commands using Bash tool, return actual data.**

- Use `datadog <command> --format json` to get structured data
- Parse natural language queries → construct CLI commands → execute → return results
- Don't provide instructions or examples - execute and deliver data
- Use command reference below for syntax

# Datadog Query Expert

## Commands Reference

### Logs (3 commands)
```bash
logs search "<query>" --from "<time>" --to "<time>" --limit <n> [--tag-filter "<prefix>"]
logs aggregate "<query>" --from "<time>" --to "<time>"  # Basic count aggregation
logs timeseries "<query>" --from "<time>" --to "<time>" --interval "<1m|5m|1h>" --aggregation "<count|avg|sum>"
```

### Metrics
```bash
metrics "<query>" --from "<time>" --to "<time>" [--max-points <n>]
```

### Monitoring
```bash
monitors list [--tags "<tags>"] [--monitor-tags "<tags>"]
monitors get <id>
events --from "<time>" --to "<time>" [--priority "<normal|low>"] [--sources "<src>"] [--tags "<tags>"]
```

### Infrastructure
```bash
hosts [--filter "<query>"] [--from "<time>"] [--start <n>] [--count <n>] [--tag-filter "<prefix>"]
```

### APM
```bash
spans "<query>" --from "<time>" --to "<time>" --limit <n> [--cursor "<token>"] [--full-stack-trace]
services [--env "<env>"]
```

### RUM
```bash
rum "<query>" --from "<time>" --to "<time>" --limit <n> [--cursor "<token>"] [--full-stack-trace]
```

### Dashboards
```bash
dashboards list
dashboards get <id>
```

### Configuration
```bash
config show              # Current config (masked secrets)
config path [--global]   # Config file location
config list              # All sources (env, .env, global)
config edit [--global]   # Edit config file
```

## Quick Patterns

**Error Investigation:**
```bash
datadog logs search "status:error service:api" --from "1 hour ago" --tag-filter "env:,service:"
```

**Basic Error Count (aggregate):**
```bash
datadog logs aggregate "status:error" --from "1h ago" --to "now"
# Note: Returns basic count only, no grouping/sorting via CLI
```

**Error Trend Analysis (timeseries):**
```bash
datadog logs timeseries "status:error" \
  --from "24 hours ago" \
  --interval "1h" \
  --aggregation "count"
```

**Performance Metrics:**
```bash
datadog metrics "avg:system.cpu.user{service:web}" --from "4 hours ago" --max-points 100
```

**Trace Investigation (large dataset):**
```bash
datadog spans "service:checkout error:true" \
  --from "1 hour ago" \
  --limit 100 \
  --tag-filter "env:,service:"

# Next page:
datadog spans "..." --cursor "TOKEN_FROM_PREVIOUS_RESPONSE"
```

**Monitor Status:**
```bash
datadog monitors list --tags "env:production"
datadog monitors get 12345
```

**RUM User Errors:**
```bash
datadog rum "@type:error" \
  --from "2 hours ago" \
  --limit 50 \
  --tag-filter "application_id:,session_id:"
```

## Unique Features

### Natural Language Time (CRITICAL)
Powered by interim library:
- **Relative:** "1 hour ago", "30 minutes ago", "2 days ago"
- **Named:** "yesterday", "last week", "last month"
- **ISO8601:** "2025-01-15T10:30:00Z"
- **Unix:** "1704067200"
- **Special:** "now"

### Tag Filtering (30-70% Response Reduction)
```bash
DD_TAG_FILTER="env:,service:" datadog logs search "..."
--tag-filter "env:,service:"      # Specific prefixes (RECOMMENDED)
--tag-filter ""                   # Exclude all tags (maximum reduction)
--tag-filter "*"                  # All tags (default, largest response)
```

### Stack Trace Truncation
- **Default:** First 10 lines + "... (N more lines)"
- **Full:** `--full-stack-trace` flag for complete traces
- **Applies to:** spans, rum

### Pagination Strategies
Auto-selected by command:
- **Logs search:** `--limit <n>` (simple limit-based)
- **Spans/RUM:** `--cursor "token"` (cursor-based, large datasets)
- **Hosts:** `--start <n> --count <n>` (offset-based)

### Logs Timeseries Analysis
- **Intervals:** "1m", "5m", "1h", "1d"
- **Aggregations:** count, avg, sum, min, max
- **Metric field:** `--metric "@duration"` (for avg/sum/min/max)

Example - Response time trend:
```bash
--interval "1h" --aggregation "avg" --metric "@duration"
```

### Output Formats
```bash
--format json       # Pretty JSON (default)
--format jsonl      # One JSON per line (Unix pipelines)
--format table      # Human-readable table
```

Unix pipeline examples:
```bash
datadog logs search "..." --format jsonl | grep "error" | wc -l
datadog monitors list --format jsonl | grep '"Alert"' | jq -r '.name'
```

## Unix Pipeline Patterns

### Count Errors by Service
```bash
datadog logs search "status:error" --format jsonl | \
  jq -r '.service' | sort | uniq -c | sort -rn
```

### High CPU Services
```bash
datadog metrics "avg:system.cpu.user{*} by {service}" \
  --from "4 hours ago" --format json | \
  jq '.series[] | select(.pointlist[-1][1] > 80)'
```

### Failed Traces
```bash
datadog spans "http.status_code:>=500" \
  --from "30 minutes ago" \
  --limit 100 \
  --format jsonl | \
  jq -r '[.resource, .error] | @tsv'
```

### Alert Monitor Summary
```bash
datadog monitors list --format jsonl | \
  grep '"status":"Alert"' | \
  jq -r '.name' | \
  sort
```

### Error Rate Monitoring Script
```bash
#!/bin/bash
ERROR_COUNT=$(datadog logs search "status:error" \
  --from "5 minutes ago" \
  --format json | \
  jq '.meta.page.total')

if [ $ERROR_COUNT -gt 10 ]; then
  echo "⚠️  High error rate: $ERROR_COUNT errors"
  # Send alert
else
  echo "✅ Normal: $ERROR_COUNT errors"
fi
```
