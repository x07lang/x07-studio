import { describe, expect, it } from 'vitest';

import { recipes } from './index';

const expected = [
	'text-core-text-utils',
	'math-bigint-factorial-100',
	'math-decimal-money-format',
	'text-unicode-normalize-casefold',
	'data-cbor-roundtrip',
	'data-msgpack-roundtrip',
	'checksum-fast-smoke',
	'diff-patch-apply',
	'compress-zstd-roundtrip',
	'fs-globwalk-list-files'
];

describe('canonical recipes', () => {
	it('uses the ten x07 agent-gate recipes as the only welcome set', () => {
		expect(recipes.map((recipe) => recipe.id)).toEqual(expected);
		expect(recipes).toHaveLength(10);
		for (const recipe of recipes) {
			expect(recipe.schema_version).toBe('x07.studio.recipe@0.1.0');
			expect(recipe.canonical_example_path).toMatch(/^docs\/examples\/agent-gate\//);
			expect(recipe.scenario_paths ?? []).not.toHaveLength(0);
			expect(recipe.preview_posture.posture_color).toBe('green');
			expect(recipe.preview_posture.worlds).toEqual(['solve-pure']);
		}
	});
});
