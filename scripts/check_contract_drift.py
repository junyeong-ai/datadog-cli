#!/usr/bin/env python3
"""Report upstream Datadog API operations and MCP tools that the reviewed contracts do not cover.

Run with: uv run --with PyYAML==6.0.3 python scripts/check_contract_drift.py [--report PATH]
A Markdown report is written only when a policy review is needed. After reviewing the
MCP tools reference, record its tool names with --write-mcp-reference.
"""

import argparse
import json
import os
from pathlib import Path
import re
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
API = ROOT / "api"
SPEC_REPOSITORY = "DataDog/datadog-api-client-rust"
METHODS = {"get", "post", "put", "patch", "delete"}
TOOL_HEADING = re.compile(r"^### (`[a-z0-9_]+`(?: / `[a-z0-9_]+`)*)", re.M)


def fetch(url):
    headers = {"User-Agent": "datadog-cli-contract-drift"}
    token = os.environ.get("GH_TOKEN")
    if token and url.startswith("https://api.github.com/"):
        headers["Authorization"] = f"Bearer {token}"
    with urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=60) as response:
        return response.read(32 * 1024 * 1024)


def documented_tools(markdown):
    return sorted({name for heading in TOOL_HEADING.findall(markdown)
                   for name in re.findall(r"`([a-z0-9_]+)`", heading)})


def operations(version, document):
    return {
        f"{version}.{operation['operationId']}": (method, path, operation.get("deprecated", False))
        for path, methods in document["paths"].items()
        for method, operation in methods.items() if method in METHODS
    }


def operation_drift(upstream, policy):
    shared = set(upstream) & set(policy)
    return {
        "Unreviewed operations": [f"`{key}` {upstream[key][0].upper()} `{upstream[key][1]}`"
                                  for key in sorted(set(upstream) - set(policy))],
        "Removed operations": [f"`{key}` ({policy[key]['access']})" for key in sorted(set(policy) - set(upstream))],
        "Moved operations": [
            f"`{key}` {policy[key]['method'].upper()} `{policy[key]['path']}` → "
            f"{upstream[key][0].upper()} `{upstream[key][1]}`"
            for key in sorted(shared) if upstream[key][:2] != (policy[key]["method"], policy[key]["path"])
        ],
        "Newly deprecated operations": [f"`{key}` ({policy[key]['access']})" for key in sorted(shared)
                                        if upstream[key][2] and policy[key]["access"] != "deprecated"],
    }


def tool_drift(documented, reference, reviewed):
    return {
        "Tools added to the reference": [f"`{name}`" for name in sorted(set(documented) - set(reference))],
        "Tools removed from the reference": [f"`{name}`{' (callable)' if name in reviewed else ''}"
                                             for name in sorted(set(reference) - set(documented))],
    }


def report(header, sections):
    body = [f"### {title} ({len(items)})\n\n" + "\n".join(f"- {item}" for item in items)
            for title, items in sections.items() if items]
    return "\n\n".join([header, *body]) if body else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="Write the report here instead of stdout")
    parser.add_argument("--write-mcp-reference", action="store_true",
                        help="Record the current MCP reference tool names as reviewed")
    args = parser.parse_args()
    mcp_source = json.loads((API / "mcp-source.json").read_text())
    tools = documented_tools(fetch(mcp_source["url"]).decode())
    reference_path = API / "mcp-reference-tools.json"
    if args.write_mcp_reference:
        reference_path.write_text("[\n" + ",\n".join(json.dumps(name) for name in tools) + "\n]\n")
        return

    import yaml

    source = json.loads((API / "source.json").read_text())
    latest = json.loads(fetch(
        f"https://api.github.com/repos/{SPEC_REPOSITORY}/commits?path=.generator/schemas&per_page=1"))[0]
    upstream = {}
    for version in ("v1", "v2"):
        raw = fetch(f"https://raw.githubusercontent.com/{SPEC_REPOSITORY}/{latest['sha']}"
                    f"/.generator/schemas/{version}/openapi.yaml")
        upstream.update(operations(version, yaml.load(raw, Loader=getattr(yaml, "CSafeLoader", yaml.SafeLoader))))
    reviewed = json.loads((API / "mcp-tools.json").read_text())
    parts = [
        report(f"## API operations\n\nPinned `{source['commit'][:7]}` ({source['date']}); upstream "
               f"`{latest['sha'][:7]}` ({latest['commit']['committer']['date']}). "
               "Review each change in `api/policy.json`, then regenerate with `scripts/update_api_catalog.py`.",
               operation_drift(upstream, json.loads((API / "policy.json").read_text()))),
        report(f"## MCP tools\n\nReviewed reference retrieved {mcp_source['retrieved_at']}: {mcp_source['url']}. "
               "Review `api/mcp-tools.json`, then record the reference with `--write-mcp-reference`.",
               tool_drift(tools, json.loads(reference_path.read_text()), reviewed)),
    ]
    text = "\n\n".join(part for part in parts if part)
    if not text:
        return
    if args.report:
        args.report.write_text(text + "\n")
    else:
        print(text)


if __name__ == "__main__":
    main()
