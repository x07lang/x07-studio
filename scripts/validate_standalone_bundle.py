#!/usr/bin/env python3
"""Validate a packaged x07 Studio standalone bundle."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import zipfile
from pathlib import Path


REQUIRED_SETTINGS = {
    "X07_STUDIO_WORKSPACE_ROOT",
    "X07_STUDIO_DAEMON_ADDR",
    "X07_STUDIO_DAEMON_URL",
    "X07_STUDIO_WEB_ADDR",
}

COMPONENT_ENV_BY_ID = {
    "x07": "X07_STUDIO_X07_EXE",
    "x07-wasm": "X07_STUDIO_X07_WASM_EXE",
    "x07lp": "X07_STUDIO_X07LP_EXE",
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, help="Bundle root to validate.")
    parser.add_argument("--dist-dir", type=Path, default=Path("dist/standalone"))
    parser.add_argument(
        "--require-component",
        action="append",
        default=[],
        choices=sorted(COMPONENT_ENV_BY_ID),
        help="Require a bundled runtime component and defaults.env wiring.",
    )
    args = parser.parse_args()

    bundle = args.bundle or single_bundle_dir(args.dist_dir)
    problems: list[str] = []
    manifest = read_manifest(bundle, problems)
    defaults = read_env(bundle / "defaults.env", problems)

    validate_manifest(bundle, manifest, problems)
    validate_defaults(bundle, defaults, args.require_component, problems)
    validate_archive(bundle, problems)
    validate_bootstrap_report(bundle, args.require_component, problems)

    if problems:
        for problem in problems:
            print(f"standalone validation failed: {problem}", file=sys.stderr)
        return 1

    print(
        json.dumps(
            {
                "ok": True,
                "bundle": str(bundle),
                "required_components": args.require_component,
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


def read_manifest(bundle: Path, problems: list[str]) -> dict[str, object]:
    path = bundle / "x07-studio-standalone.json"
    if not path.is_file():
        problems.append(f"missing manifest at {path}")
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        problems.append(f"manifest is invalid JSON: {error}")
        return {}


def read_env(path: Path, problems: list[str]) -> dict[str, str]:
    if not path.is_file():
        problems.append(f"missing defaults file at {path}")
        return {}
    entries: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, value = stripped.split("=", 1)
        entries[key.strip()] = value.strip().strip('"')
    return entries


def validate_manifest(bundle: Path, manifest: dict[str, object], problems: list[str]) -> None:
    if manifest.get("schema_version") != "x07.studio.standalone_bundle@0.1.0":
        problems.append("manifest schema_version is not x07.studio.standalone_bundle@0.1.0")
    if manifest.get("web_app") != "web":
        problems.append("manifest web_app must point at web")
    web_root = bundle / "web"
    if not (web_root / "index.html").is_file():
        problems.append("web app index.html is missing")
    for binary in manifest.get("binaries", []):
        if not isinstance(binary, str):
            problems.append(f"manifest binary entry is not a string: {binary!r}")
            continue
        if not (bundle / "bin" / binary).is_file():
            problems.append(f"manifest binary is missing from bin/: {binary}")
    for script in ["bootstrap_components.py", "launch_studio_web.py"]:
        if not (bundle / "scripts" / script).is_file():
            problems.append(f"launcher script is missing: {script}")


def validate_defaults(
    bundle: Path,
    defaults: dict[str, str],
    required_components: list[str],
    problems: list[str],
) -> None:
    missing_settings = sorted(REQUIRED_SETTINGS.difference(defaults))
    if missing_settings:
        problems.append(f"defaults.env missing onboarding settings: {', '.join(missing_settings)}")
    for component_id in required_components:
        env_var = COMPONENT_ENV_BY_ID[component_id]
        value = defaults.get(env_var)
        if not value:
            problems.append(f"defaults.env missing bundled component setting {env_var}")
            continue
        component_path = Path(value)
        if not component_path.is_absolute():
            component_path = bundle / component_path
        if not component_path.is_file():
            problems.append(f"bundled component path is missing: {component_path}")


def validate_archive(bundle: Path, problems: list[str]) -> None:
    archive = bundle.parent / f"{bundle.name}.zip"
    if not archive.is_file():
        problems.append(f"missing zip archive at {archive}")
        return
    with zipfile.ZipFile(archive) as zipped:
        names = set(zipped.namelist())
    for required in ["x07-studio-standalone.json", "defaults.env", "web/index.html"]:
        if required not in names:
            problems.append(f"archive missing {required}")


def validate_bootstrap_report(
    bundle: Path,
    required_components: list[str],
    problems: list[str],
) -> None:
    script = bundle / "scripts" / "bootstrap_components.py"
    if not script.is_file():
        return
    result = subprocess.run(
        [
            sys.executable,
            str(script),
            "--repo-root",
            str(bundle),
            "--json",
            "--allow-missing",
        ],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        problems.append(f"bootstrap report failed: {result.stderr.strip() or result.stdout.strip()}")
        return
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        problems.append(f"bootstrap report is invalid JSON: {error}")
        return
    components = {item["id"]: item for item in report.get("components", []) if isinstance(item, dict)}
    for component_id in required_components:
        status = components.get(component_id, {}).get("status")
        if status != "available":
            problems.append(f"bootstrap did not find required bundled component {component_id}")


if __name__ == "__main__":
    raise SystemExit(main())
