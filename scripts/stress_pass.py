#!/usr/bin/env python3
"""Cycle 7 stress-pass evidence recorder.

This is a *recorder*, not a full orchestrator. Boot the real daemon + Vite
yourself (or via --boot), drive a scenario through the UI (chrome-devtools
MCP, Playwright, or by hand), and call this script to snapshot session
state into a recording bundle.

Bundle layout:

    target/stress-pass/<scenario_id>/
    ├── README.md
    ├── op-log.json
    ├── turns.json
    ├── posture-snapshots.jsonl
    ├── ladder.json
    ├── process-lane.json
    ├── env.json                  # toolchain versions + commit hash + workspace path
    ├── transcripts/              # populated by --capture-transcripts
    │   ├── claude.txt
    │   ├── codex.txt
    │   └── x07-cli.jsonl
    └── screenshots/              # caller-populated

Usage:

    # one-shot snapshot of the current session
    python3 scripts/stress_pass.py snapshot \\
        --scenario scenario-1-text-utils \\
        --daemon http://127.0.0.1:7747 \\
        --session-id <uuid>

    # print latest session id (handy when piping into snapshot)
    python3 scripts/stress_pass.py latest-session --daemon http://127.0.0.1:7747

    # init a bundle dir + README from a scenario config
    python3 scripts/stress_pass.py init --scenario scenario-1-text-utils

The recorder is deliberately small: no daemon lifecycle management, no
browser driving. Each scenario's README documents the manual steps the
operator takes between snapshot calls so the bundle stays reproducible.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
BUNDLE_ROOT_DEFAULT = REPO_ROOT / "target" / "stress-pass"
SCENARIOS_DIR = REPO_ROOT / "scripts" / "scenarios"


def fetch_json(url: str) -> Any:
    with urllib.request.urlopen(url, timeout=15) as response:
        return json.loads(response.read())


def safe_fetch(url: str) -> Any | None:
    try:
        return fetch_json(url)
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as err:
        return {"__error__": str(err), "__url__": url}


def run(cmd: list[str]) -> tuple[int, str]:
    try:
        result = subprocess.run(
            cmd,
            check=False,
            capture_output=True,
            text=True,
            timeout=15,
        )
        return result.returncode, (result.stdout or result.stderr).strip()
    except FileNotFoundError:
        return 127, "<not found>"
    except subprocess.TimeoutExpired:
        return 124, "<timeout>"


def collect_env() -> dict[str, Any]:
    versions: dict[str, str] = {}
    for name, args in [
        ("x07", ["x07", "--version"]),
        ("x07_wasm", ["x07-wasm", "--version"]),
        ("x07lp", ["x07lp", "--version"]),
        ("claude", ["claude", "--version"]),
        ("codex", ["codex", "--version"]),
    ]:
        code, output = run(args)
        versions[name] = output if code == 0 else f"<unavailable: code={code}>"

    rev_code, head = run(["git", "rev-parse", "--short", "HEAD"])
    branch_code, branch = run(["git", "rev-parse", "--abbrev-ref", "HEAD"])
    dirty_code, dirty = run(["git", "status", "--porcelain"])

    return {
        "captured_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "versions": versions,
        "git": {
            "head": head if rev_code == 0 else "<unknown>",
            "branch": branch if branch_code == 0 else "<unknown>",
            "dirty": bool(dirty.strip()) if dirty_code == 0 else None,
        },
    }


def init_bundle(scenario_id: str, bundle_root: Path) -> Path:
    bundle = bundle_root / scenario_id
    (bundle / "transcripts").mkdir(parents=True, exist_ok=True)
    (bundle / "screenshots").mkdir(parents=True, exist_ok=True)

    scenario_path = SCENARIOS_DIR / f"{scenario_id}.json"
    scenario: dict[str, Any] = {}
    if scenario_path.exists():
        scenario = json.loads(scenario_path.read_text(encoding="utf-8"))
        shutil.copy(scenario_path, bundle / "scenario.json")
        fixture_rel = scenario.get("workspace_fixture")
        if fixture_rel:
            fixture = (REPO_ROOT / fixture_rel).resolve()
            if not fixture.is_relative_to(REPO_ROOT):
                raise SystemExit(f"workspace_fixture escapes repo: {fixture_rel}")
            if not fixture.is_dir():
                raise SystemExit(f"workspace_fixture not found: {fixture_rel}")
            workspace = bundle / "workspace"
            if not workspace.exists():
                shutil.copytree(fixture, workspace)

    readme = bundle / "README.md"
    if not readme.exists():
        workspace_hint = (
            f"`{bundle / 'workspace'}`"
            if (bundle / "workspace").exists()
            else "`<fresh workspace>`"
        )
        lines = [
            f"# Stress-pass bundle — {scenario_id}",
            "",
            f"Goal: **{scenario.get('goal', 'TBD')}**",
            "",
            "Expected canonical flow:",
        ]
        for step in scenario.get("expected_flow", []):
            lines.append(f"- {step}")
        lines.extend([
            "",
            "## Operator steps",
            f"1. Boot daemon with workspace {workspace_hint}: `python3 scripts/launch_studio_web.py --workspace <path>` or `cargo run -p loom-daemon -- serve --root <path>`.",
            "2. Open the UI; pick the configured recipe.",
            "3. After each phase transition (intent / build / verify / review / pause), run:",
            "",
            "       python3 scripts/stress_pass.py snapshot --scenario " + scenario_id + " --daemon <origin> --session-id <uuid>",
            "",
            "4. Drop screenshots into `screenshots/`; transcripts (claude/codex stdout) into `transcripts/`.",
            "5. Append observations to `breakages.md`.",
            "",
            "## Observed",
            "_filled by the operator_",
        ])
        readme.write_text("\n".join(lines) + "\n", encoding="utf-8")

    breakages = bundle / "breakages.md"
    if not breakages.exists():
        breakages.write_text(
            "# Breakages\n\n"
            "_One section per real-vs-fake divergence or real-toolchain failure._\n\n",
            encoding="utf-8",
        )
    return bundle


def snapshot(bundle: Path, daemon: str, session_id: str) -> dict[str, Any]:
    summary: dict[str, Any] = {"daemon": daemon, "session_id": session_id}

    op_log = safe_fetch(f"{daemon}/v1/sessions/{session_id}")
    (bundle / "op-log.json").write_text(json.dumps(op_log, indent=2), encoding="utf-8")
    summary["op_count"] = (
        len((op_log or {}).get("op_log") or []) if isinstance(op_log, dict) else "<error>"
    )

    turns = safe_fetch(f"{daemon}/v1/sessions/{session_id}/turns")
    (bundle / "turns.json").write_text(json.dumps(turns, indent=2), encoding="utf-8")
    summary["turn_count"] = len(turns) if isinstance(turns, list) else "<error>"

    posture = safe_fetch(f"{daemon}/v1/sessions/{session_id}/trust/posture")
    posture_line = json.dumps({"at": time.time(), "posture": posture})
    with (bundle / "posture-snapshots.jsonl").open("a", encoding="utf-8") as f:
        f.write(posture_line + "\n")
    summary["posture_color"] = (
        (posture or {}).get("posture_color") if isinstance(posture, dict) else "<error>"
    )

    ladder = safe_fetch(f"{daemon}/v1/sessions/{session_id}/ladder")
    (bundle / "ladder.json").write_text(json.dumps(ladder, indent=2), encoding="utf-8")
    summary["current_rung"] = (
        (ladder or {}).get("current_rung") if isinstance(ladder, dict) else "<error>"
    )

    process_lane = safe_fetch(f"{daemon}/v1/sessions/{session_id}/process-lane")
    (bundle / "process-lane.json").write_text(
        json.dumps(process_lane, indent=2), encoding="utf-8"
    )
    if isinstance(process_lane, dict):
        current = process_lane.get("current_index")
        steps = process_lane.get("steps") or []
        if isinstance(current, int) and 0 <= current < len(steps):
            summary["current_step"] = steps[current].get("id")
        else:
            summary["current_step"] = None

    env_path = bundle / "env.json"
    if not env_path.exists():
        env_path.write_text(json.dumps(collect_env(), indent=2), encoding="utf-8")

    return summary


def latest_session(daemon: str) -> str:
    sessions = fetch_json(f"{daemon}/v1/sessions")
    if not sessions:
        sys.exit("No sessions yet.")
    return sessions[-1]["session_id"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)

    init = sub.add_parser("init", help="initialize a bundle directory + README from a scenario config")
    init.add_argument("--scenario", required=True)
    init.add_argument("--bundle-root", default=str(BUNDLE_ROOT_DEFAULT))

    snap = sub.add_parser("snapshot", help="snapshot current session state into the bundle")
    snap.add_argument("--scenario", required=True)
    snap.add_argument("--daemon", required=True)
    snap.add_argument("--session-id", required=True)
    snap.add_argument("--bundle-root", default=str(BUNDLE_ROOT_DEFAULT))

    latest = sub.add_parser("latest-session", help="print the most recently created session id")
    latest.add_argument("--daemon", required=True)

    env_cmd = sub.add_parser("env", help="print toolchain + git env as JSON")

    args = parser.parse_args()

    if args.cmd == "init":
        bundle = init_bundle(args.scenario, Path(args.bundle_root))
        print(f"bundle: {bundle}")
        return 0

    if args.cmd == "snapshot":
        bundle = init_bundle(args.scenario, Path(args.bundle_root))
        summary = snapshot(bundle, args.daemon.rstrip("/"), args.session_id)
        print(json.dumps(summary, indent=2))
        return 0

    if args.cmd == "latest-session":
        print(latest_session(args.daemon.rstrip("/")))
        return 0

    if args.cmd == "env":
        print(json.dumps(collect_env(), indent=2))
        return 0

    parser.error("unknown command")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
