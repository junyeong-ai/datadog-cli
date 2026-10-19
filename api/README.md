# Datadog query contracts

`source.json` pins the official OpenAPI documents by commit and SHA-256. `policy.json`
records the reviewed method, path, access decision, and any execution policy for every
operation. New or changed endpoints require a policy review before regeneration.

`catalog.json` and `v*.schemas.json` are generated, bundled contracts. Queries use the
official parameter and body schemas; schema validation cannot establish access to a
product or the completeness of the organization's telemetry.

Regenerate with:

```sh
uv run --with PyYAML==6.0.3 python scripts/update_api_catalog.py
```

OpenAPI 3.0 schemas are converted to JSON Schema Draft 7 by translating local
references, typed `nullable`, Datadog's nullable references, and boolean exclusive bounds. Nullable references use a union with null, matching documented states such as [custom rules without revisions](https://docs.datadoghq.com/api/latest/static-analysis/show-custom-rule/). Validation constraints,
property names, enum values, and additional properties retain their API meanings.
Request schemas omit `readOnly` properties from required fields; response schemas omit `writeOnly` properties from required fields. Other required fields and property types remain validated. This normalization visits schema positions only, including local referenced definitions, without rewriting default or enum instance data. Examples and OpenAPI-only annotations are omitted. Requests reject unknown parameter
names; extensible body and response fields follow the published schema.

`overrides.json` records field-level contract corrections supported by an observed
response and a documentation reference. Apply only the evidenced constraint; retain
type validation for other values. Operation-specific documented limits and additional
statuses belong to `policy.json`.

Reviewed time, processing-state, and mutually exclusive filter mappings are explicit
policy entries. Generation verifies that each mapped JSON Pointer exists in the
published schema and that time field types agree with their declared encoding.
These mappings do not infer query syntax, units, or causality from telemetry text.

Credential-bearing resources, remote mutations, deprecated operations, internal APIs,
and operations on a different origin are listed with an exclusion reason. POST search
and aggregation endpoints are individually reviewed. A GET method does not by itself
authorize execution. Credential-bearing integration/action connections, webhook settings, and full Synthetic configurations are excluded even when retrieved by GET; dedicated Synthetic execution-result and version-history endpoints remain available. [Webhook authentication](https://docs.datadoghq.com/integrations/webhooks/#authentication) can store Basic credentials in the configured URL.

Upstream contracts are licensed under Apache-2.0; see `LICENSE` and `NOTICE`.

Cursor automation is opt-in and follows explicit request/response pointers validated against both schemas. Searches with time mappings need fixed bounds across pages. DDSQL submission never retries; completion polling uses only the returned opaque ID under the original identity and shared execution budget.

## Official MCP tools

`mcp-tools.json` contains individually reviewed read/analysis tools from the official tools reference; `mcp-source.json` records that reference and its digest. Names outside this policy cannot execute, even when a server advertises them as read-only. Mutations, remote actions, onboarding and arbitrary code execution are excluded. A contradictory `readOnlyHint: false` blocks execution. Input and declared structured output schemas come from the authenticated server and validate offline without external reference retrieval.

The official Rust MCP SDK owns protocol/session handling. The HTTP backend fixes the selected site origin, protects credential headers, disables redirects/reconnection/session replay, and bounds total calls/bytes/time including session cleanup. Requests use MCP protocol `2025-11-25`; the server negotiates the supported response version. No sampling, roots, elicitation, local subprocesses or resource fetching are enabled. A successful handshake does not establish availability of any specialist tool.
