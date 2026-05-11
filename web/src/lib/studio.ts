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
export type ProjectDifficulty = 'simple' | 'intermediate' | 'complex';

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
	id: string;
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
	report_path?: string | null;
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

export const defaultPrompt =
	'Build a certifiable workflow graph optimizer. A human gives task durations and dependency edges. The project must compute a deterministic makespan, reject cycles, prove the pure core, and keep all agent actions visible before implementation.';

export const projectTemplates: ProjectTemplate[] = [
	{
		id: 'simple',
		label: 'Simple',
		title: 'Stable sorter',
		taskType: 'new_behavior',
		prompt:
			'Build a deterministic integer sorter. It accepts a list of signed integers, returns ascending order, rejects empty input, and includes one reviewable example before implementation.',
		revision: 'Keep the operation pure and make the empty-input rejection explicit.'
	},
	{
		id: 'intermediate',
		label: 'Intermediate',
		title: 'Workflow graph optimizer',
		taskType: 'new_behavior',
		prompt: defaultPrompt,
		revision: 'Tighten examples, make proof boundary explicit, keep OS access off by default.'
	},
	{
		id: 'complex',
		label: 'Complex',
		title: 'Incident repair control plane',
		taskType: 'incident_repair',
		prompt:
			'Build an incident repair workflow for a policy-backed x07 service. It ingests a failed verification bundle, classifies whether the repair is spec-preserving, proposes a patchset, requires human approval before widening policy or architecture, reruns xtal.verify, and emits certification evidence for runtime ops.',
		revision: 'Require separate evidence for incident classification, policy widening, and certification.'
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
	const moduleId = isSorter ? 'app.sorter' : isIncident ? 'ops.incident_repair' : 'workflow.graph';
	const entry = isSorter ? 'sort_ascending' : isIncident ? 'classify_and_repair' : 'makespan_u32';
	const incidentWitness =
		inputMode === 'incident'
			? [{ kind: 'incident_report' as const, text: normalized }]
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
		policy_implications: ['OS worlds, network, budget, and trust widening require explicit review.'],
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
					mcp_tools: ['x07.search_v1', 'x07.context_pack_v1', 'x07.exec_v1'],
					doc_refs: ['x07/docs/getting-started/agent-quickstart.md', 'x07/docs/getting-started/available-skills.md']
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
			artifacts: artifacts ?? [`target/xtal/${bindingId.replaceAll('.', '/')}/summary.json`],
			notes: 'visible agent operation record'
		}
	];
	return next;
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
