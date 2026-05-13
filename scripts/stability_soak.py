#!/usr/bin/env python3
"""Drive a bounded Studio daemon soak and record health/memory metrics."""

from __future__ import annotations

import argparse
import csv
import json
import os
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path


def request_json(method: str, url: str, payload: object | None = None) -> object:
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={"content-type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.loads(response.read().decode("utf-8"))


def read_process_metric(pid: int, command: list[str]) -> str:
    try:
        completed = subprocess.run(
            command,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        return completed.stdout.strip()
    except OSError:
        return ""


def rss_kib(pid: int) -> int | None:
    out = read_process_metric(pid, ["ps", "-o", "rss=", "-p", str(pid)])
    try:
        return int(out.strip())
    except ValueError:
        return None


def fd_count(pid: int) -> int | None:
    out = read_process_metric(pid, ["lsof", "-p", str(pid)])
    if not out:
        return None
    return max(0, len(out.splitlines()) - 1)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--addr", default="127.0.0.1:7732")
    parser.add_argument(
        "--root",
        default=None,
        help="Workspace root. Defaults to target/stress-pass/stability-soak-workspace.",
    )
    parser.add_argument("--duration-seconds", type=int, default=30 * 60)
    parser.add_argument("--poll-seconds", type=float, default=5.0)
    parser.add_argument(
        "--out-dir",
        default=None,
        help="Output directory. Defaults to target/stress-pass/stability-soak/<timestamp>.",
    )
    parser.add_argument(
        "--external-daemon",
        action="store_true",
        help="Do not spawn cargo; use an already-running daemon at --addr.",
    )
    parser.add_argument(
        "--real-toolchain",
        action="store_true",
        help="Spawn a raw daemon and rely on the local x07/codex/claude toolchain.",
    )
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[1]
    started = time.strftime("%Y%m%d-%H%M%S")
    out_dir = Path(args.out_dir) if args.out_dir else repo / "target" / "stress-pass" / "stability-soak" / started
    out_dir.mkdir(parents=True, exist_ok=True)
    workspace = Path(args.root) if args.root else repo / "target" / "stress-pass" / "stability-soak-workspace"
    workspace.mkdir(parents=True, exist_ok=True)

    daemon: subprocess.Popen[bytes] | None = None
    daemon_log = None
    if args.external_daemon:
        pid = os.getpid()
    else:
        daemon_log_path = out_dir / "daemon.log"
        daemon_log = daemon_log_path.open("wb")
        command = [
            "cargo",
            "run",
            "-p",
            "loom-daemon",
            "--",
            "serve",
            "--root",
            str(workspace),
            "--addr",
            args.addr,
        ]
        if not args.real_toolchain:
            try:
                workspace_arg = str(workspace.relative_to(repo))
            except ValueError as error:
                raise SystemExit("--root must stay under the repo when using the deterministic connected toolchain") from error
            command = [
                sys.executable,
                str(repo / "scripts" / "serve_connected_e2e_daemon.py"),
                "--workspace",
                workspace_arg,
                "--bin-dir",
                "target/stress-pass/stability-soak-bin",
                "--addr",
                args.addr,
            ]
        daemon = subprocess.Popen(
            command,
            cwd=repo,
            stdout=daemon_log,
            stderr=subprocess.STDOUT,
        )
        pid = daemon.pid

    base = f"http://{args.addr}"
    deadline = time.time() + args.duration_seconds
    rows: list[dict[str, object]] = []
    sessions_started = 0
    failures: list[str] = []

    try:
        for _ in range(60):
            try:
                request_json("GET", f"{base}/v1/health")
                break
            except (urllib.error.URLError, TimeoutError):
                if daemon is not None and daemon.poll() is not None:
                    raise RuntimeError(f"daemon exited before health check; see {daemon_log_path}")
                time.sleep(1)
        else:
            raise RuntimeError("daemon did not become healthy")

        while time.time() < deadline:
            try:
                title = f"stability sorter {sessions_started}"
                session = request_json(
                    "POST",
                    f"{base}/v1/sessions",
                    {"intent_text": title, "mode": "new_behavior"},
                )
                session_id = session["session_id"]
                request_json(
                    "POST",
                    f"{base}/v1/sessions/{session_id}/intent/formalize",
                    {
                        "raw": title,
                        "input_mode": "text",
                        "revision_notes": [],
                        "provider_profile_id": None,
                        "voice_transcript": None,
                    },
                )
                request_json(
                    "POST",
                    f"{base}/v1/sessions/{session_id}/events",
                    {"event": {"event": "draft_spec"}},
                )
                request_json(
                    "POST",
                    f"{base}/v1/sessions/{session_id}/events",
                    {"event": {"event": "approve_spec"}},
                )
                request_json(
                    "POST",
                    f"{base}/v1/sessions/{session_id}/build",
                    {"vars": {}, "max_repair_rounds": 1},
                )
                sessions_started += 1
            except Exception as error:  # noqa: BLE001 - recorded in soak report
                failures.append(str(error))

            health = request_json("GET", f"{base}/v1/health")
            snapshot = request_json("GET", f"{base}/v1/health/snapshot")
            sessions = request_json("GET", f"{base}/v1/sessions")
            op_log_size = sum(len(item.get("op_log", [])) for item in sessions)
            rows.append(
                {
                    "unix_ms": int(time.time() * 1000),
                    "rss_kib": rss_kib(pid),
                    "fd_count": fd_count(pid),
                    "active_sessions": health.get("active_sessions", snapshot.get("active_sessions", 0)),
                    "subscriber_count": health.get("subscriber_count", snapshot.get("subscriber_count", 0)),
                    "op_log_size": op_log_size,
                    "sessions_started": sessions_started,
                    "failures": len(failures),
                }
            )
            time.sleep(args.poll_seconds)
    finally:
        if daemon is not None:
            daemon.send_signal(signal.SIGINT)
            try:
                daemon.wait(timeout=10)
            except subprocess.TimeoutExpired:
                daemon.kill()
        if daemon_log is not None:
            daemon_log.close()

    metrics_path = out_dir / "metrics.csv"
    with metrics_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            [
                "unix_ms",
                "rss_kib",
                "fd_count",
                "active_sessions",
                "subscriber_count",
                "op_log_size",
                "sessions_started",
                "failures",
            ],
        )
        writer.writeheader()
        writer.writerows(rows)

    peak_rss = max((row["rss_kib"] or 0 for row in rows), default=0)
    summary = {
        "schema_version": "x07.studio.stability_soak@0.1.0",
        "duration_seconds": args.duration_seconds,
        "workspace": str(workspace),
        "metrics_csv": str(metrics_path),
        "sessions_started": sessions_started,
        "peak_rss_kib": peak_rss,
        "peak_rss_under_200mb": peak_rss < 200 * 1024,
        "failures": failures,
        "status": "pass" if not failures and peak_rss < 200 * 1024 else "fail",
    }
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0 if summary["status"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
