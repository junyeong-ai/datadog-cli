# Datadog CLI Command Reference

Complete syntax for all commands.

---

## Logs

### logs search
```bash
datadog logs search "<query>" --from "<time>" --to "<time>" [options]
```
**Options:** `--limit <n>`, `--tag-filter "<prefixes>"`, `--format <format>`

**Query syntax:** `status:error`, `service:api`, `status:error AND service:api`, `*`

---

### logs aggregate
```bash
datadog logs aggregate "<query>" --from "<time>" --to "<time>"
```
**Note:** Basic count only, no grouping/sorting

---

### logs timeseries
```bash
datadog logs timeseries "<query>" --from "<time>" --to "<time>" [options]
```
**Options:**
- `--interval "<duration>"` - Time bucket: "1m", "5m", "1h", "1d"
- `--aggregation "<type>"` - count, avg, sum, min, max, pc95
- `--metric "<field>"` - Required for avg/sum/min/max/pc95

---

## Metrics

```bash
datadog metrics "<query>" --from "<time>" --to "<time>" [options]
```

**Query format:** `<aggregator>:<metric>{<scope>} by {<tag>}`

**Examples:**
- `avg:system.cpu.user{*}` - Average across all
- `avg:system.cpu.user{service:web}` - Filtered
- `avg:system.cpu.user{*} by {service}` - Grouped

**Options:** `--max-points <n>` - Downsample for large ranges

---

## Monitoring

### monitors list
```bash
datadog monitors list [--tags "<tags>"] [--monitor-tags "<tags>"]
```

### monitors get
```bash
datadog monitors get <id>
```

### events
```bash
datadog events --from "<time>" --to "<time>" [options]
```
**Options:** `--priority "<level>"`, `--sources "<source>"`, `--tags "<tags>"`

---

## Infrastructure

### hosts
```bash
datadog hosts [options]
```
**Options:** `--filter "<query>"`, `--from "<time>"`, `--start <n>`, `--count <n>`, `--tag-filter "<prefixes>"`

---

## APM

### spans
```bash
datadog spans "<query>" --from "<time>" --to "<time>" [options]
```

**Query examples:**
- `service:checkout` - By service
- `error:true` - Errors only
- `http.status_code:>=500` - Server errors
- `service:api AND error:true` - Combined

**Options:** `--limit <n>`, `--cursor "<token>"`, `--full-stack-trace`, `--tag-filter "<prefixes>"`

---

### services
```bash
datadog services [--env "<environment>"]
```

---

## RUM

```bash
datadog rum "<query>" --from "<time>" --to "<time>" [options]
```

**Query examples:**
- `@type:error` - User errors
- `@type:session AND @session.type:user` - Sessions
- `@view.url_path:/checkout` - Page views

**Options:** `--limit <n>`, `--cursor "<token>"`, `--full-stack-trace`, `--tag-filter "<prefixes>"`

---

## Dashboards

### dashboards list
```bash
datadog dashboards list
```

### dashboards get
```bash
datadog dashboards get <id>
```

---

## Configuration

### config show
```bash
datadog config show
```

### config path
```bash
datadog config path [--global]
```

### config edit
```bash
datadog config edit [--global]
```
Opens in `$EDITOR` (default: vim)

---

## Time Formats

- Natural: "1 hour ago", "30 minutes ago", "yesterday"
- ISO8601: "2025-01-15T10:30:00Z"
- Unix: "1704067200"
- Special: "now"

---

## Tag Filtering

```bash
--tag-filter "env:,service:"  # Include specific prefixes
--tag-filter ""               # Exclude all tags
--tag-filter "*"              # Include all (default)
```

Applies to: logs search, spans, rum, hosts

Environment variable: `DD_TAG_FILTER="env:,service:"`

---

## Pagination

- **logs search**: `--limit <n>` (simple limit)
- **spans, rum**: `--cursor "<token>"` (from response `meta.page.after`)
- **hosts**: `--start <n> --count <n>` (offset-based)

---

## Output Formats

```bash
--format json   # Pretty JSON (default)
--format jsonl  # JSON Lines (for Unix pipelines)
--format table  # Human-readable tables
```

---

## Global Options

```bash
--api-key <KEY>   # Override DD_API_KEY
--app-key <KEY>   # Override DD_APP_KEY
--site <SITE>     # Override DD_SITE
--format <format> # json|jsonl|table
-v, --verbose     # Debug logging
```

**Multi-region sites:**
- `datadoghq.com` (US1, default)
- `datadoghq.eu` (EU)
- `us3.datadoghq.com`, `us5.datadoghq.com`
- `ddog-gov.com` (US1-FED)
