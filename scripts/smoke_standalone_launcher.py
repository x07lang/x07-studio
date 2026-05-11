#!/usr/bin/env python3
"""Smoke-test a packaged x07 Studio web launcher."""

from __future__ import annotations

import argparse
import json
import queue
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
from pathlib import Path


DEFAULT_DAEMON_ADDR = "127.0.0.1:7719"
DEFAULT_WEB_ADDR = "127.0.0.1:7720"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, help="Bundle root to launch.")
    parser.add_argument("--dist-dir", type=Path, default=Path("dist/standalone"))
    parser.add_argument("--timeout-seconds", type=float, default=20.0)
    args = parser.parse_args()

    bundle = args.bundle.resolve() if args.bundle else single_bundle_dir(args.dist_dir).resolve()
    defaults = read_env(bundle / "defaults.env")
    daemon_default = defaults.get("X07_STUDIO_DAEMON_ADDR", DEFAULT_DAEMON_ADDR)
    web_default = defaults.get("X07_STUDIO_WEB_ADDR", DEFAULT_WEB_ADDR)

    with tempfile.TemporaryDirectory(prefix="x07-studio-launch-") as workspace:
        with occupied_addr(daemon_default), occupied_addr(web_default):
            process = launch(bundle, Path(workspace))
            try:
                launch_info = read_launch_info(process, args.timeout_seconds)
                verify_launch(launch_info, daemon_default, web_default, args.timeout_seconds)
            finally:
                stop_process(process)

    print(
        json.dumps(
            {
                "ok": True,
                "bundle": str(bundle),
                "occupied_defaults": [daemon_default, web_default],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


def single_bundle_dir(dist_dir: Path) -> Path:
    candidates = sorted(path for path in dist_dir.glob("x07-studio-*") if path.is_dir())
    if len(candidates) != 1:
        raise SystemExit(f"expected one bundle directory under {dist_dir}, found {len(candidates)}")
    return candidates[0]


def read_env(path: Path) -> dict[str, str]:
    entries: dict[str, str] = {}
    if not path.is_file():
        return entries
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, value = stripped.split("=", 1)
        entries[key.strip()] = value.strip().strip('"')
    return entries


class occupied_addr:
    def __init__(self, addr: str) -> None:
        self.addr = addr
        self.socket: socket.socket | None = None

    def __enter__(self) -> None:
        host, port = split_addr(self.addr)
        probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        try:
            probe.bind((host, port))
            probe.listen()
            self.socket = probe
        except OSError:
            probe.close()

    def __exit__(self, _exc_type, _exc_value, _traceback) -> None:
        if self.socket is not None:
            self.socket.close()


def split_addr(addr: str) -> tuple[str, int]:
    host, port = addr.rsplit(":", 1)
    return host, int(port)


def launch(bundle: Path, workspace: Path) -> subprocess.Popen:
    script = bundle / "scripts" / "launch_studio_web.py"
    if not script.is_file():
        raise SystemExit(f"launcher script is missing: {script}")
    command = [
        sys.executable,
        str(script),
        "--bundle-root",
        str(bundle),
        "--root",
        str(workspace),
        "--no-open",
        "--no-install-missing",
    ]
    creationflags = 0
    if sys.platform == "win32":
        creationflags = subprocess.CREATE_NEW_PROCESS_GROUP
    return subprocess.Popen(
        command,
        cwd=bundle,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=1,
        creationflags=creationflags,
    )


def read_launch_info(process: subprocess.Popen, timeout_seconds: float) -> dict[str, object]:
    assert process.stdout is not None
    lines: queue.Queue[str | None] = queue.Queue()
    reader = threading.Thread(target=read_lines, args=(process.stdout, lines), daemon=True)
    reader.start()

    captured: list[str] = []
    json_lines: list[str] = []
    balance = 0
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        try:
            line = lines.get(timeout=0.1)
        except queue.Empty:
            if process.poll() is not None:
                break
            continue
        if line is None:
            break
        captured.append(line)
        if json_lines or line.lstrip().startswith("{"):
            json_lines.append(line)
            balance += line.count("{") - line.count("}")
            if balance == 0:
                try:
                    parsed = json.loads("".join(json_lines))
                except json.JSONDecodeError as error:
                    raise SystemExit(f"launcher emitted invalid JSON: {error}") from error
                if not isinstance(parsed, dict):
                    raise SystemExit("launcher JSON is not an object")
                return parsed

    stderr = read_remaining(process.stderr) if process.poll() is not None else ""
    stdout = "".join(captured).strip()
    raise SystemExit(
        "launcher did not emit startup JSON before timeout"
        f"\nstdout:\n{stdout}"
        f"\nstderr:\n{stderr.strip()}"
    )


def read_lines(stream, lines: queue.Queue[str | None]) -> None:
    for line in stream:
        lines.put(line)
    lines.put(None)


def read_remaining(stream) -> str:
    if stream is None:
        return ""
    try:
        return stream.read()
    except ValueError:
        return ""


def verify_launch(
    launch_info: dict[str, object],
    daemon_default: str,
    web_default: str,
    timeout_seconds: float,
) -> None:
    studio_url = require_string(launch_info, "studio_url")
    daemon_addr = require_string(launch_info, "daemon_addr")
    web_addr = require_string(launch_info, "web_addr")
    if daemon_addr == daemon_default:
        raise SystemExit(f"daemon did not move off occupied default address {daemon_default}")
    if web_addr == web_default:
        raise SystemExit(f"web server did not move off occupied default address {web_default}")
    if studio_url != f"http://{web_addr}":
        raise SystemExit(f"studio_url {studio_url} does not match web_addr {web_addr}")

    index = fetch(studio_url, timeout_seconds)
    if b"<!doctype html" not in index.lower():
        raise SystemExit("standalone web server did not serve the built Studio app")

    health = json.loads(fetch(f"{studio_url}/v1/health", timeout_seconds).decode("utf-8"))
    defaults = health.get("defaults")
    if not isinstance(defaults, dict):
        raise SystemExit("daemon health response did not include defaults")
    if defaults.get("daemon_addr") != daemon_addr:
        raise SystemExit(
            f"daemon health reported {defaults.get('daemon_addr')!r}, expected {daemon_addr!r}"
        )


def require_string(data: dict[str, object], key: str) -> str:
    value = data.get(key)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"launcher JSON missing string field {key}")
    return value


def fetch(url: str, timeout_seconds: float) -> bytes:
    deadline = time.monotonic() + timeout_seconds
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=1.0) as response:
                return response.read()
        except OSError as error:
            last_error = error
            time.sleep(0.1)
    raise SystemExit(f"timed out fetching {url}: {last_error}")


def stop_process(process: subprocess.Popen) -> None:
    if process.poll() is not None:
        return
    if sys.platform == "win32":
        process.send_signal(signal.CTRL_BREAK_EVENT)
    else:
        process.send_signal(signal.SIGINT)
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


if __name__ == "__main__":
    raise SystemExit(main())
