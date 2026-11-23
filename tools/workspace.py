"""Read Cargo workspace metadata and run the shared crates' Wasm checks."""

import json
from pathlib import Path
import subprocess
import sys


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


def listed_tests(packages, exclusions, target_args, runner):
    artifacts = subprocess.check_output(
        ["cargo", "test", "--workspace", "--all-features", "--all-targets",
         "--locked", "--offline", "--no-run", "--message-format=json-render-diagnostics",
         *exclusions, *target_args],
        cwd=ROOT,
        text=True,
    )
    tests = {}
    for line in artifacts.splitlines():
        artifact = json.loads(line)
        if artifact["reason"] != "compiler-artifact" or not artifact["profile"]["test"]:
            continue
        if not artifact["executable"] or artifact["package_id"] not in packages:
            continue
        target = artifact["target"]
        kind = "/".join(target["kind"])
        binary = f'{packages[artifact["package_id"]]} ({kind}/{target["name"]})'
        output = subprocess.check_output(
            [*runner, artifact["executable"], "--list"], cwd=ROOT, text=True,
        )
        tests[binary] = {line.removesuffix(": test") for line in output.splitlines()
                         if line.endswith(": test")}
    return tests


def test_name_differences(host, wasm):
    differences = []
    for binary in sorted(host.keys() | wasm.keys()):
        host_tests = host.get(binary, set())
        wasm_tests = wasm.get(binary, set())
        differences.extend(f"{binary}: host only: {name}" for name in sorted(host_tests - wasm_tests))
        differences.extend(f"{binary}: Wasm only: {name}" for name in sorted(wasm_tests - host_tests))
    return differences


def check_wasm():
    metadata = workspace_metadata()
    excluded = wasm_exclusions(metadata)
    exclusions = [arg for crate in excluded for arg in ("--exclude", crate)]
    target_args = ["--target", WASM_TARGET]
    args = [*exclusions, *target_args]
    subprocess.run(
        ["cargo", "build", "--workspace", "--all-features", "--locked", *args],
        cwd=ROOT,
        check=True,
    )
    for task in ("clippy", "doc", "test"):
        subprocess.run(["mise", "run", task, "--", *args], cwd=ROOT, check=True)

    packages = {package["id"]: package["name"] for package in workspace_members(metadata)
                if package["name"] not in excluded}
    host = listed_tests(packages, exclusions, [], [])
    wasm = listed_tests(packages, exclusions, target_args, ["wasm-bindgen-test-runner"])
    differences = test_name_differences(host, wasm)
    if differences:
        sys.exit("Shared tests differ between the host and Wasm:\n" + "\n".join(differences))
    print(f"Host and Wasm list the same {sum(map(len, host.values()))} shared tests.")


if __name__ == "__main__":
    check_wasm()
