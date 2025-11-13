# Datadog CLI Command Reference

Complete syntax reference for all 13 commands.

## Logs (3 commands)

### logs search
```bash
datadog logs search "<query>" --from "<time>" --to "<time>" [options]
```

**Options:**
- `--limit <n>` - Maximum results (default: 10)
- `--tag-filter "<prefixes>"` - Tag prefixes to include (e.g., "env:,service:")
- `--format <format>` - Output format: json, jsonl, table

**Query syntax:**
- `status:error` - Filter by status
- `service:api` - Filter by service
- `status:error AND service:api` - Combine conditions
- `*` - Wildcard (all logs)

### logs aggregate
```bash
datadog logs aggregate "<query>" --from "<time>" --to "<time>"
```

**Returns:** Basic count aggregation only (no grouping/sorting)

### logs timeseries
```bash
datadog logs timeseries "<query>" --from "<time>" --to "<time>" [options]
```

**Options:**
- `--interval "<duration>"` - Time bucket: "1m", "5m", "1h", "1d"
- `--aggregation "<type>"` - count, avg, sum, min, max
- `--metric "<field>"` - Field to aggregate (required for avg/sum/min/max)

**Example:** Response time trend
```bash
datadog logs timeseries "service:api" \
  --from "24 hours ago" \
  --interval "1h" \
  --aggregation "avg" \
  --metric "@duration"
```

---

## Metrics

```bash
datadog metrics "<query>" --from "<time>" --to "<time>" [options]
```

**Query format:** `<aggregator>:<metric>{<scope>} by {<tag>}`

**Examples:**
- `avg:system.cpu.user{*}` - Average CPU across all hosts
- `avg:system.cpu.user{service:web}` - Average CPU for web service
- `avg:system.cpu.user{*} by {service}` - Average CPU grouped by service

**Options:**
- `--max-points <n>` - Downsample to N points (large time ranges)

---

## Monitoring

### monitors list
```bash
datadog monitors list [options]
```

**Options:**
- `--tags "<tags>"` - Filter by tags (e.g., "env:production")
- `--monitor-tags "<tags>"` - Filter by monitor tags

### monitors get
```bash
datadog monitors get <id>
```

**Returns:** Full monitor configuration and current state

### events
```bash
datadog events --from "<time>" --to "<time>" [options]
```

**Options:**
- `--priority "<level>"` - normal, low
- `--sources "<source>"` - Event source filter
- `--tags "<tags>"` - Tag filter

---

## Infrastructure

### hosts
```bash
datadog hosts [options]
```

**Options:**
- `--filter "<query>"` - Host filter query
- `--from "<time>"` - Start time
- `--start <n>` - Offset (pagination)
- `--count <n>` - Number of hosts
- `--tag-filter "<prefixes>"` - Tag prefixes to include

---

## APM

### spans
```bash
datadog spans "<query>" --from "<time>" --to "<time>" [options]
```

**Query examples:**
- `service:checkout` - Filter by service
- `error:true` - Only error traces
- `http.status_code:>=500` - Server errors
- `service:api AND error:true` - Combine conditions

**Options:**
- `--limit <n>` - Maximum results (default: 10)
- `--cursor "<token>"` - Pagination token from previous response
- `--full-stack-trace` - Include complete stack traces (default: 10 lines)
- `--tag-filter "<prefixes>"` - Tag prefixes to include

### services
```bash
datadog services [options]
```

**Options:**
- `--env "<environment>"` - Filter by environment (e.g., "production", "staging")

**Returns:** APM service catalog with teams, repositories, integrations

---

## RUM

```bash
datadog rum "<query>" --from "<time>" --to "<time>" [options]
```

**Query examples:**
- `@type:error` - User errors
- `@type:session AND @session.type:user` - User sessions
- `@view.url_path:/checkout` - Checkout page views

**Options:**
- `--limit <n>` - Maximum results (default: 10)
- `--cursor "<token>"` - Pagination token
- `--full-stack-trace` - Complete error stack traces
- `--tag-filter "<prefixes>"` - Tag prefixes to include

---

## Dashboards

### dashboards list
```bash
datadog dashboards list
```

**Returns:** All dashboards with IDs, titles, descriptions

### dashboards get
```bash
datadog dashboards get <id>
```

**Returns:** Full dashboard configuration (widgets, template variables, layout)

---

## Configuration

### config show
```bash
datadog config show
```

**Returns:** Current configuration with masked secrets

### config path
```bash
datadog config path [--global]
```

**Returns:**
- Without `--global`: Local `.env` file path
- With `--global`: Global config file path (`~/.config/datadog-cli/config`)

### config list
```bash
datadog config list
```

**Returns:** All configuration sources with status (environment, local, global)

### config edit
```bash
datadog config edit [--global]
```

**Opens:** Config file in `$EDITOR` (default: vim)

---

## Natural Language Time Parsing

Powered by interim library - supports intuitive time expressions:

**Relative:**
- "1 hour ago", "30 minutes ago", "2 days ago"
- "4 hours ago", "1 week ago"

**Named:**
- "yesterday", "last week", "last month"

**Absolute:**
- ISO8601: "2025-01-15T10:30:00Z"
- Unix timestamp: "1704067200"

**Special:**
- "now" - Current time

**Invalid:**
- ❌ "1h ago" (use "1 hour ago")
- ❌ "tomorrow" (future not supported)

---

## Tag Filtering (30-70% Response Reduction)

Reduce response size by filtering tags at API level:

```bash
--tag-filter "env:,service:"      # Only env: and service: tags
--tag-filter ""                   # Exclude all tags (maximum reduction)
--tag-filter "*"                  # All tags (default, largest response)
```

**Environment variable:**
```bash
DD_TAG_FILTER="env:,service:" datadog logs search "..."
```

**Applies to:** logs search, spans, rum, hosts

**Impact:**
- Typical reduction: 30-70% smaller responses
- Faster queries and reduced token usage
- Recommended for large datasets

---

## Pagination Strategies

Different commands use different pagination approaches:

### Limit-based (Simple)
**Commands:** logs search
```bash
--limit 10  # Return first 10 results
--limit 50  # Return first 50 results
```

No cursor needed - simple limit parameter.

### Cursor-based (Large Datasets)
**Commands:** spans, rum
```bash
# First page
datadog spans "error:true" --from "1h ago" --limit 100

# Response includes: "meta": {"page": {"after": "TOKEN"}}

# Next page
datadog spans "error:true" --from "1h ago" --limit 100 --cursor "TOKEN"
```

Efficient for large result sets.

### Offset-based (Infrastructure)
**Commands:** hosts
```bash
--start 0 --count 50   # First 50 hosts
--start 50 --count 50  # Next 50 hosts
--start 100 --count 50 # Next 50 hosts
```

Traditional offset/limit pattern.

---

## Stack Trace Truncation

For spans and RUM events with error stack traces:

**Default behavior:**
- First 10 lines of stack trace
- Followed by: "... (N more lines)"

**Full stack traces:**
```bash
--full-stack-trace  # Include complete stack trace
```

**Use cases:**
- Default: Quick error overview, token efficiency
- Full: Deep debugging, root cause analysis

---

## Output Formats

All commands support multiple output formats:

```bash
--format json       # Pretty JSON (default) - human-readable
--format jsonl      # JSON Lines - one JSON per line, Unix pipelines
--format table      # comfy-table - human-readable tables
```

**Default:** json (pretty-printed)

**Unix pipeline:** Use jsonl for processing with grep, jq, awk

**Human review:** Use table for terminal output

---

## Global Options

Available for all commands:

```bash
--api-key <KEY>        # Override DD_API_KEY
--app-key <KEY>        # Override DD_APP_KEY
--site <SITE>          # Override DD_SITE (multi-region)
--format <FORMAT>      # json|jsonl|table
-q, --quiet            # Suppress info messages, errors only
-v, --verbose          # Verbose/debug logging
```

**Multi-region sites:**
- `datadoghq.com` (US1, default)
- `datadoghq.eu` (EU)
- `us3.datadoghq.com` (US3)
- `us5.datadoghq.com` (US5)
- `ddog-gov.com` (US1-FED)

---

## Configuration Priority

Configuration is resolved in this order (highest to lowest):

1. **Command-line flags:** `--api-key`, `--app-key`, `--site`
2. **Environment variables:** `DD_API_KEY`, `DD_APP_KEY`, `DD_SITE`
3. **Local .env file:** `.env` in current directory
4. **Global config:** `~/.config/datadog-cli/config`

**Recommendation for AI agents:** Use environment variables for temporary overrides
