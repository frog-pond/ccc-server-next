"""Read Cargo workspace metadata and run the shared crates' Wasm checks."""

import json
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[1]
WASM_TARGET = "wasm32-unknown-unknown"


def workspace_metadata():
    return json.loads(subprocess.check_output(
        ["cargo", "metadata", "--offline", "--locked", "--no-deps", "--format-version=1"],
        cwd=ROOT,
    ))


def workspace_members(metadata):
    members = set(metadata["workspace_members"])
    return [package for package in metadata["packages"] if package["id"] in members]


def wasm_exclusions(metadata):
    return metadata["metadata"]["wasm"]["exclude"]


def check_wasm():
    args = ["--target", WASM_TARGET]
    for crate in wasm_exclusions(workspace_metadata()):
        args.extend(("--exclude", crate))
    subprocess.run(
        ["cargo", "build", "--workspace", "--all-features", "--locked", *args],
        cwd=ROOT,
        check=True,
    )
    for task in ("clippy", "doc", "test"):
        subprocess.run(["mise", "run", task, "--", *args], cwd=ROOT, check=True)


if __name__ == "__main__":
    check_wasm()
