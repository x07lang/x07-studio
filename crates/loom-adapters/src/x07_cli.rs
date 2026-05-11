use std::collections::BTreeMap;
use std::env;

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::command_runner::{now_string, CommandExecution, CommandRunner};

#[derive(Debug, Clone, Copy)]
pub enum ProgramKey {
    X07,
    X07Wasm,
    X07lp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineJsonMode {
    Disabled,
    ReportFile,
    StdoutOnly,
}

#[derive(Debug, Clone)]
pub struct BindingTemplate {
    pub id: &'static str,
    pub category: &'static str,
    pub program: ProgramKey,
    pub args: &'static [&'static str],
    pub artifacts: &'static [&'static str],
    pub notes: &'static str,
    pub machine_json: MachineJsonMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedCommand {
    pub id: String,
    pub category: String,
    pub program: String,
    pub args: Vec<String>,
    pub artifacts: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct ExecutedBinding {
    pub rendered: RenderedCommand,
    pub execution: CommandExecution,
    pub report_json: Option<Value>,
    pub report_path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone)]
pub struct BindingDescriptor {
    pub id: &'static str,
    pub category: &'static str,
    pub program: &'static str,
    pub notes: &'static str,
}

impl BindingTemplate {
    pub fn render(&self, vars: &BTreeMap<String, String>) -> RenderedCommand {
        let args = self
            .args
            .iter()
            .map(|arg| interpolate(arg, vars))
            .collect::<Vec<_>>();
        let artifacts = self
            .artifacts
            .iter()
            .map(|artifact| interpolate(artifact, vars))
            .collect::<Vec<_>>();
        RenderedCommand {
            id: self.id.to_string(),
            category: self.category.to_string(),
            program: program_name(self.program).to_string(),
            args,
            artifacts,
            notes: self.notes.to_string(),
        }
    }

    pub fn descriptor(&self) -> BindingDescriptor {
        BindingDescriptor {
            id: self.id,
            category: self.category,
            program: program_name(self.program),
            notes: self.notes,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CliAdapter {
    root: Utf8PathBuf,
    reports_dir: Utf8PathBuf,
    runner: CommandRunner,
}

impl CliAdapter {
    pub fn new(root: &Utf8Path, reports_dir: Utf8PathBuf) -> Self {
        Self {
            root: root.to_owned(),
            reports_dir,
            runner: CommandRunner,
        }
    }

    pub fn list_bindings() -> Vec<BindingDescriptor> {
        XTAL_BINDINGS
            .iter()
            .map(|binding| binding.descriptor())
            .collect()
    }

    pub async fn execute(
        &self,
        binding_id: &str,
        vars: &BTreeMap<String, String>,
    ) -> anyhow::Result<ExecutedBinding> {
        let binding =
            binding_by_id(binding_id).with_context(|| format!("unknown binding `{binding_id}`"))?;
        let mut rendered = binding.render(vars);

        let mut report_path = None;
        match binding.machine_json {
            MachineJsonMode::Disabled => {}
            MachineJsonMode::ReportFile => {
                std::fs::create_dir_all(&self.reports_dir)?;
                let path = self.reports_dir.join(format!(
                    "{}-{}.json",
                    now_string(),
                    rendered.id.replace('/', "_")
                ));
                rendered.args.extend([
                    "--json".to_string(),
                    "--report-out".to_string(),
                    path.to_string(),
                    "--quiet-json".to_string(),
                ]);
                report_path = Some(path);
            }
            MachineJsonMode::StdoutOnly => {
                rendered.args.push("--json".to_string());
            }
        }

        let program = resolve_program(binding.program);
        let execution = self
            .runner
            .run(&self.root, &program, &rendered.args, &BTreeMap::new())
            .await
            .with_context(|| format!("binding `{binding_id}` failed to spawn"))?;

        let report_json = match binding.machine_json {
            MachineJsonMode::ReportFile => report_path
                .as_ref()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok()),
            MachineJsonMode::StdoutOnly => execution
                .stdout_json
                .clone()
                .or_else(|| execution.stderr_json.clone()),
            MachineJsonMode::Disabled => None,
        };

        Ok(ExecutedBinding {
            rendered,
            execution,
            report_json,
            report_path,
        })
    }
}

fn interpolate(template: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

fn resolve_program(key: ProgramKey) -> String {
    match key {
        ProgramKey::X07 => env::var("X07_STUDIO_X07_EXE").unwrap_or_else(|_| "x07".to_string()),
        ProgramKey::X07Wasm => {
            env::var("X07_STUDIO_X07_WASM_EXE").unwrap_or_else(|_| "x07-wasm".to_string())
        }
        ProgramKey::X07lp => {
            env::var("X07_STUDIO_X07LP_EXE").unwrap_or_else(|_| "x07lp".to_string())
        }
    }
}

fn program_name(key: ProgramKey) -> &'static str {
    match key {
        ProgramKey::X07 => "x07",
        ProgramKey::X07Wasm => "x07-wasm",
        ProgramKey::X07lp => "x07lp",
    }
}

pub fn binding_by_id(id: &str) -> Option<&'static BindingTemplate> {
    XTAL_BINDINGS.iter().find(|binding| binding.id == id)
}

pub const XTAL_BINDINGS: &[BindingTemplate] = &[
    BindingTemplate {
        id: "project.init.xtal-pure",
        category: "x07/project",
        program: ProgramKey::X07,
        args: &["init", "--template", "xtal-pure"],
        artifacts: &[
            "x07.json",
            "x07.lock.json",
            "AGENT.md",
            "spec/",
            "src/",
            "gen/xtal/",
        ],
        notes: "Initialize a solve-pure XTAL project when the workspace has no x07.json.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "spec.scaffold",
        category: "xtal/spec",
        program: ProgramKey::X07,
        args: &[
            "xtal",
            "spec",
            "scaffold",
            "--module-id",
            "{module_id}",
            "--op",
            "{op}",
            "--param",
            "{param}",
            "--result",
            "{result}",
        ],
        artifacts: &["spec/{module_id}.x07spec.json"],
        notes: "Scaffold a new spec operation from intent.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "spec.format",
        category: "xtal/spec",
        program: ProgramKey::X07,
        args: &[
            "xtal",
            "spec",
            "fmt",
            "--input",
            "{input}",
            "--write",
            "--inject-ids",
        ],
        artifacts: &["{input}"],
        notes: "Canonicalize a spec file.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "spec.extract",
        category: "xtal/spec",
        program: ProgramKey::X07,
        args: &[
            "xtal",
            "spec",
            "extract",
            "--project",
            "x07.json",
            "--module-id",
            "{module_id}",
            "--patchset-out",
            "{patchset_out}",
        ],
        artifacts: &["{patchset_out}"],
        notes: "Extract brownfield spec from implementation.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "spec.lint",
        category: "xtal/spec",
        program: ProgramKey::X07,
        args: &["xtal", "spec", "lint", "--input", "{input}"],
        artifacts: &[],
        notes: "Lint a spec file.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "spec.check",
        category: "xtal/spec",
        program: ProgramKey::X07,
        args: &[
            "xtal",
            "spec",
            "check",
            "--project",
            "x07.json",
            "--input",
            "{input}",
        ],
        artifacts: &[],
        notes: "Check spec validity and linkage.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "tests.gen.write",
        category: "xtal/tests",
        program: ProgramKey::X07,
        args: &[
            "xtal",
            "tests",
            "gen-from-spec",
            "--project",
            "x07.json",
            "--write",
        ],
        artifacts: &["gen/xtal/tests.json"],
        notes: "Generate tests from spec.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "tests.gen.check",
        category: "xtal/tests",
        program: ProgramKey::X07,
        args: &[
            "xtal",
            "tests",
            "gen-from-spec",
            "--project",
            "x07.json",
            "--check",
        ],
        artifacts: &["gen/xtal/tests.json"],
        notes: "Check generated tests from spec for drift.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "impl.check",
        category: "xtal/impl",
        program: ProgramKey::X07,
        args: &["xtal", "impl", "check", "--project", "x07.json"],
        artifacts: &[],
        notes: "Check spec to implementation alignment.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "impl.sync.write",
        category: "xtal/impl",
        program: ProgramKey::X07,
        args: &["xtal", "impl", "sync", "--project", "x07.json", "--write"],
        artifacts: &["src/"],
        notes: "Synchronize implementation to spec.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "impl.sync.patchset",
        category: "xtal/impl",
        program: ProgramKey::X07,
        args: &[
            "xtal",
            "impl",
            "sync",
            "--project",
            "x07.json",
            "--patchset-out",
            "{patchset_out}",
        ],
        artifacts: &["{patchset_out}"],
        notes: "Emit an implementation sync patchset without writing it.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "xtal.dev",
        category: "xtal/loop",
        program: ProgramKey::X07,
        args: &["xtal", "dev"],
        artifacts: &["target/xtal/verify/summary.json"],
        notes: "Run the canonical XTAL development lane.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "xtal.verify",
        category: "xtal/loop",
        program: ProgramKey::X07,
        args: &["xtal", "verify"],
        artifacts: &["target/xtal/verify/summary.json"],
        notes: "Run the XTAL verify lane.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "xtal.repair",
        category: "xtal/loop",
        program: ProgramKey::X07,
        args: &["xtal", "repair"],
        artifacts: &[
            "target/xtal/repair/summary.json",
            "target/xtal/repair/patchset.json",
        ],
        notes: "Run the XTAL repair lane.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "xtal.certify",
        category: "xtal/loop",
        program: ProgramKey::X07,
        args: &["xtal", "certify"],
        artifacts: &[
            "target/xtal/cert/summary.json",
            "target/xtal/cert/bundle.json",
        ],
        notes: "Run the XTAL certify lane.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "xtal.ingest",
        category: "xtal/runtime",
        program: ProgramKey::X07,
        args: &["xtal", "ingest", "--input", "{input}"],
        artifacts: &["target/xtal/ingest/summary.json"],
        notes: "Normalize runtime incidents into XTAL inputs.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "xtal.improve",
        category: "xtal/runtime",
        program: ProgramKey::X07,
        args: &["xtal", "improve", "--input", "{input}"],
        artifacts: &["target/xtal/improve/summary.json"],
        notes: "Run the bounded improve lane.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "fmt.write",
        category: "x07/core",
        program: ProgramKey::X07,
        args: &["fmt", "--input", "{input}", "--write"],
        artifacts: &["{input}"],
        notes: "Format canonical x07 AST JSON.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "lint.report",
        category: "x07/core",
        program: ProgramKey::X07,
        args: &["lint", "--input", "{input}"],
        artifacts: &[],
        notes: "Run lint with structured output.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "fix.write",
        category: "x07/core",
        program: ProgramKey::X07,
        args: &["fix", "--input", "{input}", "--write"],
        artifacts: &["{input}"],
        notes: "Apply deterministic quickfixes.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "patch.apply",
        category: "x07/core",
        program: ProgramKey::X07,
        args: &[
            "ast",
            "apply-patch",
            "--in",
            "{input}",
            "--patch",
            "{patch}",
            "--out",
            "{output}",
            "--validate",
        ],
        artifacts: &["{output}"],
        notes: "Apply an x07 patchset.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "check.ast",
        category: "x07/core",
        program: ProgramKey::X07,
        args: &["check", "--project", "x07.json", "--ast"],
        artifacts: &[],
        notes: "Run AST/schema/lint checks without typechecking.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "check.project",
        category: "x07/core",
        program: ProgramKey::X07,
        args: &["check", "--project", "x07.json"],
        artifacts: &[],
        notes: "Run the full project check.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.web_ui.build",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "web-ui",
            "build",
            "--project",
            "x07.json",
            "--profile",
            "{profile}",
            "--out-dir",
            "{out_dir}",
        ],
        artifacts: &["{out_dir}"],
        notes: "Build a web-ui bundle.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.web_ui.serve",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "web-ui",
            "serve",
            "--dir",
            "{dir}",
            "--mode",
            "{mode}",
            "--strict-mime",
        ],
        artifacts: &["{dir}"],
        notes: "Serve a web-ui dist directory with strict wasm MIME checks.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.web_ui.test",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "web-ui",
            "test",
            "--dist-dir",
            "{dist_dir}",
            "--case",
            "{case}",
        ],
        artifacts: &[".x07-wasm/incidents"],
        notes: "Replay a web-ui trace case and emit a machine report.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.device.build",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "device",
            "build",
            "--profile",
            "{profile}",
            "--out-dir",
            "{out_dir}",
        ],
        artifacts: &["{out_dir}"],
        notes: "Build a device bundle from a device profile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.device.verify",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &["device", "verify", "--dir", "{dir}"],
        artifacts: &["{dir}"],
        notes: "Verify a device bundle.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.device.package",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "device",
            "package",
            "--bundle",
            "{bundle}",
            "--target",
            "{target}",
            "--out-dir",
            "{out_dir}",
        ],
        artifacts: &["{out_dir}"],
        notes: "Package a desktop/mobile device bundle.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.device.run.desktop_smoke",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "device",
            "run",
            "--bundle",
            "{bundle}",
            "--target",
            "desktop",
            "--headless-smoke",
        ],
        artifacts: &["{bundle}"],
        notes: "Run a desktop device bundle in headless smoke mode.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.device.provenance.attest",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "device",
            "provenance",
            "attest",
            "--bundle-dir",
            "{bundle_dir}",
            "--signing-key",
            "{signing_key}",
            "--out",
            "{out}",
        ],
        artifacts: &["{out}"],
        notes: "Create a signed provenance attestation for a device bundle.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.device.provenance.verify",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "device",
            "provenance",
            "verify",
            "--attestation",
            "{attestation}",
            "--bundle-dir",
            "{bundle_dir}",
            "--trusted-public-key",
            "{trusted_public_key}",
        ],
        artifacts: &["{attestation}"],
        notes: "Verify a signed device bundle provenance attestation.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.workload.build",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "workload",
            "build",
            "--project",
            "x07.json",
            "--manifest",
            "{manifest}",
            "--out-dir",
            "{out_dir}",
        ],
        artifacts: &["{out_dir}"],
        notes: "Build workload artifacts from the service manifest.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.workload.inspect",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "workload",
            "inspect",
            "--pack-manifest",
            "{pack_manifest}",
            "--view",
            "{view}",
        ],
        artifacts: &["{pack_manifest}"],
        notes: "Inspect a workload pack manifest.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.topology.preview",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &["topology", "preview", "--pack-manifest", "{pack_manifest}"],
        artifacts: &[],
        notes: "Inspect workload topology.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.deploy.plan",
        category: "x07/wasm",
        program: ProgramKey::X07Wasm,
        args: &[
            "deploy",
            "plan",
            "--pack-manifest",
            "{pack_manifest}",
            "--ops",
            "{ops}",
            "--out-dir",
            "{out_dir}",
        ],
        artifacts: &["{out_dir}"],
        notes: "Generate a deploy plan from an app pack and ops profile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "lp.release.query",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &["release", "query", "--release-id", "{release_id}"],
        artifacts: &[],
        notes: "Query hosted release state.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.release.rollback",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &["release", "rollback", "--release-id", "{release_id}"],
        artifacts: &[],
        notes: "Rollback a hosted release.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.rollout.status",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &["rollout", "status", "--rollout-id", "{rollout_id}"],
        artifacts: &[],
        notes: "Inspect rollout status.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{binding_by_id, CliAdapter, MachineJsonMode};

    #[test]
    fn spec_scaffold_binding_interpolates_arguments_and_artifacts() {
        let binding = binding_by_id("spec.scaffold").expect("binding exists");
        let vars = BTreeMap::from([
            ("module_id".to_string(), "app.sorter".to_string()),
            ("op".to_string(), "sort_ascending".to_string()),
            ("param".to_string(), "items:bytes".to_string()),
            ("result".to_string(), "sorted:bytes".to_string()),
        ]);

        let rendered = binding.render(&vars);

        assert_eq!(rendered.program, "x07");
        assert_eq!(
            rendered.args,
            vec![
                "xtal",
                "spec",
                "scaffold",
                "--module-id",
                "app.sorter",
                "--op",
                "sort_ascending",
                "--param",
                "items:bytes",
                "--result",
                "sorted:bytes",
            ]
        );
        assert_eq!(rendered.artifacts, vec!["spec/app.sorter.x07spec.json"]);
        assert!(!rendered.args.iter().any(|arg| arg.contains('{')));
    }

    #[test]
    fn project_init_binding_uses_xtal_pure_template() {
        let binding = binding_by_id("project.init.xtal-pure").expect("binding exists");
        let rendered = binding.render(&BTreeMap::new());

        assert_eq!(rendered.program, "x07");
        assert_eq!(rendered.args, vec!["init", "--template", "xtal-pure"]);
        assert!(rendered.artifacts.contains(&"x07.json".to_string()));
        assert_eq!(binding.machine_json, MachineJsonMode::ReportFile);
    }

    #[test]
    fn binding_catalog_exposes_core_xtal_wasm_and_platform_planes() {
        let ids = CliAdapter::list_bindings()
            .into_iter()
            .map(|binding| binding.id)
            .collect::<Vec<_>>();

        for required in [
            "project.init.xtal-pure",
            "spec.scaffold",
            "spec.check",
            "tests.gen.check",
            "impl.sync.write",
            "impl.sync.patchset",
            "xtal.dev",
            "xtal.verify",
            "xtal.repair",
            "xtal.certify",
            "xtal.ingest",
            "xtal.improve",
            "check.ast",
            "check.project",
            "wasm.web_ui.build",
            "wasm.web_ui.serve",
            "wasm.web_ui.test",
            "wasm.device.build",
            "wasm.device.package",
            "wasm.device.run.desktop_smoke",
            "wasm.device.provenance.attest",
            "wasm.device.provenance.verify",
            "wasm.workload.build",
            "wasm.workload.inspect",
            "wasm.deploy.plan",
            "lp.release.query",
            "lp.rollout.status",
        ] {
            assert!(ids.contains(&required), "missing {required}");
        }
    }

    #[test]
    fn platform_bindings_use_stdout_json_mode() {
        let binding = binding_by_id("lp.release.query").expect("binding exists");

        assert_eq!(binding.machine_json, MachineJsonMode::StdoutOnly);
    }
}
