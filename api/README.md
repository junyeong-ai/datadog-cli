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
references, typed `nullable`, and boolean exclusive bounds. Validation constraints,
property names, enum values, and additional properties retain their API meanings.
Examples and OpenAPI-only annotations are omitted. Requests reject unknown parameter
names; extensible body and response fields follow the published schema.

Credential-bearing resources, remote mutations, deprecated operations, internal APIs,
and operations on a different origin are listed with an exclusion reason. POST search
and aggregation endpoints are individually reviewed. A GET method does not by itself
authorize execution.

Upstream contracts are licensed under Apache-2.0; see `LICENSE` and `NOTICE`.
