"""Behavioral tests for affected Cargo package selection."""

import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("detect_affected_packages.py")
SPEC = importlib.util.spec_from_file_location("detect_affected_packages", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
DETECTOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DETECTOR)

ALL = ["crm", "runner", "yasweb", "tasker", "crm_updater", "wxcc_handler"]


class AffectedPackagesTests(unittest.TestCase):
    def test_runner_only_change_selects_runner(self):
        self.assertEqual(
            DETECTOR.affected_packages(["apps/runner/src/main.rs"]), ["runner"]
        )

    def test_wxcc_handler_only_change_selects_wxcc_handler(self):
        self.assertEqual(DETECTOR.affected_packages(["apps/wxcc_handler/src/main.rs"]), ["wxcc_handler"])

    def test_tasker_only_change_selects_tasker(self):
        self.assertEqual(
            DETECTOR.affected_packages(["apps/tasker/src/tasks/report.rs"]), ["tasker"]
        )

    def test_multiple_apps_select_only_changed_packages_in_stable_order(self):
        self.assertEqual(
            DETECTOR.affected_packages(
                ["apps/yasweb/src/main.rs", "apps/crm/src/lib.rs"]
            ),
            ["crm", "yasweb"],
        )

    def test_shared_root_code_and_manifests_select_all(self):
        for path in ("src/utils.rs", "tests/integration.rs", "Cargo.toml",
                     "Cargo.lock", "deny.toml"):
            with self.subTest(path=path):
                self.assertEqual(DETECTOR.affected_packages([path]), ALL)

    def test_workflow_and_detector_changes_select_all(self):
        for path in (".github/workflows/test.yml",
                     ".github/scripts/detect_affected_packages.py"):
            with self.subTest(path=path):
                self.assertEqual(DETECTOR.affected_packages([path]), ALL)

    def test_unknown_app_and_root_paths_fail_closed(self):
        for path in ("apps/new_process/src/main.rs", "build.rs", "unknown.toml"):
            with self.subTest(path=path):
                self.assertEqual(DETECTOR.affected_packages([path]), ALL)

    def test_documentation_only_changes_select_no_packages(self):
        self.assertEqual(
            DETECTOR.affected_packages(["md/BUILD.md", "md/ARCHITECTURE.md"]), []
        )

    def test_app_and_shared_change_select_all(self):
        self.assertEqual(
            DETECTOR.affected_packages(["apps/runner/src/main.rs", "Cargo.lock"]), ALL
        )

    def test_change_flags_select_only_relevant_language_and_dependency_checks(self):
        flags = DETECTOR.change_flags([
            "apps/runner/src/validation.js",
            "apps/runner/tests/validation.test.js",
            "apps/tasker/scripts/report.ps1",
            "apps/wxcc_handler/Cargo.toml",
        ])
        self.assertEqual(
            flags,
            {
                "javascript": True,
                "powershell": True,
                "runner_javascript": True,
                "dependencies": True,
            },
        )

    def test_rust_app_source_does_not_trigger_language_or_dependency_checks(self):
        self.assertEqual(
            DETECTOR.change_flags(["apps/wxcc_handler/src/wxcc/data.rs"]),
            {
                "javascript": False,
                "powershell": False,
                "runner_javascript": False,
                "dependencies": False,
            },
        )

    def test_nested_manifest_triggers_dependency_audit(self):
        self.assertTrue(
            DETECTOR.change_flags(["apps/tasker/Cargo.toml"])["dependencies"]
        )

    def test_dot_relative_and_windows_paths_are_normalized(self):
        self.assertEqual(
            DETECTOR.affected_packages([r".\.github\workflows\test.yml"]), ALL
        )


if __name__ == "__main__":
    unittest.main()
