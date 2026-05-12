import { describe, expect, test } from 'vitest';
import {
	buildStageProgress,
	currentBuildStage,
	plainOpLabel,
	stageFromOp,
	stageLabel,
	type BuildStage
} from './plainEnglish';
import type { OpRecord, SessionSnapshot } from './studio';

function op(id: string, name: string): OpRecord {
	return {
		id,
		op: name,
		backend: 'studio',
		command: [],
		started_at: '2026-05-12T12:00:00Z',
		status: 'succeeded',
		artifacts: []
	};
}

function sessionWith(ops: OpRecord[]): SessionSnapshot {
	return {
		schema_version: 'x07.studio.session_snapshot@0.1.0',
		session_id: 'sess-1',
		title: 'demo',
		root: '/workspace',
		task_type: 'new_behavior',
		room: 'intent',
		phase: 'intent_ready',
		intent: null,
		revision_notes: [],
		contract: null,
		allowed_verbs: [],
		op_log: ops
	} as unknown as SessionSnapshot;
}

describe('plainEnglish stage mapping', () => {
	test('stageLabel returns plain English for canonical build stages', () => {
		expect(stageLabel('verify')).toMatch(/correctness/i);
		expect(stageLabel('repair')).toMatch(/fixing/i);
	});

	test('stageFromOp maps canonical binding ids to build stages', () => {
		expect(stageFromOp(op('a', 'spec.scaffold'))).toBe<BuildStage>('design');
		expect(stageFromOp(op('b', 'impl.sync.write'))).toBe<BuildStage>('write');
		expect(stageFromOp(op('c', 'xtal.verify'))).toBe<BuildStage>('verify');
		expect(stageFromOp(op('d', 'xtal.repair'))).toBe<BuildStage>('repair');
		expect(stageFromOp(op('e', 'tests.gen.write'))).toBe<BuildStage>('test');
	});

	test('stageFromOp ignores unrelated ops', () => {
		expect(stageFromOp(op('z', 'agent.run.claude-code'))).toBeNull();
	});

	test('build.stage.* ops short-circuit to their stage', () => {
		expect(stageFromOp(op('s', 'build.stage.start'))).toBe<BuildStage>('start');
		expect(stageFromOp(op('s', 'build.stage.done'))).toBe<BuildStage>('done');
	});

	test('currentBuildStage tracks the latest stage-producing op', () => {
		const session = sessionWith([
			op('1', 'build.stage.start'),
			op('2', 'spec.scaffold'),
			op('3', 'impl.sync.write'),
			op('4', 'xtal.verify')
		]);
		expect(currentBuildStage(session)).toBe<BuildStage>('verify');
	});

	test('buildStageProgress partitions stages around the current marker', () => {
		const session = sessionWith([
			op('1', 'build.stage.start'),
			op('2', 'spec.scaffold'),
			op('3', 'impl.sync.write')
		]);
		const progress = buildStageProgress(session);
		expect(progress.stage).toBe<BuildStage>('write');
		expect(progress.completed).toContain<BuildStage>('design');
		expect(progress.pending).toContain<BuildStage>('verify');
	});
});

describe('plainOpLabel', () => {
	test('translates clarify_question agent events', () => {
		expect(plainOpLabel(op('q', 'agent.event.claude-code.clarify_question'))).toMatch(
			/clarifying/i
		);
		expect(plainOpLabel(op('q', 'agent.event.openai-codex.clarify_done'))).toMatch(/no more/i);
	});

	test('uses friendly labels for canonical bindings', () => {
		expect(plainOpLabel(op('a', 'spec.scaffold'))).toMatch(/Designing/i);
		expect(plainOpLabel(op('a', 'xtal.verify'))).toMatch(/correctness/i);
	});

	test('falls back to the raw op for unknown names', () => {
		expect(plainOpLabel(op('a', 'unknown.thing'))).toBe('unknown.thing');
	});
});
