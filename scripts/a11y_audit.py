#!/usr/bin/env python3
"""Run the connected Studio axe accessibility audit."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--report",
        help="Report JSON path, relative to the x07-studio repo.",
    )
    parser.add_argument("--project", default="chromium", help="Playwright project to audit.")
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[1]
    web = repo / "web"
    report = repo / (
        args.report
        or f"target/a11y/{time.strftime('%Y%m%d-%H%M%S')}/a11y-report.json"
    )
    report.parent.mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    env["X07_STUDIO_A11Y_AUDIT_OUT"] = str(report)
    env.pop("NO_COLOR", None)
    command = [
        "npx",
        "playwright",
        "test",
        "tests-connected/connected-zx-a11y-audit.spec.ts",
        "--config",
        "playwright.connected.config.ts",
        "--project",
        args.project,
    ]
    completed = subprocess.run(command, cwd=web, env=env)
    if not report.exists():
        summary = {
            "schema_version": "x07.studio.a11y_audit@0.1.0",
            "captured_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "passed": False,
            "command": command,
            "exit_code": completed.returncode,
            "error": "Playwright did not write an accessibility report.",
        }
        report.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {report}")
    if completed.returncode != 0:
        return completed.returncode
    summary = json.loads(report.read_text(encoding="utf-8"))
    return 0 if summary.get("passed") is True else 1


if __name__ == "__main__":
    sys.exit(main())
