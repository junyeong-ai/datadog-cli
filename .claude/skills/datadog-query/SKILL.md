---
name: datadog-query
description: Query Datadog observability data via CLI with natural language time parsing, automatic pagination, and tag filtering (30-70% response reduction). Use for investigating errors, analyzing metrics, checking monitors, searching APM traces, or building Unix pipelines with grep/jq. Supports logs, metrics, monitors, events, hosts, dashboards, spans, services, and RUM.
---

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

## Authentication & Configuration

### Environment Variables (AI Agent Best Practice)
```bash
DD_API_KEY=xxx DD_APP_KEY=yyy datadog logs search "..."  # Highest priority
DD_SITE=datadoghq.eu datadog monitors list               # EU region
```

### Configuration Priority
1. **Environment variables** (temporary override, best for AI agents)
2. **Local .env** (project-specific)
3. **Global config** (~/.config/datadog-cli/config)

### Multi-Region Support
DD_SITE values:
- `datadoghq.com` (US1, default)
- `datadoghq.eu` (EU)
- `us3.datadoghq.com` (US3)
- `us5.datadoghq.com` (US5)
- `ddog-gov.com` (US1-FED)

### Global Options (All Commands)
```bash
--api-key <KEY>        # Override DD_API_KEY
--app-key <KEY>        # Override DD_APP_KEY
--site <SITE>          # Override DD_SITE
--format <FORMAT>      # json|jsonl|table
-q, --quiet            # Errors only
-v, --verbose          # Verbose/debug mode
```

### Config Commands
```bash
datadog config show              # Show current config (keys masked)
datadog config path              # Show local .env path
datadog config path --global     # Show global config path
datadog config list              # List all sources with status
datadog config edit --global     # Edit global config with $EDITOR
```

## Troubleshooting

### Authentication Errors
```bash
# Check current config (keys masked)
datadog config show

# List all config sources
datadog config list

# Verify environment variables
echo $DD_API_KEY | cut -c1-8  # First 8 chars
echo $DD_APP_KEY | cut -c1-8

# Test with explicit keys
DD_API_KEY=xxx DD_APP_KEY=yyy datadog monitors list --limit 1
```

### Time Parse Errors
- **Valid:** "1 hour ago", "yesterday", "2025-01-15T10:30:00Z", "1704067200"
- **Invalid:** "1h ago" (use "hour"), "tomorrow" (future not supported)

### Large Response / Timeout
```bash
# Reduce response size with tag filtering (30-70% reduction)
--tag-filter "env:,service:"

# Reduce limit
--limit 10

# For spans/RUM, use pagination
--limit 100 --cursor "..."
```

### Empty Results
```bash
# Test connectivity
datadog logs search "*" --from "1 hour ago" --limit 1

# Verify time range
datadog logs search "..." --from "24 hours ago" --to "now"

# Check query syntax (Datadog query language)
datadog logs search "status:error AND service:api" --limit 1
```

### Configuration Issues
```bash
# Find config files
datadog config path              # Local .env
datadog config path --global     # Global config

# Create global config
mkdir -p ~/.config/datadog-cli
cat > ~/.config/datadog-cli/config << EOF
DD_API_KEY=your_key
DD_APP_KEY=your_app_key
DD_SITE=datadoghq.com
EOF

# Edit existing config
datadog config edit --global
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
