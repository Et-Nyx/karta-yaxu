#!/usr/bin/env python3
"""Build current native binaries and run isolated headless keyboard workflows."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ScreenshotResult(unittest.TextTestResult):
    def __init__(self, stream, descriptions, verbosity, artifacts):
        super().__init__(stream, descriptions, verbosity)
        self.artifacts = artifacts

    def capture_failure(self, test):
        page = getattr(test, "page", None)
        if page is None:
            return
        path = self.artifacts / f"{test.id()}.png"
        try:
            page.screenshot(path=str(path), timeout=5000)
            self.stream.writeln(f"Failure screenshot: {path.relative_to(ROOT)}")
        except Exception as error:
            self.stream.writeln(f"Screenshot unavailable ({type(error).__name__})")

    def addFailure(self, test, error):
        self.capture_failure(test)
        super().addFailure(test, error)

    def addError(self, test, error):
        self.capture_failure(test)
        super().addError(test, error)


def main():
    # Keep runs headless/bounded and avoid protocol logs containing session data.
    for key in ("PWDEBUG", "DEBUG"):
        os.environ.pop(key, None)
    try:
        import playwright.sync_api  # Fail before building if tools are missing.
    except ImportError:
        raise SystemExit(
            "Use: nix develop .#browser-tests --command python tests/browser/run.py"
        ) from None

    # Browser tests run native local binaries even if the parent shell cross-compiles.
    build_env = dict(os.environ)
    build_env.pop("CARGO_BUILD_TARGET", None)
    subprocess.run(
        ["cargo", "build", "--locked", "-p", "web-app", "--bins"],
        cwd=ROOT, env=build_env, check=True, timeout=600,
    )
    metadata = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=ROOT, env=build_env, check=True, capture_output=True, text=True, timeout=30,
    )
    os.environ["KARTA_BROWSER_BIN_DIR"] = str(
        Path(json.loads(metadata.stdout)["target_directory"]) / "debug"
    )
    artifact_root = ROOT / "target" / "browser-tests"
    artifact_root.mkdir(parents=True, exist_ok=True)
    artifacts = Path(tempfile.mkdtemp(prefix="run-", dir=artifact_root))
    suite = unittest.defaultTestLoader.discover(
        start_dir=str(Path(__file__).parent), pattern="test_*.py"
    )
    if suite.countTestCases() == 0:
        raise SystemExit("No browser tests were discovered")
    result = unittest.TextTestRunner(
        verbosity=2,
        resultclass=lambda stream, descriptions, verbosity: ScreenshotResult(
            stream, descriptions, verbosity, artifacts
        ),
    ).run(suite)
    if result.wasSuccessful():
        artifacts.rmdir()
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
