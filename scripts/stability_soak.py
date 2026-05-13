#!/usr/bin/env python3
"""Drive a bounded Studio daemon soak and record health/memory metrics.

The default workload matches the public-beta A7 gate: one formalized session
with repeated autopilot starts while health, RSS, FD, op-log, and subscriber
metrics are sampled.
"""

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


def request_json(
    method: str,
    url: str,
    payload: object | None = None,
    *,
    timeout_seconds: float = 10,
) -> object:
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=data,
        method=method,
        headers={"content-type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
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


def create_approved_session(base: str, title: str) -> tuple[str, int]:
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
    return session_id, 1


def collect_metrics(
    base: str,
    pid: int,
    session_id: str | None,
    sessions_started: int,
    autopilot_runs: int,
    failures: list[str],
) -> dict[str, object]:
    health = request_json("GET", f"{base}/v1/health")
    snapshot = request_json("GET", f"{base}/v1/health/snapshot")
    sessions = request_json("GET", f"{base}/v1/sessions")
    total_op_log_size = sum(len(item.get("op_log", [])) for item in sessions)
    session_op_log_size = 0
    if session_id:
        for item in sessions:
            if item.get("session_id") == session_id:
                session_op_log_size = len(item.get("op_log", []))
                break
    return {
        "unix_ms": int(time.time() * 1000),
        "rss_kib": rss_kib(pid),
        "fd_count": fd_count(pid),
        "active_sessions": health.get("active_sessions", snapshot.get("active_sessions", 0)),
        "subscriber_count": health.get("subscriber_count", snapshot.get("subscriber_count", 0)),
        "op_log_size": session_op_log_size or total_op_log_size,
        "total_op_log_size": total_op_log_size,
        "sessions_started": sessions_started,
        "autopilot_runs": autopilot_runs,
        "failures": len(failures),
    }


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
    parser.add_argument(
        "--workload",
        choices=("autopilot-single-session", "build-loop"),
        default="autopilot-single-session",
        help="Soak workload. Defaults to the A7 single-session autopilot path.",
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
    autopilot_runs = 0
    session_id: str | None = None
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

        if args.workload == "autopilot-single-session":
            session_id, sessions_started = create_approved_session(base, "stability autopilot sorter")

        while time.time() < deadline:
            try:
                if args.workload == "autopilot-single-session":
                    request_json(
                        "POST",
                        f"{base}/v1/sessions/{session_id}/autopilot/start",
                        {
                            "policy": {
                                "auto_answer_min_confidence": 0.7,
                                "max_clarify_rounds": 3,
                                "auto_climb_to": None,
                                "allow_repair_iters": 1,
                                "allow_quorum": False,
                            }
                        },
                        timeout_seconds=120,
                    )
                    autopilot_runs += 1
                else:
                    title = f"stability sorter {sessions_started}"
                    session_id, created = create_approved_session(base, title)
                    sessions_started += created
                    request_json(
                        "POST",
                        f"{base}/v1/sessions/{session_id}/build",
                        {"vars": {}, "max_repair_rounds": 1},
                        timeout_seconds=120,
                    )
            except Exception as error:  # noqa: BLE001 - recorded in soak report
                failures.append(str(error))

            rows.append(
                collect_metrics(
                    base,
                    pid,
                    session_id,
                    sessions_started,
                    autopilot_runs,
                    failures,
                )
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
                "total_op_log_size",
                "sessions_started",
                "autopilot_runs",
                "failures",
            ],
        )
        writer.writeheader()
        writer.writerows(rows)

    peak_rss = max((row["rss_kib"] or 0 for row in rows), default=0)
    max_op_log_size = max((int(row["op_log_size"] or 0) for row in rows), default=0)
    max_active_sessions = max((int(row["active_sessions"] or 0) for row in rows), default=0)
    initial_subscribers = int(rows[0]["subscriber_count"] or 0) if rows else 0
    max_subscribers = max((int(row["subscriber_count"] or 0) for row in rows), default=0)
    final_subscribers = int(rows[-1]["subscriber_count"] or 0) if rows else 0
    op_log_under_10k = max_op_log_size < 10_000
    summary = {
        "schema_version": "x07.studio.stability_soak@0.1.0",
        "workload": args.workload,
        "duration_seconds": args.duration_seconds,
        "workspace": str(workspace),
        "session_id": session_id,
        "metrics_csv": str(metrics_path),
        "sessions_started": sessions_started,
        "autopilot_runs": autopilot_runs,
        "peak_rss_kib": peak_rss,
        "peak_rss_under_200mb": peak_rss < 200 * 1024,
        "max_active_sessions": max_active_sessions,
        "max_op_log_size": max_op_log_size,
        "op_log_under_10000": op_log_under_10k,
        "initial_subscriber_count": initial_subscribers,
        "max_subscriber_count": max_subscribers,
        "final_subscriber_count": final_subscribers,
        "subscriber_count_stable": final_subscribers <= initial_subscribers,
        "failures": failures,
        "status": "pass" if not failures and peak_rss < 200 * 1024 and op_log_under_10k else "fail",
    }
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0 if summary["status"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
