import { describe, expect, it } from 'vitest';

import { StudioApi } from './api';
import { buildCounterexampleTheater, buildPatchReview, buildReviewSignals } from './review';
import {
	appendDemoOp,
	canonicalDocRefs,
	canonicalMcpTools,
	createIntentPacket,
	defaultAgentProfiles,
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
		expect(intent.targets[0].module_id).toBe('toy.sorter');
		expect(intent.targets[0].entry).toBe('sort_u8_asc');
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

	it('formalizes intent through an auditable Studio operation', async () => {
		const api = new StudioApi();
		await api.health();
		const session = demoSession();
		const response = await api.formalizeIntent(
			session,
			'Transcript: build a workflow graph and reject cycles.',
			'voice',
			['Keep the witness visible before spec approval.']
		);

		expect(response.intent.source.kind).toBe('voice');
		expect(response.intent.targets[0].module_id).toBe('workflow.graph');
		expect(response.op.op).toBe('intent.formalize');
		expect(response.session.phase).toBe('intent_ready');
		expect(response.session.op_log.at(-1)?.op).toBe('intent.formalize');
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
		expect(approved.contract?.global_doctrine.doc_refs).toEqual(canonicalDocRefs);
		expect(approved.contract?.global_doctrine.mcp_tools).toEqual(canonicalMcpTools);
		expect(workflowChecklist(approved).find((item) => item.label === 'Human spec approval')?.state).toBe(
			'done'
		);
	});

	it('models visible canonical operation records', () => {
		const session = appendDemoOp(demoSession(), 'xtal.verify', 'failed');

		expect(session.op_log).toHaveLength(1);
		expect(session.op_log[0].command.join(' ')).toContain('x07 xtal verify');
		expect(session.op_log[0].artifacts[0]).toBe('target/xtal/verify/summary.json');
	});

	it('derives trust review signals from canonical operation records', () => {
		let session = appendDemoOp(demoSession(), 'impl.sync.write', 'succeeded');
		session = appendDemoOp(session, 'xtal.verify', 'succeeded');

		const signals = buildReviewSignals(session.op_log);
		expect(signals.map((signal) => signal.label)).toEqual([
			'Verify evidence',
			'Implementation write'
		]);
		expect(signals[0].artifact).toBe('target/xtal/verify/summary.json');
	});

	it('derives counterexample theater state from failed verify diagnostics', () => {
		const session = appendDemoOp(demoSession(), 'xtal.verify', 'failed');
		const op = {
			...session.op_log[0],
			report_json: {
				clause_id: 'ensures.sorted',
				counterexample: { input: [3, 1, 2], expected: [1, 2, 3], actual: [3, 1, 2] },
				diagnostics: [
					{
						code: 'E_SORT_ORDER',
						severity: 'error',
						message: 'Output must be sorted ascending.'
					}
				]
			}
		};

		const theater = buildCounterexampleTheater([op]);

		expect(theater.tone).toBe('failed');
		expect(theater.title).toBe('Verification counterexample');
		expect(theater.clause).toBe('ensures.sorted');
		expect(theater.counterexample).toContain('"actual":[3,1,2]');
		expect(theater.diagnostics[0].code).toBe('E_SORT_ORDER');
		expect(theater.evidence).toContain('target/xtal/verify/summary.json');
	});

	it('derives visual patch review files from artifacts and patchset JSON', () => {
		const session = appendDemoOp(demoSession(), 'impl.sync.write', 'succeeded');
		const op = {
			...session.op_log[0],
			report_json: {
				result: {
					patchset: {
						schema_version: 'x07.patchset@0.1.0',
						patches: [
							{
								path: 'src/main.x07.json',
								patch: [
									{ op: 'add', path: '/decls/0', value: { kind: 'export', names: ['main.run'] } },
									{ op: 'replace', path: '/solve', value: ['bytes.lit', 'ok'] }
								],
								note: 'Realize approved operation'
							}
						]
					}
				}
			}
		};

		const review = buildPatchReview(op);
		expect(review?.gate).toBe('Write gate: implementation paths');
		expect(review?.files.map((file) => file.path)).toContain('src/main.x07.json');
		expect(review?.files.find((file) => file.path === 'src/main.x07.json')?.action).toBe(
			'add 1, replace 1'
		);
		expect(review?.files.map((file) => file.path)).toContain('target/xtal/impl-sync.patchset.json');
	});

	it('loads demo patchset artifact previews for visual review', async () => {
		const api = new StudioApi();
		await api.health();
		const session = appendDemoOp(demoSession(), 'impl.sync.write', 'succeeded');
		const preview = await api.previewArtifact(session, 'target/xtal/impl-sync.patchset.json');
		const op = {
			...session.op_log[0],
			report_json: {
				artifact_preview: {
					artifact: preview.artifact,
					json: preview.json,
					patchset_preview: preview.patchset_preview
				}
			}
		};

		const review = buildPatchReview(op);
		expect(preview.schema_version).toBe('x07.studio.artifact_preview@0.1.0');
		expect(review?.files.map((file) => file.path)).toContain('src/main.x07.json');
		expect(review?.files.find((file) => file.path === 'src/main.x07.json')?.operations).toBe(2);
		expect(review?.files.find((file) => file.path === 'src/main.x07.json')?.before).toContain(
			'todo'
		);
		expect(review?.files.find((file) => file.path === 'src/main.x07.json')?.after).toContain(
			'ok'
		);
	});

	it('does not replace connected artifact preview failures with demo content', async () => {
		const originalFetch = globalThis.fetch;
		const api = new StudioApi();
		globalThis.fetch = (async (input: RequestInfo | URL) => {
			const path = String(input);
			if (path.endsWith('/v1/health')) {
				return new Response(JSON.stringify({ ok: true, workspace_root: '/workspace' }), {
					status: 200
				});
			}
			if (path.includes('/artifacts/preview')) {
				return new Response('artifact is not recorded', { status: 409 });
			}
			return new Response('not found', { status: 404 });
		}) as typeof fetch;
		try {
			await api.health();
			await expect(
				api.previewArtifact(demoSession(), 'target/xtal/impl-sync.patchset.json')
			).rejects.toThrow('artifact is not recorded');
			expect(api.isDemoMode).toBe(false);
		} finally {
			globalThis.fetch = originalFetch;
		}
	});

	it('models supervised agent launch records', () => {
		const session = appendDemoOp(
			demoSession(),
			'agent.supervise.openai-codex',
			'succeeded',
			['codex', '.x07/studio/handoffs/demo-openai-codex.md'],
			['.x07/studio/handoffs/demo-openai-codex.md']
		);

		expect(session.op_log[0].op).toBe('agent.supervise.openai-codex');
		expect(session.op_log[0].command[0]).toBe('codex');
		expect(session.op_log[0].artifacts[0]).toContain('handoffs');
	});

	it('models pending human approval checkpoints', () => {
		const session = appendDemoOp(demoSession(), 'agent.approval.openai-codex', 'pending', [
			'approve-agent',
			'openai-codex'
		]);

		expect(session.op_log[0].status).toBe('pending');
		expect(session.op_log[0].exit_code).toBeNull();
		expect(session.op_log[0].command).toEqual(['approve-agent', 'openai-codex']);
	});

	it('consumes agent approval checkpoints after one supervised run', async () => {
		const api = new StudioApi();
		await api.health();
		let session = demoSession();
		let response = await api.runAgentHandoff(session, 'openai-codex', 'execute');
		expect(response.op.op).toBe('agent.approval.openai-codex');
		expect(response.op.status).toBe('pending');

		response = {
			...response,
			...(await api.resolveAgentApproval(
				response.session,
				response.op.id,
				'approve',
				'test checkpoint'
			))
		};
		session = response.session;

		response = await api.runAgentHandoff(session, 'openai-codex', 'execute');
		expect(response.op.op).toBe('agent.run.openai-codex');
		expect(response.op.status).toBe('succeeded');
		expect(response.session.op_log.some((op) => op.op === 'agent.event.openai-codex.artifact')).toBe(
			true
		);

		response = await api.runAgentHandoff(response.session, 'openai-codex', 'execute');
		expect(response.op.op).toBe('agent.approval.openai-codex');
		expect(response.op.status).toBe('pending');
	});

	it('includes project initialization and write bindings for end-to-end XTAL creation', () => {
		const ids = demoBindings().map((binding) => binding.id);

		expect(ids).toContain('project.init.xtal-pure');
		expect(ids).toContain('tests.gen.write');
		expect(ids).toContain('gen.verify');
		expect(ids).toContain('test.manifest');
		expect(ids).toContain('run.stdin');
		expect(ids).toContain('run.sandbox.os');
		expect(ids).toContain('run.sandbox.stdin.os');
		expect(ids).toContain('impl.sync.write');
		expect(ids).toContain('wasm.app.build.atlas_dev');
		expect(ids).toContain('wasm.app.verify.atlas_release');
		expect(ids).toContain('wasm.slo.eval.atlas_canary_ok');
	});

	it('models Codex and Claude Code as coding-agent profiles', () => {
		expect(defaultAgentProfiles.map((profile) => profile.id)).toEqual([
			'openai-codex',
			'claude-code'
		]);
		expect(defaultAgentProfiles[0].allowed_verbs).toContain('xtal.verify');
		expect(defaultAgentProfiles[1].command).toBe('claude');
	});

	it('exposes phase progress and primary action labels', () => {
		expect(phaseIndex('verify_running')).toBeGreaterThan(phaseIndex('intent_ready'));
		expect(nextPrimaryAction('spec_draft')).toBe('Approve spec');
	});

	it('offers project briefs that increase from simple to complex', () => {
		expect(projectTemplates.map((template) => template.id)).toEqual([
			'simple',
			'intermediate',
			'advanced',
			'complex',
			'expert',
			'atlas'
		]);
		expect(projectTemplates[0].sourcePath).toContain('agent-gate/xtal/toy-sorter');
		expect(projectTemplates[1].canonicalCommands).toContain('x07 xtal verify --project x07.json');
		expect(projectTemplates[2].sourcePath).toContain('x07-sm-arch-contracts-smoke');
		expect(projectTemplates[4].taskType).toBe('incident_repair');
		expect(projectTemplates[5].sourcePath).toContain('wasm_showcases/x07_atlas');
		expect(createIntentPacket(demoSession(), projectTemplates[2].prompt).targets[0]).toEqual({
			module_id: 'workflow.lifecycle',
			entry: 'step_v1'
		});
		expect(createIntentPacket(demoSession(), projectTemplates[3].prompt).targets[0]).toEqual({
			module_id: 'gateway.core',
			entry: 'route_request_v1'
		});
		expect(createIntentPacket(demoSession(), projectTemplates[4].prompt).targets[0]).toEqual({
			module_id: 'db.guard',
			entry: 'verify_drift'
		});
		expect(createIntentPacket(demoSession(), projectTemplates[5].prompt).targets[0]).toEqual({
			module_id: 'atlas.app',
			entry: 'atlas_dev'
		});
	});
});
