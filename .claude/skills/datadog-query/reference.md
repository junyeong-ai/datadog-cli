# Datadog CLI command reference

Generated from Clap with `cargo run --locked --example command_reference`.
Global options work before or after subcommands. JSON retains pagination metadata; JSONL emits data records only.

## Contents

- [`datadog-cli`](#datadog-cli)
- [`datadog-cli query`](#datadog-cli-query)
- [`datadog-cli capabilities`](#datadog-cli-capabilities)
- [`datadog-cli capabilities domains`](#datadog-cli-capabilities-domains)
- [`datadog-cli capabilities list`](#datadog-cli-capabilities-list)
- [`datadog-cli capabilities show`](#datadog-cli-capabilities-show)
- [`datadog-cli metrics`](#datadog-cli-metrics)
- [`datadog-cli timeseries`](#datadog-cli-timeseries)
- [`datadog-cli scalar`](#datadog-cli-scalar)
- [`datadog-cli logs`](#datadog-cli-logs)
- [`datadog-cli logs search`](#datadog-cli-logs-search)
- [`datadog-cli logs aggregate`](#datadog-cli-logs-aggregate)
- [`datadog-cli logs timeseries`](#datadog-cli-logs-timeseries)
- [`datadog-cli monitors`](#datadog-cli-monitors)
- [`datadog-cli monitors list`](#datadog-cli-monitors-list)
- [`datadog-cli monitors get`](#datadog-cli-monitors-get)
- [`datadog-cli events`](#datadog-cli-events)
- [`datadog-cli hosts`](#datadog-cli-hosts)
- [`datadog-cli dashboards`](#datadog-cli-dashboards)
- [`datadog-cli dashboards list`](#datadog-cli-dashboards-list)
- [`datadog-cli dashboards get`](#datadog-cli-dashboards-get)
- [`datadog-cli spans`](#datadog-cli-spans)
- [`datadog-cli services`](#datadog-cli-services)
- [`datadog-cli rum`](#datadog-cli-rum)
- [`datadog-cli slo`](#datadog-cli-slo)
- [`datadog-cli slo list`](#datadog-cli-slo-list)
- [`datadog-cli slo get`](#datadog-cli-slo-get)
- [`datadog-cli incidents`](#datadog-cli-incidents)
- [`datadog-cli incidents list`](#datadog-cli-incidents-list)
- [`datadog-cli incidents get`](#datadog-cli-incidents-get)
- [`datadog-cli error-tracking`](#datadog-cli-error-tracking)
- [`datadog-cli error-tracking search`](#datadog-cli-error-tracking-search)
- [`datadog-cli error-tracking get`](#datadog-cli-error-tracking-get)
- [`datadog-cli downtimes`](#datadog-cli-downtimes)
- [`datadog-cli audit`](#datadog-cli-audit)
- [`datadog-cli teams`](#datadog-cli-teams)
- [`datadog-cli llm-obs`](#datadog-cli-llm-obs)
- [`datadog-cli config`](#datadog-cli-config)
- [`datadog-cli config init`](#datadog-cli-config-init)
- [`datadog-cli config show`](#datadog-cli-config-show)
- [`datadog-cli config path`](#datadog-cli-config-path)
- [`datadog-cli config edit`](#datadog-cli-config-edit)
- [Supported sites](#supported-sites)

## `datadog-cli`

```text
High-performance Datadog CLI

Usage: datadog-cli [OPTIONS] <COMMAND>

Commands:
  query           Execute a reviewed API query with offline schema validation
  capabilities    Discover reviewed API queries and their input schemas without authentication
  metrics         Query time series metrics
  timeseries      Query timeseries with formulas across multiple metric queries
  scalar          Query scalar values (single aggregate per query) with formulas
  logs            Log operations
  monitors        Monitor operations
  events          Query events
  hosts           List infrastructure hosts
  dashboards      Dashboard operations
  spans           Search APM spans
  services        List services from catalog
  rum             Search RUM events
  slo             Service Level Objective operations
  incidents       Incident operations (requires Incident Management)
  error-tracking  Error Tracking operations
  downtimes       List downtimes
  audit           Search audit trail events
  teams           List teams
  llm-obs         Search LLM Observability spans (preview API, subject to change)
  config          Config management
  help            Print this message or the help of the given subcommand(s)

Options:
      --error-format <ERROR_FORMAT>    Final runtime error format on stderr; CLI usage errors remain text [default: text] [possible values: text, json]
      --format <FORMAT>                Output format (default from config) [possible values: json, jsonl, table]
  -v, --verbose                        Enable debug logging
      --api-key <API_KEY>              API key (otherwise DD_API_KEY or config)
      --app-key <APP_KEY>              Application key (otherwise DD_APP_KEY or config)
      --token <TOKEN>                  Bearer token: PAT, SAT, or OAuth access token
      --site <SITE>                    Datadog site (otherwise DD_SITE or config)
      --auth-provider <AUTH_PROVIDER>  Use Pup OAuth (compatibility option for --auth-method oauth) [possible values: pup]
      --auth-method <AUTH_METHOD>      Authentication method (otherwise global config [auth].method) [possible values: api-keys, token, oauth]
      --pup-org <PUP_ORG>              Named Pup OAuth session (otherwise global config [auth].pup_org)
  -h, --help                           Print help
  -V, --version                        Print version
```

## `datadog-cli query`

```text
Execute a reviewed API query with offline schema validation

Usage: datadog-cli query [OPTIONS] <DOMAIN> <OPERATION>

Arguments:
  <DOMAIN>     Exact domain ID from capabilities domains
  <OPERATION>  Version-qualified operation ID from capabilities list

Options:
      --input <INPUT>          JSON request envelope, @FILE, or - for stdin; keys: path, query, headers, body
      --from <FROM>            Start time using this operation's reviewed time mapping; conflicts with native input time fields
      --to <TO>                End time for --from (default now); resolved with the same reference instant
      --select <SELECT>        Select an exact JSON Pointer from each API response; repeat for multiple fields
      --output <OUTPUT>        Save the original API response to a new file; required for binary/CSV responses
      --deadline <DEADLINE>    Authentication and API execution deadline in seconds [default: 60]
      --max-calls <MAX_CALLS>  Maximum API attempts, including retries [default: 4]
      --max-bytes <MAX_BYTES>  Maximum response bytes across API attempts [default: 8388608]
  -h, --help                   Print help
```

## `datadog-cli capabilities`

```text
Discover reviewed API queries and their input schemas without authentication

Usage: datadog-cli capabilities <COMMAND>

Commands:
  domains  List API domains; organization access is not probed
  list     List reviewed API operations
  show     Describe an operation, including its offline JSON input schema
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `datadog-cli capabilities domains`

```text
List API domains; organization access is not probed

Usage: datadog-cli capabilities domains [OPTIONS]

Options:
      --all   Include domains with only excluded operations
  -h, --help  Print help
```

## `datadog-cli capabilities list`

```text
List reviewed API operations

Usage: datadog-cli capabilities list [OPTIONS]

Options:
      --domain <DOMAIN>  Exact domain ID from capabilities domains
      --all              Include excluded operations and their reasons
      --search <SEARCH>  Literal case-insensitive text in the operation ID or summary
      --offset <OFFSET>  [default: 0]
      --limit <LIMIT>    [default: 25]
  -h, --help             Print help
```

## `datadog-cli capabilities show`

```text
Describe an operation, including its offline JSON input schema

Usage: datadog-cli capabilities show [OPTIONS] <OPERATION>

Arguments:
  <OPERATION>  Version-qualified operation ID, for example v2.SearchCIAppPipelineEvents

Options:
      --response-schema  Include complete response schemas; large products can have large schemas
  -h, --help             Print help
```

## `datadog-cli metrics`

```text
Query time series metrics

Usage: datadog-cli metrics [OPTIONS] <QUERY>

Arguments:
  <QUERY>

Options:
      --from <FROM>  Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>      Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
  -h, --help         Print help
```

## `datadog-cli timeseries`

```text
Query timeseries with formulas across multiple metric queries

Usage: datadog-cli timeseries [OPTIONS] <QUERIES>...

Arguments:
  <QUERIES>...  Metric queries, auto-named a, b, c… for formulas

Options:
      --formula <FORMULA>    Formula over query names (e.g. "a / b * 100"); repeatable
      --from <FROM>          Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>              Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --interval <INTERVAL>  Rollup interval in seconds
  -h, --help                 Print help
```

## `datadog-cli scalar`

```text
Query scalar values (single aggregate per query) with formulas

Usage: datadog-cli scalar [OPTIONS] <QUERIES>...

Arguments:
  <QUERIES>...  Metric queries, auto-named a, b, c… for formulas

Options:
      --formula <FORMULA>        Formula over query names (e.g. "a / b * 100"); repeatable
      --aggregator <AGGREGATOR>  Aggregator applied over the time window [default: avg] [possible values: avg, min, max, sum, last, percentile, mean, l2norm, area]
      --from <FROM>              Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>                  Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
  -h, --help                     Print help
```

## `datadog-cli logs`

```text
Log operations

Usage: datadog-cli logs <COMMAND>

Commands:
  search      Search logs
  aggregate   Aggregate logs into buckets
  timeseries  Generate log timeseries
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `datadog-cli logs search`

```text
Search logs

Usage: datadog-cli logs search [OPTIONS] [QUERY]

Arguments:
  [QUERY]  [default: *]

Options:
      --from <FROM>                  Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>                      Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --limit <LIMIT>                [default: 10]
      --cursor <CURSOR>              Pagination cursor from previous response
      --sort <SORT>                  Sort order (use --sort="-timestamp" for descending) [possible values: timestamp, -timestamp]
      --storage-tier <STORAGE_TIER>  Storage tier to search (default: indexes) [possible values: indexes, online-archives, flex]
      --tag-filter <TAG_FILTER>      Tag prefixes to include (default from config)
  -h, --help                         Print help
```

## `datadog-cli logs aggregate`

```text
Aggregate logs into buckets

Usage: datadog-cli logs aggregate [OPTIONS] [QUERY]

Arguments:
  [QUERY]  [default: *]

Options:
      --from <FROM>                Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>                    Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --group-by <GROUP_BY>        Group by a facet; buckets are ordered by descending log count
      --group-limit <GROUP_LIMIT>  Maximum groups to return (default 10, max 10000)
  -h, --help                       Print help
```

## `datadog-cli logs timeseries`

```text
Generate log timeseries

Usage: datadog-cli logs timeseries [OPTIONS] [QUERY]

Arguments:
  [QUERY]  [default: *]

Options:
      --from <FROM>                Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>                    Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --interval <INTERVAL>        Rollup interval (e.g., 5m, 1h, 1d) [default: 1h]
      --aggregation <AGGREGATION>  Aggregation type (count, avg, sum, min, max) [default: count]
      --metric <METRIC>            Metric field for aggregation
      --group-by <GROUP_BY>        Group by a facet; buckets are ordered by descending log count
      --group-limit <GROUP_LIMIT>  Maximum groups to return (default 10, max 10000)
  -h, --help                       Print help
```

## `datadog-cli monitors`

```text
Monitor operations

Usage: datadog-cli monitors <COMMAND>

Commands:
  list  List monitors
  get   Get monitor details
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `datadog-cli monitors list`

```text
List monitors

Usage: datadog-cli monitors list [OPTIONS]

Options:
      --tags <TAGS>                  Filter by resource tags
      --monitor-tags <MONITOR_TAGS>  Filter by monitor tags
      --page <PAGE>                  Page number [default: 0]
      --page-size <PAGE_SIZE>        Results per page (max 1000) [default: 100]
  -h, --help                         Print help
```

## `datadog-cli monitors get`

```text
Get monitor details

Usage: datadog-cli monitors get <MONITOR_ID>

Arguments:
  <MONITOR_ID>

Options:
  -h, --help  Print help
```

## `datadog-cli events`

```text
Query events

Usage: datadog-cli events [OPTIONS] [QUERY]

Arguments:
  [QUERY]  Event search query (e.g. "source:alert status:error") [default: *]

Options:
      --from <FROM>      Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>          Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --limit <LIMIT>    [default: 10]
      --cursor <CURSOR>  Pagination cursor from previous response
      --sort <SORT>      Sort order (use --sort="-timestamp" for descending) [possible values: timestamp, -timestamp]
  -h, --help             Print help
```

## `datadog-cli hosts`

```text
List infrastructure hosts

Usage: datadog-cli hosts [OPTIONS]

Options:
      --filter <FILTER>          Filter hosts by name, alias, or tag
      --from <FROM>              Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --sort-field <SORT_FIELD>  Sort field (e.g., cpu, iowait, load)
      --sort-dir <SORT_DIR>      Sort direction (asc, desc)
      --start <START>            Pagination offset [default: 0]
      --count <COUNT>            Results per page (max 1000) [default: 100]
      --tag-filter <TAG_FILTER>  Tag prefixes to include (default from config)
  -h, --help                     Print help
```

## `datadog-cli dashboards`

```text
Dashboard operations

Usage: datadog-cli dashboards <COMMAND>

Commands:
  list  List dashboards
  get   Get dashboard details
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `datadog-cli dashboards list`

```text
List dashboards

Usage: datadog-cli dashboards list [OPTIONS]

Options:
      --count <COUNT>   Results per page [default: 100]
      --start <START>   Pagination offset [default: 0]
      --filter-shared   Include shared dashboards only
      --filter-deleted  Include deleted dashboards only
  -h, --help            Print help
```

## `datadog-cli dashboards get`

```text
Get dashboard details

Usage: datadog-cli dashboards get <DASHBOARD_ID>

Arguments:
  <DASHBOARD_ID>

Options:
  -h, --help  Print help
```

## `datadog-cli spans`

```text
Search APM spans

Usage: datadog-cli spans [OPTIONS] [QUERY]

Arguments:
  [QUERY]  [default: *]

Options:
      --from <FROM>              Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>                  Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --limit <LIMIT>            [default: 10]
      --cursor <CURSOR>          Pagination cursor from previous response
      --sort <SORT>              Sort order (use --sort="-timestamp" for descending) [possible values: timestamp, -timestamp]
      --tag-filter <TAG_FILTER>  Tag prefixes to include (default from config)
      --full-stack-trace         Show full stack traces
  -h, --help                     Print help
```

## `datadog-cli services`

```text
List services from catalog

Usage: datadog-cli services [OPTIONS]

Options:
      --kind <KIND>        Entity kind (service, system, datastore, queue, api) [default: service]
      --name <NAME>        Filter by entity name
      --owner <OWNER>      Filter by owner
      --include <INCLUDE>  Include related data [possible values: schema, raw_schema, oncall, incident, relation]
      --start <START>      Pagination offset [default: 0]
      --count <COUNT>      Results per page [default: 100]
  -h, --help               Print help
```

## `datadog-cli rum`

```text
Search RUM events

Usage: datadog-cli rum [OPTIONS] [QUERY]

Arguments:
  [QUERY]  [default: *]

Options:
      --from <FROM>              Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>                  Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --limit <LIMIT>            [default: 10]
      --cursor <CURSOR>          Pagination cursor from previous response
      --sort <SORT>              Sort order (use --sort="-timestamp" for descending) [possible values: timestamp, -timestamp]
      --tag-filter <TAG_FILTER>  Tag prefixes to include (default from config)
      --full-stack-trace         Show full stack traces
  -h, --help                     Print help
```

## `datadog-cli slo`

```text
Service Level Objective operations

Usage: datadog-cli slo <COMMAND>

Commands:
  list  List SLOs
  get   Get SLO details
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `datadog-cli slo list`

```text
List SLOs

Usage: datadog-cli slo list [OPTIONS]

Options:
      --query <QUERY>            Filter by SLO name or description text
      --tags-query <TAGS_QUERY>  Filter by tags (e.g. "team:web")
      --limit <LIMIT>            Results per page [default: 100]
      --offset <OFFSET>          Pagination offset [default: 0]
  -h, --help                     Print help
```

## `datadog-cli slo get`

```text
Get SLO details

Usage: datadog-cli slo get <SLO_ID>

Arguments:
  <SLO_ID>

Options:
  -h, --help  Print help
```

## `datadog-cli incidents`

```text
Incident operations (requires Incident Management)

Usage: datadog-cli incidents <COMMAND>

Commands:
  list  List incidents
  get   Get incident details
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `datadog-cli incidents list`

```text
List incidents

Usage: datadog-cli incidents list [OPTIONS]

Options:
      --count <COUNT>      Results per page (max 100) [default: 25]
      --start <START>      Pagination offset [default: 0]
      --include <INCLUDE>  Include related data [possible values: users, attachments]
  -h, --help               Print help
```

## `datadog-cli incidents get`

```text
Get incident details

Usage: datadog-cli incidents get [OPTIONS] <INCIDENT_ID>

Arguments:
  <INCIDENT_ID>

Options:
      --include <INCLUDE>  Include related data [possible values: users, attachments]
  -h, --help               Print help
```

## `datadog-cli error-tracking`

```text
Error Tracking operations

Usage: datadog-cli error-tracking <COMMAND>

Commands:
  search  Search error issues
  get     Get error issue details
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `datadog-cli error-tracking search`

```text
Search error issues

Usage: datadog-cli error-tracking search [OPTIONS] [QUERY]

Arguments:
  [QUERY]  Issue search query (event search syntax) [default: *]

Options:
      --track <TRACK>      Error track to search [default: trace] [possible values: trace, logs, rum]
      --from <FROM>        Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>            Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --include <INCLUDE>  Include related data [possible values: issue, issue.assignee, issue.case, issue.team_owners]
  -h, --help               Print help
```

## `datadog-cli error-tracking get`

```text
Get error issue details

Usage: datadog-cli error-tracking get <ISSUE_ID>

Arguments:
  <ISSUE_ID>

Options:
  -h, --help  Print help
```

## `datadog-cli downtimes`

```text
List downtimes

Usage: datadog-cli downtimes [OPTIONS]

Options:
      --current-only   Only downtimes active right now
      --start <START>  Pagination offset [default: 0]
      --count <COUNT>  Results per page [default: 100]
  -h, --help           Print help
```

## `datadog-cli audit`

```text
Search audit trail events

Usage: datadog-cli audit [OPTIONS] [QUERY]

Arguments:
  [QUERY]  Audit event search query [default: *]

Options:
      --from <FROM>      Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>          Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --limit <LIMIT>    [default: 10]
      --cursor <CURSOR>  Pagination cursor from previous response
      --sort <SORT>      Sort order (use --sort="-timestamp" for descending) [possible values: timestamp, -timestamp]
  -h, --help             Print help
```

## `datadog-cli teams`

```text
List teams

Usage: datadog-cli teams [OPTIONS]

Options:
      --keyword <KEYWORD>      Search teams by keyword
      --me                     Only teams the caller belongs to
      --page <PAGE>            Page number [default: 0]
      --page-size <PAGE_SIZE>  Results per page (max 100) [default: 100]
  -h, --help                   Print help
```

## `datadog-cli llm-obs`

```text
Search LLM Observability spans (preview API, subject to change)

Usage: datadog-cli llm-obs [OPTIONS] [QUERY]

Arguments:
  [QUERY]  Agent Observability query; conflicts with structured filters

Options:
      --from <FROM>            Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: "1 hour ago"]
      --to <TO>                Time format: 'now', '1 hour ago', '2024-01-01T00:00:00Z', or Unix timestamp [default: now]
      --limit <LIMIT>          Results per page (CLI max 5000) [default: 10]
      --cursor <CURSOR>        Pagination cursor from previous response
      --sort <SORT>            Sort order (use --sort="-timestamp" for descending)
      --ml-app <ML_APP>        Filter by ML application name
      --span-kind <SPAN_KIND>  Filter by span kind (e.g. agent, workflow, llm, tool)
  -h, --help                   Print help
```

## `datadog-cli config`

```text
Config management

Usage: datadog-cli config <COMMAND>

Commands:
  init  Initialize config file
  show  Show current config
  path  Show config file path
  edit  Edit config file
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `datadog-cli config init`

```text
Initialize config file

Usage: datadog-cli config init

Options:
  -h, --help  Print help
```

## `datadog-cli config show`

```text
Show current config

Usage: datadog-cli config show

Options:
  -h, --help  Print help
```

## `datadog-cli config path`

```text
Show config file path

Usage: datadog-cli config path

Options:
  -h, --help  Print help
```

## `datadog-cli config edit`

```text
Edit config file

Usage: datadog-cli config edit

Options:
  -h, --help  Print help
```

## Supported sites

- `datadoghq.com`
- `us3.datadoghq.com`
- `us5.datadoghq.com`
- `datadoghq.eu`
- `ap1.datadoghq.com`
- `ap2.datadoghq.com`
- `uk1.datadoghq.com`
- `ddog-gov.com`
- `us2.ddog-gov.com`
