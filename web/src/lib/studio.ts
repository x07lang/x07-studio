export type TaskType =
	| 'new_behavior'
	| 'bug_fix'
	| 'behavior_change'
	| 'incident_repair'
	| 'explanation'
	| 'brownfield_extract';

export type Room =
	| 'intent'
	| 'spec'
	| 'realization'
	| 'verify'
	| 'repair'
	| 'trust'
	| 'ops'
	| 'providers'
	| 'mcp';

export type SessionPhase =
	| 'intent_drafting'
	| 'intent_ready'
	| 'spec_draft'
	| 'spec_review'
	| 'spec_approved'
	| 'realization_proposed'
	| 'verify_running'
	| 'repair_eligible'
	| 'trust_review'
	| 'certify_running'
	| 'certified'
	| 'incident_ingesting'
	| 'human_intervention_required';

export type OperationStatus = 'pending' | 'running' | 'succeeded' | 'failed';
export type IntentInputMode = 'text' | 'voice' | 'incident';
export type ProjectDifficulty = 'simple' | 'intermediate' | 'advanced' | 'complex' | 'expert';

export interface IntentPacket {
	schema_version: 'x07.studio.intent_packet@0.1.0';
	session_id: string;
	workspace_root: string;
	task_type: TaskType;
	targets: Array<{ module_id: string; entry?: string | null }>;
	examples: string[];
	constraints: string[];
	policy_implications: string[];
	ambiguities: string[];
	assumptions: string[];
	witnesses: Array<{
		kind: 'desired_behavior' | 'forbidden_behavior' | 'policy_requirement' | 'incident_report';
		text: string;
	}>;
	source: { kind: 'text'; raw: string } | { kind: 'voice'; transcript: string } | { kind: 'incident'; path: string };
}

export interface OpRecord {
	schema_version?: string;
	id: string;
	session_id?: string;
	op: string;
	backend: string;
	command: string[];
	started_at: string;
	finished_at?: string | null;
	status: OperationStatus;
	exit_code?: number | null;
	artifacts: string[];
	notes?: string | null;
	stdout?: string | null;
	stderr?: string | null;
	stdout_json?: unknown;
	stderr_json?: unknown;
	report_json?: unknown;
	report_path?: string | null;
}

export interface ArtifactPreviewResponse {
	schema_version: 'x07.studio.artifact_preview@0.1.0';
	artifact: string;
	media_kind: 'json' | 'text' | 'binary';
	bytes_read: number;
	truncated: boolean;
	text?: string | null;
	json?: unknown;
	patchset_preview?: PatchsetPreview | null;
}

export interface PatchsetPreview {
	schema_version: 'x07.studio.patchset_preview@0.1.0';
	targets: PatchsetTargetPreview[];
}

export interface PatchsetTargetPreview {
	path: string;
	note?: string | null;
	operations: number;
	before_json?: unknown;
	after_json?: unknown;
	apply_error?: string | null;
	truncated: boolean;
}

export interface SessionSnapshot {
	schema_version: string;
	session_id: string;
	title: string;
	root: string;
	task_type: TaskType;
	room: Room;
	phase: SessionPhase;
	intent?: IntentPacket | null;
	contract?: {
		schema_version: string;
		allowed_verbs: string[];
		global_doctrine: { mcp_tools: string[]; doc_refs: string[] };
		project_doctrine: {
			xtal_manifest: string;
			agent_md: string;
			write_policy: { agent_write_specs: boolean; agent_write_arch: boolean; paths: string[] };
		};
		task_doctrine: { intent_ref?: string | null; focus_paths: string[]; baseline_refs: string[] };
	} | null;
	allowed_verbs: string[];
	op_log: OpRecord[];
}

export interface BindingDescriptor {
	id: string;
	category: string;
	program: string;
	notes: string;
}

export interface HealthResponse {
	ok: boolean;
	workspace_root: string;
}

export interface ProviderCard {
	id: string;
	label: string;
	model: string;
	bridge: string;
	trust: 'local' | 'remote' | 'review';
	status: 'ready' | 'needs_probe' | 'not_configured';
}

export interface AgentLane {
	id: 'codex' | 'claude-code';
	label: string;
	role: string;
	verbs: string[];
	writeScope: string;
	reviewGate: string;
	status: 'available' | 'configure' | 'review_only';
}

export type AgentStatus = 'available' | 'needs_install' | 'disabled';

export interface AgentProfile {
	schema_version: 'x07.studio.agent_profile@0.1.0';
	id: string;
	label: string;
	command: string;
	args: string[];
	allowed_verbs: string[];
	mcp_tools: string[];
	write_roots: string[];
	approval_required: boolean;
	status: AgentStatus;
	notes: string;
}

export interface AgentHandoff {
	schema_version: 'x07.studio.agent_handoff@0.1.0';
	session_id: string;
	agent_id: string;
	agent_label: string;
	command: string[];
	prompt_path: string;
	prompt: string;
	allowed_verbs: string[];
	mcp_tools: string[];
	write_roots: string[];
	approval_required: boolean;
	artifacts: string[];
	created_at: string;
}

export interface AgentHandoffResponse {
	handoff: AgentHandoff;
	session: SessionSnapshot;
}

export interface FormalizeIntentResponse {
	intent: IntentPacket;
	op: OpRecord;
	session: SessionSnapshot;
}

export type AgentRunMode = 'plan' | 'execute';

export interface AgentRunRequest {
	mode: AgentRunMode;
	timeout_seconds?: number | null;
}

export interface AgentRunResponse {
	handoff: AgentHandoff;
	op: OpRecord;
	session: SessionSnapshot;
}

export type ApprovalDecision = 'approve' | 'reject';

export interface AgentApprovalResponse {
	op: OpRecord;
	session: SessionSnapshot;
}

export interface ProjectTemplate {
	id: ProjectDifficulty;
	label: string;
	title: string;
	taskType: TaskType;
	prompt: string;
	revision: string;
	sourcePath: string;
	riskProfile: string;
	canonicalCommands: string[];
	artifacts: string[];
}

export const rooms: Array<{ id: Room; label: string }> = [
	{ id: 'intent', label: 'Intent' },
	{ id: 'spec', label: 'Spec' },
	{ id: 'realization', label: 'Realize' },
	{ id: 'verify', label: 'Verify' },
	{ id: 'repair', label: 'Repair' },
	{ id: 'trust', label: 'Trust' },
	{ id: 'ops', label: 'Ops' },
	{ id: 'providers', label: 'Agents' }
];

export const lifecycle: Array<{ phase: SessionPhase; label: string; room: Room; binding?: string }> = [
	{ phase: 'intent_drafting', label: 'Intent', room: 'intent' },
	{ phase: 'intent_ready', label: 'Intent Packet', room: 'intent' },
	{ phase: 'spec_draft', label: 'Spec Draft', room: 'spec', binding: 'spec.check' },
	{ phase: 'spec_approved', label: 'Approved Spec', room: 'realization', binding: 'impl.check' },
	{ phase: 'realization_proposed', label: 'Realization', room: 'realization', binding: 'impl.sync.patchset' },
	{ phase: 'verify_running', label: 'Verification', room: 'verify', binding: 'xtal.verify' },
	{ phase: 'trust_review', label: 'Trust Review', room: 'trust', binding: 'xtal.certify' },
	{ phase: 'certified', label: 'Certified', room: 'ops' }
];

export const providerCards: ProviderCard[] = [
	{
		id: 'openai-codex',
		label: 'OpenAI Codex',
		model: 'Responses API / Codex CLI',
		bridge: 'MCP tools + canonical x07 bindings',
		trust: 'remote',
		status: 'needs_probe'
	},
	{
		id: 'claude-code',
		label: 'Claude Code',
		model: 'Claude Code CLI',
		bridge: 'Session contract + guarded command lane',
		trust: 'review',
		status: 'not_configured'
	},
	{
		id: 'x07lang-mcp',
		label: 'x07lang MCP',
		model: 'x07.search_v1 / x07.exec_v1',
		bridge: 'MCP JSON-RPC',
		trust: 'local',
		status: 'ready'
	}
];

export const agentLanes: AgentLane[] = [
	{
		id: 'codex',
		label: 'OpenAI Codex',
		role: 'Intent polish, x07 tool selection, repair triage',
		verbs: ['intent.formalize', 'spec.check', 'xtal.verify', 'xtal.repair'],
		writeScope: 'Spec and architecture writes require human approval',
		reviewGate: 'Approval before realization',
		status: 'available'
	},
	{
		id: 'claude-code',
		label: 'Claude Code',
		role: 'Patch planning, implementation review, alternate repair proposals',
		verbs: ['impl.sync.patchset', 'impl.check', 'xtal.certify'],
		writeScope: 'Implementation paths only after approved spec',
		reviewGate: 'Human trust review before certify',
		status: 'configure'
	}
];

export const defaultAgentProfiles: AgentProfile[] = [
	{
		schema_version: 'x07.studio.agent_profile@0.1.0',
		id: 'openai-codex',
		label: 'OpenAI Codex',
		command: 'codex',
		args: [],
		allowed_verbs: ['intent.formalize', 'spec.check', 'impl.sync.write', 'xtal.verify', 'xtal.repair'],
		mcp_tools: ['x07.search_v1', 'x07.context_pack_v1', 'x07.exec_v1'],
		write_roots: ['spec/', 'src/', 'tests/'],
		approval_required: true,
		status: 'needs_install',
		notes: 'Remote coding-agent runner gated by x07 session contract.'
	},
	{
		schema_version: 'x07.studio.agent_profile@0.1.0',
		id: 'claude-code',
		label: 'Claude Code',
		command: 'claude',
		args: [],
		allowed_verbs: ['impl.sync.patchset', 'impl.check', 'xtal.certify'],
		mcp_tools: ['x07.search_v1', 'x07.context_pack_v1'],
		write_roots: ['src/', 'tests/'],
		approval_required: true,
		status: 'needs_install',
		notes: 'Alternate coding-agent runner for implementation and review lanes.'
	}
];

export const canonicalMcpTools = [
	'x07.search_v1',
	'x07.doc_v1',
	'x07.context_pack_v1',
	'x07.exec_v1',
	'x07.patch_apply_v1'
];

export const canonicalDocRefs = [
	'x07/docs/getting-started/agent-quickstart.md',
	'x07/docs/getting-started/available-skills.md',
	'x07/docs/guides',
	'x07/docs/examples',
	'x07/docs/trust'
];

export const defaultPrompt =
	'Build a certifiable workflow graph optimizer. A human gives task durations and dependency edges. The project must compute a deterministic makespan, reject cycles, prove the pure core, and keep all agent actions visible before implementation.';

export const projectTemplates: ProjectTemplate[] = [
	{
		id: 'simple',
		label: 'Simple',
		title: 'XTAL toy sorter',
		taskType: 'new_behavior',
		prompt:
			'Use docs/examples/agent-gate/xtal/toy-sorter as the model. Build a deterministic integer sorter with spec/toy.sorter.x07spec.json, reviewable examples, generated tests under gen/xtal, and x07 xtal verify evidence before implementation is trusted.',
		revision: 'Keep the operation pure and make the empty-input rejection explicit.',
		sourcePath: 'x07/docs/examples/agent-gate/xtal/toy-sorter',
		riskProfile: 'solve-pure XTAL',
		canonicalCommands: [
			'x07 xtal verify --project x07.json',
			'x07 gen verify --index arch/gen/index.x07gen.json',
			'x07 xtal impl check --project x07.json'
		],
		artifacts: [
			'spec/toy.sorter.x07spec.json',
			'gen/xtal/tests.json',
			'target/xtal/verify/summary.json'
		]
	},
	{
		id: 'intermediate',
		label: 'Intermediate',
		title: 'XTAL workflow graph',
		taskType: 'new_behavior',
		prompt:
			`${defaultPrompt} Follow docs/examples/agent-gate/xtal/workflow-graph: generate tests from spec, verify generated drift, run the generated manifest, check impl/spec alignment, and keep the manual smoke suite separate from XTAL evidence.`,
		revision: 'Tighten examples, make proof boundary explicit, keep OS access off by default.',
		sourcePath: 'x07/docs/examples/agent-gate/xtal/workflow-graph',
		riskProfile: 'solve-pure XTAL with generated properties',
		canonicalCommands: [
			'x07 xtal tests gen-from-spec --project x07.json --write',
			'x07 gen verify --index arch/gen/index.x07gen.json',
			'x07 test --all --no-fail-fast --manifest gen/xtal/tests.json',
			'x07 xtal dev --project x07.json',
			'x07 xtal verify --project x07.json'
		],
		artifacts: [
			'spec/workflow.graph.x07spec.json',
			'gen/xtal/workflow/graph/tests.x07.json',
			'target/xtal/verify/summary.json'
		]
	},
	{
		id: 'advanced',
		label: 'Advanced',
		title: 'State machine arch contracts',
		taskType: 'new_behavior',
		prompt:
			'Use docs/examples/readiness-checks/x07-sm-arch-contracts-smoke as the model. Build a lifecycle state-machine project where x07 sm gen creates the deterministic step function and tests, x07 arch check enforces contracts_v1.sm and generated outputs stay up to date, and budget.scope_from_arch_v1 wraps the hot path.',
		revision: 'Require generated-output drift checks, arch contract lock evidence, and step-level budget profile evidence.',
		sourcePath: 'x07/docs/examples/readiness-checks/x07-sm-arch-contracts-smoke',
		riskProfile: 'solve-pure + arch contract generation',
		canonicalCommands: [
			'x07 sm gen --input arch/sm/specs/lifecycle.sm.json --out gen/sm --write',
			'x07 test --manifest gen/sm/tests.manifest.json',
			'x07 arch check --write-lock',
			'x07 test --manifest tests/tests.json'
		],
		artifacts: [
			'arch/sm/index.x07sm.json',
			'arch/manifest.x07arch.json',
			'gen/sm/tests.manifest.json'
		]
	},
	{
		id: 'complex',
		label: 'Complex',
		title: 'Replayable API gateway',
		taskType: 'new_behavior',
		prompt:
			'Use docs/examples/apps/x07-api-gateway as the model. Build a production-shaped API gateway with a deterministic solve-pure routing core, a solve-rr replay adapter for upstream HTTP, sandbox policy, rr cassette fixtures, review artifacts, and CI trust scripts.',
		revision: 'Keep pure routing separate from RR replay and require committed cassette evidence before trust review.',
		sourcePath: 'x07/docs/examples/apps/x07-api-gateway',
		riskProfile: 'solve-pure + solve-rr + sandbox',
		canonicalCommands: [
			'x07 test --manifest tests/tests.json',
			'x07 run --profile sandbox',
			'x07 bundle --profile sandbox --out dist/x07-api-gateway'
		],
		artifacts: [
			'arch/rr/index.x07rr.json',
			'tests/fixtures/replay/rr/upstream_example.rrbin',
			'ci/trust.sh'
		]
	},
	{
		id: 'expert',
		label: 'Expert',
		title: 'DB drift guard',
		taskType: 'incident_repair',
		prompt:
			'Use docs/examples/apps/x07dbguard as the model. Build a DB migration and drift guard that fingerprints arch/db migration plans deterministically, applies migrations through run-os or run-os-sandboxed, verifies drift from solve-rr fixtures, and emits trust/review artifacts.',
		revision: 'Require separate evidence for deterministic plan fingerprinting, policy-gated apply, RR drift verification, and certification.',
		sourcePath: 'x07/docs/examples/apps/x07dbguard',
		riskProfile: 'solve-pure + solve-rr + run-os-sandboxed',
		canonicalCommands: [
			'x07 pkg lock --project x07.json',
			'x07 test --manifest tests/tests.json',
			'x07 run --profile sandbox -- verify',
			'x07 bundle --profile sandbox --out dist/x07dbguard'
		],
		artifacts: [
			'arch/db/index.x07db.json',
			'arch/budgets/index.x07budgets.json',
			'tests/fixtures/replay/rr/verify_ok.rrbin'
		]
	}
];

export function createIntentPacket(
	session: SessionSnapshot,
	raw: string,
	inputMode: IntentInputMode = 'text',
	revisionNotes: string[] = []
): IntentPacket {
	const normalized = raw.trim() || defaultPrompt;
	const lowered = normalized.toLowerCase();
	const isSorter = lowered.includes('sort');
	const isIncident = lowered.includes('incident') || lowered.includes('repair');
	const isStateMachine = lowered.includes('state machine') || lowered.includes('x07 sm');
	const isGateway = lowered.includes('api gateway') || lowered.includes('x07-api-gateway');
	const isCrawler = lowered.includes('crawler') || lowered.includes('x07crawl');
	const isDbGuard = lowered.includes('db migration') || lowered.includes('x07dbguard') || lowered.includes('drift guard');
	const isWorkflowGraph = lowered.includes('workflow graph') || lowered.includes('makespan') || lowered.includes('dag');
	const moduleId = isSorter
		? 'toy.sorter'
		: isDbGuard
			? 'db.guard'
			: isGateway
				? 'gateway.core'
				: isCrawler
					? 'crawl.plan'
					: isStateMachine
						? 'workflow.lifecycle'
						: isIncident
							? 'ops.incident_repair'
							: isWorkflowGraph
								? 'workflow.graph'
								: 'workflow.graph';
	const entry = isSorter
		? 'sort_u8_asc'
		: isDbGuard
			? 'verify_drift'
			: isGateway
				? 'route_request_v1'
				: isCrawler
					? 'plan_crawl_v1'
					: isStateMachine
						? 'step_v1'
						: isIncident
							? 'classify_and_repair'
							: 'makespan_u32';
	const incidentWitness =
		inputMode === 'incident'
			? [{ kind: 'incident_report' as const, text: normalized }]
			: [];
	const extraPolicyImplications =
		isGateway || isCrawler || isDbGuard
			? ['RR fixtures, sandbox policy, and OS/network/db capability widening require explicit review.']
			: isStateMachine
				? ['Generated outputs, arch contracts, and budget profiles require drift evidence before certify.']
				: [];
	return {
		schema_version: 'x07.studio.intent_packet@0.1.0',
		session_id: session.session_id,
		workspace_root: session.root,
		task_type: session.task_type,
		targets: [{ module_id: moduleId, entry }],
		examples: [
			'Input examples become spec examples before implementation.',
			'Generated tests must be reviewable before verify.'
		],
		constraints: [
			'Use spec-first XTAL flow.',
			'Keep solve worlds deterministic by default.',
			'Route spec-changing repairs back to human approval.',
			...revisionNotes.map((note) => `Revision request: ${note}`)
		],
		policy_implications: [
			'OS worlds, network, budget, and trust widening require explicit review.',
			...extraPolicyImplications
		],
		ambiguities: [
			'Acceptance examples need final human approval.',
			'Proof strictness should be selected before certify.'
		],
		assumptions: [
			'Agent may edit implementation paths after spec approval.',
			'Agent may not widen specs or architecture policy without approval.'
		],
		witnesses: [
			{ kind: 'desired_behavior', text: normalized },
			{ kind: 'policy_requirement', text: 'All agent work must flow through canonical x07/XTAL bindings.' },
			{ kind: 'forbidden_behavior', text: 'Do not turn the prompt directly into unchecked source code.' },
			...incidentWitness
		],
		source:
			inputMode === 'voice'
				? { kind: 'voice', transcript: normalized }
				: inputMode === 'incident'
					? { kind: 'incident', path: '.x07/studio/incidents/manual-note.jsonl' }
					: { kind: 'text', raw: normalized }
	};
}

export function demoSession(): SessionSnapshot {
	const session: SessionSnapshot = {
		schema_version: 'x07.studio.session_snapshot@0.1.0',
		session_id: 'st-demo-xtal',
		title: 'Workflow graph optimizer',
		root: '/workspace/x07-project',
		task_type: 'new_behavior',
		room: 'intent',
		phase: 'intent_drafting',
		intent: null,
		contract: null,
		allowed_verbs: ['intent_formalize'],
		op_log: []
	};
	return session;
}

export function demoBindings(): BindingDescriptor[] {
	return [
		{ id: 'project.init.xtal-pure', category: 'x07/project', program: 'x07', notes: 'Initialize XTAL project.' },
		{ id: 'spec.scaffold', category: 'xtal/spec', program: 'x07', notes: 'Create operation specs.' },
		{ id: 'spec.check', category: 'xtal/spec', program: 'x07', notes: 'Validate specs.' },
		{ id: 'tests.gen.write', category: 'xtal/tests', program: 'x07', notes: 'Generate tests from spec.' },
		{ id: 'tests.gen.check', category: 'xtal/tests', program: 'x07', notes: 'Check generated tests.' },
		{ id: 'gen.verify', category: 'x07/gen', program: 'x07', notes: 'Verify generated artifacts.' },
		{ id: 'test.manifest', category: 'x07/test', program: 'x07', notes: 'Run project tests.' },
		{ id: 'test.xtal.generated.all', category: 'x07/test', program: 'x07', notes: 'Run generated XTAL tests.' },
		{ id: 'test.sm.generated', category: 'x07/test', program: 'x07', notes: 'Run generated state-machine tests.' },
		{ id: 'sm.gen.write', category: 'x07/sm', program: 'x07', notes: 'Generate state-machine artifacts.' },
		{ id: 'arch.check.write_lock', category: 'x07/arch', program: 'x07', notes: 'Refresh architecture contract locks.' },
		{ id: 'pkg.lock', category: 'x07/package', program: 'x07', notes: 'Resolve project lockfile.' },
		{ id: 'run.sandbox', category: 'x07/run', program: 'x07', notes: 'Run sandbox profile.' },
		{ id: 'run.sandbox.os', category: 'x07/run', program: 'x07', notes: 'Run sandbox profile with OS-backed isolation.' },
		{ id: 'run.stdin', category: 'x07/run', program: 'x07', notes: 'Run with Studio-provided stdin.' },
		{ id: 'run.sandbox.stdin.os', category: 'x07/run', program: 'x07', notes: 'Run sandbox profile with Studio stdin and OS-backed isolation.' },
		{ id: 'bundle.api_gateway.sandbox.os', category: 'x07/bundle', program: 'x07', notes: 'Bundle API gateway with OS-backed sandbox isolation.' },
		{ id: 'bundle.dbguard.sandbox.os', category: 'x07/bundle', program: 'x07', notes: 'Bundle DB guard with OS-backed sandbox isolation.' },
		{ id: 'run.x07crawl.sandbox.os', category: 'x07/run', program: 'x07', notes: 'Run x07crawl replay with OS-backed sandbox isolation.' },
		{ id: 'bundle.x07crawl.sandbox.os', category: 'x07/bundle', program: 'x07', notes: 'Bundle x07crawl with OS-backed sandbox isolation.' },
		{ id: 'impl.check', category: 'xtal/impl', program: 'x07', notes: 'Inspect realization drift.' },
		{ id: 'impl.sync.write', category: 'xtal/impl', program: 'x07', notes: 'Synchronize implementation.' },
		{ id: 'impl.sync.patchset', category: 'xtal/impl', program: 'x07', notes: 'Generate implementation patchset.' },
		{ id: 'xtal.verify', category: 'xtal/e2e', program: 'x07', notes: 'Run coverage, proof, and tests.' },
		{ id: 'xtal.repair', category: 'xtal/e2e', program: 'x07', notes: 'Repair from diagnostics.' },
		{ id: 'xtal.certify', category: 'xtal/e2e', program: 'x07', notes: 'Generate trust evidence.' }
	];
}

export function reduceDemoEvent(session: SessionSnapshot, event: string, payload?: IntentPacket): SessionSnapshot {
	const next = structuredClone(session) as SessionSnapshot;
	switch (event) {
		case 'formalize_intent':
			next.intent = payload ?? createIntentPacket(next, defaultPrompt);
			next.phase = 'intent_ready';
			next.room = 'intent';
			next.allowed_verbs = ['intent_formalize', 'intent_review', 'spec_edit'];
			return next;
		case 'draft_spec':
			next.phase = 'spec_draft';
			next.room = 'spec';
			next.allowed_verbs = ['spec_edit', 'spec_check', 'spec_approve'];
			return next;
		case 'approve_spec':
			next.phase = 'spec_approved';
			next.room = 'realization';
			next.allowed_verbs = ['impl_sync'];
			next.contract = {
				schema_version: 'x07.studio.session_contract@0.1.0',
				allowed_verbs: next.allowed_verbs,
				global_doctrine: {
					mcp_tools: canonicalMcpTools,
					doc_refs: canonicalDocRefs
				},
				project_doctrine: {
					xtal_manifest: 'arch/xtal/xtal.json',
					agent_md: 'AGENT.md',
					write_policy: {
						agent_write_specs: false,
						agent_write_arch: false,
						paths: ['src/', 'tests/', 'spec/']
					}
				},
				task_doctrine: {
					intent_ref: `.x07/studio/sessions/${next.session_id}.json`,
					focus_paths: ['spec/', 'src/', 'tests/'],
					baseline_refs: ['target/xtal/verify/summary.json']
				}
			};
			return next;
		case 'propose_realization':
			next.phase = 'realization_proposed';
			next.room = 'realization';
			next.allowed_verbs = ['impl_review', 'verify_run'];
			return next;
		case 'accept_realization':
			next.phase = 'verify_running';
			next.room = 'verify';
			next.allowed_verbs = ['verify_run'];
			return next;
		case 'verification_passed':
			next.phase = 'trust_review';
			next.room = 'trust';
			next.allowed_verbs = ['trust_review', 'certify_run'];
			return next;
		case 'verification_failed':
			next.phase = 'repair_eligible';
			next.room = 'repair';
			next.allowed_verbs = ['repair_run', 'repair_suggest_spec_patch'];
			return next;
		case 'approve_trust':
			next.phase = 'certify_running';
			next.room = 'trust';
			next.allowed_verbs = ['certify_run'];
			return next;
		case 'certification_passed':
			next.phase = 'certified';
			next.room = 'ops';
			next.allowed_verbs = ['incident_ingest', 'improve_run'];
			return next;
		default:
			return next;
	}
}

export function appendDemoOp(
	session: SessionSnapshot,
	bindingId: string,
	status: OperationStatus,
	command?: string[],
	artifacts?: string[]
): SessionSnapshot {
	const next = structuredClone(session) as SessionSnapshot;
	next.op_log = [
		...next.op_log,
		{
			id: `op-${next.op_log.length + 1}`,
			op: bindingId,
			backend: 'demo',
			command: command ?? ['x07', ...bindingId.split('.')],
			started_at: String(Date.now()),
			finished_at: String(Date.now()),
			status,
			exit_code: status === 'succeeded' ? 0 : status === 'failed' ? 1 : null,
			artifacts: artifacts ?? demoArtifactsFor(bindingId),
			notes: 'visible agent operation record'
		}
	];
	return next;
}

function demoArtifactsFor(bindingId: string): string[] {
	switch (bindingId) {
		case 'intent.formalize':
			return ['.x07/studio/sessions/intent.json'];
		case 'project.init.xtal-pure':
			return ['x07.json', 'spec/', 'src/', 'gen/xtal/'];
		case 'spec.scaffold':
			return ['spec/workflow.graph.x07spec.json'];
		case 'spec.check':
			return ['target/xtal/spec.check.report.json'];
		case 'tests.gen.write':
			return ['gen/xtal/tests.json'];
		case 'impl.sync.write':
			return ['src/', 'target/xtal/impl-sync.patchset.json'];
		case 'impl.check':
			return ['target/xtal/impl.check.report.json'];
		case 'xtal.verify':
			return ['target/xtal/verify/summary.json'];
		case 'xtal.repair':
			return ['target/xtal/repair/summary.json', 'target/xtal/repair/patchset.json'];
		case 'xtal.certify':
			return ['target/xtal/cert/summary.json', 'target/xtal/cert/bundle.json'];
		default:
			return [`target/xtal/${bindingId.replaceAll('.', '/')}/summary.json`];
	}
}

export function phaseIndex(phase: SessionPhase): number {
	const exact = lifecycle.findIndex((item) => item.phase === phase);
	if (exact >= 0) return exact;
	if (phase === 'spec_review') return 2;
	if (phase === 'repair_eligible') return 5;
	if (phase === 'certify_running') return 6;
	if (phase === 'incident_ingesting') return 7;
	return 0;
}

export function nextPrimaryAction(phase: SessionPhase): string {
	switch (phase) {
		case 'intent_drafting':
			return 'Polish intent';
		case 'intent_ready':
			return 'Draft spec';
		case 'spec_draft':
		case 'spec_review':
			return 'Approve spec';
		case 'spec_approved':
			return 'Sync realization';
		case 'realization_proposed':
			return 'Run verify';
		case 'verify_running':
			return 'Record verify result';
		case 'repair_eligible':
			return 'Repair';
		case 'trust_review':
			return 'Certify';
		default:
			return 'Open session';
	}
}

export function workflowChecklist(session: SessionSnapshot): Array<{
	label: string;
	state: 'done' | 'active' | 'blocked';
}> {
	return [
		{
			label: 'Intent packet',
			state: session.intent ? 'done' : 'active'
		},
		{
			label: 'Human spec approval',
			state: phaseIndex(session.phase) >= phaseIndex('spec_approved') ? 'done' : session.intent ? 'active' : 'blocked'
		},
		{
			label: 'Guarded realization',
			state:
				phaseIndex(session.phase) >= phaseIndex('realization_proposed')
					? 'done'
					: session.phase === 'spec_approved'
						? 'active'
						: 'blocked'
		},
		{
			label: 'Verify or repair',
			state:
				session.phase === 'repair_eligible' || phaseIndex(session.phase) >= phaseIndex('trust_review')
					? 'done'
					: session.phase === 'verify_running'
						? 'active'
						: 'blocked'
		},
		{
			label: 'Trust evidence',
			state: session.phase === 'certified' ? 'done' : session.phase === 'trust_review' ? 'active' : 'blocked'
		}
	];
}
