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
export type IntentInputMode = 'text' | 'voice' | 'spec' | 'incident';
export type IntentWitnessKind =
	| 'desired_behavior'
	| 'forbidden_behavior'
	| 'policy_requirement'
	| 'incident_report';
export type IntentWitness = {
	kind: IntentWitnessKind;
	text: string;
};
export type ProjectDifficulty =
	| 'simple'
	| 'intermediate'
	| 'advanced'
	| 'complex'
	| 'expert'
	| 'atlas';

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
	witnesses: IntentWitness[];
	source:
		| { kind: 'text'; raw: string }
		| { kind: 'voice'; transcript: string }
		| { kind: 'spec'; raw: string }
		| { kind: 'incident'; path: string };
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

export interface DocPreviewEntry {
	path: string;
	title: string;
	kind: 'file' | 'directory';
}

export interface DocPreviewResponse {
	schema_version: 'x07.studio.doc_preview@0.1.0';
	doc_ref: string;
	resolved_path: string;
	title: string;
	media_kind: 'markdown' | 'json' | 'text' | 'directory';
	bytes_read: number;
	truncated: boolean;
	snippet: string;
	entries: DocPreviewEntry[];
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
	defaults: StudioDefaults;
	components: RuntimeComponentStatus[];
}

export interface StudioDefaults {
	daemon_addr: string;
	provider_profile_id: string;
	platform_state_dir: string;
}

export type RuntimeComponentState = 'available' | 'missing';

export interface RuntimeComponentStatus {
	id: string;
	label: string;
	command: string;
	required: boolean;
	status: RuntimeComponentState;
	source?: string | null;
	install_hint: string;
}

export type OnboardingStepState = 'ready' | 'required' | 'optional';

export interface OnboardingStep {
	id: string;
	label: string;
	state: OnboardingStepState;
	command: string;
	detail: string;
}

export interface WorkspaceRadarResponse {
	schema_version: 'x07.studio.workspace_radar@0.1.0';
	workspace_root: string;
	xtal_manifest: WorkspacePathState;
	spec_count: number;
	generated_tests: WorkspacePathState;
	latest_verify: WorkspacePathState | null;
	latest_certify: WorkspacePathState | null;
	incident_count: number;
}

export interface WorkspacePathState {
	path: string;
	exists: boolean;
	modified_unix_ms: number | null;
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

export type GuardTone = 'ok' | 'review' | 'blocked';
export type ApprovalLoopState = 'drafting' | 'awaiting' | 'changes' | 'approved';

export interface GuardRailItem {
	label: string;
	value: string;
	detail: string;
	tone: GuardTone;
}

export interface WorldBudgetGuard {
	posture: string;
	review: string;
	worlds: GuardRailItem[];
	capabilities: GuardRailItem[];
	budgets: GuardRailItem[];
	gates: string[];
}

export interface ApprovalLedgerItem {
	label: string;
	detail: string;
	state: 'done' | 'active' | 'blocked';
}

export interface AgentReadiness {
	state: AgentStatus;
	source: string;
	detail: string;
	gate: string;
	canRun: boolean;
}

export type AutomationPlanState = 'blocked' | 'ready' | 'running' | 'done' | 'failed';

export interface AutomationPlanStep {
	label: string;
	command: string;
	artifact: string;
	gate: string;
	state: AutomationPlanState;
}

export type EvidenceCoverageState = 'done' | 'active' | 'blocked' | 'failed';

export interface EvidenceCoverageItem {
	id: string;
	label: string;
	requirement: string;
	evidence: string;
	artifact: string;
	state: EvidenceCoverageState;
	opId?: string | null;
}

export type PlatformBridgeState = EvidenceCoverageState | 'optional';

export interface PlatformBridgeItem {
	id: string;
	label: string;
	command: string;
	requirement: string;
	evidence: string;
	artifact: string;
	state: PlatformBridgeState;
	opId?: string | null;
}

export interface PlatformBridge {
	posture: string;
	summary: string;
	nextAction: string;
	items: PlatformBridgeItem[];
}

export const rooms: Array<{ id: Room; label: string }> = [
	{ id: 'intent', label: 'Intent' },
	{ id: 'spec', label: 'Spec' },
	{ id: 'realization', label: 'Realize' },
	{ id: 'verify', label: 'Verify' },
	{ id: 'repair', label: 'Repair' },
	{ id: 'trust', label: 'Trust' },
	{ id: 'ops', label: 'Ops' },
	{ id: 'providers', label: 'Agents' },
	{ id: 'mcp', label: 'MCP' }
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
	},
	{
		id: 'atlas',
		label: 'Atlas',
		title: 'Full-stack WASM app',
		taskType: 'new_behavior',
		prompt:
			'Use docs/examples/wasm_showcases/x07_atlas as the model. Build the x07 Atlas full-stack WASM app with frontend and backend x07 projects, app profile validation, deterministic app trace replay, release pack verification, provenance attestation, deploy planning, and SLO evaluation.',
		revision:
			'Require the Studio run to prove profile validation, app build, smoke serve, checked-in regression replay, release pack verification, provenance, deploy plan, and SLO evidence.',
		sourcePath: 'x07/docs/examples/wasm_showcases/x07_atlas',
		riskProfile: 'full-stack x07-wasm app + provenance + SLO',
		canonicalCommands: [
			'x07-wasm app profile validate --profile atlas_dev',
			'x07-wasm app build --profile atlas_dev --out-dir dist/showcase_fullstack/app.atlas_dev --clean',
			'x07-wasm app test --dir dist/showcase_fullstack/app.atlas_dev --trace tests/traces/happy_path.trace.json',
			'x07-wasm app pack --bundle-manifest dist/showcase_fullstack/app.atlas_release/app.bundle.json --profile-id atlas_release --out-dir dist/showcase_fullstack/pack.atlas_release',
			'x07-wasm provenance verify --attestation dist/showcase_fullstack/pack.atlas_release/app.provenance.dsse.json --pack-dir dist/showcase_fullstack/pack.atlas_release --trusted-public-key arch/provenance/dev.ed25519.public_key.b64'
		],
		artifacts: [
			'arch/app/index.x07app.json',
			'dist/showcase_fullstack/app.atlas_dev/app.bundle.json',
			'dist/showcase_fullstack/pack.atlas_release/app.pack.json',
			'dist/showcase_fullstack/deploy.atlas_release'
		]
	}
];

export function buildWorldBudgetGuard(
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate,
	ops: OpRecord[] = []
): WorldBudgetGuard {
	const haystack = [
		template.riskProfile,
		template.prompt,
		...template.canonicalCommands,
		...template.artifacts,
		...ops.flatMap((op) => [op.op, ...op.command, ...op.artifacts])
	]
		.join(' ')
		.toLowerCase();
	const has = (needle: string) => haystack.includes(needle);
	const contract = session?.contract ?? null;
	const phase = session?.phase ?? 'intent_drafting';
	const approved = Boolean(contract);
	const osWidening = has('run-os') || has('db ') || has('dbguard') || has('migration');
	const rrWidening = has('solve-rr') || has('/rr/') || has('cassette') || has('replay');
	const sandboxWidening = has('sandbox');
	const wasmWidening = has('x07-wasm') || has('wasm') || has('app build');
	const releaseWidening = has('provenance') || has('deploy') || has('release') || has('pack');
	const budgetWidening = has('budget') || has('slo') || has('profile');

	const worlds: GuardRailItem[] = [
		{
			label: 'solve-pure',
			value: 'default',
			detail: 'deterministic verification lane',
			tone: 'ok'
		}
	];
	if (rrWidening) {
		worlds.push({
			label: 'solve-rr',
			value: approved ? 'review gated' : 'planned',
			detail: 'replay fixtures must be committed evidence',
			tone: approved ? 'review' : 'blocked'
		});
	}
	if (sandboxWidening || osWidening) {
		worlds.push({
			label: sandboxWidening ? 'sandbox' : 'run-os',
			value: approved ? 'approval required' : 'blocked',
			detail: osWidening ? 'OS or DB access is capability widening' : 'policy profile must be reviewed',
			tone: approved ? 'review' : 'blocked'
		});
	}
	if (wasmWidening) {
		worlds.push({
			label: 'wasm app',
			value: approved ? 'artifact lane' : 'planned',
			detail: 'profile validation, traces, and pack verification stay visible',
			tone: approved ? 'review' : 'blocked'
		});
	}

	const writePolicy = contract?.project_doctrine.write_policy;
	const capabilities: GuardRailItem[] = [
		{
			label: 'spec writes',
			value: writePolicy?.agent_write_specs ? 'agent writable' : 'human approval',
			detail: 'spec-changing repairs return to review',
			tone: writePolicy?.agent_write_specs ? 'review' : 'ok'
		},
		{
			label: 'architecture',
			value: writePolicy?.agent_write_arch ? 'agent writable' : 'locked',
			detail: 'world, policy, and budget widening are gated',
			tone: writePolicy?.agent_write_arch ? 'review' : 'ok'
		}
	];
	if (rrWidening || sandboxWidening || osWidening) {
		capabilities.push({
			label: 'network / OS',
			value: approved ? 'review lane' : 'not granted',
			detail: 'RR, sandbox, and OS access cannot be hidden inside agent steps',
			tone: approved ? 'review' : 'blocked'
		});
	}
	if (releaseWidening) {
		capabilities.push({
			label: 'release',
			value: approved ? 'trust review' : 'blocked',
			detail: 'pack, provenance, deploy, and SLO evidence are separate gates',
			tone: approved ? 'review' : 'blocked'
		});
	}

	const budgets: GuardRailItem[] = [
		{
			label: 'verify budget',
			value: phaseIndex(phase) >= phaseIndex('verify_running') ? 'spent' : 'reserved',
			detail: 'coverage, proofs, and generated tests consume bounded lanes',
			tone: 'ok'
		}
	];
	if (budgetWidening) {
		budgets.push({
			label: has('slo') ? 'SLO budget' : 'arch budget',
			value: approved ? 'evidence required' : 'planned',
			detail: 'budget profiles must be visible before certify',
			tone: approved ? 'review' : 'blocked'
		});
	}
	if (rrWidening || releaseWidening) {
		budgets.push({
			label: rrWidening ? 'replay budget' : 'release budget',
			value: approved ? 'metered' : 'blocked',
			detail: rrWidening ? 'cassette replay expands test cost' : 'pack and provenance checks expand release cost',
			tone: approved ? 'review' : 'blocked'
		});
	}

	const gates = [
		approved ? 'Session contract locked' : 'Approve spec to lock the session contract',
		writePolicy
			? `Write roots: ${writePolicy.paths.join(', ')}`
			: 'Write roots pending spec approval',
		releaseWidening ? 'Release/provenance gate required' : 'No release gate requested',
		rrWidening || sandboxWidening || osWidening
			? 'Capability widening requires review'
			: 'Deterministic world boundary preserved'
	];
	const reviewCount = [...worlds, ...capabilities, ...budgets].filter((item) => item.tone !== 'ok').length;
	return {
		posture: reviewCount ? `${reviewCount} gated surfaces` : 'solve-pure ready',
		review: reviewCount ? 'Review before agent execution' : 'No widening detected',
		worlds,
		capabilities,
		budgets,
		gates
	};
}

export function buildApprovalLedger(
	session: SessionSnapshot | null | undefined,
	revisionNotes: string[],
	approvalState: ApprovalLoopState
): ApprovalLedgerItem[] {
	const phase = session?.phase ?? 'intent_drafting';
	const hasIntent = Boolean(session?.intent);
	const hasFormalizeOp = Boolean(
		session?.op_log.some((op) => op.op === 'intent.formalize' && op.status === 'succeeded')
	);
	const specApproved = phaseIndex(phase) >= phaseIndex('spec_approved');
	const revisionItems = revisionNotes
		.map((note) => note.trim())
		.filter(Boolean)
		.map((note, index) => ({
			label: `Revision ${index + 1}`,
			detail: note,
			state: approvalState === 'changes' ? ('active' as const) : ('done' as const)
		}));
	return [
		{
			label: 'Intent source',
			detail: intentSourceLabel(session),
			state: hasIntent || hasFormalizeOp ? 'done' : 'active'
		},
		{
			label: 'Agent polish',
			detail: hasFormalizeOp || hasIntent ? 'intent packet ready for human review' : 'waiting for Polish Intent',
			state: hasFormalizeOp || hasIntent ? 'done' : 'blocked'
		},
		...revisionItems,
		{
			label: 'Human decision',
			detail:
				approvalState === 'changes'
					? 'approval blocked until the agent repolishes revisions'
					: specApproved
						? 'spec approved and realization unlocked'
						: 'approve or request changes before realization',
			state: specApproved ? 'done' : hasIntent && approvalState !== 'changes' ? 'active' : 'blocked'
		},
		{
			label: 'Write contract',
			detail: session?.contract
				? 'agent writes are constrained by the session contract'
				: 'contract locks after human approval',
			state: session?.contract ? 'done' : 'blocked'
		}
	];
}

export function agentReadiness(
	agent: AgentProfile,
	components: RuntimeComponentStatus[]
): AgentReadiness {
	const component = components.find((item) => item.id === componentIdForAgent(agent.id));
	let state: AgentStatus = 'needs_install';
	if (agent.status === 'disabled') {
		state = 'disabled';
	} else if (agent.status === 'available' || component?.status === 'available') {
		state = 'available';
	}
	let source = component?.source ?? 'not found on PATH';
	if (state === 'available') {
		source = component?.source ?? agent.command;
	} else if (state === 'disabled') {
		source = 'disabled by profile';
	}
	const installHint = component?.install_hint ?? `Install ${agent.label} or update the agent profile command.`;
	const detail =
		state === 'available'
			? `${agent.label} is available for supervised ${agent.approval_required ? 'approval-gated' : 'autonomous'} runs.`
			: state === 'disabled'
				? `${agent.label} is disabled for this workspace.`
				: installHint;
	return {
		state,
		source,
		detail,
		gate: agent.approval_required
			? 'Human checkpoint before execute'
			: 'Plan and execute allowed by profile',
		canRun: state === 'available'
	};
}

function componentIdForAgent(agentId: string): string {
	if (agentId === 'openai-codex') return 'codex';
	return agentId;
}

const ONBOARDING_BOOTSTRAP_COMMAND =
	'python3 scripts/bootstrap_components.py --install-missing --write-env .x07/studio/defaults.env';

export function buildOnboardingPlan(
	health: HealthResponse,
	components: RuntimeComponentStatus[] = health.components
): OnboardingStep[] {
	const missingRequired = components.some(
		(component) => component.required && component.status !== 'available'
	);
	const defaultsStep: OnboardingStep = {
		id: 'defaults',
		label: 'First-run defaults',
		state: missingRequired ? 'required' : 'ready',
		command: ONBOARDING_BOOTSTRAP_COMMAND,
		detail: [
			`workspace ${health.workspace_root}`,
			`daemon ${health.defaults.daemon_addr}`,
			`platform ${health.defaults.platform_state_dir}`
		].join(' / ')
	};
	const componentSteps = components.map((component): OnboardingStep => {
		const ready = component.status === 'available';
		const envVar = componentEnvVar(component.id);
		const state: OnboardingStepState = ready ? 'ready' : component.required ? 'required' : 'optional';
		return {
			id: `component.${component.id}`,
			label: component.required ? `${component.label} runtime` : `${component.label} agent`,
			state,
			command: ready
				? (component.source ?? component.command)
				: envVar
					? ONBOARDING_BOOTSTRAP_COMMAND
					: component.command,
			detail: ready
				? `${component.label} resolved for local runs.`
				: envVar
					? `${component.install_hint} Override with ${envVar}.`
					: component.install_hint
		};
	});
	const rank: Record<OnboardingStepState, number> = { required: 0, ready: 1, optional: 2 };
	return [defaultsStep, ...componentSteps].sort(
		(left, right) => rank[left.state] - rank[right.state]
	);
}

function componentEnvVar(componentId: string): string | null {
	if (componentId === 'x07') return 'X07_STUDIO_X07_EXE';
	if (componentId === 'x07-wasm') return 'X07_STUDIO_X07_WASM_EXE';
	if (componentId === 'x07lp') return 'X07_STUDIO_X07LP_EXE';
	return null;
}

export function buildAutomationPlan(
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate,
	approvalState: ApprovalLoopState
): AutomationPlanStep[] {
	const ops = session?.op_log ?? [];
	const approved = Boolean(session?.contract) || (session ? phaseIndex(session.phase) >= phaseIndex('spec_approved') : false);
	const intentReady = Boolean(session?.intent) || latestOpState(ops, ['intent.formalize']) === 'done';
	const taskType = session?.task_type ?? template.taskType;
	const setupSteps: AutomationPlanStep[] = [
		{
			label: 'Plan polish',
			command: 'intent.formalize',
			artifact: '.x07/studio/sessions/intent.json',
			gate: 'Human reviews the polished intent before spec approval',
			state: intentReady ? 'done' : session ? 'ready' : 'blocked'
		},
		{
			label: 'Human approval',
			command: 'approve_spec',
			artifact: '.x07/studio/sessions/session_contract.json',
			gate:
				approvalState === 'changes'
					? 'Blocked until the agent repolishes requested changes'
					: 'Locks write roots, docs, MCP tools, worlds, and budgets',
			state: approved ? 'done' : approvalState === 'changes' ? 'blocked' : intentReady ? 'ready' : 'blocked'
		},
		{
			label: 'Project scaffold',
			command: 'project.init.xtal-pure',
			artifact: 'x07.json',
			gate: 'Creates the x07 project only after approval',
			state: stateForOps(ops, ['project.init.xtal-pure'], approved)
		}
	];

	if (taskType === 'brownfield_extract') {
		setupSteps.push({
			label: 'Brownfield extraction',
			command: 'spec.extract',
			artifact: 'target/xtal/spec.extract.report.json',
			gate: 'Extract current behavior before implementation writes',
			state: stateForOps(ops, ['spec.extract'], approved)
		});
	} else if (taskType === 'incident_repair') {
		setupSteps.push(
			{
				label: 'Incident normalization',
				command: 'xtal.ingest --normalize-only',
				artifact: 'target/xtal/ingest/summary.json',
				gate: 'Converts incident notes into canonical XTAL evidence',
				state: stateForOps(ops, ['xtal.ingest'], approved)
			},
			{
				label: 'Improve from incident',
				command: 'xtal.improve',
				artifact: 'target/xtal/improve/summary.json',
				gate: 'Creates regression evidence before repair trust',
				state: stateForOps(ops, ['xtal.improve'], approved)
			}
		);
	} else {
		setupSteps.push(
			{
				label: 'Spec scaffold',
				command: 'spec.scaffold',
				artifact: template.artifacts[0] ?? 'spec/',
				gate: 'Creates spec artifacts before implementation',
				state: stateForOps(ops, ['spec.scaffold'], approved)
			},
			{
				label: 'Generated tests',
				command: 'tests.gen.write',
				artifact: template.artifacts[1] ?? 'gen/xtal/tests.json',
				gate: 'Examples become executable tests',
				state: stateForOps(ops, ['tests.gen.write'], approved)
			}
		);
	}

	setupSteps.push({
		label: 'Implementation sync',
		command: 'impl.sync.write',
		artifact: 'target/xtal/impl-sync.patchset.json',
		gate: 'Implementation writes stay inside approved roots',
		state: stateForOps(ops, ['impl.sync.write', 'wasm.app.build.atlas_dev'], approved)
	});

	const templateSteps = template.canonicalCommands.map((command, index) => ({
		label: `${template.label} evidence ${index + 1}`,
		command,
		artifact: template.artifacts[index] ?? template.artifacts.at(-1) ?? 'target/xtal/',
		gate: template.riskProfile,
		state: stateForCommand(ops, command, approved)
	}));

	return [...setupSteps, ...templateSteps].slice(0, 9);
}

export function buildEvidenceCoverage(
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate,
	approvalState: ApprovalLoopState
): EvidenceCoverageItem[] {
	const ops = session?.op_log ?? [];
	const approved = Boolean(session?.contract) || (session ? phaseIndex(session.phase) >= phaseIndex('spec_approved') : false);
	const intentOp = latestMatchingOp(ops, ['intent.formalize']);
	const scaffoldOp = latestMatchingOp(ops, ['project.init.xtal-pure', 'project.seed.']);
	const specOp =
		latestMatchingOp(ops, ['spec.extract', 'spec.scaffold', 'spec.check', 'tests.gen.write', 'project.seed.']) ??
		(session?.intent?.source.kind === 'spec'
			? latestMatchingOp(ops, ['intent.formalize'])
			: null);
	const implOp = latestMatchingOp(ops, ['impl.sync.write', 'impl.check', 'wasm.app.build.atlas_dev']);
	const verifyOp = latestMatchingOp(ops, [
		'xtal.verify',
		'test.manifest',
		'wasm.app.verify.atlas_release',
		'wasm.app.test.'
	]);
	const agentOp = latestMatchingOp(ops, ['agent.handoff.', 'agent.run.', 'agent.event.', 'agent.approval.']);
	const trustOp = latestMatchingOp(ops, [
		'xtal.certify',
		'wasm.provenance.verify',
		'wasm.deploy.plan',
		'lp.deploy.status.local',
		'lp.deploy.query.local'
	]);
	const visibleOp = ops.at(-1) ?? null;
	const specArtifact = template.artifacts[0] ?? 'spec/';
	const verifyArtifact =
		template.artifacts.find((artifact) => artifact.includes('verify') || artifact.includes('pack')) ??
		'target/xtal/verify/summary.json';
	const releaseArtifact =
		template.artifacts.find(
			(artifact) => artifact.includes('pack') || artifact.includes('deploy') || artifact.includes('provenance')
		) ?? 'target/xtal/cert/';

	return [
		coverageItem({
			id: 'intent',
			label: 'Initial plan or spec',
			requirement: 'Human input is preserved as an intent packet before code generation.',
			evidence: session?.intent
				? intentSourceEvidence(session)
				: intentOp
					? 'intent.formalize operation recorded'
					: 'waiting for written plan, voice transcript, existing spec, or incident note',
			artifact: '.x07/studio/sessions/<session>.json',
			state: session?.intent || intentOp ? 'done' : session ? 'active' : 'blocked',
			op: intentOp
		}),
		coverageItem({
			id: 'approval',
			label: 'Human approval loop',
			requirement: 'Agent-polished intent cannot realize implementation until humans approve or repolish changes.',
			evidence: approved
				? 'session contract locked'
				: approvalState === 'changes'
					? 'revision requested; approval blocked until repolish'
					: session?.intent
						? 'awaiting human approve or request-changes decision'
						: 'approval waits for intent polish',
			artifact: '.x07/studio/sessions/session_contract.json',
			state: approved ? 'done' : approvalState === 'changes' ? 'blocked' : session?.intent ? 'active' : 'blocked',
			op: intentOp
		}),
		coverageItem({
			id: 'project',
			label: 'Project scaffold',
			requirement: 'Studio creates or seeds the x07 project only after the approval gate.',
			evidence: scaffoldOp?.op ?? (approved ? 'ready to initialize x07 project' : 'blocked before approval'),
			artifact: 'x07.json',
			state: stateFromOpOrGate(scaffoldOp, approved),
			op: scaffoldOp
		}),
		coverageItem({
			id: 'spec-tests',
			label: 'Spec and generated tests',
			requirement: 'Behavior is represented as x07 specs, examples, and generated tests.',
			evidence: specOp?.op ?? (approved ? 'ready to scaffold/check specs and tests' : 'blocked before approval'),
			artifact: specArtifact,
			state: stateFromOpOrGate(specOp, approved),
			op: specOp
		}),
		coverageItem({
			id: 'implementation',
			label: 'Implementation realization',
			requirement: 'Implementation changes are synced through canonical bindings and visible patch evidence.',
			evidence: implOp?.op ?? (approved ? 'ready for guarded implementation sync' : 'blocked before approval'),
			artifact: 'target/xtal/impl-sync.patchset.json',
			state: stateFromOpOrGate(implOp, approved),
			op: implOp
		}),
		coverageItem({
			id: 'verify',
			label: 'Verification evidence',
			requirement: 'Checks, generated tests, proofs, app traces, or SLO gates run before trust.',
			evidence: verifyOp?.op ?? (approved ? 'ready for verify/test/app evidence' : 'blocked before approval'),
			artifact: verifyArtifact,
			state: stateFromOpOrGate(verifyOp, approved),
			op: verifyOp
		}),
		coverageItem({
			id: 'agent-visible',
			label: 'Visible agent work',
			requirement: 'Codex, Claude Code, and x07 command activity stays inspectable in the worklog.',
			evidence: agentOp?.op ?? visibleOp?.op ?? (session ? 'worklog ready for operations' : 'no session selected'),
			artifact: agentOp?.artifacts[0] ?? visibleOp?.artifacts[0] ?? '.x07/studio/handoffs/',
			state: agentOp || visibleOp ? 'done' : session ? 'active' : 'blocked',
			op: agentOp ?? visibleOp
		}),
		coverageItem({
			id: 'trust-platform',
			label: 'Trust and platform evidence',
			requirement: 'Certification, provenance, deploy, or local platform delivery evidence is visible for release-shaped projects.',
			evidence:
				trustOp?.op ??
				(session?.phase === 'trust_review'
					? 'trust review is open'
					: approved
						? 'waiting for trust/certify/platform evidence'
						: 'blocked before approval'),
			artifact: releaseArtifact,
			state: trustOp
				? opStatusToCoverageState(trustOp.status)
				: session?.phase === 'trust_review'
					? 'active'
					: approved
						? 'active'
						: 'blocked',
			op: trustOp
		})
	];
}

function coverageItem(input: {
	id: string;
	label: string;
	requirement: string;
	evidence: string;
	artifact: string;
	state: EvidenceCoverageState;
	op?: OpRecord | null;
}): EvidenceCoverageItem {
	return {
		id: input.id,
		label: input.label,
		requirement: input.requirement,
		evidence: input.evidence,
		artifact: input.artifact,
		state: input.state,
		opId: input.op?.id ?? null
	};
}

export function buildPlatformBridge(
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate
): PlatformBridge {
	const ops = session?.op_log ?? [];
	const approved = Boolean(session?.contract) || (session ? phaseIndex(session.phase) >= phaseIndex('spec_approved') : false);
	const releaseExpected = platformExpected(template, ops);
	const appOp = latestMatchingOp(ops, [
		'wasm.app.pack',
		'wasm.app.verify',
		'wasm.app.build.atlas_release',
		'wasm.app.build.atlas_dev'
	]);
	const provenanceOp = latestMatchingOp(ops, ['wasm.provenance.verify', 'wasm.provenance.attest']);
	const deployPlanOp = latestMatchingOp(ops, ['wasm.deploy.plan', 'lp.deploy.accept.local']);
	const platformOp = latestMatchingOp(ops, [
		'lp.deploy.status.local',
		'lp.deploy.query.local',
		'lp.deploy.run.local',
		'lp.deploy.accept.local'
	]);
	const sloOp = latestMatchingOp(ops, ['wasm.slo.eval', 'wasm.slo.validate']);
	const feedbackOp = latestMatchingOp(ops, [
		'xtal.improve',
		'xtal.ingest',
		'wasm.app.test.regress',
		'lp.incident'
	]);
	const items = [
		platformBridgeItem({
			id: 'app-pack',
			label: 'App package',
			command: 'x07-wasm app build / pack / verify',
			requirement: 'Build and verify the user-facing app artifact before platform delivery.',
			evidence: appOp?.op ?? (releaseExpected ? 'waiting for x07-wasm app package evidence' : 'not required for solve-pure local work'),
			artifact: appOp?.artifacts[0] ?? 'dist/showcase_fullstack/pack.atlas_release/app.pack.json',
			state: platformState(appOp, approved, releaseExpected),
			op: appOp
		}),
		platformBridgeItem({
			id: 'provenance',
			label: 'Provenance',
			command: 'x07-wasm provenance attest / verify',
			requirement: 'Keep release provenance separate from app build evidence.',
			evidence: provenanceOp?.op ?? (releaseExpected ? 'waiting for provenance evidence' : 'optional before release'),
			artifact: provenanceOp?.artifacts[0] ?? 'dist/showcase_fullstack/pack.atlas_release/app.provenance.dsse.json',
			state: platformState(provenanceOp, approved, releaseExpected),
			op: provenanceOp
		}),
		platformBridgeItem({
			id: 'deploy-plan',
			label: 'Deploy plan',
			command: 'x07-wasm deploy plan / x07lp accept',
			requirement: 'Materialize the release plan before running local platform delivery.',
			evidence: deployPlanOp?.op ?? (releaseExpected ? 'waiting for deploy plan evidence' : 'optional before release'),
			artifact: deployPlanOp?.artifacts[0] ?? 'dist/showcase_fullstack/deploy.atlas_release',
			state: platformState(deployPlanOp, approved, releaseExpected),
			op: deployPlanOp
		}),
		platformBridgeItem({
			id: 'platform-delivery',
			label: 'Platform delivery',
			command: 'x07lp deploy run / query / status',
			requirement: 'Use the x07 platform lane for visible local delivery state.',
			evidence: platformOp?.op ?? (releaseExpected ? 'waiting for local platform status evidence' : 'optional before release'),
			artifact: platformOp?.artifacts[0] ?? '.x07/platform',
			state: platformState(platformOp, approved, releaseExpected),
			op: platformOp
		}),
		platformBridgeItem({
			id: 'slo-budget',
			label: 'SLO and budget',
			command: 'x07-wasm slo eval',
			requirement: 'Preserve budget and SLO evidence before trust review.',
			evidence: sloOp?.op ?? (releaseExpected ? 'waiting for SLO budget evidence' : 'optional before release'),
			artifact: sloOp?.artifacts[0] ?? 'tests/fixtures/metrics/atlas_canary_ok.json',
			state: platformState(sloOp, approved, releaseExpected),
			op: sloOp
		}),
		platformBridgeItem({
			id: 'feedback',
			label: 'Runtime feedback',
			command: 'xtal.ingest / xtal.improve / app regress',
			requirement: 'Feed incidents or regression traces back into the XTAL repair loop.',
			evidence: feedbackOp?.op ?? (session?.task_type === 'incident_repair' ? 'waiting for incident improve evidence' : 'optional unless an incident is linked'),
			artifact: feedbackOp?.artifacts[0] ?? 'target/xtal/improve/summary.json',
			state: feedbackOp
				? platformState(feedbackOp, approved, true)
				: session?.task_type === 'incident_repair'
					? platformState(null, approved, true)
					: 'optional',
			op: feedbackOp
		})
	];
	const requiredItems = items.filter((item) => item.state !== 'optional');
	const failed = items.find((item) => item.state === 'failed');
	const doneRequired = requiredItems.filter((item) => item.state === 'done').length;
	const posture = failed
		? 'Platform blocked'
		: requiredItems.length && doneRequired === requiredItems.length
			? 'Platform delivery covered'
			: releaseExpected
				? approved
					? 'Platform delivery in progress'
					: 'Platform gated by approval'
				: 'Platform optional';
	const nextItem =
		items.find((item) => item.state === 'failed') ??
		items.find((item) => item.state === 'active' || item.state === 'blocked') ??
		items.find((item) => item.state === 'optional') ??
		items[0];

	return {
		posture,
		summary: requiredItems.length
			? `${doneRequired} / ${requiredItems.length} required platform gates covered`
			: 'No platform delivery gate is required for this solve-pure session yet',
		nextAction:
			requiredItems.length && doneRequired === requiredItems.length && !failed
				? 'Platform evidence is complete; review trust and certification gates'
				: nextItem
					? `${nextItem.label}: ${nextItem.evidence}`
					: 'No platform action pending',
		items
	};
}

function platformExpected(template: ProjectTemplate, ops: OpRecord[]): boolean {
	const haystack = [
		template.id,
		template.riskProfile,
		template.sourcePath,
		...template.canonicalCommands,
		...template.artifacts,
		...ops.flatMap((op) => [op.op, ...op.command, ...op.artifacts])
	]
		.join(' ')
		.toLowerCase();
	return ['x07-wasm', 'wasm', 'platform', 'deploy', 'release', 'provenance', 'slo'].some((needle) =>
		haystack.includes(needle)
	);
}

function platformBridgeItem(input: {
	id: string;
	label: string;
	command: string;
	requirement: string;
	evidence: string;
	artifact: string;
	state: PlatformBridgeState;
	op?: OpRecord | null;
}): PlatformBridgeItem {
	return {
		id: input.id,
		label: input.label,
		command: input.command,
		requirement: input.requirement,
		evidence: input.evidence,
		artifact: input.artifact,
		state: input.state,
		opId: input.op?.id ?? null
	};
}

function platformState(
	op: OpRecord | null,
	approved: boolean,
	required: boolean
): PlatformBridgeState {
	if (op) return opStatusToCoverageState(op.status);
	if (!required) return 'optional';
	return approved ? 'active' : 'blocked';
}

function latestMatchingOp(ops: OpRecord[], needles: string[]): OpRecord | null {
	return (
		[...ops]
			.reverse()
			.find((op) => needles.some((needle) => op.op.includes(needle) || op.command.join(' ').includes(needle))) ?? null
	);
}

function stateFromOpOrGate(op: OpRecord | null, gateOpen: boolean): EvidenceCoverageState {
	if (op) return opStatusToCoverageState(op.status);
	return gateOpen ? 'active' : 'blocked';
}

function opStatusToCoverageState(status: OperationStatus): EvidenceCoverageState {
	if (status === 'succeeded') return 'done';
	if (status === 'failed') return 'failed';
	if (status === 'running') return 'active';
	return 'active';
}

function intentSourceEvidence(session: SessionSnapshot): string {
	const source = session.intent?.source;
	if (!source) return 'intent not formalized';
	if (source.kind === 'voice') return 'voice transcript formalized';
	if (source.kind === 'spec') return 'existing x07 spec formalized';
	if (source.kind === 'incident') return 'incident bundle formalized';
	return 'written plan formalized';
}

function stateForOps(
	ops: OpRecord[],
	bindingIds: string[],
	approved: boolean
): AutomationPlanState {
	const state = latestOpState(ops, bindingIds);
	if (state) return state;
	return approved ? 'ready' : 'blocked';
}

function latestOpState(ops: OpRecord[], bindingIds: string[]): AutomationPlanState | null {
	const matched = [...ops].reverse().find((op) => bindingIds.includes(op.op));
	if (!matched) return null;
	return opStatusToPlanState(matched.status);
}

function stateForCommand(
	ops: OpRecord[],
	command: string,
	approved: boolean
): AutomationPlanState {
	const normalizedCommand = normalizeCommand(command);
	const matched = [...ops]
		.reverse()
		.find((op) => normalizedCommand.includes(normalizeCommand(op.op)) || normalizeCommand(op.command.join(' ')).includes(normalizedCommand.split(' ').slice(0, 5).join(' ')));
	if (matched) return opStatusToPlanState(matched.status);
	return approved ? 'ready' : 'blocked';
}

function opStatusToPlanState(status: OperationStatus): AutomationPlanState {
	if (status === 'succeeded') return 'done';
	if (status === 'failed') return 'failed';
	if (status === 'running') return 'running';
	return 'ready';
}

function normalizeCommand(value: string): string {
	return value
		.toLowerCase()
		.replace(/x07-wasm/g, 'x07 wasm')
		.replace(/[^a-z0-9]+/g, ' ')
		.trim();
}

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
	const isAtlas =
		lowered.includes('x07_atlas') ||
		lowered.includes('x07 atlas') ||
		lowered.includes('wasm_showcases/x07_atlas');
	const isWorkflowGraph = lowered.includes('workflow graph') || lowered.includes('makespan') || lowered.includes('dag');
	const specTarget = inputMode === 'spec' ? specTargetFromRaw(normalized) : null;
	const moduleId = specTarget?.moduleId ?? (isSorter
		? 'toy.sorter'
		: isAtlas
			? 'atlas.app'
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
									: 'workflow.graph');
	const entry = specTarget?.entry ?? (isSorter
		? 'sort_u8_asc'
		: isAtlas
			? 'atlas_dev'
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
								: 'makespan_u32');
	const incidentWitness =
		inputMode === 'incident'
			? [{ kind: 'incident_report' as const, text: normalized }]
			: [];
	const specWitness =
		inputMode === 'spec'
			? [
					{
						kind: 'policy_requirement' as const,
						text: 'Use the provided x07 spec as the canonical behavioral source.'
					}
				]
			: [];
	const extraPolicyImplications =
		isAtlas
			? [
					'Full-stack WASM app, provenance signing material, deploy planning, and SLO gates require explicit trust review.'
				]
			: isGateway || isCrawler || isDbGuard
			? ['RR fixtures, sandbox policy, and OS/network/db capability widening require explicit review.']
			: isStateMachine
				? ['Generated outputs, arch contracts, and budget profiles require drift evidence before certify.']
				: [];
	const draftWitnesses = previewIntentWitnesses(normalized, inputMode);
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
			...(inputMode === 'spec'
				? ['Treat the provided spec as already-authored behavioral intent.']
				: []),
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
		witnesses: uniqueWitnesses([
			...draftWitnesses,
			{ kind: 'policy_requirement', text: 'All agent work must flow through canonical x07/XTAL bindings.' },
			{ kind: 'forbidden_behavior', text: 'Do not turn the prompt directly into unchecked source code.' },
			...specWitness,
			...incidentWitness
		]),
		source:
			inputMode === 'voice'
				? { kind: 'voice', transcript: normalized }
				: inputMode === 'spec'
					? { kind: 'spec', raw: normalized }
				: inputMode === 'incident'
					? { kind: 'incident', path: `.x07/studio/incidents/${session.session_id}` }
					: { kind: 'text', raw: normalized }
	};
}

export function previewIntentWitnesses(raw: string, inputMode: IntentInputMode = 'text'): IntentWitness[] {
	const normalized = raw.replace(/\s+/g, ' ').trim();
	if (!normalized) return [];
	const lowered = normalized.toLowerCase();
	const witnesses: IntentWitness[] = [];
	if (inputMode === 'incident') {
		witnesses.push({ kind: 'incident_report', text: normalized });
	} else {
		witnesses.push({ kind: 'desired_behavior', text: normalized });
	}
	if (inputMode === 'spec') {
		witnesses.push({
			kind: 'policy_requirement',
			text: 'Use the provided x07 spec as the canonical behavioral source.'
		});
	}
	if (hasForbiddenCue(lowered)) {
		witnesses.push({ kind: 'forbidden_behavior', text: forbiddenWitnessText(normalized) });
	}
	if (hasPolicyCue(lowered)) {
		witnesses.push({ kind: 'policy_requirement', text: policyWitnessText(normalized) });
	}
	return uniqueWitnesses(witnesses);
}

function hasForbiddenCue(lowered: string) {
	return /\b(rejects?|rejected|rejecting|forbids?|forbidden|never|must not|do not|don't|without|no\s+unchecked)\b/.test(
		lowered
	);
}

function hasPolicyCue(lowered: string) {
	return /\b(network|sandbox|policy|capability|capabilities|budget|world|os world|trust|approval|provenance|slo)\b/.test(
		lowered
	);
}

function forbiddenWitnessText(text: string) {
	return (
		sentenceWithCue(text, /(rejects?|rejected|rejecting|forbids?|forbidden|never|must not|do not|don't|without|no unchecked)/i) ??
		text
	);
}

function policyWitnessText(text: string) {
	return (
		sentenceWithCue(
			text,
			/(network|sandbox|policy|capability|capabilities|budget|world|os world|trust|approval|provenance|slo)/i
		) ?? text
	);
}

function sentenceWithCue(text: string, cue: RegExp) {
	return (
		text
			.match(/[^.!?]+[.!?]?/g)
			?.map((sentence) => sentence.trim())
			.find((sentence) => cue.test(sentence)) ?? null
	);
}

function uniqueWitnesses(witnesses: IntentWitness[]) {
	const seen = new Set<string>();
	return witnesses.filter((witness) => {
		const key = `${witness.kind}:${witness.text}`;
		if (seen.has(key)) return false;
		seen.add(key);
		return true;
	});
}

function specTargetFromRaw(raw: string): { moduleId: string; entry: string } | null {
	try {
		const value = JSON.parse(raw) as {
			module_id?: unknown;
			operations?: Array<{ name?: unknown; id?: unknown }>;
		};
		const moduleId = typeof value.module_id === 'string' ? value.module_id.trim() : '';
		const operation = value.operations?.[0];
		const operationName =
			typeof operation?.name === 'string'
				? operation.name
				: typeof operation?.id === 'string'
					? operation.id
					: '';
		if (!moduleId || !operationName) return null;
		return {
			moduleId,
			entry: entryFromSpecOperation(moduleId, operationName)
		};
	} catch {
		return null;
	}
}

function entryFromSpecOperation(moduleId: string, operationName: string): string {
	let entry = operationName.trim();
	if (entry.startsWith('op.')) entry = entry.slice(3);
	if (entry.startsWith(`${moduleId}.`)) entry = entry.slice(moduleId.length + 1);
	if (entry.endsWith('.v1')) entry = entry.slice(0, -3);
	return entry.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '') || 'run_v1';
}

function intentSourceLabel(session: SessionSnapshot | null | undefined): string {
	const source = session?.intent?.source;
	if (!source) return 'waiting for written, spoken, spec, or incident input';
	if (source.kind === 'voice') return 'voice transcript';
	if (source.kind === 'spec') return 'existing x07 spec';
	if (source.kind === 'incident') return 'incident note';
	return 'written plan';
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

export function demoHealth(): HealthResponse {
	return {
		ok: true,
		workspace_root: demoSession().root,
		defaults: {
			daemon_addr: '127.0.0.1:7719',
			provider_profile_id: 'ollama-local',
			platform_state_dir: '.x07/platform'
		},
		components: [
			{
				id: 'x07',
				label: 'x07 CLI',
				command: 'x07',
				required: true,
				status: 'available',
				source: 'demo projection',
				install_hint: 'Install the x07 toolchain or set X07_STUDIO_X07_EXE.'
			},
			{
				id: 'x07-wasm',
				label: 'x07-wasm',
				command: 'x07-wasm',
				required: true,
				status: 'available',
				source: 'demo projection',
				install_hint: 'Install x07-wasm or set X07_STUDIO_X07_WASM_EXE.'
			},
			{
				id: 'x07lp',
				label: 'x07 platform',
				command: 'x07lp',
				required: true,
				status: 'available',
				source: 'demo projection',
				install_hint: 'Install x07lp or set X07_STUDIO_X07LP_EXE.'
			},
			{
				id: 'codex',
				label: 'OpenAI Codex',
				command: 'codex',
				required: false,
				status: 'available',
				source: 'demo projection',
				install_hint: 'Install Codex CLI when supervised Codex handoffs should execute locally.'
			},
			{
				id: 'claude-code',
				label: 'Claude Code',
				command: 'claude',
				required: false,
				status: 'available',
				source: 'demo projection',
				install_hint: 'Install Claude Code when supervised Claude handoffs should execute locally.'
			}
		]
	};
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
		{ id: 'pkg.lock.atlas.frontend', category: 'x07/package', program: 'x07', notes: 'Resolve the x07 Atlas frontend lockfile.' },
		{ id: 'wasm.app.profile.validate.atlas_dev', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Validate the x07 Atlas app profile.' },
		{ id: 'wasm.app.contracts.validate', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Validate app contracts.' },
		{ id: 'wasm.web_ui.contracts.validate', category: 'x07/wasm/web-ui', program: 'x07-wasm', notes: 'Validate web-ui contracts.' },
		{ id: 'wasm.http.contracts.validate', category: 'x07/wasm/http', program: 'x07-wasm', notes: 'Validate HTTP reducer contracts.' },
		{ id: 'wasm.caps.validate.atlas_release', category: 'x07/wasm/caps', program: 'x07-wasm', notes: 'Validate x07 Atlas release capabilities.' },
		{ id: 'wasm.ops.validate', category: 'x07/wasm/ops', program: 'x07-wasm', notes: 'Validate app ops profiles.' },
		{ id: 'wasm.slo.validate.atlas', category: 'x07/wasm/slo', program: 'x07-wasm', notes: 'Validate x07 Atlas SLO profile.' },
		{ id: 'wasm.app.build.atlas_dev', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Build the x07 Atlas development app.' },
		{ id: 'wasm.app.serve.smoke.atlas_dev', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Smoke-serve the x07 Atlas development app.' },
		{ id: 'wasm.app.test.happy_path', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Replay the x07 Atlas happy-path trace.' },
		{ id: 'wasm.app.test.validation_error', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Replay the x07 Atlas validation trace.' },
		{ id: 'wasm.app.test.regress.atlas_incident', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Replay the x07 Atlas incident regression.' },
		{ id: 'wasm.app.build.atlas_release', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Build the x07 Atlas release app.' },
		{ id: 'wasm.app.pack.atlas_release', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Pack the x07 Atlas release app.' },
		{ id: 'wasm.app.verify.atlas_release', category: 'x07/wasm/app', program: 'x07-wasm', notes: 'Verify the x07 Atlas release app pack.' },
		{ id: 'wasm.provenance.attest.atlas_release', category: 'x07/wasm/provenance', program: 'x07-wasm', notes: 'Attest the x07 Atlas release pack.' },
		{ id: 'wasm.provenance.verify.atlas_release', category: 'x07/wasm/provenance', program: 'x07-wasm', notes: 'Verify the x07 Atlas release pack provenance.' },
		{ id: 'wasm.deploy.plan.atlas_release', category: 'x07/wasm/deploy', program: 'x07-wasm', notes: 'Generate the x07 Atlas release deploy plan.' },
		{ id: 'wasm.slo.eval.atlas_canary_ok', category: 'x07/wasm/slo', program: 'x07-wasm', notes: 'Evaluate x07 Atlas canary SLO metrics.' },
		{ id: 'lp.release.query', category: 'x07/platform', program: 'x07lp', notes: 'Query hosted release state.' },
		{ id: 'lp.release.rollback', category: 'x07/platform', program: 'x07lp', notes: 'Rollback a hosted release.' },
		{ id: 'lp.deploy.accept.local', category: 'x07/platform', program: 'x07lp', notes: 'Accept a local deployment candidate from a verified pack manifest.' },
		{ id: 'lp.deploy.run.local', category: 'x07/platform', program: 'x07lp', notes: 'Run an accepted deployment locally from an x07 deploy plan.' },
		{ id: 'lp.deploy.run.local.metrics', category: 'x07/platform', program: 'x07lp', notes: 'Run an accepted local deployment with explicit metrics evidence.' },
		{ id: 'lp.deploy.query.local', category: 'x07/platform', program: 'x07lp', notes: 'Query full local deployment state.' },
		{ id: 'lp.deploy.status.local', category: 'x07/platform', program: 'x07lp', notes: 'Inspect local deployment status.' },
		{ id: 'lp.incident.list.local', category: 'x07/platform', program: 'x07lp', notes: 'List local deployment incidents.' },
		{ id: 'lp.regress.from_incident.local', category: 'x07/platform', program: 'x07lp', notes: 'Create a local regression fixture from a platform incident.' },
		{ id: 'lp.ui.serve.local', category: 'x07/platform', program: 'x07lp', notes: 'Serve the local platform control-plane UI.' },
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
		case 'ingest_incident':
			next.phase = 'incident_ingesting';
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
		case 'xtal.ingest':
			return ['target/xtal/ingest/summary.json'];
		case 'xtal.improve':
			return ['target/xtal/improve/summary.json', 'target/xtal/improve/tests.shadow.json'];
		case 'xtal.manifest.ensure':
			return ['arch/xtal/xtal.json'];
		case 'wasm.app.verify.atlas_release':
			return ['dist/showcase_fullstack/pack.atlas_release/app.pack.json'];
		case 'wasm.provenance.verify.atlas_release':
			return ['dist/showcase_fullstack/pack.atlas_release/app.provenance.dsse.json'];
		case 'wasm.deploy.plan.atlas_release':
			return ['dist/showcase_fullstack/deploy.atlas_release'];
		case 'wasm.slo.eval.atlas_canary_ok':
			return ['tests/fixtures/metrics/atlas_canary_ok.json'];
		case 'lp.deploy.accept.local':
		case 'lp.deploy.run.local.metrics':
		case 'lp.deploy.query.local':
		case 'lp.deploy.status.local':
			return ['.x07/platform'];
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
