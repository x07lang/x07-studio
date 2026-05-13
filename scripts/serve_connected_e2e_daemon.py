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


def intent_target(cwd: Path) -> tuple[str, str, Path]:
    specs = sorted((cwd / "spec").glob("*.x07spec.json"))
    for spec in specs:
        try:
            payload = json.loads(spec.read_text(encoding="utf-8"))
        except Exception:
            continue
        module_id = payload.get("module_id") or spec.name.removesuffix(".x07spec.json")
        operation = (payload.get("operations") or [{}])[0]
        entry = str(operation.get("name") or f"{module_id}.run_v1").split(".")[-1]
        return module_id, entry, cwd / "src" / f"{module_id.replace('.', '/')}.x07.json"
    return "app.main", "run_v1", cwd / "src/main.x07.json"


def target_module_body(module_id: str, entry: str, stub: bool, agent: str = "agent") -> object:
    full = f"{module_id}.{entry}"
    body = ["bytes.empty"] if stub else [
        "begin",
        ["let", "n", ["bytes.len", "payload"]],
        ["let", "out", ["view.to_bytes", ["bytes.view", "payload"]]],
        ["for", "i", 0, "n",
            ["set", "out",
                ["bytes.set_u8", "out", "i",
                    ["+", ["bytes.get_u8", "out", "i"], 0]]]],
        "out",
    ]
    if agent == "codex":
        body = ["begin", ["let", "out", ["view.to_bytes", ["bytes.view", "payload"]]], "out"]
    return {
        "kind": "module",
        "module_id": module_id,
        "schema_version": "x07.x07ast@0.8.0",
        "imports": [],
        "decls": [
            {"kind": "export", "names": [full]},
            {
                "kind": "defn",
                "name": full,
                "params": [{"name": "payload", "ty": "bytes"}],
                "result": "bytes",
                "body": body,
            },
        ],
    }


def lint_diagnostics(cwd: Path) -> list[object]:
    if (cwd / ".x07/studio/lint-fixed").exists():
        return []
    return [
        {
            "code": "X07-LINT-0042",
            "severity": "warning",
            "file": "src/main.x07.json",
            "line": 1,
            "column": 1,
            "message": "connected-e2e lint quickfix fixture",
            "quickfix": {"kind": "json_patch"},
        }
    ]


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
        # Emit a stubby module (single defn with `bytes.empty` body) so
        # Studio's stub-scanner flags scaffold_only=true. The connected
        # realize test then asks the fake claude to overwrite this with
        # a non-trivial body.
        module_id, entry, target = intent_target(cwd)
        write_json(target, target_module_body(module_id, entry, stub=True))
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

    if args[:1] == ["doctor"]:
        return

    if args[:4] == ["pkg", "lock", "--project", "x07.json"]:
        return

    if args[:3] == ["migrate", "--check", "--to"] or args[:3] == ["migrate", "--write", "--to"]:
        return

    if args[:3] == ["project", "migrate", "--check"] or args[:3] == ["project", "migrate", "--write"]:
        return

    if args[:1] == ["lint"]:
        print(
            json.dumps(
                {
                    "schema_version": "x07diag.report@0.1.0",
                    "diagnostics": lint_diagnostics(cwd),
                }
            )
        )
        return

    if args[:1] == ["fix"]:
        module_id, entry, target = intent_target(cwd)
        if "--input" in args:
            body = target_module_body(module_id, entry, stub=False)
            target.parent.mkdir(parents=True, exist_ok=True)
            write_json(target, body)
            write_json(cwd / ".x07/studio/lint-fixed", {"fixed": True})
        if "--from-pbt" in args:
            (cwd / "tests").mkdir(parents=True, exist_ok=True)
            write_json(
                cwd / "tests/pbt_regression.json",
                {"schema_version": "x07.tests@0.1.0", "tests": [{"id": "pbt.regression"}]},
            )
        print(
            json.dumps(
                {
                    "schema_version": "x07.patchset@0.1.0",
                    "patches": [
                        {
                            "path": "src/main.x07.json",
                            "patch": [
                                {"op": "add", "path": "/metadata", "value": {"fixed": True}}
                            ],
                        }
                    ],
                }
            )
        )
        return

    if args[:2] == ["test", "--pbt"]:
        repro = cwd / ".x07/cache/pbt/repros/connected-repro.json"
        write_json(
            repro,
            {
                "schema_version": "x07.pbt.repro@0.1.0",
                "repro_id": "connected-repro",
                "counterexample": {"case_bytes_b64": "AA=="},
            },
        )
        print(
            json.dumps(
                {
                    "schema_version": "x07.pbt.report@0.1.0",
                    "properties_run": 47,
                    "counterexamples": [
                        {
                            "repro_id": "connected-repro",
                            "property": "connected property",
                            "shrunk_input": [0],
                            "repro_path": ".x07/cache/pbt/repros/connected-repro.json",
                        }
                    ],
                }
            )
        )
        return

    if args[:2] == ["arch", "check"]:
        print(
            json.dumps(
                {
                    "schema_version": "x07.arch.check@0.1.0",
                    "passed": True,
                    "violations": [],
                }
            )
        )
        return

    if args[:2] == ["pkg", "provides"]:
        module_id = args[2] if len(args) > 2 else "text.normalize_v1"
        print(
            json.dumps(
                {
                    "schema_version": "x07.pkg.provides@0.1.0",
                    "module_id": module_id,
                    "candidates": [
                        {
                            "package": "ext-text",
                            "version": "0.5.0",
                            "source": "registry",
                            "install_command": "x07 pkg add ext-text@0.5.0",
                        }
                    ],
                }
            )
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
    prompt = " ".join(args)
    if "-realize" in prompt_path or "realize" in prompt.lower() or "implementation" in prompt.lower():
        if command == "claude" and "no-write realize regression" in prompt.lower():
            print(
                json.dumps(
                    {
                        "schema_version": "x07.studio.agent_event@0.1.0",
                        "kind": "diagnostic",
                        "summary": "connected-e2e forced no-write realize failure",
                    }
                )
            )
            print(json.dumps({"type": "result", "subtype": "error", "exit_code": 1}))
            print("connected-e2e forced no-write realize failure", file=sys.stderr)
            raise SystemExit(1)
        # Connected-E2E realize mode: write a non-stub target module
        # body so Studio's stub-scanner stops flagging it. The body is
        # arbitrary — what matters is that body_is_stub() returns false.
        cwd = Path.cwd()
        module_id, entry, target = intent_target(cwd)
        body = target_module_body(module_id, entry, stub=False, agent=command)
        target.parent.mkdir(parents=True, exist_ok=True)
        print(json.dumps({"type": "assistant", "message": {"content": [{"type": "text", "text": f"{command} is editing {target.relative_to(cwd)}"}]}}))
        print(json.dumps({"type": "assistant", "message": {"content": [{"type": "tool_use", "name": "Write", "input": {"file_path": str(target.relative_to(cwd)), "content": json.dumps(body)}}]}}))
        write_json(target, body)
        print(json.dumps({"type": "user", "message": {"content": [{"type": "tool_result", "name": "Write", "content": f"wrote {target.relative_to(cwd)}", "is_error": False}]}}))
        print(
            json.dumps(
                {
                    "schema_version": "x07.studio.agent_event@0.1.0",
                    "kind": "write",
                    "summary": f"{command} filled in {module_id}.{entry}",
                    "artifact": str(target.relative_to(cwd)),
                }
            )
        )
        print(json.dumps({"type": "result", "subtype": "success", "exit_code": 0}))
        print(f"write: {target.relative_to(cwd)}")
        return
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
    if tool == "x07" and args[:1] == ["doctor"]:
        result["ok"] = True
        result["warnings"] = []
        result["blockers"] = []
    if tool == "x07" and args[:1] == ["lint"]:
        result["diagnostics"] = lint_diagnostics(Path.cwd())
    if tool == "x07" and args[:1] == ["fix"]:
        result["patches"] = [
            {
                "path": "src/main.x07.json",
                "patch": [{"op": "add", "path": "/metadata", "value": {"fixed": True}}],
            }
        ]
    if tool == "x07" and args[:2] == ["test", "--pbt"]:
        result["properties_run"] = 47
        result["counterexamples"] = [
            {
                "repro_id": "connected-repro",
                "property": "connected property",
                "shrunk_input": [0],
                "repro_path": ".x07/cache/pbt/repros/connected-repro.json",
            }
        ]
    if tool == "x07" and args[:2] == ["arch", "check"]:
        result["passed"] = True
        result["violations"] = []
    if tool == "x07" and args[:2] == ["pkg", "provides"]:
        result["candidates"] = [
            {
                "package": "ext-text",
                "version": "0.5.0",
                "source": "registry",
                "install_command": "x07 pkg add ext-text@0.5.0",
            }
        ]
    if tool == "x07lp" and args[:1] == ["accept"]:
        result["deployment_id"] = "connected-e2e-atlas"
        result["exec_id"] = "connected-e2e-atlas"
    if tool == "x07lp" and args[:1] in [["run"], ["status"], ["query"]]:
        result["deployment_id"] = option(args, "--deployment", "connected-e2e-atlas")
        result["state"] = "ready"
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
