# Datadog CLI Command Reference

Complete syntax for all commands. **Remember**: `--format` is a global option and must be placed BEFORE the command.

---

## Logs

### logs search
```bash
datadog-cli [--format <format>] logs search [query] [OPTIONS]
```

**Arguments:**
- `[query]` - Log search query (default: "*")

**Options:**
- `--from <time>` - Start time (default: "1 hour ago")
- `--to <time>` - End time (default: "now")
- `--limit <n>` - Max results (default: 10)
- `--cursor <token>` - Pagination cursor from previous response
- `--sort <field>` - Sort order (use `--sort="-timestamp"` for descending)
- `--tag-filter "<prefixes>"` - Tag filter (e.g., "env:,service:")

**Query syntax:**
- `status:error` - Filter by status
- `service:api` - Filter by service
- `status:error AND service:api` - Combine filters
- `*` - Match all

**Example:**
```bash
datadog-cli --format json logs search "status:error service:api" --from "1 hour ago" --limit 20
```

---

### logs aggregate
```bash
datadog-cli [--format <format>] logs aggregate "<query>" --from "<time>" --to "<time>"
```

**Arguments:**
- `<query>` - Log search query (default: "*")

**Options:**
- `--from <time>` - Start time (required)
- `--to <time>` - End time (required)

**Note:** Returns basic count only, no grouping/sorting.

**Example:**
```bash
datadog-cli --format json logs aggregate "status:error" --from "1 hour ago" --to "now"
```

---

### logs timeseries
```bash
datadog-cli [--format <format>] logs timeseries "<query>" --from "<time>" --to "<time>" [OPTIONS]
```

**Arguments:**
- `<query>` - Log search query (default: "*")

**Options:**
- `--from <time>` - Start time (required)
- `--to <time>` - End time (required)
- `--interval "<duration>"` - Time bucket (default: "1h")
- `--aggregation "<type>"` - Aggregation type (default: "count")
- `--metric "<field>"` - Field to aggregate (required for avg/sum/min/max/pc95)

**Example:**
```bash
datadog-cli --format json logs timeseries "service:api" \
  --from "6 hours ago" --to "now" \
  --interval "1h" --aggregation "count"
```

---

## Metrics

```bash
datadog-cli [--format <format>] metrics "<query>" [OPTIONS]
```

**Arguments:**
- `<query>` - Metrics query (required)

**Query format:** `<aggregator>:<metric>{<scope>}[ by {<tag>}]`

**Examples:**
- `avg:system.cpu.user{*}` - Average across all hosts
- `avg:system.cpu.user{service:web}` - Filtered by service
- `avg:system.cpu.user{*} by {service}` - Grouped by service
- `sum:system.mem.used{env:prod}` - Sum of memory usage

**Options:**
- `--from <time>` - Start time (default: "1 hour ago")
- `--to <time>` - End time (default: "now")
- `--max-points <n>` - Downsample for large time ranges

**Example:**
```bash
datadog-cli --format json metrics "avg:system.cpu.user{*}" --from "1 hour ago"
```

---

## Monitoring

### monitors list
```bash
datadog-cli [--format <format>] monitors list [OPTIONS]
```

**Options:**
- `--tags "<tags>"` - Filter by tags
- `--monitor-tags "<tags>"` - Filter by monitor tags
- `--page <n>` - Page number (default: 0)
- `--page-size <n>` - Results per page (default: 100)

**Example:**
```bash
datadog-cli --format json monitors list --tags "env:prod" --page-size 50
```

---

### monitors get
```bash
datadog-cli [--format <format>] monitors get <id>
```

**Arguments:**
- `<id>` - Monitor ID (required)

**Example:**
```bash
datadog-cli --format json monitors get 12345678
```

---

### events
```bash
datadog-cli [--format <format>] events [OPTIONS]
```

**Options:**
- `--from <time>` - Start time (default: "1 hour ago")
- `--to <time>` - End time (default: "now")
- `--priority "<level>"` - Filter by priority (low, normal)
- `--sources "<source>"` - Filter by source
- `--tags "<tags>"` - Filter by tags

**Example:**
```bash
datadog-cli --format json events --from "1 hour ago" --priority "normal"
```

---

## Infrastructure

### hosts
```bash
datadog-cli [--format <format>] hosts [OPTIONS]
```

**Options:**
- `--filter "<query>"` - Host filter query
- `--from <time>` - Start time (default: "1 hour ago")
- `--start <n>` - Pagination offset (default: 0)
- `--count <n>` - Results per page (default: 100)
- `--tag-filter "<prefixes>"` - Tag filter
- `--sort-field "<field>"` - Sort field
- `--sort-dir "<dir>"` - Sort direction (asc, desc)

**Example:**
```bash
datadog-cli --format json hosts --filter "env:prod" --count 50 --tag-filter "env:,service:"
```

---

## APM

### spans
```bash
datadog-cli [--format <format>] spans "<query>" --from "<time>" --to "<time>" [OPTIONS]
```

**Arguments:**
- `<query>` - Span search query (default: "*")

**Query examples:**
- `service:checkout` - Filter by service
- `error:true` - Errors only
- `http.status_code:>=500` - Server errors
- `service:api AND error:true` - Combined filters
- `trace_id:<id>` - Specific trace

**Options:**
- `--from <time>` - Start time (default: "1 hour ago")
- `--to <time>` - End time (default: "now")
- `--limit <n>` - Max results (default: 10)
- `--cursor "<token>"` - Pagination cursor
- `--sort "<field>"` - Sort order (use `--sort="-timestamp"` for descending)
- `--tag-filter "<prefixes>"` - Tag filter
- `--full-stack-trace` - Include full stack traces

**Example:**
```bash
datadog-cli --format jsonl spans "error:true service:api" \
  --from "10 minutes ago" --to "now" \
  --limit 20 --sort="-timestamp" --tag-filter "env:,service:"
```

---

### services
```bash
datadog-cli [--format <format>] services [OPTIONS]
```

**Options:**
- `--env "<environment>"` - Filter by environment

**Example:**
```bash
datadog-cli --format json services --env "production"
```

---

## RUM

```bash
datadog-cli [--format <format>] rum "<query>" [OPTIONS]
```

**Arguments:**
- `<query>` - RUM search query (default: "*")

**Query examples:**
- `@type:error` - User-facing errors
- `@type:session AND @session.type:user` - User sessions
- `@view.url_path:/checkout` - Specific page views
- `@user.plan:premium` - Filter by user attributes

**Options:**
- `--from <time>` - Start time (default: "1 hour ago")
- `--to <time>` - End time (default: "now")
- `--limit <n>` - Max results (default: 10)
- `--cursor "<token>"` - Pagination cursor
- `--sort "<field>"` - Sort field
- `--tag-filter "<prefixes>"` - Tag filter
- `--full-stack-trace` - Include full stack traces

**Example:**
```bash
datadog-cli --format jsonl rum "@type:error @user.plan:premium" \
  --from "1 hour ago" --limit 50 --tag-filter "user_id:,session_id:"
```

---

## Dashboards

### dashboards list
```bash
datadog-cli [--format <format>] dashboards list [OPTIONS]
```

**Options:**
- `--count <n>` - Results per page (default: 100)
- `--start <n>` - Pagination offset (default: 0)
- `--filter-shared` - Include shared dashboards only
- `--filter-deleted` - Include deleted dashboards only

**Example:**
```bash
datadog-cli --format json dashboards list --count 50
```

---

### dashboards get
```bash
datadog-cli [--format <format>] dashboards get <id>
```

**Arguments:**
- `<id>` - Dashboard ID (required)

**Example:**
```bash
datadog-cli --format json dashboards get "abc-123-def"
```

---

## Configuration

### config init
```bash
datadog-cli config init
```

Creates config file at `~/.config/datadog-cli/config.toml`.

---

### config show
```bash
datadog-cli config show
```

Displays current config with masked secrets.

---

### config path
```bash
datadog-cli config path
```

Shows path to active config file.

---

### config edit
```bash
datadog-cli config edit
```

Opens config in `$EDITOR` (default: vim).

---

## Time Formats

**Relative intervals:**
- "1 hour ago", "30 minutes ago", "2 days ago", "3 weeks ago"
- Short forms: "3h ago", "2d ago", "5m ago"

**Absolute formats:**
- ISO8601/RFC3339: "2025-01-15T10:30:00Z"
- Unix timestamp: "1704067200"
- Special: "now"

---

## Tag Filtering

Reduce response size by including only specific tag prefixes:

```bash
--tag-filter "env:,service:,version:"  # Include these prefixes
--tag-filter ""                        # Exclude all tags
--tag-filter "*"                       # Include all (default)
```

**Commands supporting tag filtering:**
- `logs search`
- `spans`
- `rum`
- `hosts`

**Environment variable:**
```bash
export DD_TAG_FILTER="env:,service:"
```

---

## Pagination Patterns

### Logs (Simple Limit)
```bash
datadog-cli --format json logs search "query" --limit 100
```

### Spans/RUM (Cursor-based)
```bash
# First page
datadog-cli --format json spans "query" --from "1h ago" --to "now" --limit 100 > page1.json

# Get cursor from response
CURSOR=$(jq -r '.meta.page.after' page1.json)

# Next page
datadog-cli --format json spans "query" --from "1h ago" --to "now" --limit 100 --cursor "$CURSOR"
```

### Hosts (Offset-based)
```bash
# First 100
datadog-cli --format json hosts --start 0 --count 100

# Next 100
datadog-cli --format json hosts --start 100 --count 100
```

---

## Output Formats

```bash
# Pretty JSON (default) - human-readable
datadog-cli --format json <command>

# JSON Lines - one object per line for Unix pipelines
datadog-cli --format jsonl <command> | jq -r '.service'

# Table - human-readable tables
datadog-cli --format table <command>
```

---

## Global Options

These apply to ALL commands:

```bash
--format <format>     # Output format: json|jsonl|table
--api-key <KEY>       # Override DD_API_KEY env var
--app-key <KEY>       # Override DD_APP_KEY env var
--site <SITE>         # Override DD_SITE env var
-v, --verbose         # Enable debug logging
-h, --help            # Show help
-V, --version         # Show version
```

**Multi-region sites:**
- `datadoghq.com` - US1 (default)
- `datadoghq.eu` - EU
- `us3.datadoghq.com` - US3
- `us5.datadoghq.com` - US5
- `ddog-gov.com` - US1-FED
