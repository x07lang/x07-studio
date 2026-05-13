use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::Value;

const CANONICAL_EXAMPLES: [&str; 10] = [
    "docs/examples/agent-gate/text-core/text-utils/",
    "docs/examples/agent-gate/math-bigint/factorial-100/",
    "docs/examples/agent-gate/math-decimal/money-format/",
    "docs/examples/agent-gate/text-unicode/normalize-casefold/",
    "docs/examples/agent-gate/data-cbor/roundtrip/",
    "docs/examples/agent-gate/data-msgpack/roundtrip/",
    "docs/examples/agent-gate/checksum-fast/smoke/",
    "docs/examples/agent-gate/diff-patch/apply/",
    "docs/examples/agent-gate/compress-zstd/roundtrip/",
    "docs/examples/agent-gate/fs-globwalk/list-files/",
];

#[test]
fn canonical_recipes_match_agent_workflow_doc() {
    let studio_root = studio_root();
    let recipes_path = studio_root.join("web/src/lib/recipes/canonical.json");
    let workflow_path = studio_root
        .parent()
        .expect("x07lang workspace parent")
        .join("x07/docs/getting-started/agent-workflow.md");
    let recipes_raw = std::fs::read_to_string(&recipes_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", recipes_path.display()));
    let workflow = std::fs::read_to_string(&workflow_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", workflow_path.display()));
    let recipes = serde_json::from_str::<Value>(&recipes_raw).expect("canonical recipes JSON");
    let items = recipes.as_array().expect("canonical recipes array");

    assert_eq!(items.len(), 10);
    let mut paths = BTreeSet::new();
    for item in items {
        let example = item
            .get("canonical_example_path")
            .and_then(Value::as_str)
            .expect("canonical_example_path");
        assert!(
            paths.insert(example.to_string()),
            "duplicate recipe path {example}"
        );
        assert!(
            workflow.contains(example),
            "agent workflow doc is missing recipe example path {example}"
        );
        for scenario in item
            .get("scenario_paths")
            .and_then(Value::as_array)
            .expect("scenario_paths")
        {
            let scenario = scenario.as_str().expect("scenario path string");
            assert!(
                workflow.contains(scenario),
                "agent workflow doc is missing scenario path {scenario}"
            );
        }
    }
    assert_eq!(
        paths,
        CANONICAL_EXAMPLES
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>()
    );
}

fn studio_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates")
        .parent()
        .expect("studio root")
        .to_path_buf()
}
