#!/usr/bin/env python3
"""Render a local Studio SLO snapshot from metrics and opt-in error reports."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from urllib.error import URLError
from urllib.request import urlopen


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="Workspace root that owns .loom/error-ring.jsonl.")
    parser.add_argument("--addr", default="http://127.0.0.1:7719", help="Daemon base URL.")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    errors = read_jsonl(root / ".loom" / "error-ring.jsonl")
    metrics = read_metrics(args.addr.rstrip("/") + "/v1/metrics")
    rows = [
        ("active sessions", metrics.get("loom_daemon_active_sessions", "n/a")),
        ("sse subscribers", metrics.get("loom_daemon_sse_subscribers", "n/a")),
        ("session summaries", metrics.get("loom_telemetry_session_summaries_total", "n/a")),
        ("error ring entries", str(len(errors))),
    ]
    print("x07 Studio SLO")
    print("=" * 14)
    for label, value in rows:
        print(f"{label:18} {value}")
    if errors:
        print("\nlatest errors")
        for item in errors[-5:]:
            source = item.get("source", "unknown")
            severity = item.get("severity", "error")
            message = item.get("message", "")
            print(f"- {severity:<7} {source:<8} {message}")
    return 0


def read_jsonl(path: Path) -> list[dict[str, object]]:
    if not path.exists():
        return []
    rows: list[dict[str, object]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            rows.append(value)
    return rows


def read_metrics(url: str) -> dict[str, str]:
    try:
        with urlopen(url, timeout=2) as response:
            raw = response.read().decode("utf-8", errors="replace")
    except (OSError, URLError):
        return {}
    metrics: dict[str, str] = {}
    for line in raw.splitlines():
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 1)
        if len(parts) == 2:
            metrics[parts[0]] = parts[1]
    return metrics


if __name__ == "__main__":
    sys.exit(main())
