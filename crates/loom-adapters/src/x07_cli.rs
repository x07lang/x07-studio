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
        let execution = if let Some(stdin) = vars.get("stdin") {
            self.runner
                .run_with_stdin(
                    &self.root,
                    &program,
                    &rendered.args,
                    &BTreeMap::new(),
                    stdin,
                )
                .await
        } else {
            self.runner
                .run(&self.root, &program, &rendered.args, &BTreeMap::new())
                .await
        }
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
        id: "gen.verify",
        category: "x07/gen",
        program: ProgramKey::X07,
        args: &["gen", "verify", "--index", "arch/gen/index.x07gen.json"],
        artifacts: &["gen/"],
        notes: "Verify generated artifacts against the generator index.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "test.manifest",
        category: "x07/test",
        program: ProgramKey::X07,
        args: &["test", "--manifest", "tests/tests.json"],
        artifacts: &["target/x07test"],
        notes: "Run the project test manifest.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "test.xtal.generated.all",
        category: "x07/test",
        program: ProgramKey::X07,
        args: &[
            "test",
            "--all",
            "--no-fail-fast",
            "--manifest",
            "gen/xtal/tests.json",
        ],
        artifacts: &["target/x07test"],
        notes: "Run generated XTAL examples and properties.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "test.sm.generated",
        category: "x07/test",
        program: ProgramKey::X07,
        args: &["test", "--manifest", "gen/sm/tests.manifest.json"],
        artifacts: &["target/x07test"],
        notes: "Run generated state-machine tests.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "sm.gen.write",
        category: "x07/sm",
        program: ProgramKey::X07,
        args: &[
            "sm",
            "gen",
            "--input",
            "arch/sm/specs/lifecycle.sm.json",
            "--out",
            "gen/sm",
            "--write",
        ],
        artifacts: &["gen/sm"],
        notes: "Generate state-machine implementation and tests.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "arch.check.write_lock",
        category: "x07/arch",
        program: ProgramKey::X07,
        args: &["arch", "check", "--write-lock"],
        artifacts: &["arch/manifest.lock.json", "arch/contracts.lock.json"],
        notes: "Check architecture contracts and refresh locks.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "pkg.lock",
        category: "x07/package",
        program: ProgramKey::X07,
        args: &["pkg", "lock", "--project", "x07.json"],
        artifacts: &["x07.lock.json"],
        notes: "Resolve and write the project lockfile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "pkg.lock.atlas.frontend",
        category: "x07/package",
        program: ProgramKey::X07,
        args: &["pkg", "lock", "--project", "frontend/x07.json"],
        artifacts: &["frontend/x07.lock.json"],
        notes: "Resolve the x07 Atlas frontend project lockfile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "run.sandbox",
        category: "x07/run",
        program: ProgramKey::X07,
        args: &["run", "--profile", "sandbox"],
        artifacts: &["target/x07run"],
        notes: "Run the project through the sandbox profile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "run.sandbox.os",
        category: "x07/run",
        program: ProgramKey::X07,
        args: &[
            "run",
            "--profile",
            "sandbox",
            "--sandbox-backend",
            "os",
            "--i-accept-weaker-isolation",
        ],
        artifacts: &["target/x07run"],
        notes: "Run the sandbox profile with explicit OS-backed weaker isolation.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "run.stdin",
        category: "x07/run",
        program: ProgramKey::X07,
        args: &["run", "--stdin"],
        artifacts: &["target/x07run"],
        notes: "Run the project with stdin supplied by Studio.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "run.sandbox.stdin",
        category: "x07/run",
        program: ProgramKey::X07,
        args: &["run", "--profile", "sandbox", "--stdin"],
        artifacts: &["target/x07run"],
        notes: "Run the sandbox profile with stdin supplied by Studio.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "run.sandbox.stdin.os",
        category: "x07/run",
        program: ProgramKey::X07,
        args: &[
            "run",
            "--profile",
            "sandbox",
            "--sandbox-backend",
            "os",
            "--i-accept-weaker-isolation",
            "--stdin",
        ],
        artifacts: &["target/x07run"],
        notes: "Run the sandbox profile with Studio stdin and explicit OS-backed weaker isolation.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "bundle.api_gateway.sandbox",
        category: "x07/bundle",
        program: ProgramKey::X07,
        args: &[
            "bundle",
            "--profile",
            "sandbox",
            "--out",
            "dist/x07-api-gateway",
        ],
        artifacts: &["dist/x07-api-gateway"],
        notes: "Bundle the API gateway sandbox executable.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "bundle.api_gateway.sandbox.os",
        category: "x07/bundle",
        program: ProgramKey::X07,
        args: &[
            "bundle",
            "--profile",
            "sandbox",
            "--sandbox-backend",
            "os",
            "--i-accept-weaker-isolation",
            "--out",
            "dist/x07-api-gateway",
        ],
        artifacts: &["dist/x07-api-gateway"],
        notes:
            "Bundle the API gateway sandbox executable with explicit OS-backed weaker isolation.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "bundle.dbguard.sandbox",
        category: "x07/bundle",
        program: ProgramKey::X07,
        args: &["bundle", "--profile", "sandbox", "--out", "dist/x07dbguard"],
        artifacts: &["dist/x07dbguard"],
        notes: "Bundle the DB drift guard sandbox executable.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "bundle.dbguard.sandbox.os",
        category: "x07/bundle",
        program: ProgramKey::X07,
        args: &[
            "bundle",
            "--profile",
            "sandbox",
            "--sandbox-backend",
            "os",
            "--i-accept-weaker-isolation",
            "--out",
            "dist/x07dbguard",
        ],
        artifacts: &["dist/x07dbguard"],
        notes:
            "Bundle the DB drift guard sandbox executable with explicit OS-backed weaker isolation.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "run.x07crawl.sandbox",
        category: "x07/run",
        program: ProgramKey::X07,
        args: &[
            "run",
            "--profile",
            "sandbox",
            "--",
            "--mode",
            "replay",
            "--out",
            "out/crawl.json",
        ],
        artifacts: &["out/crawl.json"],
        notes: "Run the x07crawl replay flow through the sandbox profile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "run.x07crawl.sandbox.os",
        category: "x07/run",
        program: ProgramKey::X07,
        args: &[
            "run",
            "--profile",
            "sandbox",
            "--sandbox-backend",
            "os",
            "--i-accept-weaker-isolation",
            "--",
            "--mode",
            "replay",
            "--out",
            "out/crawl.json",
        ],
        artifacts: &["out/crawl.json"],
        notes: "Run x07crawl replay with explicit OS-backed weaker sandbox isolation.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "bundle.x07crawl.sandbox",
        category: "x07/bundle",
        program: ProgramKey::X07,
        args: &["bundle", "--profile", "sandbox", "--out", "dist/x07crawl"],
        artifacts: &["dist/x07crawl"],
        notes: "Bundle the x07crawl sandbox executable.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "bundle.x07crawl.sandbox.os",
        category: "x07/bundle",
        program: ProgramKey::X07,
        args: &[
            "bundle",
            "--profile",
            "sandbox",
            "--sandbox-backend",
            "os",
            "--i-accept-weaker-isolation",
            "--out",
            "dist/x07crawl",
        ],
        artifacts: &["dist/x07crawl"],
        notes: "Bundle x07crawl with explicit OS-backed weaker sandbox isolation.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.profile.validate.atlas_dev",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &["app", "profile", "validate", "--profile", "atlas_dev"],
        artifacts: &[],
        notes: "Validate the x07 Atlas app development profile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.contracts.validate",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &["app", "contracts", "validate"],
        artifacts: &[],
        notes: "Validate x07-wasm app contract fixtures.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.web_ui.contracts.validate",
        category: "x07/wasm/web-ui",
        program: ProgramKey::X07Wasm,
        args: &["web-ui", "contracts", "validate"],
        artifacts: &[],
        notes: "Validate x07-wasm web-ui contract fixtures.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.http.contracts.validate",
        category: "x07/wasm/http",
        program: ProgramKey::X07Wasm,
        args: &["http", "contracts", "validate"],
        artifacts: &[],
        notes: "Validate x07-wasm HTTP reducer contract fixtures.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.caps.validate.atlas_release",
        category: "x07/wasm/caps",
        program: ProgramKey::X07Wasm,
        args: &[
            "caps",
            "validate",
            "--profile",
            "arch/app/ops/caps_release.json",
        ],
        artifacts: &["arch/app/ops/caps_release.json"],
        notes: "Validate the x07 Atlas release capability profile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.ops.validate",
        category: "x07/wasm/ops",
        program: ProgramKey::X07Wasm,
        args: &["ops", "validate"],
        artifacts: &["arch/app/ops/index.x07ops.json"],
        notes: "Validate app ops profiles and policy cards.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.slo.validate.atlas",
        category: "x07/wasm/slo",
        program: ProgramKey::X07Wasm,
        args: &["slo", "validate", "--profile", "arch/slo/slo_min.json"],
        artifacts: &["arch/slo/slo_min.json"],
        notes: "Validate the x07 Atlas SLO profile.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.build.atlas_dev",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &[
            "app",
            "build",
            "--profile",
            "atlas_dev",
            "--clean",
            "--out-dir",
            "dist/showcase_fullstack/app.atlas_dev",
        ],
        artifacts: &["dist/showcase_fullstack/app.atlas_dev"],
        notes: "Build the x07 Atlas development app bundle.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.serve.smoke.atlas_dev",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &[
            "app",
            "serve",
            "--dir",
            "dist/showcase_fullstack/app.atlas_dev",
            "--mode",
            "smoke",
        ],
        artifacts: &["dist/showcase_fullstack/app.atlas_dev"],
        notes: "Run the x07 Atlas development app serve smoke check.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.test.happy_path",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &[
            "app",
            "test",
            "--dir",
            "dist/showcase_fullstack/app.atlas_dev",
            "--trace",
            "tests/traces/happy_path.trace.json",
        ],
        artifacts: &["tests/traces/happy_path.trace.json"],
        notes: "Replay the x07 Atlas happy-path app trace.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.test.validation_error",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &[
            "app",
            "test",
            "--dir",
            "dist/showcase_fullstack/app.atlas_dev",
            "--trace",
            "tests/traces/validation_error.trace.json",
        ],
        artifacts: &["tests/traces/validation_error.trace.json"],
        notes: "Replay the x07 Atlas validation-error app trace.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.test.regress.atlas_incident",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &[
            "app",
            "test",
            "--dir",
            "dist/showcase_fullstack/app.atlas_dev",
            "--trace",
            "tests/regress/atlas_incident.trace.json",
        ],
        artifacts: &["tests/regress/atlas_incident.trace.json"],
        notes: "Replay the checked-in x07 Atlas incident regression trace.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.build.atlas_release",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &[
            "app",
            "build",
            "--profile",
            "atlas_release",
            "--clean",
            "--out-dir",
            "dist/showcase_fullstack/app.atlas_release",
        ],
        artifacts: &["dist/showcase_fullstack/app.atlas_release"],
        notes: "Build the x07 Atlas release app bundle.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.pack.atlas_release",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &[
            "app",
            "pack",
            "--bundle-manifest",
            "dist/showcase_fullstack/app.atlas_release/app.bundle.json",
            "--profile-id",
            "atlas_release",
            "--out-dir",
            "dist/showcase_fullstack/pack.atlas_release",
        ],
        artifacts: &["dist/showcase_fullstack/pack.atlas_release"],
        notes: "Pack the x07 Atlas release app bundle.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.app.verify.atlas_release",
        category: "x07/wasm/app",
        program: ProgramKey::X07Wasm,
        args: &[
            "app",
            "verify",
            "--pack-manifest",
            "dist/showcase_fullstack/pack.atlas_release/app.pack.json",
        ],
        artifacts: &["dist/showcase_fullstack/pack.atlas_release/app.pack.json"],
        notes: "Verify the x07 Atlas release app pack.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.provenance.attest.atlas_release",
        category: "x07/wasm/provenance",
        program: ProgramKey::X07Wasm,
        args: &[
            "provenance",
            "attest",
            "--pack-manifest",
            "dist/showcase_fullstack/pack.atlas_release/app.pack.json",
            "--ops",
            "arch/app/ops/ops_release.json",
            "--signing-key",
            "arch/provenance/dev.ed25519.signing_key.b64",
            "--out",
            "dist/showcase_fullstack/pack.atlas_release/app.provenance.dsse.json",
        ],
        artifacts: &["dist/showcase_fullstack/pack.atlas_release/app.provenance.dsse.json"],
        notes: "Attest x07 Atlas release pack provenance with the docs test key.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.provenance.verify.atlas_release",
        category: "x07/wasm/provenance",
        program: ProgramKey::X07Wasm,
        args: &[
            "provenance",
            "verify",
            "--attestation",
            "dist/showcase_fullstack/pack.atlas_release/app.provenance.dsse.json",
            "--pack-dir",
            "dist/showcase_fullstack/pack.atlas_release",
            "--trusted-public-key",
            "arch/provenance/dev.ed25519.public_key.b64",
        ],
        artifacts: &["dist/showcase_fullstack/pack.atlas_release/app.provenance.dsse.json"],
        notes: "Verify x07 Atlas release pack provenance.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.deploy.plan.atlas_release",
        category: "x07/wasm/deploy",
        program: ProgramKey::X07Wasm,
        args: &[
            "deploy",
            "plan",
            "--pack-manifest",
            "dist/showcase_fullstack/pack.atlas_release/app.pack.json",
            "--ops",
            "arch/app/ops/ops_release.json",
            "--out-dir",
            "dist/showcase_fullstack/deploy.atlas_release",
        ],
        artifacts: &["dist/showcase_fullstack/deploy.atlas_release"],
        notes: "Generate the x07 Atlas release deploy plan.",
        machine_json: MachineJsonMode::ReportFile,
    },
    BindingTemplate {
        id: "wasm.slo.eval.atlas_canary_ok",
        category: "x07/wasm/slo",
        program: ProgramKey::X07Wasm,
        args: &[
            "slo",
            "eval",
            "--profile",
            "arch/slo/slo_min.json",
            "--metrics",
            "tests/fixtures/metrics/atlas_canary_ok.json",
        ],
        artifacts: &["tests/fixtures/metrics/atlas_canary_ok.json"],
        notes: "Evaluate x07 Atlas SLOs against the canary fixture.",
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
        args: &["xtal", "ingest", "--input", "{input}", "--normalize-only"],
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
        args: &["release-query", "--release", "{release_id}"],
        artifacts: &[],
        notes: "Query hosted release state.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.release.rollback",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &[
            "release-rollback",
            "--release",
            "{release_id}",
            "--reason",
            "{reason}",
        ],
        artifacts: &[],
        notes: "Rollback a hosted release.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.deploy.accept.local",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &[
            "accept",
            "--target",
            "__local__",
            "--pack-manifest",
            "{pack_manifest}",
            "--pack-dir",
            "{pack_dir}",
            "--state-dir",
            "{state_dir}",
        ],
        artifacts: &["{state_dir}"],
        notes: "Accept a local deployment candidate from a verified pack manifest.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.deploy.run.local",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &[
            "run",
            "--target",
            "__local__",
            "--deployment",
            "{deployment_id}",
            "--plan",
            "{plan}",
            "--state-dir",
            "{state_dir}",
        ],
        artifacts: &["{state_dir}"],
        notes: "Run an accepted deployment locally from an x07 deploy plan.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.deploy.run.local.metrics",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &[
            "run",
            "--target",
            "__local__",
            "--deployment",
            "{deployment_id}",
            "--plan",
            "{plan}",
            "--metrics-dir",
            "{metrics_dir}",
            "--state-dir",
            "{state_dir}",
        ],
        artifacts: &["{state_dir}"],
        notes: "Run an accepted local deployment with explicit metrics evidence.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.deploy.query.local",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &[
            "query",
            "--target",
            "__local__",
            "--deployment",
            "{deployment_id}",
            "--view",
            "full",
            "--state-dir",
            "{state_dir}",
        ],
        artifacts: &["{state_dir}"],
        notes: "Query full local deployment state.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.deploy.status.local",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &[
            "status",
            "--target",
            "__local__",
            "--deployment",
            "{deployment_id}",
            "--state-dir",
            "{state_dir}",
        ],
        artifacts: &[],
        notes: "Inspect local deployment status.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.incident.list.local",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &[
            "incident-list",
            "--target",
            "__local__",
            "--deployment",
            "{deployment_id}",
            "--state-dir",
            "{state_dir}",
        ],
        artifacts: &["{state_dir}"],
        notes: "List local deployment incidents.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.regress.from_incident.local",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &[
            "regress-from-incident",
            "--target",
            "__local__",
            "--incident-id",
            "{incident_id}",
            "--name",
            "{regression_name}",
            "--out-dir",
            "{out_dir}",
            "--state-dir",
            "{state_dir}",
        ],
        artifacts: &["{out_dir}"],
        notes: "Create a local regression fixture from a platform incident.",
        machine_json: MachineJsonMode::StdoutOnly,
    },
    BindingTemplate {
        id: "lp.ui.serve.local",
        category: "x07/platform",
        program: ProgramKey::X07lp,
        args: &["ui-serve", "--state-dir", "{state_dir}", "--addr", "{addr}"],
        artifacts: &["{state_dir}"],
        notes: "Serve the local platform control-plane UI.",
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
            "gen.verify",
            "test.manifest",
            "test.xtal.generated.all",
            "test.sm.generated",
            "sm.gen.write",
            "arch.check.write_lock",
            "pkg.lock",
            "pkg.lock.atlas.frontend",
            "run.sandbox",
            "run.sandbox.os",
            "run.stdin",
            "run.sandbox.stdin",
            "run.sandbox.stdin.os",
            "bundle.api_gateway.sandbox",
            "bundle.api_gateway.sandbox.os",
            "bundle.dbguard.sandbox",
            "bundle.dbguard.sandbox.os",
            "run.x07crawl.sandbox",
            "run.x07crawl.sandbox.os",
            "bundle.x07crawl.sandbox",
            "bundle.x07crawl.sandbox.os",
            "wasm.app.profile.validate.atlas_dev",
            "wasm.app.contracts.validate",
            "wasm.web_ui.contracts.validate",
            "wasm.http.contracts.validate",
            "wasm.caps.validate.atlas_release",
            "wasm.ops.validate",
            "wasm.slo.validate.atlas",
            "wasm.app.build.atlas_dev",
            "wasm.app.serve.smoke.atlas_dev",
            "wasm.app.test.happy_path",
            "wasm.app.test.validation_error",
            "wasm.app.test.regress.atlas_incident",
            "wasm.app.build.atlas_release",
            "wasm.app.pack.atlas_release",
            "wasm.app.verify.atlas_release",
            "wasm.provenance.attest.atlas_release",
            "wasm.provenance.verify.atlas_release",
            "wasm.deploy.plan.atlas_release",
            "wasm.slo.eval.atlas_canary_ok",
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
            "lp.release.rollback",
            "lp.deploy.accept.local",
            "lp.deploy.run.local",
            "lp.deploy.run.local.metrics",
            "lp.deploy.query.local",
            "lp.deploy.status.local",
            "lp.incident.list.local",
            "lp.regress.from_incident.local",
            "lp.ui.serve.local",
        ] {
            assert!(ids.contains(&required), "missing {required}");
        }
    }

    #[test]
    fn platform_bindings_use_stdout_json_mode() {
        let binding = binding_by_id("lp.release.query").expect("binding exists");

        assert_eq!(binding.machine_json, MachineJsonMode::StdoutOnly);
    }

    #[test]
    fn platform_release_bindings_use_current_driver_shape() {
        let query = binding_by_id("lp.release.query").expect("binding exists");
        let rendered = query.render(&BTreeMap::from([(
            "release_id".to_string(),
            "rel_123".to_string(),
        )]));

        assert_eq!(rendered.program, "x07lp");
        assert_eq!(rendered.args, vec!["release-query", "--release", "rel_123"]);
        assert!(!rendered.args.iter().any(|arg| arg.contains('{')));

        let rollback = binding_by_id("lp.release.rollback").expect("binding exists");
        let rendered = rollback.render(&BTreeMap::from([
            ("release_id".to_string(), "rel_123".to_string()),
            ("reason".to_string(), "failed canary".to_string()),
        ]));

        assert_eq!(
            rendered.args,
            vec![
                "release-rollback",
                "--release",
                "rel_123",
                "--reason",
                "failed canary",
            ]
        );
        assert!(!rendered.args.iter().any(|arg| arg.contains('{')));
    }

    #[test]
    fn platform_local_delivery_bindings_use_current_driver_shape() {
        let accept = binding_by_id("lp.deploy.accept.local").expect("binding exists");
        let rendered = accept.render(&BTreeMap::from([
            (
                "pack_manifest".to_string(),
                "dist/pack/app.pack.json".to_string(),
            ),
            ("pack_dir".to_string(), "dist/pack".to_string()),
            ("state_dir".to_string(), ".x07/platform".to_string()),
        ]));

        assert_eq!(
            rendered.args,
            vec![
                "accept",
                "--target",
                "__local__",
                "--pack-manifest",
                "dist/pack/app.pack.json",
                "--pack-dir",
                "dist/pack",
                "--state-dir",
                ".x07/platform",
            ]
        );
        assert_eq!(rendered.artifacts, vec![".x07/platform"]);
        assert!(!rendered.args.iter().any(|arg| arg.contains('{')));

        let run = binding_by_id("lp.deploy.run.local.metrics").expect("binding exists");
        let rendered = run.render(&BTreeMap::from([
            ("deployment_id".to_string(), "lpexec_example".to_string()),
            (
                "plan".to_string(),
                "dist/deploy/deploy.plan.json".to_string(),
            ),
            (
                "metrics_dir".to_string(),
                "tests/fixtures/metrics".to_string(),
            ),
            ("state_dir".to_string(), ".x07/platform".to_string()),
        ]));

        assert_eq!(
            rendered.args,
            vec![
                "run",
                "--target",
                "__local__",
                "--deployment",
                "lpexec_example",
                "--plan",
                "dist/deploy/deploy.plan.json",
                "--metrics-dir",
                "tests/fixtures/metrics",
                "--state-dir",
                ".x07/platform",
            ]
        );
        assert!(!rendered.args.iter().any(|arg| arg.contains('{')));

        let query = binding_by_id("lp.deploy.query.local").expect("binding exists");
        let rendered = query.render(&BTreeMap::from([
            ("deployment_id".to_string(), "lpexec_example".to_string()),
            ("state_dir".to_string(), ".x07/platform".to_string()),
        ]));

        assert_eq!(
            rendered.args,
            vec![
                "query",
                "--target",
                "__local__",
                "--deployment",
                "lpexec_example",
                "--view",
                "full",
                "--state-dir",
                ".x07/platform",
            ]
        );
        assert!(!rendered.args.iter().any(|arg| arg.contains('{')));
    }
}
