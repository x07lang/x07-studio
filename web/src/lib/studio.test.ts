import { describe, expect, it } from 'vitest';

import { StudioApi } from './api';
import {
	buildCounterexampleTheater,
	buildPatchReview,
	buildReviewSignals,
	writeAuditFromOp
} from './review';
import {
	appendDemoOp,
	agentReadiness,
	buildAgentHandoffReview,
	buildApprovalLedger,
	buildAutomationPlan,
	buildCertifyCommandPreview,
	buildEvidenceCoverage,
	buildOnboardingPlan,
	buildPlatformBridge,
	buildProviderProbeGates,
	buildProofCacheLedger,
	buildRepairCommandPreview,
	buildVerifyEvidenceBoard,
	buildVerifyCommandPreview,
	buildWorldBudgetGuard,
	canonicalDocRefs,
	canonicalMcpTools,
	createIntentPacket,
	defaultAgentProfiles,
	defaultProviderProfiles,
	demoBindings,
	demoHealth,
	demoSession,
	nextPrimaryAction,
	phaseIndex,
	previewIntentWitnesses,
	projectTemplates,
	reduceDemoEvent,
	certifyRunVars,
	repairRunVars,
	verifyRunVars,
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

	it('reports onboarding readiness for standalone runtime components', () => {
		const health = demoHealth();
		const required = health.components
			.filter((component) => component.required)
			.map((component) => component.id);

		expect(health.defaults.platform_state_dir).toBe('.x07/platform');
		expect(required).toEqual(['x07', 'x07-wasm', 'x07lp']);
		expect(health.components.every((component) => component.install_hint.length > 0)).toBe(true);
	});

	it('builds an actionable onboarding plan from health readiness', () => {
		const health = demoHealth();
		const components = health.components.map((component) => {
			if (component.id === 'x07-wasm') {
				return { ...component, status: 'missing' as const, source: null };
			}
			if (component.id === 'codex') {
				return { ...component, source: '/usr/local/bin/codex' };
			}
			return component;
		});
		const plan = buildOnboardingPlan(health, components);
		const defaults = plan.find((step) => step.id === 'defaults');
		const wasm = plan.find((step) => step.id === 'component.x07-wasm');
		const codex = plan.find((step) => step.id === 'component.codex');

		expect(defaults?.state).toBe('required');
		expect(defaults?.command).toContain('bootstrap_components.py');
		expect(defaults?.detail).toContain('daemon 127.0.0.1:7719');
		expect(wasm?.state).toBe('required');
		expect(wasm?.detail).toContain('X07_STUDIO_X07_WASM_EXE');
		expect(codex?.state).toBe('ready');
		expect(codex?.command).toBe('/usr/local/bin/codex');
	});

	it('derives coding-agent readiness from runtime components and profile policy', () => {
		const codex = defaultAgentProfiles[0];
		const ready = agentReadiness(codex, [
			{
				id: 'codex',
				label: 'OpenAI Codex',
				command: 'codex',
				required: false,
				status: 'available',
				source: '/usr/local/bin/codex',
				install_hint: 'Install Codex CLI.'
			}
		]);

		expect(ready.state).toBe('available');
		expect(ready.canRun).toBe(true);
		expect(ready.source).toBe('/usr/local/bin/codex');
		expect(ready.gate).toBe('Human checkpoint before execute');

		const missing = agentReadiness(defaultAgentProfiles[1], []);
		expect(missing.state).toBe('needs_install');
		expect(missing.canRun).toBe(false);
		expect(missing.source).toBe('not found on PATH');

		const disabled = agentReadiness({ ...codex, status: 'disabled' }, [
			{
				id: 'codex',
				label: 'OpenAI Codex',
				command: 'codex',
				required: false,
				status: 'available',
				source: '/usr/local/bin/codex',
				install_hint: 'Install Codex CLI.'
			}
		]);
		expect(disabled.state).toBe('disabled');
		expect(disabled.canRun).toBe(false);
		expect(disabled.source).toBe('disabled by profile');
	});

	it('preserves spoken, spec, and incident inputs as auditable intent sources', () => {
		const session = demoSession();
		const voice = createIntentPacket(session, 'Transcript: make the graph cycle rejection explicit.', 'voice');
		const spec = createIntentPacket(
			session,
			JSON.stringify({
				schema_version: 'x07.x07spec@0.1.0',
				module_id: 'toy.sorter',
				operations: [{ id: 'op.sort_u8_asc.v1', name: 'toy.sorter.sort_u8_asc' }]
			}),
			'spec'
		);
		const partialPrefixSpec = createIntentPacket(
			session,
			JSON.stringify({
				schema_version: 'x07.x07spec@0.1.0',
				module_id: 'toy.sort',
				operations: [{ id: 'op.sort_u8_asc.v1', name: 'toy.sorter.sort_u8_asc' }]
			}),
			'spec'
		);
		const incident = createIntentPacket(session, 'Runtime cycle report from production.', 'incident', [
			'Keep repair spec-preserving unless a witness changes.'
		]);

		expect(voice.source.kind).toBe('voice');
		expect(spec.source.kind).toBe('spec');
		expect(spec.targets[0]).toEqual({ module_id: 'toy.sorter', entry: 'sort_u8_asc' });
		expect(partialPrefixSpec.targets[0]).toEqual({
			module_id: 'toy.sort',
			entry: 'toy_sorter_sort_u8_asc'
		});
		expect(spec.constraints).toContain('Treat the provided spec as already-authored behavioral intent.');
		expect(incident.source.kind).toBe('incident');
		if (incident.source.kind === 'incident') {
			expect(incident.source.path).toMatch(/^\.x07\/studio\/incidents\/st-demo/);
		}
		expect(incident.witnesses.map((witness) => witness.kind)).toContain('incident_report');
		expect(incident.constraints).toContain(
			'Revision request: Keep repair spec-preserving unless a witness changes.'
		);
	});

	it('previews witness types before intent polish', () => {
		const witnesses = previewIntentWitnesses(
			'Build a workflow graph. Reject cycles. Never call the network in solve-pure.',
			'voice'
		);

		expect(witnesses.map((witness) => witness.kind)).toEqual([
			'desired_behavior',
			'forbidden_behavior',
			'policy_requirement'
		]);
		expect(witnesses.find((witness) => witness.kind === 'forbidden_behavior')?.text).toBe(
			'Reject cycles.'
		);
		expect(witnesses.find((witness) => witness.kind === 'policy_requirement')?.text).toBe(
			'Never call the network in solve-pure.'
		);
		expect(previewIntentWitnesses('Production crashed on payload length 0.', 'incident')[0]?.kind).toBe(
			'incident_report'
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

	it('sends provider profile selection for connected intent polish', async () => {
		const originalFetch = globalThis.fetch;
		const api = new StudioApi();
		const session = demoSession();
		const intent = createIntentPacket(session, 'Create a sorter.');
		const logged = appendDemoOp(session, 'intent.formalize', 'succeeded');
		let requestBody: Record<string, unknown> = {};
		globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
			const path = String(input);
			if (path.endsWith('/v1/health')) {
				return new Response(JSON.stringify({ ok: true, workspace_root: '/workspace' }), {
					status: 200
				});
			}
			if (path.includes('/intent/formalize')) {
				requestBody = JSON.parse(String(init?.body ?? '{}'));
				return new Response(
					JSON.stringify({
						intent,
						op: logged.op_log[0],
						session: { ...logged, intent }
					}),
					{ status: 200 }
				);
			}
			return new Response('not found', { status: 404 });
		}) as typeof fetch;
		try {
			await api.health();
			await api.formalizeIntent(session, 'Create a sorter.', 'text', [], 'ollama-local');
			expect(requestBody?.provider_profile_id).toBe('ollama-local');
		} finally {
			globalThis.fetch = originalFetch;
		}
	});

	it('builds provider capability gates from a deep probe report', () => {
		const profile = { ...defaultProviderProfiles[0], model: 'qwen3-coder' };
		const gates = buildProviderProbeGates(profile, {
			schema_version: 'x07.studio.provider_probe_report@0.1.0',
			profile_id: profile.id,
			base_url: profile.base_url,
			observed_at: '2026-05-11T00:00:00Z',
			ok: true,
			http_status: 200,
			models: ['qwen3-coder'],
			capabilities: {
				models_endpoint: 'supported',
				responses: 'supported',
				chat_completions: 'supported',
				tools: 'supported',
				json_schema: 'supported',
				streaming: 'unknown'
			},
			notes: []
		});

		expect(gates.find((gate) => gate.label === 'Model catalog')?.state).toBe('ready');
		expect(gates.find((gate) => gate.label === 'Intent polish API')?.state).toBe('ready');
		expect(gates.find((gate) => gate.label === 'Streaming')?.state).toBe('review');
		expect(gates.find((gate) => gate.label === 'Trust tier')?.detail).toContain('Local provider');
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

	it('marks revision requests as an active approval blocker until repolished', () => {
		let session = demoSession();
		session = reduceDemoEvent(session, 'formalize_intent', createIntentPacket(session, 'Create a sorter.'));

		const blockedLedger = buildApprovalLedger(session, ['Keep empty input explicit.'], 'changes');
		expect(blockedLedger.find((item) => item.label === 'Revision 1')?.state).toBe('active');
		expect(blockedLedger.find((item) => item.label === 'Human decision')?.state).toBe('blocked');

		const awaitingLedger = buildApprovalLedger(session, ['Keep empty input explicit.'], 'awaiting');
		expect(awaitingLedger.find((item) => item.label === 'Revision 1')?.state).toBe('done');
		expect(awaitingLedger.find((item) => item.label === 'Human decision')?.state).toBe('active');
	});

	it('derives an approval-gated automation plan from the selected project template', () => {
		let session = demoSession();
		const draftPlan = buildAutomationPlan(session, projectTemplates[0], 'drafting');
		expect(draftPlan.find((step) => step.label === 'Human approval')?.state).toBe('blocked');
		expect(draftPlan.find((step) => step.label === 'Project scaffold')?.state).toBe('blocked');

		session = reduceDemoEvent(session, 'formalize_intent', createIntentPacket(session, 'Create a sorter.'));
		session = reduceDemoEvent(session, 'approve_spec');
		session = appendDemoOp(session, 'project.init.xtal-pure', 'succeeded');
		session = appendDemoOp(session, 'xtal.verify', 'succeeded');

		const runPlan = buildAutomationPlan(session, projectTemplates[0], 'approved');
		expect(runPlan.find((step) => step.label === 'Human approval')?.state).toBe('done');
		expect(runPlan.find((step) => step.label === 'Project scaffold')?.state).toBe('done');
		expect(runPlan.find((step) => step.command.includes('x07 xtal verify'))?.state).toBe('done');
	});

	it('maps prompt-to-artifact coverage from approval gates and operation evidence', () => {
		let session = demoSession();
		let coverage = buildEvidenceCoverage(session, projectTemplates[5], 'drafting');
		expect(coverage.find((item) => item.id === 'intent')?.state).toBe('active');
		expect(coverage.find((item) => item.id === 'project')?.state).toBe('blocked');

		session = reduceDemoEvent(
			session,
			'formalize_intent',
			createIntentPacket(session, projectTemplates[5].prompt)
		);
		session = reduceDemoEvent(session, 'draft_spec');
		session = reduceDemoEvent(session, 'approve_spec');
		session = appendDemoOp(session, 'project.seed.x07_atlas', 'succeeded');
		session = appendDemoOp(session, 'wasm.app.build.atlas_dev', 'succeeded');
		session = appendDemoOp(session, 'wasm.app.verify.atlas_release', 'succeeded');
		session = appendDemoOp(session, 'lp.deploy.status.local', 'succeeded');

		coverage = buildEvidenceCoverage(session, projectTemplates[5], 'approved');
		expect(coverage.find((item) => item.id === 'project')?.state).toBe('done');
		expect(coverage.find((item) => item.id === 'implementation')?.evidence).toBe(
			'wasm.app.build.atlas_dev'
		);
		expect(coverage.find((item) => item.id === 'verify')?.state).toBe('done');
		expect(coverage.find((item) => item.id === 'trust-platform')?.artifact).toBe(
			'dist/showcase_fullstack/pack.atlas_release/app.pack.json'
		);
	});

	it('maps x07 platform bridge gates from Atlas operations', () => {
		let session = demoSession();
		let bridge = buildPlatformBridge(session, projectTemplates[0]);
		expect(bridge.posture).toBe('Platform optional');
		expect(bridge.items.find((item) => item.id === 'platform-delivery')?.state).toBe('optional');

		session = reduceDemoEvent(
			session,
			'formalize_intent',
			createIntentPacket(session, projectTemplates[5].prompt)
		);
		session = reduceDemoEvent(session, 'draft_spec');
		session = reduceDemoEvent(session, 'approve_spec');
		session = appendDemoOp(session, 'wasm.app.pack.atlas_release', 'succeeded');
		session = appendDemoOp(session, 'wasm.provenance.verify.atlas_release', 'succeeded');
		session = appendDemoOp(session, 'wasm.deploy.plan.atlas_release', 'succeeded');
		session = appendDemoOp(session, 'lp.deploy.status.local', 'succeeded');
		session = appendDemoOp(session, 'wasm.slo.eval.atlas_canary_ok', 'succeeded');

		bridge = buildPlatformBridge(session, projectTemplates[5]);
		expect(bridge.posture).toBe('Platform delivery covered');
		expect(bridge.summary).toBe('5 / 5 required platform gates covered');
		expect(bridge.nextAction).toBe(
			'Platform evidence is complete; review trust and certification gates'
		);
		expect(bridge.items.find((item) => item.id === 'platform-delivery')?.evidence).toBe(
			'lp.deploy.status.local'
		);
		expect(bridge.items.find((item) => item.id === 'feedback')?.state).toBe('optional');
	});

	it('builds proof cache readiness from verified XTAL evidence', () => {
		let session = demoSession();
		session = reduceDemoEvent(session, 'formalize_intent', createIntentPacket(session, 'Create a sorter.'));
		session = reduceDemoEvent(session, 'approve_spec');
		session = appendDemoOp(session, 'spec.check', 'succeeded');
		session = appendDemoOp(session, 'impl.sync.write', 'succeeded');
		session = appendDemoOp(session, 'xtal.verify', 'succeeded');

		const ledger = buildProofCacheLedger(session, projectTemplates[0], null);
		expect(ledger.find((item) => item.label === 'Cache key preview')?.value).toContain('contract-locked');
		expect(ledger.find((item) => item.label === 'Cache key preview')?.value).toContain('balanced');
		expect(ledger.find((item) => item.label === 'Verify artifact')?.state).toBe('ready');
		expect(ledger.find((item) => item.label === 'Proof policy')?.value).toBe('solve-pure proof / balanced');
		expect(ledger.find((item) => item.label === 'Certification dependency')?.state).toBe('pending');

		const draftLedger = buildProofCacheLedger(demoSession(), projectTemplates[0], null);
		expect(draftLedger.find((item) => item.label === 'Certification dependency')?.state).toBe('blocked');
	});

	it('builds bounded xtal verify command options', () => {
		const options = {
			proofPolicy: 'strict' as const,
			allowOsWorld: true,
			unwind: '3',
			maxBytesLen: '16',
			inputLenBytes: '24'
		};

		expect(buildVerifyCommandPreview(options)).toBe(
			'x07 xtal verify --proof-policy strict --allow-os-world --unwind 3 --max-bytes-len 16 --input-len-bytes 24'
		);
		expect(verifyRunVars(options)).toEqual({
			proof_policy: 'strict',
			allow_os_world: 'true',
			unwind: '3',
			max_bytes_len: '16',
			input_len_bytes: '24'
		});

		const ledger = buildProofCacheLedger(demoSession(), projectTemplates[0], null, options);
		expect(ledger.find((item) => item.label === 'Cache key preview')?.value).toContain('strict');
		expect(ledger.find((item) => item.label === 'Proof policy')?.detail).toContain('OS-capable worlds');
	});

	it('builds bounded xtal repair command options', () => {
		const options = {
			entry: 'toy.sorter.sort_u8_asc',
			strategy: 'spec_patch' as const,
			write: true,
			allowEditNonStubs: true,
			maxRounds: '2',
			maxCandidates: '4',
			semanticMaxDepth: '3'
		};

		expect(buildRepairCommandPreview(options)).toBe(
			'x07 xtal repair --entry toy.sorter.sort_u8_asc --write --max-rounds 2 --max-candidates 4 --semantic-max-depth 3 --allow-edit-non-stubs --suggest-spec-patch'
		);
		expect(repairRunVars(options)).toEqual({
			repair_entry: 'toy.sorter.sort_u8_asc',
			repair_strategy: 'spec_patch',
			repair_write: 'true',
			repair_allow_edit_non_stubs: 'true',
			repair_max_rounds: '2',
			repair_max_candidates: '4',
			repair_semantic_max_depth: '3'
		});
	});

	it('builds bounded xtal certify command options', () => {
		const options = {
			specDir: 'spec',
			entry: 'toy.sorter.sort_u8_asc',
			allEntries: false,
			noPrechecks: true
		};

		expect(buildCertifyCommandPreview(options)).toBe(
			'x07 xtal certify --no-prechecks --spec-dir spec --entry toy.sorter.sort_u8_asc'
		);
		expect(certifyRunVars(options)).toEqual({
			cert_spec_dir: 'spec',
			cert_entry: 'toy.sorter.sort_u8_asc',
			cert_all: 'false',
			cert_no_prechecks: 'true'
		});

		expect(
			buildCertifyCommandPreview({
				...options,
				allEntries: true
			})
		).toBe('x07 xtal certify --no-prechecks --spec-dir spec --all');
	});

	it('builds verify evidence from xtal verify summary reports', () => {
		let session = demoSession();
		session = reduceDemoEvent(session, 'formalize_intent', createIntentPacket(session, 'Create a sorter.'));
		session = appendDemoOp(session, 'xtal.verify', 'succeeded', undefined, undefined, {
			schema_version: 'x07.xtal.verify_summary@0.1.0',
			settings: {
				world: 'solve-pure',
				proof_policy: 'balanced',
				verify_bounds: { unwind: 2, max_bytes_len: 12 }
			},
			results: {
				outcome: 'warn',
				prechecks: { spec: 'pass', generation: 'pass', impl: 'pass' },
				verification: {
					coverage_outcome: 'pass',
					prove_outcome: 'warn',
					counts: {
						entries_total: 1,
						coverage_fail: 0,
						prove_proven: 0,
						prove_counterexample: 0,
						prove_inconclusive: 0,
						prove_unsupported: 1,
						prove_timeout: 0,
						prove_tool_missing: 0
					}
				},
				tests: {
					outcome: 'pass',
					passed: 6,
					failed: 0,
					skipped: 0,
					report: {
						kind: 'x07_tests_report',
						path: 'target/xtal/tests.report.json',
						schema_version: 'x07.x07test@0.4.0'
					}
				},
				diagnostics: {
					outcome: 'warn',
					error_count: 0,
					warning_count: 1,
					top_codes: [{ code: 'WXTAL_VERIFY_PROVE_UNSUPPORTED', count: 1 }],
					report: {
						kind: 'xtal_diag_report',
						path: 'target/xtal/xtal.verify.diag.json',
						schema_version: 'x07.x07diag@0.1.0'
					}
				}
			},
			artifacts: {
				verify_dir: 'target/xtal/verify',
				reports: [
					{
						kind: 'x07_verify_coverage_report',
						path: 'target/xtal/verify/coverage/toy/sorter/sort_u8_asc.report.json',
						schema_version: 'x07.verify.report@0.8.0'
					}
				]
			},
			entries: [
				{
					entry: 'toy.sorter.sort_u8_asc',
					op_id: 'op.sort_u8_asc.v1',
					spec_path: 'spec/toy.sorter.x07spec.json',
					coverage: {
						outcome: 'pass',
						report: {
							kind: 'x07_verify_coverage_report',
							path: 'target/xtal/verify/coverage/toy/sorter/sort_u8_asc.report.json',
							schema_version: 'x07.verify.report@0.8.0'
						}
					},
					prove: {
						raw: 'unsupported',
						policy_outcome: 'warn',
						report: {
							kind: 'x07_verify_prove_report',
							path: 'target/xtal/verify/prove/toy/sorter/sort_u8_asc.report.json',
							schema_version: 'x07.verify.report@0.8.0'
						},
						first_diagnostic: {
							code: 'WXTAL_VERIFY_PROVE_UNSUPPORTED',
							message: 'proof unsupported'
						}
					}
				}
			]
		});

		const board = buildVerifyEvidenceBoard(session.op_log[0], session, projectTemplates[0]);
		expect(board.source).toBe('report');
		expect(board.outcome).toBe('warn');
		expect(board.entries[0].entry).toBe('toy.sorter.sort_u8_asc');
		expect(board.entries[0].proveRaw).toBe('unsupported');
		expect(board.tests.passed).toBe('6');
		expect(board.diagnostics.topCodes).toContain('WXTAL_VERIFY_PROVE_UNSUPPORTED x1');
		expect(board.artifacts.some((artifact) => artifact.path.includes('coverage'))).toBe(true);
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

	it('surfaces Atlas release and platform evidence in trust review signals', () => {
		let session = appendDemoOp(demoSession(), 'wasm.app.verify.atlas_release', 'succeeded');
		session = appendDemoOp(session, 'wasm.deploy.plan.atlas_release', 'succeeded');
		session = appendDemoOp(session, 'wasm.slo.eval.atlas_canary_ok', 'succeeded');
		session = appendDemoOp(session, 'lp.deploy.query.local', 'succeeded');

		const labels = buildReviewSignals(session.op_log).map((signal) => signal.label);
		expect(labels).toEqual([
			'Local platform delivery',
			'SLO evidence',
			'Deploy plan',
			'Release evidence'
		]);
	});

	it('surfaces failed agent write-root audits in review signals', () => {
		const session = appendDemoOp(demoSession(), 'agent.run.openai-codex', 'failed');
		const op = {
			...session.op_log[0],
			report_json: {
				write_audit: {
					schema_version: 'x07.studio.agent_write_audit@0.1.0',
					allowed_roots: ['src/', '.x07/studio/'],
					created: ['src/ok.txt', 'private/bad.txt'],
					modified: [],
					deleted: [],
					violations: ['private/bad.txt'],
					truncated: false
				}
			}
		};

		const audit = writeAuditFromOp(op);
		const signal = buildReviewSignals([op])[0];

		expect(audit?.violations).toEqual(['private/bad.txt']);
		expect(signal.label).toBe('Write-root audit');
		expect(signal.detail).toContain('private/bad.txt');
		expect(signal.tone).toBe('warn');
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
		expect(
			review?.files.find((file) => file.path === 'src/main.x07.json')?.semantics.map((row) => row.surface)
		).toEqual(['Exports / declarations', 'Implementation body']);
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
		const semantics = review?.files.find((file) => file.path === 'src/main.x07.json')?.semantics ?? [];
		expect(semantics.find((row) => row.pointer === '/solve')?.before).toContain('todo');
		expect(semantics.find((row) => row.pointer === '/solve')?.after).toContain('ok');
	});

	it('loads demo documentation previews for doctrine refs', async () => {
		const api = new StudioApi();
		await api.health();
		const preview = await api.previewDoc(
			demoSession(),
			'x07/docs/getting-started/agent-quickstart.md'
		);
		const directory = await api.previewDoc(demoSession(), 'x07/docs/examples');

		expect(preview.schema_version).toBe('x07.studio.doc_preview@0.1.0');
		expect(preview.title).toContain('agent quickstart');
		expect(preview.snippet).toContain('x07 run');
		expect(directory.media_kind).toBe('directory');
		expect(directory.entries.map((entry) => entry.path)).toContain(
			'x07/docs/examples/workflow-graph'
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

	it('does not replace connected agent policy failures with demo content', async () => {
		const originalFetch = globalThis.fetch;
		const api = new StudioApi();
		globalThis.fetch = (async (input: RequestInfo | URL) => {
			const path = String(input);
			if (path.endsWith('/v1/health')) {
				return new Response(JSON.stringify({ ok: true, workspace_root: '/workspace' }), {
					status: 200
				});
			}
			if (path.includes('/agents/openai-codex/run')) {
				return new Response('agent command `codex` is not available', { status: 409 });
			}
			return new Response('not found', { status: 404 });
		}) as typeof fetch;
		try {
			await api.health();
			await expect(api.runAgentHandoff(demoSession(), 'openai-codex', 'execute')).rejects.toThrow(
				'agent command `codex` is not available'
			);
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

	it('extracts a reviewable agent handoff contract', () => {
		const session = demoSession();
		const handoff = {
			schema_version: 'x07.studio.agent_handoff@0.1.0' as const,
			session_id: session.session_id,
			agent_id: 'openai-codex',
			agent_label: 'OpenAI Codex',
			command: ['codex', '.x07/studio/handoffs/demo.md'],
			prompt_path: '.x07/studio/handoffs/demo.md',
			prompt: [
				'# x07 Studio Agent Handoff',
				'',
				'## Execution Boundary',
				'',
				'- Use `x07 run` as the default execution front door.',
				'- SLO/budget: preserve budget evidence before certification.',
				'',
				'## Automation Runbook',
				'',
				'- `approve_spec` -> session contract lock.',
				'- `xtal.verify` -> verify summary.',
				'',
				'## Agent Event Protocol',
				'',
				'Emit `x07.studio.agent_event@0.1.0` JSONL milestones.'
			].join('\n'),
			allowed_verbs: ['intent.formalize', 'xtal.verify'],
			mcp_tools: ['x07.search_v1'],
			write_roots: ['spec/', 'src/'],
			approval_required: true,
			artifacts: ['.x07/studio/handoffs/demo.md'],
			created_at: 'now'
		};

		const review = buildAgentHandoffReview(session, 'openai-codex', handoff);
		expect(review.agentLabel).toBe('OpenAI Codex');
		expect(review.command).toBe('codex .x07/studio/handoffs/demo.md');
		expect(review.approval).toBe('Human checkpoint before execute');
		expect(review.boundaries).toContain('Use `x07 run` as the default execution front door.');
		expect(review.runbook).toContain('`xtal.verify` -> verify summary.');
		expect(review.eventProtocol).toContain('x07.studio.agent_event@0.1.0');
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
		expect(ids).toContain('lp.deploy.accept.local');
		expect(ids).toContain('lp.deploy.run.local.metrics');
		expect(ids).toContain('lp.deploy.query.local');
		expect(ids).not.toContain('lp.rollout.status');
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

	it('derives world, capability, and budget gates from complex project briefs', () => {
		let session = demoSession();
		session = reduceDemoEvent(session, 'formalize_intent', createIntentPacket(session, projectTemplates[3].prompt));
		session = reduceDemoEvent(session, 'draft_spec');
		session = reduceDemoEvent(session, 'approve_spec');

		const gatewayGuard = buildWorldBudgetGuard(session, projectTemplates[3]);
		expect(gatewayGuard.worlds.map((item) => item.label)).toContain('solve-rr');
		expect(gatewayGuard.worlds.map((item) => item.label)).toContain('sandbox');
		expect(gatewayGuard.capabilities.map((item) => item.label)).toContain('network / OS');
		expect(gatewayGuard.budgets.map((item) => item.label)).toContain('replay budget');
		expect(gatewayGuard.gates).toContain('Capability widening requires review');

		const atlasGuard = buildWorldBudgetGuard(session, projectTemplates[5]);
		expect(atlasGuard.worlds.map((item) => item.label)).toContain('wasm app');
		expect(atlasGuard.capabilities.map((item) => item.label)).toContain('release');
		expect(atlasGuard.budgets.map((item) => item.label)).toContain('SLO budget');
		expect(atlasGuard.gates).toContain('Release/provenance gate required');
	});
});
