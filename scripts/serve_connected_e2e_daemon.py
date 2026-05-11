#!/usr/bin/env python3
"""Start a connected-e2e Loom daemon with a deterministic local toolchain."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil


FAKE_TOOL = """#!/usr/bin/env python3
from __future__ import annotations

import json
import os
from pathlib import Path
import sys
import time


def split_args(argv: list[str]) -> tuple[list[str], str | None]:
    args: list[str] = []
    report_out = None
    index = 0
    while index < len(argv):
        value = argv[index]
        if value == "--report-out" and index + 1 < len(argv):
            report_out = argv[index + 1]
            index += 2
            continue
        if value in {"--json", "--quiet-json"}:
            index += 1
            continue
        args.append(value)
        index += 1
    return args, report_out


def option(args: list[str], name: str, fallback: str = "") -> str:
    try:
        return args[args.index(name) + 1]
    except (ValueError, IndexError):
        return fallback


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2) + "\\n", encoding="utf-8")


def run_x07(args: list[str]) -> None:
    cwd = Path.cwd()
    if args[:3] == ["init", "--template", "xtal-pure"]:
        write_json(cwd / "x07.json", {"schema_version": "x07.project@0.1.0", "name": "connected-e2e"})
        write_json(cwd / "x07.lock.json", {"schema_version": "x07.lock@0.1.0", "packages": []})
        (cwd / "AGENT.md").write_text("# Connected E2E workspace\\n", encoding="utf-8")
        for relative in ["spec", "src", "gen/xtal"]:
            (cwd / relative).mkdir(parents=True, exist_ok=True)
        return

    if args[:3] == ["xtal", "spec", "scaffold"]:
        module_id = option(args, "--module-id", "toy.sorter")
        op_name = option(args, "--op", f"{module_id}.run_v1")
        write_json(
            cwd / "spec" / f"{module_id}.x07spec.json",
            {
                "schema_version": "x07.x07spec@0.1.0",
                "module_id": module_id,
                "operations": [{"id": f"op.{op_name}.v1", "name": op_name}],
            },
        )
        return

    if args[:3] == ["xtal", "tests", "gen-from-spec"]:
        write_json(
            cwd / "gen/xtal/tests.json",
            {"schema_version": "x07.tests@0.1.0", "tests": [{"id": "connected-e2e.sorter"}]},
        )
        return

    if args[:3] == ["xtal", "impl", "sync"]:
        write_json(cwd / "src/main.x07.json", {"schema_version": "x07.ast@0.1.0", "decls": []})
        return

    if args[:3] == ["xtal", "impl", "check"]:
        return

    if args[:3] == ["xtal", "spec", "check"]:
        return

    if args[:2] == ["xtal", "verify"]:
        write_json(
            cwd / "target/xtal/verify/summary.json",
            {"schema_version": "x07.xtal.verify.summary@0.1.0", "ok": True},
        )
        return


def run_x07_wasm(args: list[str]) -> None:
    cwd = Path.cwd()
    out_dir = option(args, "--out-dir")
    if out_dir:
        out_path = cwd / out_dir
        out_path.mkdir(parents=True, exist_ok=True)
        if args[:2] == ["app", "build"]:
            write_json(
                out_path / "app.bundle.json",
                {"schema_version": "x07.wasm.app.bundle@0.1.0", "ok": True},
            )
        elif args[:2] == ["app", "pack"]:
            write_json(
                out_path / "app.pack.json",
                {"schema_version": "x07.wasm.app.pack@0.1.0", "ok": True},
            )
        elif args[:2] == ["deploy", "plan"]:
            write_json(
                out_path / "deploy.plan.json",
                {"schema_version": "x07.wasm.deploy.plan@0.1.0", "ok": True},
            )

    out = option(args, "--out")
    if out:
        write_json(
            cwd / out,
            {"schema_version": "x07.wasm.provenance@0.1.0", "ok": True},
        )


def run_agent(command: str, args: list[str]) -> None:
    prompt_path = args[-1] if args else ".x07/studio/handoffs/unknown.md"
    print(
        json.dumps(
            {
                "schema_version": "x07.studio.agent_event@0.1.0",
                "kind": "artifact",
                "summary": f"{command} produced connected-e2e artifact evidence",
                "artifact": prompt_path,
            }
        )
    )
    print(f"write: {prompt_path}")
    print("approval required: connected-e2e policy checkpoint resolved by test")


def main() -> int:
    args, report_out = split_args(sys.argv[1:])
    tool = Path(sys.argv[0]).name
    if tool == "x07":
        run_x07(args)
    elif tool == "x07-wasm":
        run_x07_wasm(args)
    elif tool in {"codex", "claude"}:
        run_agent(tool, args)
    result = {"status": "ok", "time": int(time.time())}
    if tool == "x07lp" and args[:1] == ["accept"]:
        result["deployment_id"] = "connected-e2e-atlas"
        result["exec_id"] = "connected-e2e-atlas"
    report = {
        "schema_version": "x07.connected_e2e.report@0.1.0",
        "ok": True,
        "tool": tool,
        "argv": args,
        "result": result,
    }
    if report_out:
        write_json(Path(report_out), report)
    print(json.dumps(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
"""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--bin-dir", required=True)
    parser.add_argument("--addr", default="127.0.0.1:7729")
    return parser.parse_args()


def write_fake_tool(bin_dir: Path, name: str) -> Path:
    path = bin_dir / name
    path.write_text(FAKE_TOOL, encoding="utf-8")
    path.chmod(0o755)
    return path


def reset_target_path(repo_root: Path, path: Path) -> None:
    target_root = (repo_root / "target").resolve()
    if path != target_root and target_root not in path.parents:
        raise SystemExit(f"refusing to reset path outside target/: {path}")
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[1]
    workspace = (repo_root / args.workspace).resolve()
    bin_dir = (repo_root / args.bin_dir).resolve()
    reset_target_path(repo_root, workspace)
    reset_target_path(repo_root, bin_dir)

    x07 = write_fake_tool(bin_dir, "x07")
    x07_wasm = write_fake_tool(bin_dir, "x07-wasm")
    x07lp = write_fake_tool(bin_dir, "x07lp")
    write_fake_tool(bin_dir, "codex")
    write_fake_tool(bin_dir, "claude")

    env = os.environ.copy()
    env["PATH"] = f"{bin_dir}{os.pathsep}{env.get('PATH', '')}"
    env["X07_STUDIO_X07_EXE"] = str(x07)
    env["X07_STUDIO_X07_WASM_EXE"] = str(x07_wasm)
    env["X07_STUDIO_X07LP_EXE"] = str(x07lp)

    os.chdir(repo_root)
    os.execvpe(
        "cargo",
        [
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
        ],
        env,
    )


if __name__ == "__main__":
    raise SystemExit(main())
