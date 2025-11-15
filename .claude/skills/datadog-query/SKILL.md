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

---

## Commands

logs (search|aggregate|timeseries), metrics, monitors (list|get), events, hosts, dashboards (list|get), spans, services, rum, config

---

## Key Features

- **Natural time**: "1 hour ago", ISO8601, Unix timestamps
- **Tag filtering**: `--tag-filter "env:,service:"` reduces response size
- **Output formats**: json (default), jsonl (pipelines), table
- **Pagination**: logs (--limit), spans/rum (--cursor), hosts (--start/--count)

---

## Reference

- **Complex patterns**: [examples.md](examples.md)
- **Complete syntax**: [reference.md](reference.md)
