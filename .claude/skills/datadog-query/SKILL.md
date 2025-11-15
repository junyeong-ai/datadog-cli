---
name: datadog-query
version: 0.1.0
description: Execute Datadog CLI observability queries. Use when investigating production errors, analyzing metrics/performance, checking monitors/alerts, searching APM traces, querying logs, or building dashboards. Activates on - Datadog, observability, monitoring, logs, metrics, APM, RUM, traces, spans, monitors, alerts, performance. Supports natural time ("1 hour ago"), tag filtering, Unix pipelines (jq/grep).
allowed-tools: Bash, Read
---

# Datadog CLI Query Skill

Execute Datadog observability queries automatically via command-line interface.

## Execution Directive

**When user requests Datadog data: Execute commands using Bash tool, return actual data.**

- Use `datadog <command> --format json` for structured output
- Parse natural language queries → construct CLI commands → execute → return results
- Don't provide instructions - execute and deliver data
- Reference [examples.md](examples.md) for complex patterns, [reference.md](reference.md) for complete syntax

---

## Quick Patterns

### Error Investigation
```bash
# Recent errors with context
datadog logs search "status:error service:api" --from "1 hour ago" --tag-filter "env:,service:"

# Error trend analysis
datadog logs timeseries "status:error" --from "24 hours ago" --interval "1h" --aggregation "count"
```

### Performance Monitoring
```bash
# CPU metrics
datadog metrics "avg:system.cpu.user{service:web}" --from "4 hours ago" --max-points 100

# Response time P95
datadog logs timeseries "service:api @http.status_code:200" --from "24h ago" --interval "1h" --aggregation "pc95" --metric "@duration"
```

### APM Trace Analysis
```bash
# Failed traces
datadog spans "service:checkout error:true" --from "1 hour ago" --limit 100 --tag-filter "env:,service:"

# Slow queries
datadog spans "resource:*SELECT* @duration:>1000000000" --from "1 hour ago" --limit 50
```

### Monitor & Alert Status
```bash
# Alert state monitors
datadog monitors list --tags "env:production" --format jsonl | grep '"status":"Alert"'

# Specific monitor details
datadog monitors get 12345
```

### Real User Monitoring
```bash
# User errors
datadog rum "@type:error" --from "2 hours ago" --limit 50 --tag-filter "application_id:,session_id:"
```

---

## Key Capabilities

**Natural Language Time**: `"1 hour ago"`, `"yesterday"`, `"last week"`, ISO8601, Unix timestamps

**Tag Filtering** (30-70% response reduction):
- `--tag-filter "env:,service:"` - Specific prefixes (recommended)
- `--tag-filter ""` - Exclude all tags (maximum reduction)

**Output Formats**:
- `--format json` - Pretty JSON (default)
- `--format jsonl` - One JSON per line (Unix pipelines with jq/grep)
- `--format table` - Human-readable tables

**Pagination**:
- logs: `--limit <n>`
- spans/rum: `--cursor "token"` (from previous response)
- hosts: `--start <n> --count <n>`

**Stack Traces**: Default 10 lines, use `--full-stack-trace` for complete traces

---

## Available Commands

**Log Analytics**: `logs search`, `logs aggregate`, `logs timeseries`
**Metrics**: `metrics`
**Monitoring**: `monitors list`, `monitors get`, `events`
**Infrastructure**: `hosts`
**APM**: `spans`, `services`
**User Experience**: `rum`
**Dashboards**: `dashboards list`, `dashboards get`
**Config**: `config show`, `config edit`, `config list`, `config path`

---

## Documentation

**For production workflows and Unix pipelines**: See [examples.md](examples.md)
- Error monitoring scripts
- Multi-command investigations
- Performance analysis patterns
- Integration examples (Slack, CSV export)

**For complete command syntax and options**: See [reference.md](reference.md)
- Full parameter documentation
- Query syntax details
- Pagination strategies
- Configuration priority
