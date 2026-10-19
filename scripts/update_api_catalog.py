#!/usr/bin/env python3
"""Regenerate the reviewed, offline Datadog query contracts.

Run with: uv run --with PyYAML==6.0.3 python scripts/update_api_catalog.py
"""

import argparse
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
API = ROOT / "api"
METHODS = {"get", "post", "put", "patch", "delete"}


def resolve(document, value):
    while isinstance(value, dict) and "$ref" in value:
        reference = value["$ref"]
        if not reference.startswith("#/"):
            raise ValueError(f"External reference is unsupported: {reference}")
        value = document
        for component in reference[2:].split("/"):
            value = value[component.replace("~1", "/").replace("~0", "~")]
    return value


def schema(value):
    if not isinstance(value, dict):
        return value
    result = {
        key: item
        for key, item in value.items()
        if key not in {"example", "examples", "externalDocs", "xml", "discriminator"}
        and not key.startswith("x-")
    }
    for key in ("properties", "patternProperties", "definitions", "$defs"):
        if key in result:
            result[key] = {name: schema(item) for name, item in result[key].items()}
    for key in ("items", "additionalProperties", "not", "contains", "propertyNames"):
        if key in result:
            result[key] = schema(result[key])
    for key in ("allOf", "oneOf", "anyOf"):
        if key in result:
            result[key] = [schema(item) for item in result[key]]
    if "$ref" in result:
        result["$ref"] = result["$ref"].replace(
            "#/components/schemas/", "#/definitions/"
        )
    if result.pop("nullable", False) and "type" in result:
        result["type"] = [result["type"], "null"]
    for exclusive, inclusive in [
        ("exclusiveMinimum", "minimum"),
        ("exclusiveMaximum", "maximum"),
    ]:
        if isinstance(result.get(exclusive), bool):
            enabled = result.pop(exclusive)
            if enabled:
                result[exclusive] = result.pop(inclusive)
    return result


def media(content):
    if not content:
        return None
    for name in ("application/json", "application/vnd.api+json"):
        if name in content:
            return name
    return next(iter(content))


def references(value):
    if not isinstance(value, dict):
        return
    reference = value.get("$ref")
    if isinstance(reference, str) and reference.startswith("#/definitions/"):
        yield reference.removeprefix("#/definitions/")
    for key in ("properties", "patternProperties", "definitions", "$defs"):
        for item in value.get(key, {}).values():
            yield from references(item)
    for key in ("items", "additionalProperties", "not", "contains", "propertyNames"):
        yield from references(value.get(key))
    for key in ("allOf", "oneOf", "anyOf"):
        for item in value.get(key, []):
            yield from references(item)


def lines_object(value):
    return "{\n" + ",\n".join(
        json.dumps(key) + ":" + json.dumps(item, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
        for key, item in sorted(value.items())
    ) + "\n}\n"


def property_at(document, root, pointer):
    if not pointer.startswith("/"):
        raise ValueError(f"Invalid contract pointer: {pointer}")
    for component in pointer[1:].split("/"):
        root = resolve(document, root)
        root = root["properties"][component.replace("~1", "/").replace("~0", "~")]
    return resolve(document, root)


def main():
    import yaml

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, help="Use already downloaded v1.yaml/v2.yaml")
    args = parser.parse_args()
    source = json.loads((API / "source.json").read_text())
    decisions = json.loads((API / "policy.json").read_text())
    overrides = json.loads((API / "overrides.json").read_text())
    operations = {}
    definitions = {}
    seen = set()
    for version in ("v1", "v2"):
        url = f"https://raw.githubusercontent.com/DataDog/datadog-api-client-rust/{source['commit']}/.generator/schemas/{version}/openapi.yaml"
        if args.source_dir:
            raw = (args.source_dir / f"{version}.yaml").read_bytes()
        else:
            with urllib.request.urlopen(url, timeout=30) as response:
                raw = response.read(32 * 1024 * 1024)
        if hashlib.sha256(raw).hexdigest() != source[version]["sha256"]:
            raise ValueError(f"{version} source digest does not match {API / 'source.json'}")
        document = yaml.safe_load(raw)
        definitions[version] = {
            name: schema(value)
            for name, value in document["components"]["schemas"].items()
        }
        for name, override in overrides.items():
            schema_version, model = name.split(".", 1)
            if schema_version == version:
                for field, constraints in override["properties"].items():
                    definitions[version][model]["properties"][field].update(constraints)
        for path, methods in document["paths"].items():
            for method, operation in methods.items():
                if method not in METHODS:
                    continue
                key = f"{version}.{operation['operationId']}"
                decision = decisions[key]
                seen.add(key)
                if decision["method"] != method or decision["path"] != path:
                    raise ValueError(f"Reviewed endpoint changed: {key}")
                entry = {
                    "id": key,
                    "operation_id": operation["operationId"],
                    "version": version,
                    "method": method.upper(),
                    "path": path,
                    "domains": operation.get("tags", []),
                    "summary": operation.get("summary", ""),
                    "description": operation.get("description", ""),
                    "preview": operation.get("x-unstable"),
                    "deprecated": operation.get("deprecated", False),
                    "security": operation.get("security", document.get("security", [])),
                    "access": decision["access"],
                    "reason": decision.get("reason"),
                }
                if decision["access"] == "query":
                    parameters = []
                    for item in methods.get("parameters", []) + operation.get("parameters", []):
                        parameter = resolve(document, item)
                        parameter_schema = schema(parameter.get("schema", {}))
                        parameter_schema.update(decision.get("parameter_constraints", {}).get(
                            f"{parameter['in']}:{parameter['name']}", {}
                        ))
                        parameters.append({
                            "name": parameter["name"],
                            "location": parameter["in"],
                            "required": parameter.get("required", False),
                            "description": parameter.get("description", ""),
                            "style": parameter.get("style", "form" if parameter["in"] == "query" else "simple"),
                            "explode": parameter.get("explode", parameter["in"] == "query"),
                            "schema": parameter_schema,
                        })
                    entry["parameters"] = parameters
                    request = resolve(document, operation.get("requestBody", {}))
                    request_media = media(request.get("content", {}))
                    entry["body_required"] = request.get("required", False)
                    entry["request_media"] = request_media
                    entry["body_schema"] = schema(request["content"][request_media].get("schema", {})) if request_media else None
                    entry["responses"] = {}
                    for status, response in operation.get("responses", {}).items():
                        if not str(status).startswith("2") and str(status) != "304":
                            continue
                        response = resolve(document, response)
                        response_media = media(response.get("content", {}))
                        entry["responses"][str(status)] = {
                            "media": response_media,
                            "schema": schema(response["content"][response_media].get("schema", {})) if response_media else None,
                        }
                    entry["responses"].update(decision.get("additional_responses", {}))
                    entry["retry"] = decision.get("retry", "read")
                    entry["pagination"] = decision.get("pagination")
                    entry["servers"] = operation.get("servers")
                    entry["time_range"] = decision.get("time_range")
                    entry["processing"] = decision.get("processing")
                    entry["exclusive_input"] = decision.get("exclusive_input", [])
                operations[key] = entry
    if seen != set(decisions):
        raise ValueError(f"Stale review decisions: {set(decisions) - seen}")
    for operation in operations.values():
        if operation["access"] != "query":
            continue
        document = {"definitions": definitions[operation["version"]]}
        groups = {"properties": {"body": operation["body_schema"]}}
        for container, location in [("path", "path"), ("query", "query"), ("headers", "header")]:
            groups["properties"][container] = {"properties": {
                parameter["name"]: parameter["schema"] for parameter in operation["parameters"]
                if parameter["location"] == location
            }}
        if contract := operation["time_range"]:
            expected_type = "string" if contract["encoding"] == "rfc3339" else "integer"
            for pointer in [contract["from"], contract["to"]]:
                if property_at(document, groups, pointer).get("type") != expected_type:
                    raise ValueError(f"Incorrect time field type: {operation['id']} {pointer}")
        if contract := operation["processing"]:
            status = property_at(document, operation["responses"]["200"]["schema"], contract["pointer"])
            if status.get("type") != "string":
                raise ValueError(f"Incorrect processing status: {operation['id']}")
        for rule in operation["exclusive_input"]:
            for pointer in [rule["field"], *rule["conflicts"]]:
                property_at(document, groups, pointer)
    (API / "catalog.json").write_text(lines_object(operations))
    for version, values in definitions.items():
        pending = []
        for operation in operations.values():
            if operation["version"] != version or operation["access"] != "query":
                continue
            roots = [parameter["schema"] for parameter in operation["parameters"]]
            roots.append(operation["body_schema"])
            roots.extend(response["schema"] for response in operation["responses"].values())
            for root in roots:
                pending.extend(references(root))
        needed = set()
        while pending:
            name = pending.pop()
            if name not in needed:
                needed.add(name)
                pending.extend(references(values[name]))
        (API / f"{version}.schemas.json").write_text(lines_object({name: values[name] for name in needed}))
    print(json.dumps({"operations": len(operations), "query": sum(o["access"] == "query" for o in operations.values())}))


if __name__ == "__main__":
    main()
