#!/usr/bin/env python3
"""Run the connected Studio XTAL smoke across Playwright browser engines."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--browser",
        action="append",
        choices=("chromium", "firefox", "webkit"),
        help="Limit the run to one or more browser projects.",
    )
    parser.add_argument(
        "--report",
        default="target/stress-pass/cross-browser-smoke/summary.json",
        help="Summary JSON path, relative to the x07-studio repo.",
    )
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[1]
    web = repo / "web"
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)

    command = [
        "npx",
        "playwright",
        "test",
        "--config",
        "playwright.cross-browser.config.ts",
    ]
    for browser in args.browser or []:
        command.extend(["--project", browser])

    env = os.environ.copy()
    env.pop("NO_COLOR", None)
    completed = subprocess.run(command, cwd=web, env=env)
    summary = {
        "schema_version": "x07.studio.cross_browser_smoke@0.1.0",
        "command": command,
        "status": "pass" if completed.returncode == 0 else "fail",
        "exit_code": completed.returncode,
        "browsers": args.browser or ["chromium", "firefox", "webkit"],
    }
    report_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {report_path}")
    return completed.returncode


if __name__ == "__main__":
    sys.exit(main())
