import importlib.util
from pathlib import Path
import unittest


script = Path(__file__).resolve().parents[1] / "scripts" / "check_contract_drift.py"
spec = importlib.util.spec_from_file_location("check_contract_drift", script)
drift = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drift)


class ContractDriftTest(unittest.TestCase):
    def test_documented_tools_include_every_name_in_combined_headings(self):
        markdown = (
            "### User-controlled content in tool results\n"
            "### `search_datadog_logs`{% #search_datadog_logs %}\n"
            "### `get_entity_tags` / `update_entity_tags`{% #get_entity_tags--update_entity_tags %}\n"
            "Mentions `inline_code` outside headings.\n"
        )
        self.assertEqual(drift.documented_tools(markdown),
                         ["get_entity_tags", "search_datadog_logs", "update_entity_tags"])

    def test_operation_drift_lists_only_changes_that_need_review(self):
        policy = {
            "v2.Same": {"method": "get", "path": "/same", "access": "query"},
            "v2.Moved": {"method": "get", "path": "/old", "access": "query"},
            "v2.Gone": {"method": "get", "path": "/gone", "access": "query"},
            "v2.Deprecated": {"method": "get", "path": "/deprecated", "access": "query"},
            "v2.AlreadyDeprecated": {"method": "get", "path": "/already", "access": "deprecated"},
        }
        upstream = {
            "v2.Same": ("get", "/same", False),
            "v2.Moved": ("get", "/new", False),
            "v2.Deprecated": ("get", "/deprecated", True),
            "v2.AlreadyDeprecated": ("get", "/already", True),
            "v2.New": ("post", "/new-search", False),
        }
        self.assertEqual(drift.operation_drift(upstream, policy), {
            "Unreviewed operations": ["`v2.New` POST `/new-search`"],
            "Removed operations": ["`v2.Gone` (query)"],
            "Moved operations": ["`v2.Moved` GET `/old` → GET `/new`"],
            "Newly deprecated operations": ["`v2.Deprecated` (query)"],
        })
        unchanged = {key: (value["method"], value["path"], value["access"] == "deprecated")
                     for key, value in policy.items()}
        self.assertIsNone(drift.report("## API operations", drift.operation_drift(unchanged, policy)))

    def test_tool_drift_marks_callable_tools_removed_from_the_reference(self):
        sections = drift.tool_drift(["kept", "added"], ["kept", "removed", "dropped"], {"removed": {}})
        self.assertEqual(sections, {
            "Tools added to the reference": ["`added`"],
            "Tools removed from the reference": ["`dropped`", "`removed` (callable)"],
        })


if __name__ == "__main__":
    unittest.main()
