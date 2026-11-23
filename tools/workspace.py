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


def list_test_names(executable, runner, *flags):
    output = subprocess.check_output(
        [*runner, executable, "--list", *flags], cwd=ROOT, text=True,
    )
    return {line.removesuffix(": test") for line in output.splitlines() if line.endswith(": test")}


def listed_tests(packages, exclusions, target_args, runner):
    artifacts = subprocess.check_output(
        ["cargo", "test", "--workspace", "--all-features", "--all-targets",
         "--locked", "--offline", "--no-run", "--message-format=json-render-diagnostics",
         *exclusions, *target_args],
        cwd=ROOT,
        text=True,
    )
    tests = {}
    ignored = {}
    for line in artifacts.splitlines():
        artifact = json.loads(line)
        if artifact["reason"] != "compiler-artifact" or not artifact["profile"]["test"]:
            continue
        if not artifact["executable"] or artifact["package_id"] not in packages:
            continue
        target = artifact["target"]
        kind = "/".join(target["kind"])
        binary = (packages[artifact["package_id"]], f'{kind}/{target["name"]}')
        tests[binary] = list_test_names(artifact["executable"], runner)
        ignored[binary] = list_test_names(artifact["executable"], runner, "--ignored")
    return tests, ignored


def test_listing_errors(expected_packages, host, wasm):
    errors = []
    for platform, listing in (("host", host), ("Wasm", wasm)):
        listed_packages = {package for package, _ in listing}
        for package in sorted(expected_packages - listed_packages):
            errors.append(f"{package}: no test binary listed on {platform}")
    if not any(host.values()):
        errors.append("The host runner listed no shared tests")
    return errors


def test_name_differences(host, wasm):
    differences = []
    for binary in sorted(host.keys() | wasm.keys()):
        package, target = binary
        label = f"{package} ({target})"
        host_tests = host.get(binary, set())
        wasm_tests = wasm.get(binary, set())
        differences.extend(f"{label}: host only: {name}" for name in sorted(host_tests - wasm_tests))
        differences.extend(f"{label}: Wasm only: {name}" for name in sorted(wasm_tests - host_tests))
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

    shared = [package for package in workspace_members(metadata) if package["name"] not in excluded]
    packages = {package["id"]: package["name"] for package in shared}
    expected = {package["name"] for package in shared
                if any(target["test"] for target in package["targets"])}
    host, host_ignored = listed_tests(packages, exclusions, [], [])
    wasm, wasm_ignored = listed_tests(packages, exclusions, target_args, ["wasm-bindgen-test-runner"])
    errors = test_listing_errors(expected, host, wasm)
    errors.extend(test_name_differences(host, wasm))
    errors.extend(f"Ignored test mismatch: {difference}" for difference in test_name_differences(
        host_ignored, wasm_ignored,
    ))
    if errors:
        sys.exit("Shared test discovery check failed:\n" + "\n".join(errors))
    print(f"Host and Wasm agree on {sum(map(len, host.values()))} shared tests and which are ignored.")


if __name__ == "__main__":
    check_wasm()
