"""Offline checks for workspace lint inheritance and shared test attributes."""

import os
from pathlib import Path
import re
import sys
import tomllib
import unittest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from workspace import ROOT, wasm_exclusions, workspace_members, workspace_metadata


# Skip comments and string literals so examples in docs or fixtures don't count.
TEST_TOKENS = re.compile(
    r'//[^\n]*|/\*[\s\S]*?\*/|r(?P<hashes>\#*)"[\s\S]*?"(?P=hashes)'
    r"""|"(?:\\[\s\S]|[^"\\])*"|'(?:\\.|[^'\\\n])'|(?P<bare_test>#\s*\[\s*test\s*\])"""
)


class WorkspaceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.metadata = workspace_metadata()
        cls.members = workspace_members(cls.metadata)

    def test_workspace_members_inherit_lints(self):
        for package in self.members:
            manifest = Path(package["manifest_path"])
            with self.subTest(manifest=manifest.relative_to(ROOT)), manifest.open("rb") as file:
                self.assertIs(tomllib.load(file).get("lints", {}).get("workspace"), True)

    def test_shared_crates_use_wasm_compatible_tests(self):
        excluded = set(wasm_exclusions(self.metadata))
        skipped_dirs = {Path(package["manifest_path"]).parent for package in self.members}
        skipped_dirs.add(Path(self.metadata["target_directory"]))
        for package in self.members:
            if package["name"] in excluded:
                continue
            crate_dir = Path(package["manifest_path"]).parent
            for directory, subdirs, files in os.walk(crate_dir):
                path = Path(directory)
                subdirs[:] = [
                    name for name in subdirs
                    if name not in {".git", "target"} and path / name not in skipped_dirs
                ]
                for filename in files:
                    if not filename.endswith(".rs"):
                        continue
                    source = path / filename
                    code = source.read_text(encoding="utf-8")
                    for match in TEST_TOKENS.finditer(code):
                        if match.lastgroup != "bare_test":
                            continue
                        line = code.count("\n", 0, match.start()) + 1
                        with self.subTest(source=source.relative_to(ROOT), line=line):
                            self.fail("Use #[wasm_bindgen_test(unsupported = test)] for shared tests")
