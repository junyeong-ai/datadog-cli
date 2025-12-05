---
name: datadog-query
version: 0.1.0
description: Execute Datadog CLI observability queries. Use when investigating production errors, analyzing metrics/performance, checking monitors/alerts, searching APM traces, querying logs, or building dashboards. Activates on - Datadog, observability, monitoring, logs, metrics, APM, RUM, traces, spans, monitors, alerts, performance. Supports natural time ("1 hour ago"), tag filtering, Unix pipelines (jq/grep).
allowed-tools: Bash, Read
---

# Datadog CLI Query Skill

Execute Datadog observability queries via `datadog-cli` command-line tool.

## Execution Directive

**When user requests Datadog data: Execute commands using Bash tool, return actual data.**

- Use `datadog-cli --format json <command>` for structured output
- Parse natural language → construct CLI commands → execute → return results
- Don't provide instructions - execute and deliver data

---

## Critical Usage Pattern

**`--format` is a GLOBAL option - must be placed BEFORE the command:**

```bash
# ✅ CORRECT
datadog-cli --format json logs search "query" --from "1 hour ago"
datadog-cli --format jsonl spans "service:api" --from "10 minutes ago" --to "now"

# ❌ WRONG (will fail with "unexpected argument" error)
datadog-cli logs search "query" --from "1 hour ago" --format json
```

---

## Available Commands

**Logs**: `search`, `aggregate`, `timeseries`
**Monitoring**: `monitors list|get`, `events`
**Infrastructure**: `hosts`, `dashboards list|get`
**APM**: `spans`, `services`
**RUM**: `rum`
**Metrics**: `metrics`
**Config**: `config init|show|path|edit`

---

## Key Features

### Natural Time Parsing

**Relative intervals:**
- "1 hour ago", "30 minutes ago", "2 days ago", "3 weeks ago"
- Short forms: "3h ago", "2d ago"

**Absolute:**
- ISO8601: "2025-01-15T10:30:00Z"
- Unix timestamp: "1704067200"
- Special: "now"

### Tag Filtering

Reduce response size by filtering tag prefixes:

```bash
--tag-filter "env:,service:"  # Include only these tag prefixes
```

Applies to: `logs search`, `spans`, `rum`, `hosts`

### Output Formats

```bash
datadog-cli --format json <command>   # Pretty JSON (default)
datadog-cli --format jsonl <command>  # JSON Lines (for jq/grep)
datadog-cli --format table <command>  # Human-readable tables
```

### Pagination

- **logs search**: `--limit <n>` + `--cursor "<token>"` + `--sort "<field>"` (default: limit=10)
- **spans, rum**: `--limit <n>` + `--cursor "<token>"` + `--sort "<field>"` (default: limit=10)
- **monitors list**: `--page <n>` + `--page-size <n>` (default: page=0, page_size=100)
- **dashboards list**: `--start <n>` + `--count <n>` (default: start=0, count=100)
- **hosts**: `--start <n>` + `--count <n>` (default: start=0, count=100)

---

## Quick Examples

```bash
# Error logs in last hour
datadog-cli --format json logs search "status:error" --from "1 hour ago" --limit 20

# APM spans for a service
datadog-cli --format jsonl spans "service:api" --from "10 minutes ago" --to "now" --limit 10

# Metrics query
datadog-cli --format json metrics "avg:system.cpu.user{*}" --from "1 hour ago" --to "now"

# List monitors
datadog-cli --format json monitors list

# RUM errors with tag filtering
datadog-cli --format jsonl rum "@type:error" --from "1 hour ago" --to "now" --tag-filter "user_id:,session_id:"
```

---

## Reference Documentation

- **Complete command syntax**: [reference.md](reference.md)
- **Complex multi-step patterns**: [examples.md](examples.md)

---

## Configuration

Commands use credentials from (priority order):

1. CLI args: `--api-key`, `--app-key`, `--site`
2. Environment: `DD_API_KEY`, `DD_APP_KEY`, `DD_SITE`, `DD_TAG_FILTER`
3. Project config: `.datadog.toml` (walks up directory tree)
4. Global config: `~/.config/datadog-cli/config.toml`

**Config sections:**
- `[defaults]`: format, time_range, limit, page_size, tag_filter
- `[network]`: timeout_secs, max_retries

View config: `datadog-cli config show`
Edit config: `datadog-cli config edit`
