---
name: datadog-query
description: Query Datadog observability data via CLI with natural language time parsing, automatic pagination, and tag filtering (30-70% response reduction). Use for investigating errors, analyzing metrics, checking monitors, searching APM traces, or building Unix pipelines with grep/jq. Supports logs, metrics, monitors, events, hosts, dashboards, spans, services, and RUM.
allowed-tools: Bash, Read
---

# EXECUTION DIRECTIVE

**When user requests Datadog data: Execute commands using Bash tool, return actual data.**

- Use `datadog <command> --format json` to get structured data
- Parse natural language queries → construct CLI commands → execute → return results
- Don't provide instructions or examples - execute and deliver data
- Use command reference below for syntax

---

## Quick Patterns

### Error Investigation
```bash
datadog logs search "status:error service:api" --from "1 hour ago" --tag-filter "env:,service:"
```

### Performance Metrics
```bash
datadog metrics "avg:system.cpu.user{service:web}" --from "4 hours ago" --max-points 100
```

### Monitor Status
```bash
datadog monitors list --tags "env:production"
datadog monitors get 12345
```

### Trace Investigation
```bash
datadog spans "service:checkout error:true" --from "1 hour ago" --limit 100 --tag-filter "env:,service:"
```

### Error Trend Analysis
```bash
datadog logs timeseries "status:error" --from "24 hours ago" --interval "1h" --aggregation "count"
```

### RUM User Errors
```bash
datadog rum "@type:error" --from "2 hours ago" --limit 50 --tag-filter "application_id:,session_id:"
```

---

## Key Features

### Natural Language Time
- "1 hour ago", "30 minutes ago", "yesterday", "last week"
- ISO8601: "2025-01-15T10:30:00Z"
- Unix timestamp: "1704067200"
- Special: "now"

### Tag Filtering (30-70% Response Reduction)
```bash
--tag-filter "env:,service:"      # Specific prefixes (recommended)
--tag-filter ""                   # Exclude all tags (maximum reduction)
```

### Stack Trace Truncation
- Default: First 10 lines
- Full: `--full-stack-trace` flag

### Pagination
- **logs search**: `--limit <n>`
- **spans/rum**: `--cursor "token"`
- **hosts**: `--start <n> --count <n>`

### Output Formats
```bash
--format json       # Pretty JSON (default)
--format jsonl      # One JSON per line (Unix pipelines)
--format table      # Human-readable table
```

---

## Commands Overview

**13 commands available:**
- `logs search/aggregate/timeseries` - Log analytics
- `metrics` - Time series metrics
- `monitors list/get` - Monitor management
- `events` - Event stream
- `hosts` - Infrastructure
- `spans` - APM traces
- `services` - Service catalog
- `rum` - Real User Monitoring
- `dashboards list/get` - Dashboards
- `config show/path/list/edit` - Configuration

**For complete syntax:** See [reference.md](reference.md)

**For Unix pipeline patterns and scripts:** See [examples.md](examples.md)
