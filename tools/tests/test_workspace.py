"""Offline checks for workspace lint inheritance and shared test attributes."""

import os
from pathlib import Path
import re
import sys
import tomllib
import unittest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from workspace import (
    ROOT,
    test_listing_errors,
    test_name_differences,
    wasm_exclusions,
    workspace_members,
    workspace_metadata,
)


RUST_TOKENS = re.compile(
    r'(?P<line>//[^\n]*)|(?P<block>/\*)'
    r'|(?:[bc]?r(?P<hashes>\#*)"[\s\S]*?"(?P=hashes))'
    r"""|[bc]?"(?:\\[\s\S]|[^"\\])*"|b?'(?:\\.|[^'\\\n])'"""
    r'|(?:r#)?[^\W\d]\w*|::|\S'
)
BLOCK_DELIMITERS = re.compile(r"/\*|\*/")


def rust_tokens(code):
    position = 0
    while match := RUST_TOKENS.search(code, position):
        position = match.end()
        if match.lastgroup == "block":
            depth = 1
            for delimiter in BLOCK_DELIMITERS.finditer(code, position):
                depth += 1 if delimiter[0] == "/*" else -1
                if depth == 0:
                    position = delimiter.end()
                    break
            else:
                return
        elif match.lastgroup != "line":
            yield match[0], match.start()


def attribute_arguments(tokens):
    argument = []
    depth = 0
    for token in tokens:
        if token == "," and depth == 0:
            yield argument
            argument = []
            continue
        argument.append(token)
        if token in ("(", "[", "{"):
            depth += 1
        elif token in (")", "]", "}"):
            depth -= 1
    if argument:
        yield argument


def incompatible_test_attribute(tokens):
    if "(" in tokens:
        opening = tokens.index("(")
        path = tokens[:opening]
        arguments = list(attribute_arguments(tokens[opening + 1:-1]))
    else:
        path, arguments = tokens, []
    name = path[-1].removeprefix("r#") if path else ""
    if name == "cfg_attr":
        return any(incompatible_test_attribute(arg) for arg in arguments[1:])
    if name == "wasm_bindgen_test":
        return ["unsupported", "=", "test"] not in arguments
    return name == "test"


def incompatible_test_lines(code):
    tokens = iter(rust_tokens(code))
    for token, start in tokens:
        if token != "#" or next(tokens, (None, None))[0] != "[":
            continue
        attribute = []
        depth = 1
        for token, _ in tokens:
            depth += (token == "[") - (token == "]")
            if depth == 0:
                break
            attribute.append(token)
        if incompatible_test_attribute(attribute):
            yield code.count("\n", 0, start) + 1


class TestAttributeCases(unittest.TestCase):
    def test_rejects_single_target_attributes(self):
        for code in (
            "#[test]",
            "#[/* comment */ test /* comment */]",
            '#[cfg_attr(not(target_arch = "wasm32"), test)]',
            "#[cfg_attr(unix, cfg_attr(test, test))]",
            "#[cfg_attr(unix, wasm_bindgen_test::wasm_bindgen_test)]",
            "#[wasm_bindgen_test]",
            "#[wasm_bindgen_test::wasm_bindgen_test]",
            "#[wasm_bindgen_test(unsupported = ignore)]",
        ):
            with self.subTest(code=code):
                self.assertEqual(list(incompatible_test_lines(code)), [1])

    def test_accepts_dual_target_attributes_and_examples(self):
        for code in (
            "#[wasm_bindgen_test(unsupported = test)]",
            "#[wasm_bindgen_test::wasm_bindgen_test(unsupported /* comment */ = test)]",
            '#[cfg_attr(test, wasm_bindgen_test(unsupported = test))]',
            "/* Outer /* inner */ #[test] */",
            'const EXAMPLE: &str = r#"#[test]"#;',
            "// #[test]",
        ):
            with self.subTest(code=code):
                self.assertEqual(list(incompatible_test_lines(code)), [])

    def test_compares_test_names_per_binary(self):
        host = {("schema", "lib/schema"): {"tests::shared", "tests::host"}}
        wasm = {("schema", "lib/schema"): {"tests::shared", "tests::wasm"}}
        self.assertEqual(test_name_differences(host, host), [])
        self.assertEqual(test_name_differences(host, wasm), [
            "schema (lib/schema): host only: tests::host",
            "schema (lib/schema): Wasm only: tests::wasm",
        ])

    def test_rejects_missing_test_binaries_on_either_target(self):
        schema = {("schema", "lib/schema"): {"tests::shared"}}
        freshness = {("freshness", "lib/freshness"): {"tests::shared"}}
        self.assertEqual(test_listing_errors({"schema", "freshness"}, schema, freshness), [
            "freshness: no test binary listed on host",
            "schema: no test binary listed on Wasm",
        ])
        self.assertEqual(test_listing_errors({"schema"}, {}, {}), [
            "schema: no test binary listed on host",
            "schema: no test binary listed on Wasm",
            "The host runner listed no shared tests",
        ])

    def test_rejects_empty_test_names_even_when_binaries_are_listed(self):
        listing = {("schema", "lib/schema"): set()}
        self.assertEqual(test_listing_errors({"schema"}, listing, listing), [
            "The host runner listed no shared tests",
        ])

    def test_accepts_an_empty_binary_when_shared_tests_are_listed(self):
        listing = {("schema", "lib/schema"): {"tests::shared"}, ("other", "lib/other"): set()}
        self.assertEqual(test_listing_errors({"schema", "other"}, listing, listing), [])


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

    def test_workspace_members_use_root_clippy_config(self):
        for package in self.members:
            crate_dir = Path(package["manifest_path"]).parent
            for directory in (crate_dir, *crate_dir.parents):
                if directory == ROOT:
                    break
                for name in ("clippy.toml", ".clippy.toml"):
                    config = directory / name
                    with self.subTest(config=config):
                        self.assertFalse(config.exists(), "Use the workspace Clippy config")

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
                    for line in incompatible_test_lines(source.read_text(encoding="utf-8")):
                        with self.subTest(source=source.relative_to(ROOT), line=line):
                            self.fail("Use #[wasm_bindgen_test(unsupported = test)] for shared tests")
