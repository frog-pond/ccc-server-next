"""Exercise reference drift reporting using two local commits."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ReferenceDrift(unittest.TestCase):
    def test_reports_copied_fixtures_and_generator_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            reference = root / "reference"
            reference.mkdir()
            env = {
                **os.environ,
                "CCC_REFERENCE_DIR": str(reference),
                "GIT_CONFIG_GLOBAL": os.devnull,
                "GIT_CONFIG_NOSYSTEM": "1",
            }

            def git(*args):
                return subprocess.run(
                    ["git", "-C", str(reference), *args],
                    env=env,
                    check=True,
                    capture_output=True,
                    text=True,
                ).stdout.strip()

            def write(file, contents):
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_text(contents)

            git("init")
            git("config", "user.name", "Fixture Test")
            git("config", "user.email", "fixtures@example.invalid")

            copied = "workers/test/fixtures/stolaf-posts.json"
            generator_input = "source/athletics/fixtures/calendar/2026-10-03-stolaf.json"
            conversion = "source/athletics/shape.ts"
            unrelated = "workers/src/unrelated.ts"
            for path in (copied, generator_input, conversion, unrelated):
                write(reference / path, path)
            git("add", ".")
            git("commit", "-m", "Pin")
            pin = git("rev-parse", "HEAD")

            (root / "tools").mkdir()
            for tool in ("reference-drift", "reference-path"):
                shutil.copy2(ROOT / "tools" / tool, root / "tools" / tool)

            fixtures = root / "fixtures/ccc-server"
            for path in (copied, generator_input):
                write(fixtures / path, path)
            shutil.copy2(
                ROOT / "fixtures/ccc-server/REFERENCE_CONVERSIONS",
                fixtures / "REFERENCE_CONVERSIONS",
            )
            write(root / "docs/reference/REFERENCE_COMMIT", pin + "\n")
            write(root / "docs/reference/README.md", unrelated)
            write(root / "docs/reference/routes.toml", "")

            git("mv", copied, "workers/test/fixtures/renamed.json")
            for path in (generator_input, conversion, unrelated):
                write(reference / path, "changed " + path)
            git("add", ".")
            git("commit", "-m", "Change reference")
            result = subprocess.run(
                [str(root / "tools/reference-drift")],
                env=env,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 1, result.stderr)
            _, _, report = result.stdout.partition("Changed conversion fixtures and generator modules:\n")
            self.assertEqual(
                report.split("\n\n", 1)[0].splitlines(),
                [f"  {path}" for path in sorted((copied, generator_input, conversion))],
            )
