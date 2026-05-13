import canonical from './canonical.json';
import type { Recipe, TaskType, TrustPosture } from '$lib/studio';

function posture(id: string): TrustPosture {
	return {
		schema_version: 'x07.studio.trust_posture@0.1.0',
		session_id: `recipe-${id}`,
		captured_at: 'recipe',
		trust_profile: 'local_preview',
		worlds: ['solve-pure'],
		capabilities: [],
		budgets: { local_cap_ms: null, arch_profile: null, prover_seconds_used: 0, prover_seconds_cap: 30 },
		proof_coverage: { support_pct: 100, proved_pct: 100, proof_count: 1, assumptions_open: 0 },
		deltas: [],
		posture_color: 'green'
	};
}

export const recipes: Recipe[] = canonical.map((item) => ({
	schema_version: 'x07.studio.recipe@0.1.0',
	id: item.id,
	title: item.title,
	one_liner: item.one_liner,
	intent_text: item.intent_text,
	task_type: item.task_type as TaskType,
	module_id: item.module_id,
	canonical_example_path: item.canonical_example_path,
	scenario_paths: item.scenario_paths,
	preview_posture: posture(item.id)
}));
