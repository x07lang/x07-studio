//! Architect-role semantic enrichment for scaffolded specs.
//!
//! `x07 xtal spec scaffold` produces a spec with an empty `doc` field and
//! empty `requires`/`ensures` arrays. The downstream coder agent only has a
//! `bytes -> bytes` signature to grip, so on prompts with rich semantics
//! ("normalize-and-casefold text helper") it emits a passthrough impl and
//! the reviewer keeps voting `revise`.
//!
//! This module owns the deterministic floor: when the intent heuristic
//! routes to a recognised `(module_id, entry)` archetype, we know what the
//! operation is supposed to do. We fill the operation's `doc` string with a
//! concrete behaviour description so the coder agent has something to
//! implement against.
//!
//! Future cycles can extend this with predicate-based `ensures` (e.g.
//! length bounds the prover can check) and `ensures_props` references to
//! generated property tests. The current pass intentionally limits itself
//! to the `doc` field — predicates are gated by `spec.check`, and a wrong
//! predicate would break the whole flow.

use camino::Utf8Path;
use serde_json::Value;

/// Description of what an archetype's operation is supposed to do.
///
/// `doc` lands directly in the spec's `operations[].doc` field. Any other
/// fields (predicates, property references) would land in their respective
/// JSON slots when we add them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchetypeSemantic {
    pub doc: &'static str,
}

/// Look up the semantic descriptor for a given `(module_id, entry)` pair.
///
/// The lookup is exact. Keys mirror the routing decisions made in
/// `intent_packet_from_raw` — adding a new archetype to the heuristic
/// without adding it here is a no-op (the spec stays unenriched).
pub fn archetype_for(module_id: &str, entry: &str) -> Option<ArchetypeSemantic> {
    let key = (module_id, entry);
    Some(match key {
        ("app.text", "normalize_v1") => ArchetypeSemantic {
            doc: "Normalize UTF-8 bytes to canonical NFC form, then casefold so the result \
                  is suitable for case-insensitive comparison. Return the normalized bytes. \
                  Reject input that is not valid UTF-8 with a structured error (do not \
                  return the input unchanged).",
        },
        ("app.checksum", "digest_v1") => ArchetypeSemantic {
            doc: "Compute a deterministic fixed-size digest over the input bytes. The same \
                  input must always produce the same digest. Different inputs should produce \
                  different digests with high probability.",
        },
        ("app.codec", "roundtrip_v1") => ArchetypeSemantic {
            doc: "Encode the input payload and decode it again. The decoded bytes must equal \
                  the original input bytes. Reject malformed payloads with a structured error.",
        },
        ("app.compress", "roundtrip_v1") => ArchetypeSemantic {
            doc: "Compress the input bytes, then decompress them. The decompressed output must \
                  equal the original input exactly. The intermediate compressed bytes are not \
                  required to be shorter than the input.",
        },
        ("toy.sorter", "sort_u8_asc") => ArchetypeSemantic {
            doc: "Return a new byte string containing the bytes of the input in ascending \
                  order. Equal bytes preserve their original relative order (stable sort). \
                  Output length equals input length.",
        },
        ("app.greeter", "greet_v1") => ArchetypeSemantic {
            doc: "Produce a greeting message as UTF-8 bytes from the input payload. The output \
                  is always non-empty and valid UTF-8.",
        },
        ("app.calculator", "compute_v1") => ArchetypeSemantic {
            doc: "Compute the arithmetic result described by the input payload and return it \
                  as bytes. Reject malformed input with a structured error.",
        },
        ("app.parser", "parse_v1") => ArchetypeSemantic {
            doc: "Parse the input bytes according to the agreed grammar. Return the parsed \
                  structure encoded as bytes. Reject malformed input with a structured error.",
        },
        ("app.validator", "validate_v1") => ArchetypeSemantic {
            doc: "Validate the input payload against the agreed schema. Return a structured \
                  status indicating pass or fail. Do not mutate the input.",
        },
        ("app.cli", "run_v1") => ArchetypeSemantic {
            doc: "Interpret the input payload as a command request and return the command's \
                  result as bytes. Reject unknown commands with a structured error.",
        },
        ("app.service", "handle_v1") => ArchetypeSemantic {
            doc: "Handle the request encoded in the input payload and return the response as \
                  bytes. Reject malformed requests with a structured error.",
        },
        _ => return None,
    })
}

/// Result of an enrichment pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrichmentReport {
    pub spec_path: String,
    pub module_id: String,
    pub entry: String,
    pub doc_added: bool,
    pub archetype_recognized: bool,
}

impl EnrichmentReport {
    pub fn fields_added(&self) -> u32 {
        let mut count = 0;
        if self.doc_added {
            count += 1;
        }
        count
    }
}

/// Merge an archetype's semantic descriptor into a scaffolded x07 spec JSON
/// document. The merge is conservative: it only fills empty fields, never
/// overwrites existing content.
///
/// Returns `true` if the document was mutated.
pub fn merge_semantic_into_spec(
    spec_value: &mut Value,
    operation_name: &str,
    semantic: &ArchetypeSemantic,
) -> bool {
    let mut mutated = false;
    let Some(operations) = spec_value
        .get_mut("operations")
        .and_then(Value::as_array_mut)
    else {
        return false;
    };
    for op in operations.iter_mut() {
        let name_matches = op
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name == operation_name);
        if !name_matches {
            continue;
        }
        let current_doc = op.get("doc").and_then(Value::as_str).unwrap_or("");
        if current_doc.trim().is_empty() {
            op.as_object_mut()
                .expect("operation entry is JSON object")
                .insert("doc".to_string(), Value::String(semantic.doc.to_string()));
            mutated = true;
        }
        break;
    }
    mutated
}

/// Read the scaffolded spec at `spec_path`, merge in archetype semantics
/// (if recognised), and write back. Idempotent: calling twice produces no
/// additional changes.
///
/// `root` is the workspace root. `spec_relative_path` is workspace-relative
/// (e.g. `spec/app.text.x07spec.json`).
pub fn enrich_scaffolded_spec(
    root: &Utf8Path,
    spec_relative_path: &str,
    module_id: &str,
    entry: &str,
) -> anyhow::Result<EnrichmentReport> {
    let operation_name = format!("{module_id}.{entry}");
    let mut report = EnrichmentReport {
        spec_path: spec_relative_path.to_string(),
        module_id: module_id.to_string(),
        entry: entry.to_string(),
        doc_added: false,
        archetype_recognized: false,
    };
    let Some(semantic) = archetype_for(module_id, entry) else {
        return Ok(report);
    };
    report.archetype_recognized = true;
    let absolute = root.join(spec_relative_path);
    let raw = match std::fs::read_to_string(&absolute) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(report);
        }
        Err(error) => {
            return Err(anyhow::anyhow!(
                "failed to read scaffolded spec at `{absolute}`: {error}"
            ));
        }
    };
    let mut spec_value: Value = serde_json::from_str(&raw).map_err(|error| {
        anyhow::anyhow!("scaffolded spec `{absolute}` is not valid JSON: {error}")
    })?;
    let mutated = merge_semantic_into_spec(&mut spec_value, &operation_name, &semantic);
    if !mutated {
        return Ok(report);
    }
    report.doc_added = true;
    let mut serialized = serde_json::to_string_pretty(&spec_value)
        .map_err(|error| anyhow::anyhow!("failed to re-serialize enriched spec: {error}"))?;
    serialized.push('\n');
    std::fs::write(&absolute, serialized).map_err(|error| {
        anyhow::anyhow!("failed to write enriched spec to `{absolute}`: {error}")
    })?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use serde_json::json;
    use uuid::Uuid;

    fn fresh_root() -> Utf8PathBuf {
        let path =
            std::env::temp_dir().join(format!("x07-studio-architect-test-{}", Uuid::new_v4()));
        Utf8PathBuf::from_path_buf(path).expect("utf8 temp path")
    }

    #[test]
    fn archetype_lookup_recognizes_text_normalize() {
        let descriptor = archetype_for("app.text", "normalize_v1").expect("archetype known");
        assert!(descriptor.doc.contains("NFC"));
        assert!(descriptor.doc.contains("casefold"));
    }

    #[test]
    fn archetype_lookup_returns_none_for_unknown() {
        assert!(archetype_for("app.unknown", "do_thing_v1").is_none());
        assert!(archetype_for("app.main", "run_v1").is_none());
    }

    #[test]
    fn merge_fills_empty_doc_field() {
        let mut spec = json!({
            "schema_version": "x07.x07spec@0.1.0",
            "module_id": "app.text",
            "operations": [{
                "id": "op.normalize_v1.v1",
                "name": "app.text.normalize_v1",
                "doc": "",
                "params": [{"name": "payload", "ty": "bytes"}],
                "result": "bytes",
                "requires": [],
                "ensures": [],
                "ensures_props": [],
                "invariant": [],
            }],
        });
        let semantic = archetype_for("app.text", "normalize_v1").unwrap();
        let mutated = merge_semantic_into_spec(&mut spec, "app.text.normalize_v1", &semantic);
        assert!(mutated);
        let doc = spec["operations"][0]["doc"].as_str().unwrap();
        assert!(doc.contains("NFC"));
    }

    #[test]
    fn merge_preserves_existing_doc() {
        let mut spec = json!({
            "operations": [{
                "name": "app.text.normalize_v1",
                "doc": "Custom user-authored doc that the architect must not clobber.",
            }],
        });
        let semantic = archetype_for("app.text", "normalize_v1").unwrap();
        let mutated = merge_semantic_into_spec(&mut spec, "app.text.normalize_v1", &semantic);
        assert!(!mutated);
        assert_eq!(
            spec["operations"][0]["doc"].as_str().unwrap(),
            "Custom user-authored doc that the architect must not clobber."
        );
    }

    #[test]
    fn merge_skips_when_operation_name_does_not_match() {
        let mut spec = json!({
            "operations": [{
                "name": "app.text.different_op_v1",
                "doc": "",
            }],
        });
        let semantic = archetype_for("app.text", "normalize_v1").unwrap();
        let mutated = merge_semantic_into_spec(&mut spec, "app.text.normalize_v1", &semantic);
        assert!(!mutated);
    }

    #[test]
    fn enrich_scaffolded_spec_is_idempotent() {
        let root = fresh_root();
        let spec_relative = "spec/app.text.x07spec.json";
        std::fs::create_dir_all(root.join("spec")).expect("create spec dir");
        let scaffolded = json!({
            "schema_version": "x07.x07spec@0.1.0",
            "module_id": "app.text",
            "operations": [{
                "id": "op.normalize_v1.v1",
                "name": "app.text.normalize_v1",
                "doc": "",
                "params": [{"name": "payload", "ty": "bytes"}],
                "result": "bytes",
                "requires": [],
                "ensures": [],
                "ensures_props": [],
                "invariant": [],
            }],
            "sorts": [],
            "assumptions": [],
        });
        std::fs::write(
            root.join(spec_relative),
            serde_json::to_string_pretty(&scaffolded).unwrap(),
        )
        .unwrap();

        let first =
            enrich_scaffolded_spec(&root, spec_relative, "app.text", "normalize_v1").unwrap();
        assert!(first.archetype_recognized);
        assert!(first.doc_added);

        let on_disk: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(spec_relative)).unwrap())
                .unwrap();
        assert!(on_disk["operations"][0]["doc"]
            .as_str()
            .unwrap()
            .contains("NFC"));

        let second =
            enrich_scaffolded_spec(&root, spec_relative, "app.text", "normalize_v1").unwrap();
        assert!(second.archetype_recognized);
        assert!(!second.doc_added);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn enrich_scaffolded_spec_returns_unrecognized_for_unknown_archetype() {
        let root = fresh_root();
        let report =
            enrich_scaffolded_spec(&root, "spec/missing.x07spec.json", "app.main", "run_v1")
                .unwrap();
        assert!(!report.archetype_recognized);
        assert!(!report.doc_added);
    }

    #[test]
    fn enrich_scaffolded_spec_tolerates_missing_file() {
        let root = fresh_root();
        let report = enrich_scaffolded_spec(
            &root,
            "spec/missing.x07spec.json",
            "app.text",
            "normalize_v1",
        )
        .unwrap();
        assert!(report.archetype_recognized);
        assert!(!report.doc_added);
    }
}
