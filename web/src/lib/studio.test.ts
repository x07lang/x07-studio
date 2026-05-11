import { describe, expect, it } from 'vitest';

import {
	appendDemoOp,
	createIntentPacket,
	demoBindings,
	demoSession,
	nextPrimaryAction,
	phaseIndex,
	projectTemplates,
	reduceDemoEvent,
	workflowChecklist
} from './studio';

describe('x07 Studio XTAL web model', () => {
	it('formalizes human text into a reviewable intent packet', () => {
		const session = demoSession();
		const intent = createIntentPacket(session, 'Create a stable sorter and reject empty input.');

		expect(intent.schema_version).toBe('x07.studio.intent_packet@0.1.0');
		expect(intent.targets[0].module_id).toBe('app.sorter');
		expect(intent.witnesses.map((witness) => witness.kind)).toContain('policy_requirement');
		expect(intent.constraints).toContain('Use spec-first XTAL flow.');
	});

	it('preserves spoken and incident inputs as auditable intent sources', () => {
		const session = demoSession();
		const voice = createIntentPacket(session, 'Transcript: make the graph cycle rejection explicit.', 'voice');
		const incident = createIntentPacket(session, 'Runtime cycle report from production.', 'incident', [
			'Keep repair spec-preserving unless a witness changes.'
		]);

		expect(voice.source.kind).toBe('voice');
		expect(incident.source.kind).toBe('incident');
		expect(incident.witnesses.map((witness) => witness.kind)).toContain('incident_report');
		expect(incident.constraints).toContain(
			'Revision request: Keep repair spec-preserving unless a witness changes.'
		);
	});

	it('keeps the approval gate before realization', () => {
		const session = demoSession();
		const intentReady = reduceDemoEvent(
			session,
			'formalize_intent',
			createIntentPacket(session, 'Build workflow graph optimizer.')
		);
		const specDraft = reduceDemoEvent(intentReady, 'draft_spec');
		const approved = reduceDemoEvent(specDraft, 'approve_spec');

		expect(intentReady.phase).toBe('intent_ready');
		expect(specDraft.phase).toBe('spec_draft');
		expect(approved.phase).toBe('spec_approved');
		expect(approved.contract?.project_doctrine.write_policy.agent_write_specs).toBe(false);
		expect(workflowChecklist(approved).find((item) => item.label === 'Human spec approval')?.state).toBe(
			'done'
		);
	});

	it('models visible canonical operation records', () => {
		const session = appendDemoOp(demoSession(), 'xtal.verify', 'failed');

		expect(session.op_log).toHaveLength(1);
		expect(session.op_log[0].command.join(' ')).toContain('x07 xtal verify');
		expect(session.op_log[0].artifacts[0]).toContain('target/xtal');
	});

	it('includes project initialization and write bindings for end-to-end XTAL creation', () => {
		const ids = demoBindings().map((binding) => binding.id);

		expect(ids).toContain('project.init.xtal-pure');
		expect(ids).toContain('tests.gen.write');
		expect(ids).toContain('impl.sync.write');
	});

	it('exposes phase progress and primary action labels', () => {
		expect(phaseIndex('verify_running')).toBeGreaterThan(phaseIndex('intent_ready'));
		expect(nextPrimaryAction('spec_draft')).toBe('Approve spec');
	});

	it('offers project briefs that increase from simple to complex', () => {
		expect(projectTemplates.map((template) => template.id)).toEqual([
			'simple',
			'intermediate',
			'complex'
		]);
		expect(projectTemplates[0].prompt).toContain('integer sorter');
		expect(projectTemplates[2].taskType).toBe('incident_repair');
		expect(createIntentPacket(demoSession(), projectTemplates[2].prompt).targets[0]).toEqual({
			module_id: 'ops.incident_repair',
			entry: 'classify_and_repair'
		});
	});
});
