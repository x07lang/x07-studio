import { describe, expect, it } from 'vitest';

import type { ProcessLane as ProcessLaneType } from '$lib/studio';

const lane: ProcessLaneType = {
	schema_version: 'x07.studio.process_lane@0.1.0',
	session_id: 'st-test',
	current_index: 1,
	next_index: 2,
	steps: [
		{
			schema_version: 'x07.studio.canonical_step@0.1.0',
			id: 'intent',
			label: 'Capture intent',
			actor: 'conductor',
			status: 'done',
			started_at: '1',
			finished_at: '2',
			elapsed_ms: 1000,
			op_id: 'op-intent',
			narration: 'Studio captured intent.',
			next_actor: null
		},
		{
			schema_version: 'x07.studio.canonical_step@0.1.0',
			id: 'impl',
			label: 'Write implementation',
			actor: 'coder',
			status: 'running',
			started_at: '2',
			finished_at: null,
			elapsed_ms: null,
			op_id: 'op-impl',
			narration: 'Codex is writing src/app/main.x07.json.',
			next_actor: 'reviewer',
			budget: { wall_clock_ms: 60000, prover_seconds: null, on_exhaust: 'pause' }
		},
		{
			schema_version: 'x07.studio.canonical_step@0.1.0',
			id: 'review',
			label: 'Review implementation',
			actor: 'reviewer',
			status: 'pending',
			started_at: null,
			finished_at: null,
			elapsed_ms: null,
			op_id: null,
			narration: 'Claude will review against the spec.',
			next_actor: null
		}
	]
};

describe('ProcessLane', () => {
	it('keeps current, next, actors, and budget data in one fixture shape', () => {
		const current = lane.steps[lane.current_index ?? 0];
		const next = lane.steps[lane.next_index ?? 0];

		expect(current.label).toBe('Write implementation');
		expect(current.actor).toBe('coder');
		expect(current.status).toBe('running');
		expect(current.budget?.wall_clock_ms).toBe(60000);
		expect(next.label).toBe('Review implementation');
		expect(next.actor).toBe('reviewer');
	});
});
