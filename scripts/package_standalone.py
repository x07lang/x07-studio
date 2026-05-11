#!/usr/bin/env python3
"""Create a portable x07 Studio standalone bundle."""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
from pathlib import Path


BINARIES = ("loom-daemon", "x07-studio", "x07-studio-forge")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target-dir", type=Path, default=Path("target/release"))
    parser.add_argument("--web-dir", type=Path, default=Path("web/build"))
    parser.add_argument("--out-dir", type=Path, default=Path("dist/standalone"))
    parser.add_argument("--version", default="0.1.0")
    parser.add_argument(
        "--component-bin",
        action="append",
        default=[],
        metavar="ID=PATH",
        help="Copy a runtime component into the bundle and wire defaults.env.",
    )
    args = parser.parse_args()

    repo_root = Path.cwd()
    bundle_name = f"x07-studio-{args.version}-{platform_tag()}"
    bundle_root = args.out_dir / bundle_name
    if bundle_root.exists():
        shutil.rmtree(bundle_root)
    (bundle_root / "bin").mkdir(parents=True)

    for binary in BINARIES:
        copy_binary(args.target_dir, bundle_root / "bin", binary)
    copy_tree(args.web_dir, bundle_root / "web")
    copy_optional_tree(repo_root / "config", bundle_root / "config")
    copy_script(repo_root / "scripts" / "bootstrap_components.py", bundle_root)
    copy_script(repo_root / "scripts" / "launch_studio_web.py", bundle_root)
    bundled_components = copy_component_bins(args.component_bin, bundle_root)

    manifest = {
        "schema_version": "x07.studio.standalone_bundle@0.1.0",
        "name": "x07 Studio",
        "version": args.version,
        "platform": platform_tag(),
        "binaries": [binary_name(binary) for binary in BINARIES],
        "web_app": "web",
        "required_components": ["x07", "x07-wasm", "x07lp"],
        "default_workspace": "~/x07-studio-workspace",
        "daemon_addr": "127.0.0.1:7719",
        "web_addr": "127.0.0.1:7720",
        "bundled_components": sorted(bundled_components),
    }
    (bundle_root / "x07-studio-standalone.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    (bundle_root / "defaults.env").write_text(default_env(bundled_components), encoding="utf-8")
    (bundle_root / "README.md").write_text(readme(manifest), encoding="utf-8")

    archive = shutil.make_archive(str(bundle_root), "zip", root_dir=bundle_root)
    print(json.dumps({"bundle": str(bundle_root), "archive": archive}, indent=2))
    return 0


def copy_binary(target_dir: Path, out_dir: Path, binary: str) -> None:
    source = target_dir / binary_name(binary)
    if not source.exists():
        raise SystemExit(f"missing built binary: {source}")
    target = out_dir / source.name
    shutil.copy2(source, target)
    target.chmod(target.stat().st_mode | 0o755)


def binary_name(binary: str) -> str:
    return f"{binary}.exe" if os.name == "nt" else binary


def copy_tree(source: Path, target: Path) -> None:
    if not source.exists():
        raise SystemExit(f"missing required directory: {source}")
    shutil.copytree(source, target)


def copy_optional_tree(source: Path, target: Path) -> None:
    if source.exists():
        shutil.copytree(source, target)


def copy_script(source: Path, bundle_root: Path) -> None:
    target_dir = bundle_root / "scripts"
    target_dir.mkdir(exist_ok=True)
    target = target_dir / source.name
    shutil.copy2(source, target)
    target.chmod(target.stat().st_mode | 0o755)


def copy_component_bins(entries: list[str], bundle_root: Path) -> dict[str, str]:
    env_by_id = {
        "x07": "X07_STUDIO_X07_EXE",
        "x07-wasm": "X07_STUDIO_X07_WASM_EXE",
        "x07lp": "X07_STUDIO_X07LP_EXE",
    }
    copied: dict[str, str] = {}
    if not entries:
        return copied
    component_dir = bundle_root / "components"
    component_dir.mkdir(exist_ok=True)
    for entry in entries:
        if "=" not in entry:
            raise SystemExit(f"component entry must be ID=PATH: {entry}")
        component_id, raw_path = entry.split("=", 1)
        if component_id not in env_by_id:
            raise SystemExit(f"unsupported component id: {component_id}")
        source = executable_variant(Path(raw_path))
        if not source:
            raise SystemExit(f"component binary is missing: {raw_path}")
        target_name = binary_name(component_id) if component_id != "x07lp" else source.name
        target = component_dir / target_name
        shutil.copy2(source, target)
        target.chmod(target.stat().st_mode | 0o755)
        copied[component_id] = f"components/{target.name}"
    return copied


def executable_variant(path: Path) -> Path | None:
    if path.exists():
        return path
    if os.name == "nt" and path.suffix.lower() != ".exe":
        exe = path.with_suffix(path.suffix + ".exe")
        if exe.exists():
            return exe
    return None


def platform_tag() -> str:
    system = platform.system().lower() or "unknown"
    machine = platform.machine().lower() or "unknown"
    return f"{system}-{machine}"


def default_env(bundled_components: dict[str, str]) -> str:
    env_by_id = {
        "x07": "X07_STUDIO_X07_EXE",
        "x07-wasm": "X07_STUDIO_X07_WASM_EXE",
        "x07lp": "X07_STUDIO_X07LP_EXE",
    }
    lines = [
        "# Optional local overrides generated for x07 Studio standalone bundles.",
        "# Fill these only when the commands are not available on PATH.",
    ]
    for component_id, relative_path in sorted(bundled_components.items()):
        lines.append(f'{env_by_id[component_id]}="{relative_path}"')
    lines.extend(
        [
            "# X07_STUDIO_X07_EXE=\"/path/to/x07\"",
            "# X07_STUDIO_X07_WASM_EXE=\"/path/to/x07-wasm\"",
            "# X07_STUDIO_X07LP_EXE=\"/path/to/x07lp\"",
            "",
        ]
    )
    return "\n".join(
        lines
    )


def readme(manifest: dict[str, object]) -> str:
    return f"""# x07 Studio Standalone

This bundle contains the native Studio shell, Loom daemon, Forge terminal shell,
and the built Svelte Studio web app.

## Start

```bash
python3 scripts/bootstrap_components.py --repo-root . --write-env defaults.env
python3 scripts/launch_studio_web.py --bundle-root .
```

The launcher starts `loom-daemon`, serves the static web app on
`http://{manifest["web_addr"]}`, and proxies `/v1/**` to the daemon. The native
desktop shell is available at `bin/{binary_name("x07-studio")}`.

Required local components for full Atlas/platform delivery:

- `x07`
- `x07-wasm`
- `x07lp`

Set the `X07_STUDIO_*_EXE` variables in `defaults.env` when a component is not
on `PATH`. CI or release automation may prefill `defaults.env` when a component
binary is bundled under `components/`.
"""


if __name__ == "__main__":
    raise SystemExit(main())
