# Canonical recipes

The welcome surface is sourced from `web/src/lib/recipes/canonical.json`.
That JSON mirrors the package to example to scenario map in
`x07/docs/getting-started/agent-workflow.md`.

## Canonical set

- `text-core/text-utils`
- `math-bigint/factorial-100`
- `math-decimal/money-format`
- `text-unicode/normalize-casefold`
- `data-cbor/roundtrip`
- `data-msgpack/roundtrip`
- `checksum-fast/smoke`
- `diff-patch/apply`
- `compress-zstd/roundtrip`
- `fs-globwalk/list-files`

`cargo test -p loom-core --test canonical_recipes` reads both the Studio JSON
and the x07 agent workflow doc, then fails if paths or scenario fixtures drift.
