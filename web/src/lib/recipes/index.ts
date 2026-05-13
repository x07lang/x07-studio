import type { Recipe, TrustPosture } from '$lib/studio';

function posture(id: string): TrustPosture {
	return {
		schema_version: 'x07.studio.trust_posture@0.1.0',
		session_id: `recipe-${id}`,
		captured_at: 'recipe',
		trust_profile: 'local_preview',
		worlds: ['solve-pure'],
		capabilities: [],
		budgets: { local_cap_ms: null, arch_profile: null, prover_seconds_used: 0, prover_seconds_cap: 30 },
		proof_coverage: { support_pct: 0, proved_pct: 0, proof_count: 0, assumptions_open: 0 },
		deltas: [],
		posture_color: 'green'
	};
}

export const recipes: Recipe[] = [
	{
		schema_version: 'x07.studio.recipe@0.1.0',
		id: 'sentiment',
		title: 'Sort emails by sentiment',
		one_liner: 'Classify inbound text and return a stable ordered list.',
		intent_text: 'Build a thing that sorts emails by sentiment and keeps equal scores stable.',
		task_type: 'new_behavior',
		preview_posture: posture('sentiment')
	},
	{
		schema_version: 'x07.studio.recipe@0.1.0',
		id: 'csv-parser',
		title: 'CSV parser',
		one_liner: 'Parse rows, reject malformed quotes, and report line numbers.',
		intent_text: 'Build a CSV parser that accepts text input, validates quotes, and returns rows with line-numbered errors.',
		task_type: 'new_behavior',
		preview_posture: posture('csv-parser')
	},
	{
		schema_version: 'x07.studio.recipe@0.1.0',
		id: 'folder-watch',
		title: 'Watch a folder',
		one_liner: 'Turn filesystem events into replayable incident evidence.',
		intent_text: 'Build a folder watcher that records changed file paths and emits deterministic replay evidence.',
		task_type: 'new_behavior',
		preview_posture: { ...posture('folder-watch'), posture_color: 'amber', worlds: ['solve-pure', 'os-fs'] }
	}
];
