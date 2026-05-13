use std::fs;

use anyhow::{anyhow, bail, Context};
use camino::Utf8Path;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use loom_types::api::{AgentContract, ContractSection};
use loom_types::artifacts::IntentPacket;

const CONTRACT_PATH: &str = "AGENT.md";

pub fn read(
    root: &Utf8Path,
    session_id: Uuid,
    intent: Option<&IntentPacket>,
) -> anyhow::Result<AgentContract> {
    let path = root.join(CONTRACT_PATH);
    let exists = path.is_file();
    let markdown = if exists {
        fs::read_to_string(&path).with_context(|| format!("read {path}"))?
    } else if let Some(intent) = intent {
        render_template(intent)
    } else {
        render_empty_template()
    };
    Ok(AgentContract {
        schema_version: "x07.studio.agent_contract@0.1.0".to_string(),
        session_id,
        path: CONTRACT_PATH.to_string(),
        exists,
        sections: parse_sections(&markdown),
        last_modified: if exists {
            fs::metadata(&path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis().to_string())
        } else {
            None
        },
        hash: hash_markdown(&markdown),
        markdown,
    })
}

pub fn write(root: &Utf8Path, markdown: &str, prior_hash: Option<&str>) -> anyhow::Result<()> {
    if markdown.trim().is_empty() {
        bail!("AGENT.md cannot be empty");
    }
    let path = root.join(CONTRACT_PATH);
    if let Some(expected) = prior_hash {
        if path.is_file() {
            let existing = fs::read_to_string(&path).with_context(|| format!("read {path}"))?;
            let actual = hash_markdown(&existing);
            if actual != expected {
                bail!("AGENT.md changed on disk; reload before saving");
            }
        }
    }
    let tmp = root.join(format!(".{CONTRACT_PATH}.{}.tmp", Uuid::new_v4()));
    fs::write(&tmp, markdown).with_context(|| format!("write {tmp}"))?;
    fs::rename(&tmp, &path).with_context(|| format!("rename {tmp} to {path}"))?;
    Ok(())
}

pub fn render_template(intent: &IntentPacket) -> String {
    let purpose = intent_text(intent);
    let targets = if intent.targets.is_empty() {
        "- core (pure)\n- adapters (world-specific)\n- entrypoints (thin glue)".to_string()
    } else {
        intent
            .targets
            .iter()
            .map(|target| {
                let entry = target.entry.as_deref().unwrap_or("entrypoint");
                format!("- `{}` -> `{entry}`", target.module_id)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let constraints = render_bullets(
        intent
            .constraints
            .iter()
            .chain(intent.policy_implications.iter())
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "- Keep the XTAL spec-first lifecycle visible.",
    );
    let forbidden = render_bullets(
        intent
            .witnesses
            .iter()
            .filter(|witness| {
                matches!(
                    witness.kind,
                    loom_types::artifacts::WitnessKind::ForbiddenBehavior
                        | loom_types::artifacts::WitnessKind::PolicyRequirement
                )
            })
            .map(|witness| witness.text.as_str())
            .collect::<Vec<_>>(),
        "- Do not widen specs, architecture, worlds, capabilities, or budgets without review.",
    );

    format!(
        "# AGENT.md\n\n\
         ## Purpose\n\
         {purpose}\n\n\
         ## Non-goals\n\
         - Do not bypass canonical x07 CLI, MCP, or platform contracts.\n\
         - Do not turn natural language directly into unchecked source.\n\n\
         ## Invariants\n\
         {constraints}\n\n\
         ## Module map (ports & adapters)\n\
         {targets}\n\n\
         ## Tooling commands\n\
         - format: `x07 fmt --input <file.x07.json> --check`\n\
         - lint: `x07 lint --input <file.x07.json>`\n\
         - test: `x07 test --manifest tests/tests.json`\n\
         - verify: `x07 xtal verify`\n\n\
         ## Budgets / gates\n\
         - Keep solve-pure deterministic unless a reviewed profile allows OS access.\n\
         - Run `x07 pkg lock --project x07.json --check` when dependencies change.\n\
         - Run `x07 arch check` before Shareable or stricter rungs.\n\n\
         ## Forbidden changes\n\
         {forbidden}\n"
    )
}

pub fn parse_sections(markdown: &str) -> Vec<ContractSection> {
    let mut sections = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_body = Vec::new();
    for line in markdown.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            if let Some(title) = current_title.replace(title.trim().to_string()) {
                sections.push(ContractSection {
                    title,
                    body: current_body.join("\n").trim().to_string(),
                });
                current_body.clear();
            }
        } else if current_title.is_some() {
            current_body.push(line.to_string());
        }
    }
    if let Some(title) = current_title {
        sections.push(ContractSection {
            title,
            body: current_body.join("\n").trim().to_string(),
        });
    }
    if sections.is_empty() && !markdown.trim().is_empty() {
        sections.push(ContractSection {
            title: "Body".to_string(),
            body: markdown.trim().to_string(),
        });
    }
    sections
}

fn render_empty_template() -> String {
    "# AGENT.md\n\n\
     ## Purpose\n\
     Describe the x07 project and the user-visible behavior it owns.\n\n\
     ## Non-goals\n\
     - Do not bypass canonical x07 CLI, MCP, or platform contracts.\n\n\
     ## Invariants\n\
     - Keep deterministic logic separated from world-specific adapters.\n\n\
     ## Module map (ports & adapters)\n\
     - core (pure)\n\
     - adapters (world-specific)\n\
     - entrypoints (thin glue)\n\n\
     ## Tooling commands\n\
     - format: `x07 fmt --input <file.x07.json> --check`\n\
     - lint: `x07 lint --input <file.x07.json>`\n\
     - test: `x07 test --manifest tests/tests.json`\n\
     - verify: `x07 xtal verify`\n\n\
     ## Budgets / gates\n\
     - Run `x07 arch check` before Shareable or stricter rungs.\n\n\
     ## Forbidden changes\n\
     - Do not widen specs, architecture, worlds, capabilities, or budgets without review.\n"
        .to_string()
}

fn render_bullets(items: Vec<&str>, fallback: &str) -> String {
    let bullets = items
        .into_iter()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>();
    if bullets.is_empty() {
        fallback.to_string()
    } else {
        bullets.join("\n")
    }
}

fn intent_text(intent: &IntentPacket) -> String {
    match &intent.source {
        loom_types::artifacts::IntentSource::Text { raw }
        | loom_types::artifacts::IntentSource::Spec { raw } => raw.clone(),
        loom_types::artifacts::IntentSource::Voice { transcript } => transcript.clone(),
        loom_types::artifacts::IntentSource::Incident { path }
        | loom_types::artifacts::IntentSource::Sketch { path } => {
            format!("Repair or explain artifact `{path}`.")
        }
        loom_types::artifacts::IntentSource::Image { path, .. } => {
            format!("Implement behavior captured by image witness `{path}`.")
        }
    }
}

fn hash_markdown(markdown: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(markdown.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn hash(markdown: &str) -> String {
    hash_markdown(markdown)
}

pub fn ensure_relative_agent_path(path: &str) -> anyhow::Result<()> {
    if path != CONTRACT_PATH {
        return Err(anyhow!("only AGENT.md is supported"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_sections, read, render_template, write};
    use loom_types::artifacts::IntentPacket;
    use uuid::Uuid;

    #[test]
    fn parses_markdown_sections() {
        let sections = parse_sections("# Title\n\n## Purpose\nA\n\n## Invariants\nB");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "Purpose");
        assert_eq!(sections[0].body, "A");
    }

    #[test]
    fn renders_template_from_intent_target() {
        let intent = IntentPacket::demo(Uuid::new_v4(), "/tmp/demo");
        let markdown = render_template(&intent);
        assert!(markdown.contains("app.sorter"));
        assert!(markdown.contains("reject empty input"));
    }

    #[test]
    fn read_returns_template_when_missing() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-agent-contract-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("mkdir");
        let session_id = Uuid::new_v4();
        let contract = read(root.as_path(), session_id, None).expect("read");
        assert!(!contract.exists);
        assert!(contract.markdown.contains("## Purpose"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn write_round_trips_agent_md() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-agent-write-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("mkdir");
        write(root.as_path(), "# AGENT.md\n\n## Purpose\nTest\n", None).expect("write");
        let contract = read(root.as_path(), Uuid::new_v4(), None).expect("read");
        assert!(contract.exists);
        assert_eq!(contract.sections[0].body, "Test");
        std::fs::remove_dir_all(root).ok();
    }
}
