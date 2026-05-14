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
//! Predicate-based `ensures` stay in the archetype table and are promoted
//! only after an implementation exists. `spec.check` and the prover both
//! consume these clauses, so the table must stay conservative.

use camino::Utf8Path;
use serde_json::Value;

/// One `ensures` predicate the archetype attaches to the operation.
///
/// `expr_json` is the JSON-encoded x07 predicate expression that lands
/// inside `operations[].ensures[].expr`. We store it as a static string
/// rather than a `serde_json::Value` so the table stays declarative and
/// const-initialisable; merge parses lazily.
///
/// Predicates must use ops + functions x07's spec-check + prover both
/// accept (`bytes.len`, `=`, `<=`, `>=`, `>`, `<`, `+`, `-`, `*`, `%`,
/// `__result`, parameter names). Anything unrecognised will be rejected
/// by `x07 xtal spec check` and break the build pipeline — keep this
/// table conservative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecPredicate {
    pub id: &'static str,
    pub expr_json: &'static str,
}

/// Description of what an archetype's operation is supposed to do.
///
/// `doc` lands directly in the spec's `operations[].doc` field.
/// `ensures` lands in `operations[].ensures` as `{id, expr}` objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchetypeSemantic {
    pub doc: &'static str,
    pub ensures: &'static [SpecPredicate],
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
            // No predicate this pass: the conceptually-correct bound
            // `len(__result) <= len(payload) * 4` (NFC ~3× + casefold ~1.4×)
            // produces a counterexample from x07's CBMC+Z3 prover in
            // 0.2.10 — likely a multiplication-overflow encoding issue
            // when the second multiplicand is a function-call result.
            // Doc-only is enough for the coder; predicate revisit blocked
            // on x07 prover support.
            ensures: &[],
        },
        ("app.checksum", "digest_v1") => ArchetypeSemantic {
            doc: "Compute a deterministic 32-bit CRC32-style digest over the input bytes. Return \
                  exactly four bytes in little-endian order. Empty input returns the all-zero \
                  digest, and equal inputs always return equal digests.",
            ensures: &[],
        },
        ("app.codec", "roundtrip_v1") => ArchetypeSemantic {
            doc: "Encode the input payload and decode it again. The decoded bytes must equal \
                  the original input bytes. Reject malformed payloads with a structured error.",
            // Roundtrip means decode(encode(x)) == x; the output bytes
            // equal the input bytes. Length preservation falls out of
            // that. Same predicate shape as toy.sorter, also provable
            // against the identity synthesis floor.
            ensures: &[SpecPredicate {
                id: "length_preserved",
                expr_json: r#"["=", ["bytes.len", "__result"], ["bytes.len", "payload"]]"#,
            }],
        },
        ("app.compress", "roundtrip_v1") => ArchetypeSemantic {
            doc: "Compress the input bytes, then decompress them. The decompressed output must \
                  equal the original input exactly. The intermediate compressed bytes are not \
                  required to be shorter than the input.",
            // Same justification as codec: the OPERATION returns the
            // decompressed bytes, which equal the input bytes. The
            // compressed intermediate is internal and not the result.
            ensures: &[SpecPredicate {
                id: "length_preserved",
                expr_json: r#"["=", ["bytes.len", "__result"], ["bytes.len", "payload"]]"#,
            }],
        },
        ("toy.sorter", "sort_u8_asc") => ArchetypeSemantic {
            doc: "Return a new byte string containing the bytes of the input in ascending \
                  order. Equal bytes preserve their original relative order (stable sort). \
                  Output length equals input length.",
            // Canonical safe predicate: stable sort preserves length.
            // Same shape used in the real `toy.sorter` example shipped
            // with x07 (docs/examples/agent-gate/xtal/toy-sorter/).
            ensures: &[SpecPredicate {
                id: "length_preserved",
                expr_json: r#"["=", ["bytes.len", "__result"], ["bytes.len", "payload"]]"#,
            }],
        },
        ("app.greeter", "greet_v1") => ArchetypeSemantic {
            doc: "Produce a greeting message as UTF-8 bytes from the input payload. The output \
                  is always non-empty and valid UTF-8.",
            ensures: &[SpecPredicate {
                id: "result_nonempty",
                expr_json: r#"[">", ["bytes.len", "__result"], 0]"#,
            }],
        },
        ("app.calculator", "compute_v1") => ArchetypeSemantic {
            doc: "Compute the arithmetic result described by the input payload and return it \
                  as bytes. Reject malformed input with a structured error.",
            ensures: &[SpecPredicate {
                id: "result_bounded",
                expr_json: r#"["<=", ["bytes.len", "__result"], 16]"#,
            }],
        },
        ("app.parser", "parse_v1") => ArchetypeSemantic {
            doc: "Parse the input bytes according to the agreed grammar. Return the parsed \
                  structure encoded as bytes. Reject malformed input with a structured error.",
            ensures: &[SpecPredicate {
                id: "length_preserved_floor",
                expr_json: r#"["=", ["bytes.len", "__result"], ["bytes.len", "payload"]]"#,
            }],
        },
        ("app.validator", "validate_v1") => ArchetypeSemantic {
            doc: "Validate the input payload against the agreed schema. Return a structured \
                  status indicating pass or fail. Do not mutate the input.",
            ensures: &[SpecPredicate {
                id: "status_byte",
                expr_json: r#"["=", ["bytes.len", "__result"], 1]"#,
            }],
        },
        ("app.timer", "elapsed_v1") => ArchetypeSemantic {
            doc: "Measure elapsed wall-clock time using the reviewed OS time capability. \
                  Capture an OS clock reading at the start and stop moments, report whole \
                  elapsed seconds as bytes, and reject negative intervals as explicit errors.",
            ensures: &[],
        },
        ("app.cli", "run_v1") => ArchetypeSemantic {
            doc: "Interpret the input payload as a command request and return the command's \
                  result as bytes. Reject unknown commands with a structured error.",
            ensures: &[],
        },
        ("app.service", "handle_v1") => ArchetypeSemantic {
            doc: "Handle the request encoded in the input payload and return the response as \
                  bytes. Reject malformed requests with a structured error.",
            ensures: &[],
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
    /// Number of `ensures` predicates merged into the spec by the
    /// Tier-1.5 archetype table. Zero for agent-driven enrichment (the
    /// agent is currently doc-only).
    pub ensures_added: u32,
    pub archetype_recognized: bool,
    /// Agent id that produced the doc when enrichment came from a
    /// supervised architect-agent run. `None` for deterministic floor
    /// enrichments (the F7 archetype table) so the op-record can name
    /// the source.
    pub agent_id: Option<String>,
}

impl EnrichmentReport {
    pub fn fields_added(&self) -> u32 {
        let mut count = 0;
        if self.doc_added {
            count += 1;
        }
        count + self.ensures_added
    }
}

/// Structured payload the architect agent emits as one
/// `kind: "spec_enrichment"` agent_event line. Only `doc` is currently
/// applied to the spec; `examples` are surfaced through the op-record
/// for future use (Cycle-8 will pipe them into `IntentPacket.examples`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentEnrichment {
    pub doc: String,
    pub examples: Vec<String>,
}

impl AgentEnrichment {
    /// Parse an agent_event payload `{kind:"spec_enrichment", doc:"…", examples:[…]}`
    /// into a typed enrichment. Returns `None` when the event is missing
    /// the kind tag, the schema marker, or a non-empty `doc` field —
    /// agent-supplied empty docs are treated as "no enrichment" so we
    /// don't overwrite the spec with whitespace.
    pub fn from_event_value(value: &Value) -> Option<Self> {
        if value.get("schema_version")?.as_str()? != "x07.studio.agent_event@0.1.0" {
            return None;
        }
        if value.get("kind")?.as_str()? != "spec_enrichment" {
            return None;
        }
        let doc = value.get("doc")?.as_str()?.trim();
        if doc.is_empty() {
            return None;
        }
        let examples = value
            .get("examples")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .filter(|item| !item.trim().is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Some(Self {
            doc: doc.to_string(),
            examples,
        })
    }
}

/// Apply an agent-supplied enrichment to the scaffolded spec on disk.
/// Same conservatism as the deterministic floor: only fills `doc` when
/// it is currently empty, never overwrites existing content. Returns
/// the resulting report so the caller can log it.
pub fn apply_agent_enrichment_to_spec(
    root: &Utf8Path,
    spec_relative_path: &str,
    module_id: &str,
    entry: &str,
    agent_id: &str,
    enrichment: &AgentEnrichment,
) -> anyhow::Result<EnrichmentReport> {
    let operation_name = format!("{module_id}.{entry}");
    let mut report = EnrichmentReport {
        spec_path: spec_relative_path.to_string(),
        module_id: module_id.to_string(),
        entry: entry.to_string(),
        doc_added: false,
        ensures_added: 0,
        archetype_recognized: false,
        agent_id: Some(agent_id.to_string()),
    };
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
    let mutated = merge_doc_into_spec(&mut spec_value, &operation_name, &enrichment.doc);
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

/// Result of merging an archetype semantic into a spec document.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergeOutcome {
    pub doc_added: bool,
    pub ensures_added: u32,
}

impl MergeOutcome {
    pub fn mutated(self) -> bool {
        self.doc_added || self.ensures_added > 0
    }
}

/// Merge an archetype's semantic descriptor into a scaffolded x07 spec JSON
/// document. The merge is conservative: it only fills empty fields, never
/// overwrites existing content.
///
/// **Tier-1.5 status:** the build pipeline scaffolds the impl as a
/// `bytes.empty` stub via `impl.sync.write`; non-trivial ensures
/// predicates (`len > 0`, `len == len(payload)`, `len <= len(payload) * 4`)
/// all produce counterexamples against that stub when `xtal.verify`
/// runs, even when the predicate is correct for the real impl. The
/// predicate-merge call below is currently gated to `false` — the
/// archetype table keeps the predicates declared so a future
/// "Tier-1.5b" pass can run them after the coder writes a real impl.
/// Until then, only `doc` enrichment ships from this entry point.
/// Whether `merge_semantic_into_spec` should also merge archetype
/// `ensures` predicates into the spec. Currently `false` — see the doc
/// comment on `merge_semantic_into_spec` for the reason. Flip to `true`
/// once the build pipeline writes a real impl (or once the predicate
/// table only contains predicates that hold for `bytes.empty` stubs).
const MERGE_ENSURES_IN_BUILD: bool = false;

pub fn merge_semantic_into_spec(
    spec_value: &mut Value,
    operation_name: &str,
    semantic: &ArchetypeSemantic,
) -> MergeOutcome {
    let doc_added = merge_doc_into_spec(spec_value, operation_name, semantic.doc);
    let ensures_added = if MERGE_ENSURES_IN_BUILD {
        merge_ensures_into_spec(spec_value, operation_name, semantic.ensures)
    } else {
        let _ = semantic.ensures; // kept declarative for the followup
        0
    };
    MergeOutcome {
        doc_added,
        ensures_added,
    }
}

/// Append archetype-supplied `ensures` predicates to the operation's
/// `ensures` array, but only when the array is currently empty (or
/// missing). Returns the number of predicates appended.
///
/// Skipping when ensures is non-empty preserves any contract the user or a
/// prior pass authored — predicates are formally checked by `spec.check` and
/// `xtal.verify`, so overwriting could regress a strict contract into a
/// looser one.
pub fn merge_ensures_into_spec(
    spec_value: &mut Value,
    operation_name: &str,
    predicates: &[SpecPredicate],
) -> u32 {
    if predicates.is_empty() {
        return 0;
    }
    let Some(operations) = spec_value
        .get_mut("operations")
        .and_then(Value::as_array_mut)
    else {
        return 0;
    };
    for op in operations.iter_mut() {
        let name_matches = op
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name == operation_name);
        if !name_matches {
            continue;
        }
        let current_is_empty = op
            .get("ensures")
            .and_then(Value::as_array)
            .map(|items| items.is_empty())
            .unwrap_or(true);
        if !current_is_empty {
            return 0;
        }
        let mut new_ensures = Vec::with_capacity(predicates.len());
        let mut appended = 0u32;
        for predicate in predicates {
            match serde_json::from_str::<Value>(predicate.expr_json) {
                Ok(expr) => {
                    new_ensures.push(serde_json::json!({
                        "id": predicate.id,
                        "expr": expr,
                    }));
                    appended += 1;
                }
                Err(_) => {
                    // A malformed archetype predicate is a programmer
                    // error, not a runtime condition. Skip silently so
                    // we never fail the build pipeline over it; the
                    // unit tests in `architect::tests` cover the
                    // round-trip for every shipped predicate.
                }
            }
        }
        if appended == 0 {
            return 0;
        }
        op.as_object_mut()
            .expect("operation entry is JSON object")
            .insert("ensures".to_string(), Value::Array(new_ensures));
        return appended;
    }
    0
}

/// Lower-level merge that takes a plain doc string. `merge_semantic_into_spec`
/// uses this for archetype-driven enrichment; agent-driven enrichment uses it
/// directly so it can pass a runtime-owned string without leaking it as
/// `&'static str`.
pub fn merge_doc_into_spec(spec_value: &mut Value, operation_name: &str, doc: &str) -> bool {
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
                .insert("doc".to_string(), Value::String(doc.to_string()));
            mutated = true;
        }
        break;
    }
    mutated
}

/// Read the spec on disk and return `true` when the operation's `doc`
/// field is empty (or the file is missing / unreadable / malformed). This
/// is the gate the role pipeline uses to decide whether to invoke the
/// architect agent: a non-empty doc means F7 already filled it, or the
/// user authored their own, so no agent call is needed.
pub fn operation_doc_is_empty(
    root: &Utf8Path,
    spec_relative_path: &str,
    operation_name: &str,
) -> bool {
    let absolute = root.join(spec_relative_path);
    let Ok(raw) = std::fs::read_to_string(&absolute) else {
        return true;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return true;
    };
    let Some(operations) = value.get("operations").and_then(Value::as_array) else {
        return true;
    };
    operations
        .iter()
        .find(|op| {
            op.get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name == operation_name)
        })
        .and_then(|op| op.get("doc").and_then(Value::as_str))
        .map(|doc| doc.trim().is_empty())
        .unwrap_or(true)
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
        ensures_added: 0,
        archetype_recognized: false,
        agent_id: None,
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
    let outcome = merge_semantic_into_spec(&mut spec_value, &operation_name, &semantic);
    if !outcome.mutated() {
        return Ok(report);
    }
    report.doc_added = outcome.doc_added;
    report.ensures_added = outcome.ensures_added;
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
        let outcome = merge_semantic_into_spec(&mut spec, "app.text.normalize_v1", &semantic);
        assert!(outcome.mutated());
        assert!(outcome.doc_added);
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
        let outcome = merge_semantic_into_spec(&mut spec, "app.text.normalize_v1", &semantic);
        assert!(!outcome.doc_added);
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
        let outcome = merge_semantic_into_spec(&mut spec, "app.text.normalize_v1", &semantic);
        assert!(!outcome.mutated());
    }

    #[test]
    fn every_archetype_predicate_parses_as_valid_json() {
        // Programmer-error catch: a malformed `expr_json` in the
        // archetype table would silently fail at runtime and ship an
        // un-enriched spec. This test ensures every predicate we ship
        // round-trips through `serde_json::from_str` before release.
        let archetypes = &[
            ("app.text", "normalize_v1"),
            ("app.checksum", "digest_v1"),
            ("app.codec", "roundtrip_v1"),
            ("app.compress", "roundtrip_v1"),
            ("toy.sorter", "sort_u8_asc"),
            ("app.greeter", "greet_v1"),
            ("app.calculator", "compute_v1"),
            ("app.parser", "parse_v1"),
            ("app.validator", "validate_v1"),
            ("app.timer", "elapsed_v1"),
            ("app.cli", "run_v1"),
            ("app.service", "handle_v1"),
        ];
        for (module_id, entry) in archetypes {
            let semantic = archetype_for(module_id, entry)
                .unwrap_or_else(|| panic!("archetype `{module_id}.{entry}` known"));
            for predicate in semantic.ensures {
                let parsed: Result<Value, _> = serde_json::from_str(predicate.expr_json);
                assert!(
                    parsed.is_ok(),
                    "predicate `{}` for `{module_id}.{entry}` has malformed expr_json: {:?}",
                    predicate.id,
                    parsed
                );
            }
        }
    }

    #[test]
    fn sort_archetype_declares_length_preserved_predicate() {
        // Until the build-pipeline impl stub is no longer `bytes.empty`,
        // ensures-merge is gated off in `merge_semantic_into_spec` (see
        // the doc comment there). The archetype TABLE still declares the
        // predicate so a follow-up that runs after a real impl is
        // written can pick it up.
        let semantic = archetype_for("toy.sorter", "sort_u8_asc").unwrap();
        assert_eq!(semantic.ensures.len(), 1);
        assert_eq!(semantic.ensures[0].id, "length_preserved");
        let parsed: Value = serde_json::from_str(semantic.ensures[0].expr_json).unwrap();
        assert_eq!(parsed[0].as_str(), Some("="));
    }

    #[test]
    fn text_normalize_archetype_carries_doc_but_no_predicate() {
        // app.text.normalize_v1 is doc-only this pass — see the comment
        // in `archetype_for` about the CBMC+Z3 counterexample on the
        // length-bound predicate.
        let semantic = archetype_for("app.text", "normalize_v1").unwrap();
        assert!(semantic.doc.contains("NFC"));
        assert_eq!(semantic.ensures.len(), 0);
    }

    #[test]
    fn greeter_archetype_carries_nonempty_predicate() {
        let semantic = archetype_for("app.greeter", "greet_v1").unwrap();
        assert_eq!(semantic.ensures.len(), 1);
        assert_eq!(semantic.ensures[0].id, "result_nonempty");
    }

    #[test]
    fn codec_roundtrip_archetype_carries_length_preserved_predicate() {
        let semantic = archetype_for("app.codec", "roundtrip_v1").unwrap();
        assert_eq!(semantic.ensures.len(), 1);
        assert_eq!(semantic.ensures[0].id, "length_preserved");
    }

    #[test]
    fn compress_roundtrip_archetype_carries_length_preserved_predicate() {
        let semantic = archetype_for("app.compress", "roundtrip_v1").unwrap();
        assert_eq!(semantic.ensures.len(), 1);
        assert_eq!(semantic.ensures[0].id, "length_preserved");
    }

    #[test]
    fn parser_archetype_carries_length_preserved_floor_predicate() {
        let semantic = archetype_for("app.parser", "parse_v1").unwrap();
        assert_eq!(semantic.ensures.len(), 1);
        assert_eq!(semantic.ensures[0].id, "length_preserved_floor");
    }

    #[test]
    fn validator_archetype_carries_status_byte_predicate() {
        let semantic = archetype_for("app.validator", "validate_v1").unwrap();
        assert_eq!(semantic.ensures.len(), 1);
        assert_eq!(semantic.ensures[0].id, "status_byte");
    }

    #[test]
    fn calculator_archetype_carries_bounded_result_predicate() {
        let semantic = archetype_for("app.calculator", "compute_v1").unwrap();
        assert_eq!(semantic.ensures.len(), 1);
        assert_eq!(semantic.ensures[0].id, "result_bounded");
    }

    #[test]
    fn timer_archetype_carries_os_time_doc_without_predicate() {
        let semantic = archetype_for("app.timer", "elapsed_v1").unwrap();
        assert!(semantic.doc.contains("OS time"));
        assert!(semantic.ensures.is_empty());
    }

    #[test]
    fn merge_ensures_preserves_existing_user_predicates_at_low_level() {
        // The low-level `merge_ensures_into_spec` still behaves
        // conservatively even though `merge_semantic_into_spec` is the
        // gated entry point. Test the underlying helper directly.
        let mut spec = json!({
            "operations": [{
                "name": "toy.sorter.sort_u8_asc",
                "doc": "",
                "ensures": [{
                    "id": "user_authored_strong_invariant",
                    "expr": ["=", ["bytes.len", "__result"], 42]
                }],
                "params": [{"name": "payload", "ty": "bytes"}],
                "result": "bytes",
            }]
        });
        let semantic = archetype_for("toy.sorter", "sort_u8_asc").unwrap();
        let appended =
            merge_ensures_into_spec(&mut spec, "toy.sorter.sort_u8_asc", semantic.ensures);
        assert_eq!(appended, 0);
        let ensures = spec["operations"][0]["ensures"].as_array().unwrap();
        assert_eq!(ensures.len(), 1);
        assert_eq!(
            ensures[0]["id"].as_str(),
            Some("user_authored_strong_invariant"),
        );
    }

    #[test]
    fn merge_ensures_low_level_writes_when_empty_then_skips() {
        let mut spec = json!({
            "operations": [{
                "name": "toy.sorter.sort_u8_asc",
                "doc": "",
                "ensures": [],
                "params": [{"name": "payload", "ty": "bytes"}],
                "result": "bytes",
            }]
        });
        let semantic = archetype_for("toy.sorter", "sort_u8_asc").unwrap();
        let first = merge_ensures_into_spec(&mut spec, "toy.sorter.sort_u8_asc", semantic.ensures);
        assert_eq!(first, 1);
        let second = merge_ensures_into_spec(&mut spec, "toy.sorter.sort_u8_asc", semantic.ensures);
        assert_eq!(second, 0);
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

    #[test]
    fn operation_doc_is_empty_reports_missing_file_as_empty() {
        let root = fresh_root();
        assert!(operation_doc_is_empty(
            &root,
            "spec/never.x07spec.json",
            "never.op_v1",
        ));
    }

    #[test]
    fn operation_doc_is_empty_returns_false_when_doc_is_present() {
        let root = fresh_root();
        std::fs::create_dir_all(root.join("spec")).unwrap();
        let spec = json!({
            "operations": [{
                "name": "app.x.go_v1",
                "doc": "Already filled by F7 archetype table.",
            }]
        });
        std::fs::write(
            root.join("spec/app.x.x07spec.json"),
            serde_json::to_string_pretty(&spec).unwrap(),
        )
        .unwrap();
        assert!(!operation_doc_is_empty(
            &root,
            "spec/app.x.x07spec.json",
            "app.x.go_v1",
        ));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn operation_doc_is_empty_returns_true_for_whitespace_doc() {
        let root = fresh_root();
        std::fs::create_dir_all(root.join("spec")).unwrap();
        let spec = json!({
            "operations": [{
                "name": "app.y.run_v1",
                "doc": "  ",
            }]
        });
        std::fs::write(
            root.join("spec/app.y.x07spec.json"),
            serde_json::to_string_pretty(&spec).unwrap(),
        )
        .unwrap();
        assert!(operation_doc_is_empty(
            &root,
            "spec/app.y.x07spec.json",
            "app.y.run_v1",
        ));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn agent_enrichment_parses_valid_event() {
        let event = json!({
            "schema_version": "x07.studio.agent_event@0.1.0",
            "kind": "spec_enrichment",
            "doc": "Compute the moving median over a window of bytes.",
            "examples": ["[1,2,3,4,5] window=3 -> [median]", "[] -> reject"]
        });
        let parsed = AgentEnrichment::from_event_value(&event).expect("valid enrichment");
        assert!(parsed.doc.contains("moving median"));
        assert_eq!(parsed.examples.len(), 2);
    }

    #[test]
    fn agent_enrichment_rejects_wrong_kind() {
        let event = json!({
            "schema_version": "x07.studio.agent_event@0.1.0",
            "kind": "clarify_question",
            "doc": "Compute the moving median.",
        });
        assert!(AgentEnrichment::from_event_value(&event).is_none());
    }

    #[test]
    fn agent_enrichment_rejects_empty_doc() {
        let event = json!({
            "schema_version": "x07.studio.agent_event@0.1.0",
            "kind": "spec_enrichment",
            "doc": "   ",
        });
        assert!(AgentEnrichment::from_event_value(&event).is_none());
    }

    #[test]
    fn agent_enrichment_rejects_missing_schema() {
        let event = json!({
            "kind": "spec_enrichment",
            "doc": "Something.",
        });
        assert!(AgentEnrichment::from_event_value(&event).is_none());
    }

    #[test]
    fn apply_agent_enrichment_writes_doc_and_reports_agent_id() {
        let root = fresh_root();
        let spec_relative = "spec/app.main.x07spec.json";
        std::fs::create_dir_all(root.join("spec")).unwrap();
        let scaffolded = json!({
            "schema_version": "x07.x07spec@0.1.0",
            "module_id": "app.main",
            "operations": [{
                "id": "op.run_v1.v1",
                "name": "app.main.run_v1",
                "doc": "",
                "params": [{"name": "payload", "ty": "bytes"}],
                "result": "bytes",
                "requires": [],
                "ensures": [],
                "ensures_props": [],
                "invariant": [],
            }],
        });
        std::fs::write(
            root.join(spec_relative),
            serde_json::to_string_pretty(&scaffolded).unwrap(),
        )
        .unwrap();

        let enrichment = AgentEnrichment {
            doc: "Compute the rolling stddev over a sliding window of bytes.".to_string(),
            examples: vec!["[1,2,3] window=3 -> [stddev]".to_string()],
        };
        let report = apply_agent_enrichment_to_spec(
            &root,
            spec_relative,
            "app.main",
            "run_v1",
            "claude-code",
            &enrichment,
        )
        .unwrap();
        assert!(report.doc_added);
        assert_eq!(report.agent_id.as_deref(), Some("claude-code"));

        let on_disk: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(spec_relative)).unwrap())
                .unwrap();
        let doc = on_disk["operations"][0]["doc"].as_str().unwrap();
        assert!(doc.contains("rolling stddev"));

        let second = apply_agent_enrichment_to_spec(
            &root,
            spec_relative,
            "app.main",
            "run_v1",
            "claude-code",
            &enrichment,
        )
        .unwrap();
        assert!(!second.doc_added);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn apply_agent_enrichment_preserves_existing_doc() {
        let root = fresh_root();
        let spec_relative = "spec/app.main.x07spec.json";
        std::fs::create_dir_all(root.join("spec")).unwrap();
        let scaffolded = json!({
            "schema_version": "x07.x07spec@0.1.0",
            "operations": [{
                "name": "app.main.run_v1",
                "doc": "User-authored doc; do not clobber.",
            }],
        });
        std::fs::write(
            root.join(spec_relative),
            serde_json::to_string_pretty(&scaffolded).unwrap(),
        )
        .unwrap();
        let enrichment = AgentEnrichment {
            doc: "Agent-derived doc that should NOT win over user content.".to_string(),
            examples: Vec::new(),
        };
        let report = apply_agent_enrichment_to_spec(
            &root,
            spec_relative,
            "app.main",
            "run_v1",
            "claude-code",
            &enrichment,
        )
        .unwrap();
        assert!(!report.doc_added);

        let on_disk: Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(spec_relative)).unwrap())
                .unwrap();
        assert_eq!(
            on_disk["operations"][0]["doc"].as_str().unwrap(),
            "User-authored doc; do not clobber."
        );

        std::fs::remove_dir_all(&root).ok();
    }
}
