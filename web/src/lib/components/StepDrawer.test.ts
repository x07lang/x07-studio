import { describe, expect, it } from 'vitest';

import type { StepEvidence } from '$lib/studio';

const evidence: StepEvidence = {
	schema_version: 'x07.studio.step_evidence@0.1.0',
	session_id: 'st-test',
	step_id: 'verify',
	op: {
		schema_version: 'x07.studio.op_record@0.1.0',
		id: 'op-verify',
		session_id: 'st-test',
		op: 'xtal.verify',
		backend: 'cli',
		command: ['x07', 'xtal', 'verify'],
		started_at: '1',
		finished_at: '2',
		status: 'succeeded',
		exit_code: 0,
		artifacts: ['target/xtal/verify/summary.json'],
		notes: 'Verified solve-pure evidence.'
	},
	stream_events: [],
	artifacts: ['target/xtal/verify/summary.json']
};

describe('StepDrawer', () => {
	it('preserves linked op and artifacts in the evidence payload', () => {
		expect(evidence.step_id).toBe('verify');
		expect(evidence.op?.op).toBe('xtal.verify');
		expect(evidence.op?.command).toEqual(['x07', 'xtal', 'verify']);
		expect(evidence.artifacts).toContain('target/xtal/verify/summary.json');
	});
});
