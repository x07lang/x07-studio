#!/usr/bin/env python3
"""Check and optionally build local x07 Studio runtime components."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Component:
    id: str
    label: str
    command: str
    env_var: str | None
    required: bool
    bundle_candidates: tuple[str, ...]
    sibling_candidates: tuple[str, ...]
    build_repo: str | None
    build_command: tuple[str, ...]
    install_hint: str


COMPONENTS = (
    Component(
        id="x07",
        label="x07 CLI",
        command="x07",
        env_var="X07_STUDIO_X07_EXE",
        required=True,
        bundle_candidates=("components/x07",),
        sibling_candidates=("x07/target/release/x07", "x07/target/debug/x07"),
        build_repo="x07",
        build_command=("cargo", "build", "--release", "-p", "x07"),
        install_hint=(
            "Install the x07 toolchain, use a bundle with components/x07, "
            "build the sibling x07 repo, or set X07_STUDIO_X07_EXE."
        ),
    ),
    Component(
        id="x07-wasm",
        label="x07-wasm",
        command="x07-wasm",
        env_var="X07_STUDIO_X07_WASM_EXE",
        required=True,
        bundle_candidates=("components/x07-wasm",),
        sibling_candidates=(
            "x07-wasm-backend/target/release/x07-wasm",
            "x07-wasm-backend/target/debug/x07-wasm",
        ),
        build_repo="x07-wasm-backend",
        build_command=("cargo", "build", "--release", "-p", "x07-wasm"),
        install_hint=(
            "Install x07-wasm, use a bundle with components/x07-wasm, "
            "build the sibling x07-wasm-backend repo, or set X07_STUDIO_X07_WASM_EXE."
        ),
    ),
    Component(
        id="x07lp",
        label="x07 platform",
        command="x07lp",
        env_var="X07_STUDIO_X07LP_EXE",
        required=True,
        bundle_candidates=("components/x07lp", "components/x07lp-driver"),
        sibling_candidates=("x07-platform/scripts/x07lp-driver",),
        build_repo=None,
        build_command=(),
        install_hint=(
            "Install x07lp, use a bundle with components/x07lp, "
            "place x07-platform beside Studio, or set X07_STUDIO_X07LP_EXE."
        ),
    ),
    Component(
        id="codex",
        label="OpenAI Codex",
        command="codex",
        env_var=None,
        required=False,
        bundle_candidates=(),
        sibling_candidates=(),
        build_repo=None,
        build_command=(),
        install_hint="Install Codex CLI when supervised Codex handoffs should execute locally.",
    ),
    Component(
        id="claude-code",
        label="Claude Code",
        command="claude",
        env_var=None,
        required=False,
        bundle_candidates=(),
        sibling_candidates=(),
        build_repo=None,
        build_command=(),
        install_hint="Install Claude Code when supervised Claude handoffs should execute locally.",
    ),
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", default=Path(__file__).resolve().parents[1])
    parser.add_argument("--json", action="store_true", help="Emit machine-readable status.")
    parser.add_argument(
        "--install-missing",
        action="store_true",
        help="Build missing components from sibling source repos when available.",
    )
    parser.add_argument(
        "--write-env",
        type=Path,
        help="Write env overrides for discovered local component paths.",
    )
    parser.add_argument(
        "--allow-missing",
        action="store_true",
        help="Return success even when required components are missing.",
    )
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    if args.install_missing:
        build_missing(repo_root)

    report = {
        "schema_version": "x07.studio.components@0.1.0",
        "repo_root": str(repo_root),
        "components": [component_status(repo_root, component) for component in COMPONENTS],
    }
    report["ok"] = all(
        item["status"] == "available" or not item["required"] for item in report["components"]
    )

    if args.write_env:
        write_env_file(args.write_env, report["components"])

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_human_report(report)

    return 0 if report["ok"] or args.allow_missing else 1


def build_missing(repo_root: Path) -> None:
    for component in COMPONENTS:
        if not component.required:
            continue
        if component_status(repo_root, component)["status"] == "available":
            continue
        if not component.build_repo:
            continue
        source_root = source_repo_root(repo_root, component.build_repo)
        if source_root is None:
            continue
        print(f"building {component.label} from {source_root}", file=sys.stderr)
        subprocess.run(component.build_command, cwd=source_root, check=True)


def component_status(repo_root: Path, component: Component) -> dict[str, object]:
    source = env_source(component.env_var)
    if source is None:
        source = bundled_source(repo_root, component)
    if source is None:
        source = sibling_source(repo_root, component)
    if source is None:
        source = path_source(component.command)
    return {
        "id": component.id,
        "label": component.label,
        "command": component.command,
        "required": component.required,
        "status": "available" if source else "missing",
        "source": str(source) if source else None,
        "install_hint": component.install_hint,
    }


def env_source(env_var: str | None) -> Path | None:
    if not env_var:
        return None
    value = os.environ.get(env_var)
    if not value:
        return None
    path = Path(value).expanduser()
    return path if executable_exists(path) else None


def bundled_source(repo_root: Path, component: Component) -> Path | None:
    for candidate in component.bundle_candidates:
        path = executable_variant(repo_root / candidate)
        if path:
            return path
    return None


def sibling_source(repo_root: Path, component: Component) -> Path | None:
    for base in component_search_bases(repo_root):
        for ancestor in ancestry(base):
            for candidate in component.sibling_candidates:
                path = executable_variant(ancestor / candidate)
                if path:
                    return path
    return None


def source_repo_root(repo_root: Path, repo_name: str) -> Path | None:
    for base in component_search_bases(repo_root):
        for ancestor in ancestry(base):
            source_root = ancestor / repo_name
            if source_root.exists():
                return source_root
    return None


def component_search_bases(repo_root: Path) -> list[Path]:
    return [repo_root, repo_root.parent, Path.cwd()]


def ancestry(base: Path) -> list[Path]:
    resolved = base.resolve()
    return [resolved, *list(resolved.parents)[:8]]


def path_source(command: str) -> Path | None:
    found = shutil.which(command)
    return Path(found) if found else None


def executable_variant(path: Path) -> Path | None:
    if executable_exists(path):
        return path
    if os.name == "nt" and path.suffix.lower() != ".exe":
        exe = path.with_suffix(path.suffix + ".exe")
        if executable_exists(exe):
            return exe
    return None


def executable_exists(path: Path) -> bool:
    return path.is_file()


def write_env_file(path: Path, components: list[dict[str, object]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# Generated by x07-studio bootstrap_components.py",
        "# Source this file before launching Studio when using local component builds.",
    ]
    env_by_id = {
        "x07": "X07_STUDIO_X07_EXE",
        "x07-wasm": "X07_STUDIO_X07_WASM_EXE",
        "x07lp": "X07_STUDIO_X07LP_EXE",
    }
    for component in components:
        env_var = env_by_id.get(str(component["id"]))
        source = component.get("source")
        if env_var and source:
            lines.append(f'{env_var}="{source}"')
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def print_human_report(report: dict[str, object]) -> None:
    print(f"x07 Studio setup: {'ready' if report['ok'] else 'needs setup'}")
    for component in report["components"]:
        assert isinstance(component, dict)
        marker = "ok" if component["status"] == "available" else "missing"
        required = "required" if component["required"] else "optional"
        detail = component["source"] or component["install_hint"]
        print(f"- {marker} {component['label']} ({required}): {detail}")


if __name__ == "__main__":
    raise SystemExit(main())
