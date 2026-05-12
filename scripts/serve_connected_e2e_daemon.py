#!/usr/bin/env python3
"""Start a connected-e2e Loom daemon with a deterministic local toolchain."""

from __future__ import annotations

import argparse
import json
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
    if args[:3] == ["service", "genpack", "schema"]:
        archetype = option(args, "--archetype", "api-cell")
        print(
            json.dumps(
                {
                    "schema_version": "x07.service.genpack.schema_v1",
                    "archetype": archetype,
                    "type": "object",
                    "required": ["service", "operations"],
                    "properties": {
                        "service": {"type": "string"},
                        "operations": {"type": "array"},
                    },
                }
            )
        )
        return

    if args[:3] == ["service", "genpack", "grammar"]:
        archetype = option(args, "--archetype", "api-cell")
        print(f"{archetype} ::= service operations policy")
        return

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
    if "-clarify" in prompt_path:
        # Connected-E2E clarify mode: emit two structured clarify_question
        # events that mirror what a real Claude Code / Codex run would emit
        # for an unfamiliar intent. The Timeline UI ingests these into
        # the intent packet's clarification_history.
        print(
            json.dumps(
                {
                    "schema_version": "x07.studio.agent_event@0.1.0",
                    "kind": "clarify_question",
                    "id": f"q-{command}-empty-input",
                    "text": "What should happen for empty input?",
                    "witness_kind": "forbidden_behavior",
                    "options": [
                        "Reject with an error",
                        "Return an empty result",
                    ],
                }
            )
        )
        print(
            json.dumps(
                {
                    "schema_version": "x07.studio.agent_event@0.1.0",
                    "kind": "clarify_question",
                    "id": f"q-{command}-stability",
                    "text": "Should equal items keep their original order?",
                    "witness_kind": "desired_behavior",
                    "options": ["Yes, stable", "No, any permutation"],
                }
            )
        )
        return
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


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def write_connected_examples(examples_root: Path) -> None:
    atlas = examples_root / "wasm_showcases/x07_atlas"
    for relative in [
        "arch/app",
        "arch/app/ops",
        "arch/provenance",
        "arch/slo",
        "backend",
        "frontend",
        "tests/fixtures/metrics",
        "tests/regress",
        "tests/traces",
    ]:
        (atlas / relative).mkdir(parents=True, exist_ok=True)

    (atlas / "README.md").write_text("# Connected E2E x07 Atlas\n", encoding="utf-8")
    write_json(
        atlas / "arch/app/index.x07app.json",
        {"schema_version": "x07.app.index@0.1.0", "profiles": ["atlas_dev", "atlas_release"]},
    )
    write_json(atlas / "arch/app/ops/caps_release.json", {"schema_version": "x07.caps@0.1.0"})
    write_json(atlas / "arch/app/ops/ops_release.json", {"schema_version": "x07.ops@0.1.0"})
    write_json(atlas / "arch/slo/slo_min.json", {"schema_version": "x07.slo@0.1.0"})
    (atlas / "arch/provenance/dev.ed25519.signing_key.b64").write_text("test\n", encoding="utf-8")
    (atlas / "arch/provenance/dev.ed25519.public_key.b64").write_text("test\n", encoding="utf-8")
    for project in ["backend", "frontend"]:
        write_json(
            atlas / project / "x07.json",
            {"schema_version": "x07.project@0.1.0", "name": f"atlas-{project}"},
        )
        write_json(
            atlas / project / "x07.lock.json",
            {"schema_version": "x07.lock@0.1.0", "packages": []},
        )
    write_json(atlas / "tests/traces/happy_path.trace.json", {"schema_version": "x07.trace@0.1.0"})
    write_json(
        atlas / "tests/traces/validation_error.trace.json",
        {"schema_version": "x07.trace@0.1.0"},
    )
    write_json(
        atlas / "tests/regress/atlas_incident.trace.json",
        {"schema_version": "x07.trace@0.1.0"},
    )
    write_json(
        atlas / "tests/fixtures/metrics/atlas_canary_ok.json",
        {"schema_version": "x07.metrics@0.1.0", "ok": True},
    )


def write_connected_incident(workspace: Path) -> None:
    write_json(
        workspace / ".x07-wasm/incidents/demo-incident/run.report.json",
        {
            "kind": "runtime_violation",
            "summary": "connected-e2e incident should re-enter the repair loop",
            "at": "2026-05-12T12:00:00Z",
        },
    )


def write_connected_cassettes(workspace: Path) -> None:
    cassette_dir = workspace / ".x07_rr/http"
    cassette_dir.mkdir(parents=True, exist_ok=True)
    first = cassette_dir / "001-request.json"
    second = cassette_dir / "002-response.json"
    write_json(first, {"request": "/v1/accounts", "status": "seeded"})
    write_json(second, {"response": {"ok": True}, "status": "later"})
    os.utime(first, (1_778_600_001, 1_778_600_001))
    os.utime(second, (1_778_600_002, 1_778_600_002))


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
    examples_root = (repo_root / "target/connected-e2e-examples").resolve()
    reset_target_path(repo_root, workspace)
    reset_target_path(repo_root, bin_dir)
    reset_target_path(repo_root, examples_root)
    write_connected_examples(examples_root)
    write_connected_incident(workspace)
    write_connected_cassettes(workspace)

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
    env["X07_STUDIO_X07_EXAMPLES_ROOT"] = str(examples_root)

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
