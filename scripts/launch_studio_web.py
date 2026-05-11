#!/usr/bin/env python3
"""Launch the packaged Studio web app with a local Loom daemon."""

from __future__ import annotations

import argparse
import http.server
import json
import os
import shutil
import socket
import socketserver
import subprocess
import sys
import time
import urllib.error
import urllib.request
import webbrowser
from pathlib import Path


COMPONENT_ENV_KEYS = {
    "X07_STUDIO_X07_EXE",
    "X07_STUDIO_X07_WASM_EXE",
    "X07_STUDIO_X07LP_EXE",
}
PATH_SETTING_KEYS = {
    "X07_STUDIO_WORKSPACE_ROOT",
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle-root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--root", type=Path)
    parser.add_argument("--daemon-addr")
    parser.add_argument("--web-addr")
    parser.add_argument("--no-open", action="store_true")
    parser.add_argument(
        "--no-install-missing",
        action="store_true",
        help="Detect components without building missing sibling source checkouts.",
    )
    parser.add_argument(
        "--skip-bootstrap",
        action="store_true",
        help="Skip startup component discovery and defaults.env refresh.",
    )
    args = parser.parse_args()

    bundle_root = args.bundle_root.resolve()
    web_root = bundle_root / "web"
    if not web_root.exists():
        web_root = bundle_root / "web" / "build"
    if not web_root.exists():
        print(f"web app not found under {bundle_root}", file=sys.stderr)
        return 1

    env = os.environ.copy()
    defaults_path = bundle_root / "defaults.env"
    load_env_file(defaults_path, env)
    if not args.skip_bootstrap:
        run_component_bootstrap(
            bundle_root,
            defaults_path,
            env,
            install_missing=not args.no_install_missing,
        )
        load_env_file(defaults_path, env)

    workspace_root = configured_path(
        args.root,
        env.get("X07_STUDIO_WORKSPACE_ROOT"),
        Path.home() / "x07-studio-workspace",
    )
    daemon_addr = choose_available_addr(
        args.daemon_addr or env.get("X07_STUDIO_DAEMON_ADDR") or "127.0.0.1:7719",
        "daemon",
    )
    web_addr = choose_available_addr(
        args.web_addr or env.get("X07_STUDIO_WEB_ADDR") or "127.0.0.1:7720",
        "web",
        reserved={daemon_addr},
    )
    apply_runtime_addresses(env, daemon_addr, web_addr)
    workspace_root.mkdir(parents=True, exist_ok=True)
    daemon = start_daemon(bundle_root, workspace_root, daemon_addr, env)
    wait_for_daemon(f"http://{daemon_addr}/v1/health")
    try:
        url = f"http://{web_addr}"
        server = make_server(web_addr, web_root, f"http://{daemon_addr}")
        print(
            json.dumps(
                {
                    "studio_url": url,
                    "workspace_root": str(workspace_root),
                    "daemon_addr": daemon_addr,
                    "daemon_url": f"http://{daemon_addr}",
                    "web_addr": web_addr,
                },
                indent=2,
            ),
            flush=True,
        )
        if not args.no_open:
            webbrowser.open(url)
        server.serve_forever()
    except KeyboardInterrupt:
        return 0
    finally:
        daemon.terminate()
    return 0


def load_env_file(path: Path, env: dict[str, str]) -> None:
    if not path.exists():
        return
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, value = stripped.split("=", 1)
        key = key.strip()
        normalized = normalize_env_value(key, value.strip().strip('"'), path.parent)
        env[key] = normalized


def normalize_env_value(key: str, value: str, base: Path) -> str:
    if not value:
        return value
    if key in COMPONENT_ENV_KEYS and not Path(value).is_absolute():
        return str((base / value).resolve())
    if key in PATH_SETTING_KEYS:
        return os.path.expandvars(os.path.expanduser(value))
    return value


def configured_path(cli_value: Path | None, env_value: str | None, default: Path) -> Path:
    value = cli_value if cli_value is not None else Path(env_value) if env_value else default
    return Path(os.path.expandvars(os.path.expanduser(str(value))))


def choose_available_addr(addr: str, label: str, reserved: set[str] | None = None) -> str:
    reserved = reserved or set()
    host, port = split_addr(addr)
    if port == 0:
        candidate = f"{host}:{ephemeral_port(host)}"
        return choose_available_addr(candidate, label, reserved)
    for candidate_port in range(port, min(port + 50, 65535) + 1):
        candidate = f"{host}:{candidate_port}"
        if candidate in reserved:
            continue
        if can_bind(host, candidate_port):
            if candidate != addr:
                print(f"{label} port {port} is busy; using {candidate}", file=sys.stderr)
            return candidate
    fallback_port = ephemeral_port(host)
    fallback = f"{host}:{fallback_port}"
    print(f"{label} port {port} is busy; using {fallback}", file=sys.stderr)
    return fallback


def can_bind(host: str, port: int) -> bool:
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
            probe.bind((host, port))
        return True
    except OSError:
        return False


def ephemeral_port(host: str) -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind((host, 0))
        return int(probe.getsockname()[1])


def apply_runtime_addresses(env: dict[str, str], daemon_addr: str, web_addr: str) -> None:
    env["X07_STUDIO_DAEMON_ADDR"] = daemon_addr
    env["X07_STUDIO_DAEMON_URL"] = f"http://{daemon_addr}"
    env["X07_STUDIO_WEB_ADDR"] = web_addr


def run_component_bootstrap(
    bundle_root: Path,
    defaults_path: Path,
    env: dict[str, str],
    *,
    install_missing: bool,
) -> None:
    script = bundle_root / "scripts" / "bootstrap_components.py"
    if not script.exists():
        return
    command = [
        sys.executable,
        str(script),
        "--repo-root",
        str(bundle_root),
        "--write-env",
        str(defaults_path),
        "--allow-missing",
    ]
    if install_missing:
        command.append("--install-missing")
    result = subprocess.run(command, env=env, text=True, capture_output=True, check=False)
    if result.stdout.strip():
        print(result.stdout.rstrip())
    if result.stderr.strip():
        print(result.stderr.rstrip(), file=sys.stderr)
    if result.returncode != 0:
        print(
            "component bootstrap failed; launching Studio so setup guidance remains visible",
            file=sys.stderr,
        )


def start_daemon(bundle_root: Path, workspace_root: Path, addr: str, env: dict[str, str]) -> subprocess.Popen:
    daemon = bundled_binary(bundle_root, "loom-daemon") or shutil.which("loom-daemon")
    if not daemon:
        raise SystemExit("loom-daemon binary is missing from the bundle and PATH")
    return subprocess.Popen(
        [str(daemon), "serve", "--root", str(workspace_root), "--addr", addr],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def wait_for_daemon(url: str) -> None:
    deadline = time.time() + 10
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(url, timeout=0.5):
                return
        except OSError:
            time.sleep(0.1)
    print("daemon health check did not respond before web launch", file=sys.stderr)


def bundled_binary(bundle_root: Path, name: str) -> Path | None:
    candidates = [bundle_root / "bin" / name, bundle_root / "bin" / f"{name}.exe"]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return None


def make_server(web_addr: str, web_root: Path, daemon_origin: str) -> socketserver.TCPServer:
    host, port = split_addr(web_addr)

    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=str(web_root), **kwargs)

        def do_GET(self) -> None:
            if self.path.startswith("/v1/"):
                self.proxy()
                return
            super().do_GET()

        def do_POST(self) -> None:
            if self.path.startswith("/v1/"):
                self.proxy()
                return
            self.send_error(404)

        def do_DELETE(self) -> None:
            if self.path.startswith("/v1/"):
                self.proxy()
                return
            self.send_error(404)

        def proxy(self) -> None:
            body = None
            if self.headers.get("content-length"):
                body = self.rfile.read(int(self.headers["content-length"]))
            request = urllib.request.Request(
                f"{daemon_origin}{self.path}",
                data=body,
                method=self.command,
                headers={"content-type": self.headers.get("content-type", "application/json")},
            )
            try:
                with urllib.request.urlopen(request, timeout=30) as response:
                    payload = response.read()
                    self.send_response(response.status)
                    self.send_header("content-type", response.headers.get("content-type", "application/json"))
                    self.send_header("content-length", str(len(payload)))
                    self.end_headers()
                    self.wfile.write(payload)
            except urllib.error.HTTPError as error:
                payload = error.read()
                self.send_response(error.code)
                self.send_header("content-type", error.headers.get("content-type", "text/plain"))
                self.send_header("content-length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

    socketserver.TCPServer.allow_reuse_address = True
    return socketserver.TCPServer((host, port), Handler)


def split_addr(addr: str) -> tuple[str, int]:
    host, port = addr.rsplit(":", 1)
    return host, int(port)


if __name__ == "__main__":
    raise SystemExit(main())
