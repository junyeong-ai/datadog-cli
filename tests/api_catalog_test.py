import importlib.util
from pathlib import Path
import unittest


script = Path(__file__).resolve().parents[1] / "scripts" / "update_api_catalog.py"
spec = importlib.util.spec_from_file_location("update_api_catalog", script)
catalog = importlib.util.module_from_spec(spec)
spec.loader.exec_module(catalog)


class SchemaConversionTest(unittest.TestCase):
    def test_property_names_and_instance_values_are_not_schema_keywords(self):
        source = {"type": "object", "properties": {
            "example": {"type": "string"},
            "nullable": {"type": "boolean"},
            "type": {"enum": ["x", "y"]},
        }, "default": {"nullable": True, "type": "user-data"}}
        self.assertEqual(catalog.schema(source), source)

    def test_nullable_and_exclusive_bounds_retain_their_meaning(self):
        self.assertEqual(catalog.schema({"type": "number", "nullable": True,
                                        "minimum": 1, "exclusiveMinimum": True}),
                         {"type": ["number", "null"], "exclusiveMinimum": 1})
        self.assertEqual(catalog.schema({"type": "integer", "maximum": 2,
                                        "exclusiveMaximum": False}),
                         {"type": "integer", "maximum": 2})

    def test_references_are_rewritten_only_in_schema_locations(self):
        source = {"properties": {"ref": {"$ref": "#/components/schemas/Item"}},
                  "default": {"$ref": "application-value"}}
        converted = catalog.schema(source)
        self.assertEqual(converted["properties"]["ref"]["$ref"], "#/definitions/Item")
        self.assertEqual(converted["default"], source["default"])
        self.assertEqual(list(catalog.references(converted)), ["Item"])

    def test_reference_resolution_cannot_fetch_external_resources(self):
        with self.assertRaises(ValueError):
            catalog.resolve({}, {"$ref": "https://example.test/schema"})


if __name__ == "__main__":
    unittest.main()
