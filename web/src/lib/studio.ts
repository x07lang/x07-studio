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

export interface ClarificationTurn {
	question_id: string;
	question_text: string;
	witness_kind: IntentWitnessKind;
	round: number;
	agent_id: string;
	options: string[];
	question_recorded_at: string;
	answer_text?: string | null;
	answer_recorded_at?: string | null;
}

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
		| { kind: 'incident'; path: string }
		| { kind: 'sketch'; path: string }
		| { kind: 'image'; path: string; mime: string };
	clarification_history?: ClarificationTurn[];
}

export type SessionStreamEvent =
	| { kind: 'op'; op: OpRecord }
	| { kind: 'snapshot'; session: SessionSnapshot }
	| { kind: 'heartbeat'; unix_ms: number };

export interface IntentClarifyRequest {
	agent_id: string;
	round_max?: number;
	timeout_seconds?: number;
}

export interface IntentClarifyResponse {
	handoff: AgentHandoff;
	op: OpRecord;
	session: SessionSnapshot;
}

export interface IntentAnswer {
	question_id: string;
	text: string;
	witness_kind?: IntentWitnessKind;
}

export interface IntentAnswerRequest {
	answers: IntentAnswer[];
}

export interface IntentAnswerResponse {
	intent: IntentPacket;
	op: OpRecord;
	session: SessionSnapshot;
}

export interface RunBuildRequest {
	vars?: Record<string, string>;
	max_repair_rounds?: number;
}

export interface PlainEnglishSummary {
	schema_version: 'x07.studio.plain_english_summary@0.1.0';
	headline: string;
	behavior_promises: string[];
	behavior_promise_ids?: string[];
	boundaries: string[];
	evidence: string[];
	run_invocation?: string | null;
	followups: string[];
	scaffold_only?: boolean;
	stub_paths?: string[];
}

export type AgentStreamEvent =
	| { kind: 'reasoning'; id: string; at: string; text: string }
	| {
			kind: 'tool_use';
			id: string;
			at: string;
			agent_id: string;
			tool: string;
			input: unknown;
	  }
	| {
			kind: 'tool_result';
			id: string;
			at: string;
			agent_id: string;
			tool: string;
			success: boolean;
			snippet?: string | null;
	  }
	| { kind: 'agent_message'; id: string; at: string; agent_id: string; text: string }
	| { kind: 'done'; id: string; at: string; agent_id: string; exit_code: number }
	| {
			kind: 'mcp_call';
			id: string;
			at: string;
			agent_id: string;
			tool: string;
			server: string;
			input: unknown;
			output?: unknown;
	  };

export interface LiveDiff {
	schema_version: 'x07.studio.live_diff@0.1.0';
	path: string;
	before?: string | null;
	after?: string | null;
	unified_diff: string;
}

export interface RealizeProposal {
	schema_version: 'x07.studio.realize_proposal@0.1.0';
	agent_id: string;
	path: string;
	body: unknown;
	digest: string;
	stdout_excerpt: string;
	stderr_excerpt: string;
	status: 'ok' | 'no_write' | 'audit_fail' | 'spawn_fail' | string;
}

export interface RealizeQuorumRound {
	schema_version: 'x07.studio.realize_quorum_round@0.1.0';
	session_id: string;
	started_at: string;
	finished_at?: string | null;
	proposals: RealizeProposal[];
	agreed: boolean;
	judge?: string | null;
}

export interface PickRealizeProposalResponse {
	round: RealizeQuorumRound;
	session: SessionSnapshot;
}

export type SessionTurn =
	| { kind: 'user_intent'; id: string; at: string; raw: string; source_kind: string }
	| { kind: 'agent_clarify'; id: string; at: string; agent_id: string; questions: TurnQuestion[] }
	| { kind: 'user_answer'; id: string; at: string; question_id: string; text: string }
	| { kind: 'agent_draft'; id: string; at: string; agent_id: string; summary: string; evidence: TurnEvidence[] }
	| { kind: 'user_approved'; id: string; at: string; by: string }
	| { kind: 'build_stage'; id: string; at: string; stage: string; op_ids: string[] }
	| { kind: 'verified'; id: string; at: string; summary: PlainEnglishSummary; op_ids: string[] }
	| { kind: 'incident'; id: string; at: string; incident_id: string; summary: string; repair_available: boolean }
	| { kind: 'repair'; id: string; at: string; incident_id: string; op_ids: string[] }
	| {
			kind: 'agent_realize';
			id: string;
			at: string;
			agent_id: string;
			ok: boolean;
			wrote_files: string[];
			op_ids: string[];
	  }
	| {
			kind: 'agent_stream';
			id: string;
			at: string;
			agent_id: string;
			event: AgentStreamEvent;
			op_id: string;
	  }
	| {
			kind: 'quorum_realize';
			id: string;
			at: string;
			round: RealizeQuorumRound;
			op_ids: string[];
	  }
	| {
			kind: 'lint';
			id: string;
			at: string;
			count_by_severity: Record<string, number>;
			diagnostic_ids: string[];
	  }
	| { kind: 'trust_posture_changed'; id: string; at: string; posture: TrustPosture }
	| { kind: 'mcp_call'; id: string; at: string; call: AgentStreamEvent; op_id: string };

export interface RealizeRequest {
	agent_id?: string;
	timeout_seconds?: number;
}

export interface RealizeResponse {
	agent_id: string;
	ok: boolean;
	wrote_files: string[];
	session: SessionSnapshot;
}

export interface TurnQuestion {
	id: string;
	text: string;
	witness_kind: IntentWitnessKind;
	options: string[];
	answer?: string | null;
}

export interface TurnEvidence {
	label: string;
	op_id?: string | null;
	artifact?: string | null;
}

export interface TryItRequest {
	input_kind: 'text' | 'file' | 'b64' | 'argv';
	input_text?: string | null;
	input_b64?: string | null;
	input_path?: string | null;
	argv: string[];
	profile?: string | null;
}

export interface ProofCitation {
	clause_id: string;
	proof_report?: string | null;
	summary: string;
}

export interface TryItResult {
	output_kind: 'json' | 'text' | 'binary' | string;
	output_text?: string | null;
	output_json?: unknown;
	stats: unknown;
	proof_citations: ProofCitation[];
	op_id: string;
}

export interface LadderRung {
	id: 'local_preview' | 'shareable' | 'team' | 'production' | string;
	label: string;
	profile_path?: string | null;
	satisfied: boolean;
	missing: string[];
	evidence: string[];
	gates?: RungGate[];
}

export interface LadderState {
	current_rung: string;
	rungs: LadderRung[];
}

export interface RungGate {
	id: string;
	label: string;
	description: string;
	currently_satisfied: boolean;
}

export interface Capability {
	id: string;
	source: string;
	justification: string;
}

export interface BudgetSummary {
	local_cap_ms?: number | null;
	arch_profile?: string | null;
	prover_seconds_used: number;
	prover_seconds_cap?: number | null;
}

export interface ProofCoverage {
	support_pct: number;
	proved_pct: number;
	proof_count: number;
	assumptions_open: number;
}

export interface PostureDelta {
	at: string;
	kind: string;
	summary: string;
}

export interface TrustPosture {
	schema_version: 'x07.studio.trust_posture@0.1.0';
	session_id: string;
	captured_at: string;
	trust_profile: string;
	worlds: string[];
	capabilities: Capability[];
	budgets: BudgetSummary;
	proof_coverage: ProofCoverage;
	deltas: PostureDelta[];
	posture_color: 'green' | 'amber' | 'red' | string;
}

export type DiffRef =
	| { kind: 'op_id'; op_id: string }
	| { kind: 'turn_id'; turn_id: string }
	| { kind: 'hash'; hash: string }
	| { kind: 'current' }
	| { kind: 'quorum_proposal'; round: string; agent_id: string };

export interface SemanticDiffRequest {
	schema_version?: 'x07.studio.semantic_diff_request@0.1.0';
	from: DiffRef;
	to: DiffRef;
	mode?: 'project' | 'ast-only' | string;
}

export interface SemanticDiff {
	schema_version: 'x07.studio.semantic_diff@0.1.0';
	from: DiffRef;
	to: DiffRef;
	headline: string;
	trust_delta_color: 'green' | 'amber' | 'red' | string;
	raw: unknown;
	world_changes: string[];
	capability_changes: string[];
	budget_changes: string[];
	proof_changes: string[];
}

export interface ProofEvidenceCitation {
	kind: string;
	file: string;
	region?: string | null;
}

export interface ProofObligation {
	id: string;
	goal: string;
	status: string;
	note?: string | null;
}

export interface ProofEvidence {
	schema_version: 'x07.studio.proof_evidence@0.1.0';
	session_id: string;
	behavior_id: string;
	status: 'proved' | 'test-evidence' | 'assumed' | string;
	citations: ProofEvidenceCitation[];
	obligations: ProofObligation[];
	z3_ms?: number | null;
	assumptions: string[];
}

export interface QuickfixRecord {
	schema_version: 'x07.studio.quickfix_record@0.1.0';
	diagnostic_code: string;
	severity: string;
	summary: string;
	patch_ast: unknown;
	citations: ProofEvidenceCitation[];
	before_snippet?: string | null;
	after_snippet?: string | null;
}

export interface ContractSection {
	title: string;
	body: string;
}

export interface AgentContract {
	schema_version: 'x07.studio.agent_contract@0.1.0';
	session_id: string;
	path: string;
	exists: boolean;
	markdown: string;
	sections: ContractSection[];
	last_modified?: string | null;
	hash: string;
}

export interface LintDiagnostic {
	id: string;
	severity: string;
	file: string;
	line: number;
	column: number;
	summary: string;
	fixable: boolean;
}

export interface LintReport {
	schema_version: 'x07.studio.lint_report@0.1.0';
	session_id: string;
	generated_at: string;
	diagnostics: LintDiagnostic[];
	raw: unknown;
}

export interface DoctorStatus {
	ok: boolean;
	blockers: string[];
	warnings: string[];
}

export interface LockfileStatus {
	ok: boolean;
	stale: boolean;
	yanked: string[];
	advisories: string[];
}

export interface MigrateStatus {
	needs_migration: boolean;
	from_schema?: string | null;
	to_schema?: string | null;
	project_schema_legacy: boolean;
}

export interface HealthSnapshot {
	schema_version: 'x07.studio.health_snapshot@0.1.0';
	captured_at: string;
	doctor: DoctorStatus;
	lockfile: LockfileStatus;
	migrate: MigrateStatus;
	overall_color: 'green' | 'amber' | 'red' | string;
}

export interface PbtCounterexample {
	repro_id: string;
	property: string;
	shrunk_input: unknown;
	repro_path: string;
}

export interface PbtRound {
	schema_version: 'x07.studio.pbt_round@0.1.0';
	session_id: string;
	started_at: string;
	finished_at?: string | null;
	properties_run: number;
	counterexamples: PbtCounterexample[];
	raw: unknown;
}

export interface ArchViolation {
	rule: string;
	file: string;
	summary: string;
}

export interface ArchCheckReport {
	schema_version: 'x07.studio.arch_check_report@0.1.0';
	passed: boolean;
	violations: ArchViolation[];
	raw: unknown;
}

export interface PkgCandidate {
	package: string;
	version: string;
	source: string;
	install_command: string;
}

export interface PkgProvidesResult {
	schema_version: 'x07.studio.pkg_provides_result@0.1.0';
	module_id: string;
	candidates: PkgCandidate[];
}

export interface BoundaryEntry {
	at: string;
	kind: string;
	policy: string;
	summary: string;
	cassette_path: string;
}

export interface CassetteRibbon {
	schema_version: 'x07.studio.cassette_ribbon@0.1.0';
	boundaries: BoundaryEntry[];
}

export interface CertificateSummary {
	schema_version: 'x07.studio.certificate_summary@0.1.0';
	session_id: string;
	profile: string;
	operational_entry: string;
	issued_at: string;
	expires_at?: string | null;
	proof_summary: unknown;
	trust_report: unknown;
	html_summary_path: string;
	signature: string;
}

export interface Recipe {
	schema_version: 'x07.studio.recipe@0.1.0';
	id: string;
	title: string;
	one_liner: string;
	intent_text: string;
	task_type: TaskType;
	module_id?: string | null;
	canonical_example_path?: string | null;
	scenario_paths?: string[];
	preview_posture: TrustPosture;
}

export interface QuorumRound {
	round: number;
	agents: Array<{ agent_id: string; questions: TurnQuestion[] }>;
	diff: Array<{ label: string; detail: string }>;
}

export interface AutopilotPolicy {
	auto_answer_min_confidence: number;
	max_clarify_rounds: number;
	auto_climb_to?: string | null;
	allow_repair_iters: number;
	allow_quorum: boolean;
}

export interface AutopilotDecision {
	at: string;
	stage: string;
	action: string;
	reason: string;
}

export interface AutopilotState {
	schema_version: 'x07.studio.autopilot_state@0.1.0';
	session_id: string;
	engaged: boolean;
	policy: AutopilotPolicy;
	last_decision?: AutopilotDecision | null;
}

export interface AutopilotResponse {
	state: AutopilotState;
	session: SessionSnapshot;
}

export interface CassetteEntry {
	idx: number;
	kind: string;
	key: string;
	ts: string;
	size_bytes: number;
}

export interface AskAnswer {
	text: string;
	citations: Array<{ kind: string; path: string; locator: string }>;
}

export interface SyncCode {
	code: string;
	expires_at: string;
	session_id: string;
	state_blob?: unknown;
}

export interface SyncClaimResponse {
	session: SessionSnapshot;
	state_blob?: unknown;
}

export interface VoiceTranscript {
	schema_version: 'x07.studio.voice_transcript@0.1.0';
	text: string;
	confidence: number;
	language: string;
	recorded_at: string;
}

export interface ReleaseStatus {
	schema_version: 'x07.studio.release_status@0.1.0';
	release_id: string;
	rung: string;
	environment: string;
	status: OperationStatus;
	op_ids: string[];
	message: string;
}

export interface ReplayCapsule {
	schema_version: 'x07.studio.replay_capsule@0.1.0';
	capsule_id: string;
	session: SessionSnapshot;
	manifest: unknown;
	signature: unknown;
}

export interface ReplayExportResponse {
	capsule_id: string;
	artifact: string;
	signature: unknown;
}

export type VisualKind = 'streampipe' | 'statemachine' | 'tasks';

export interface VisualResponse {
	schema_version: 'x07.studio.visual@0.1.0';
	kind: VisualKind | string;
	value: unknown;
}

export interface StudioMemory {
	preferences: {
		default_agent?: string | null;
		default_trust_profile?: string | null;
		naming_style?: string | null;
		verbosity?: string | null;
	};
	recent_projects: Array<{ root: string; last_session_id?: string | null; label: string }>;
	reusable_specs: Array<{ module_id: string; path: string; summary: string }>;
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
	revision_notes?: string[];
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

export type ProviderProbeMode = 'shallow' | 'deep';
export type ProviderTrustTier = 'local_trusted' | 'remote_untrusted' | 'remote_trusted';
export type ProviderProbeStatus = 'supported' | 'unsupported' | 'unknown' | 'error';
export type ProviderGateState = 'ready' | 'review' | 'blocked';

export interface ProviderProfile {
	schema_version: 'x07.studio.provider_profile@0.1.0';
	id: string;
	label: string;
	base_url: string;
	api_key_env?: string | null;
	api_key?: string | null;
	api_kind: 'openai_compatible';
	model?: string | null;
	default_headers: Record<string, string>;
	local: boolean;
	trust_tier: ProviderTrustTier;
	probe_mode: ProviderProbeMode;
	disabled: boolean;
}

export interface ProviderCapabilities {
	models_endpoint: ProviderProbeStatus;
	responses: ProviderProbeStatus;
	chat_completions: ProviderProbeStatus;
	tools: ProviderProbeStatus;
	json_schema: ProviderProbeStatus;
	streaming: ProviderProbeStatus;
}

export interface ProviderProbeReport {
	schema_version: 'x07.studio.provider_probe_report@0.1.0';
	profile_id: string;
	base_url: string;
	observed_at: string;
	ok: boolean;
	http_status?: number | null;
	models: string[];
	capabilities: ProviderCapabilities;
	notes: string[];
	raw?: unknown;
}

export interface ProviderProbeResponse {
	profile: ProviderProfile;
	report: ProviderProbeReport;
}

export interface ProviderGateItem {
	label: string;
	value: string;
	detail: string;
	state: ProviderGateState;
}

export type ProofCacheState = 'ready' | 'pending' | 'blocked';

export interface ProofCacheItem {
	label: string;
	value: string;
	artifact: string;
	detail: string;
	state: ProofCacheState;
	opId?: string | null;
}

export type VerifyEvidenceState = 'pass' | 'warn' | 'fail' | 'skip' | 'pending';

export interface VerifyEvidenceEntry {
	entry: string;
	opId: string;
	specPath: string;
	coverage: VerifyEvidenceState;
	prove: VerifyEvidenceState;
	proveRaw: string;
	coverageReport: string;
	proveReport: string;
	proofObject: string;
	diagnostic: string;
}

export interface VerifyEvidenceArtifact {
	label: string;
	kind: string;
	path: string;
	schemaVersion: string;
}

export interface VerifyEvidenceBoard {
	source: 'report' | 'operation' | 'pending';
	outcome: VerifyEvidenceState;
	world: string;
	proofPolicy: VerifyProofPolicy | string;
	bounds: string;
	prechecks: Array<{ label: string; state: VerifyEvidenceState }>;
	coverageOutcome: VerifyEvidenceState;
	proveOutcome: VerifyEvidenceState;
	tests: {
		outcome: VerifyEvidenceState;
		passed: string;
		failed: string;
		skipped: string;
		report: string;
	};
	diagnostics: {
		outcome: VerifyEvidenceState;
		errors: string;
		warnings: string;
		topCodes: string[];
		report: string;
	};
	counts: Array<{ label: string; value: string }>;
	entries: VerifyEvidenceEntry[];
	artifacts: VerifyEvidenceArtifact[];
	generatedTestManifest: string;
	verifyDir: string;
}

export type CertEvidenceState = VerifyEvidenceState;

export interface CertEvidenceProjectRef {
	label: string;
	path: string;
	sha256: string;
	state: CertEvidenceState;
}

export interface CertEvidenceEntry {
	entry: string;
	state: CertEvidenceState;
	outDir: string;
	certificatePath: string;
	certificateSha256: string;
	trustReportPath: string;
	trustReportSha256: string;
	reviewDiffJsonPath: string;
	reviewDiffTxtPath: string;
	digestStatus: CertEvidenceState;
}

export interface CertEvidenceBoard {
	source: 'report' | 'operation' | 'pending';
	outcome: CertEvidenceState;
	scope: string;
	specDir: string;
	outDir: string;
	prechecks: CertEvidenceState;
	generatedAt: string;
	reviewGates: string[];
	entriesRequested: string[];
	projectRefs: CertEvidenceProjectRef[];
	summary: Array<{ label: string; value: string; detail: string }>;
	entries: CertEvidenceEntry[];
	artifacts: VerifyEvidenceArtifact[];
}

export interface CertBundleDigest {
	path: string;
	sha256: string;
	bytesLen: string;
}

export interface CertBundleEntry {
	entry: string;
	dir: string;
}

export interface CertBundlePreview {
	schemaVersion: string;
	outcome: CertEvidenceState;
	outDir: string;
	specDir: string;
	generatedAt: string;
	entries: CertBundleEntry[];
	files: CertBundleDigest[];
	externalFiles: CertBundleDigest[];
	specDigests: CertBundleDigest[];
	examplesDigests: CertBundleDigest[];
	totals: Array<{ label: string; value: string; detail: string }>;
}

export type VerifyProofPolicy = 'balanced' | 'strict';

export interface VerifyRunOptions {
	proofPolicy: VerifyProofPolicy;
	allowOsWorld: boolean;
	unwind: string;
	maxBytesLen: string;
	inputLenBytes: string;
}

export type RepairStrategy = 'semantic' | 'semantic_only' | 'quickfix_only' | 'spec_patch';

export interface RepairRunOptions {
	entry: string;
	strategy: RepairStrategy;
	write: boolean;
	allowEditNonStubs: boolean;
	maxRounds: string;
	maxCandidates: string;
	semanticMaxDepth: string;
}

export interface CertifyRunOptions {
	specDir: string;
	entry: string;
	allEntries: boolean;
	noPrechecks: boolean;
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

export interface AgentHandoffReview {
	agentLabel: string;
	status: string;
	command: string;
	promptPath: string;
	approval: string;
	source: string;
	allowedVerbs: string[];
	mcpTools: string[];
	writeRoots: string[];
	envContract: string[];
	boundaries: string[];
	runbook: string[];
	eventProtocol: string;
	promptExcerpt: string;
	opId?: string | null;
}

export type AgentExecutionStepState = 'done' | 'active' | 'blocked' | 'failed';

export interface AgentExecutionStep {
	id: string;
	label: string;
	state: AgentExecutionStepState;
	evidence: string;
	detail: string;
	artifact: string;
	opId?: string | null;
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

export interface RequestIntentRevisionResponse {
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

export interface IntentReviewItem {
	kind: 'ambiguity' | 'assumption';
	text: string;
	state: 'review';
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
	opId?: string | null;
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

export interface SpecSourcePreview {
	state: 'empty' | 'ready' | 'invalid';
	moduleId: string;
	entry: string;
	detail: string;
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

export const defaultProviderProfiles: ProviderProfile[] = [
	{
		schema_version: 'x07.studio.provider_profile@0.1.0',
		id: 'ollama-local',
		label: 'Ollama local',
		base_url: 'http://127.0.0.1:11434/v1',
		api_key_env: null,
		api_key: null,
		api_kind: 'openai_compatible',
		model: null,
		default_headers: {},
		local: true,
		trust_tier: 'local_trusted',
		probe_mode: 'deep',
		disabled: false
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

export function buildProviderProbeGates(
	profile: ProviderProfile | null | undefined,
	report: ProviderProbeReport | null | undefined
): ProviderGateItem[] {
	if (!profile) {
		return [
			{
				label: 'Provider profile',
				value: 'missing',
				detail: 'Configure an OpenAI-compatible provider before provider-backed polish.',
				state: 'blocked'
			}
		];
	}
	if (profile.disabled) {
		return [
			{
				label: 'Provider profile',
				value: 'disabled',
				detail: `${profile.label} is disabled; deterministic intent polish remains active.`,
				state: 'blocked'
			}
		];
	}
	const capabilities = report?.capabilities;
	const models = report?.models ?? [];
	const hasModel = Boolean(profile.model || models.length);
	return [
		{
			label: 'Model catalog',
			value: report ? `${models.length} model${models.length === 1 ? '' : 's'}` : 'probe pending',
			detail: profile.model
				? `Pinned model ${profile.model}`
				: hasModel
					? `Defaulting to ${models[0]}`
					: 'Deep probe should confirm an available model.',
			state: report ? (hasModel ? 'ready' : 'blocked') : 'review'
		},
		capabilityGate('Intent polish API', capabilities?.responses, capabilities?.chat_completions),
		capabilityGate('Tool calls', capabilities?.tools),
		capabilityGate('JSON schema', capabilities?.json_schema),
		capabilityGate('Streaming', capabilities?.streaming),
		{
			label: 'Trust tier',
			value: profile.trust_tier.replaceAll('_', ' '),
			detail: profile.local
				? 'Local provider output can be reviewed without remote data transfer.'
				: 'Remote provider output is advisory evidence and needs human review.',
			state: profile.trust_tier === 'remote_untrusted' ? 'review' : 'ready'
		}
	];
}

function capabilityGate(
	label: string,
	primary: ProviderProbeStatus | undefined,
	fallback?: ProviderProbeStatus | undefined
): ProviderGateItem {
	const status = combinedProviderStatus(primary, fallback);
	const fallbackText = fallback ? `; fallback ${providerStatusLabel(fallback)}` : '';
	return {
		label,
		value: providerStatusLabel(status),
		detail: providerStatusDetail(label, status, fallbackText),
		state: providerStatusState(status)
	};
}

function combinedProviderStatus(
	primary: ProviderProbeStatus | undefined,
	fallback: ProviderProbeStatus | undefined
): ProviderProbeStatus {
	if (primary === 'supported' || fallback === 'supported') return 'supported';
	if (primary === 'error' || fallback === 'error') return 'error';
	if (primary === 'unsupported' || fallback === 'unsupported') return 'unsupported';
	return primary ?? fallback ?? 'unknown';
}

function providerStatusLabel(status: ProviderProbeStatus): string {
	return status.replaceAll('_', ' ');
}

function providerStatusState(status: ProviderProbeStatus): ProviderGateState {
	if (status === 'supported') return 'ready';
	if (status === 'unsupported' || status === 'error') return 'blocked';
	return 'review';
}

function providerStatusDetail(label: string, status: ProviderProbeStatus, suffix: string): string {
	if (status === 'supported') return `${label} is available for provider-backed review${suffix}.`;
	if (status === 'unsupported') return `${label} is not available; keep deterministic fallback active${suffix}.`;
	if (status === 'error') return `${label} probe failed; do not rely on this provider yet${suffix}.`;
	return `${label} has not been proven by a deep probe${suffix}.`;
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

export function buildIntentReviewItems(intent: IntentPacket | null | undefined): IntentReviewItem[] {
	if (!intent) return [];
	return [
		...intent.ambiguities.map((text) => ({
			kind: 'ambiguity' as const,
			text: text.trim(),
			state: 'review' as const
		})),
		...intent.assumptions.map((text) => ({
			kind: 'assumption' as const,
			text: text.trim(),
			state: 'review' as const
		}))
	].filter((item) => item.text.length > 0);
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

export function buildAgentHandoffReview(
	session: SessionSnapshot | null | undefined,
	agentId: string,
	latestHandoff: AgentHandoff | null | undefined
): AgentHandoffReview {
	const ops = session?.op_log ?? [];
	const matchingHandoff =
		latestHandoff &&
		latestHandoff.session_id === session?.session_id &&
		(!agentId || latestHandoff.agent_id === agentId)
			? latestHandoff
			: null;
	const sourceOp = latestAgentHandoffOp(ops, agentId);
	const handoff = matchingHandoff ?? handoffFromOp(sourceOp);
	if (!handoff) {
		return {
			agentLabel: agentId === 'claude-code' ? 'Claude Code' : 'OpenAI Codex',
			status: 'Not generated',
			command: 'Generate a handoff before supervised execution',
			promptPath: '.x07/studio/handoffs/',
			approval: 'Human checkpoint before execute',
			source: sourceOp?.op ?? 'No handoff operation recorded',
			allowedVerbs: session?.allowed_verbs.slice(0, 6) ?? [],
			mcpTools: session?.contract?.global_doctrine.mcp_tools.slice(0, 5) ?? canonicalMcpTools,
			writeRoots: session?.contract?.project_doctrine.write_policy.paths ?? ['spec/', 'src/', 'tests/'],
			envContract: handoffEnvironmentContract(
				session?.session_id ?? '<pending>',
				agentId,
				'.x07/studio/handoffs/',
				session?.allowed_verbs ?? [],
				session?.contract?.global_doctrine.mcp_tools ?? canonicalMcpTools,
				session?.contract?.project_doctrine.write_policy.paths ?? ['spec/', 'src/', 'tests/'],
				true
			),
			boundaries: ['Generate a handoff to compile execution boundaries from the approved session.'],
			runbook: ['Approve the spec before supervised agent execution.'],
			eventProtocol: 'agent_event JSONL is advertised after handoff generation',
			promptExcerpt: 'No handoff prompt generated yet.',
			opId: sourceOp?.id ?? null
		};
	}
	const boundaries = markdownSectionLines(handoff.prompt, 'Execution Boundary');
	const runbook = markdownSectionLines(handoff.prompt, 'Automation Runbook');
	const eventProtocol = markdownSectionText(handoff.prompt, 'Agent Event Protocol')
		.split('\n')
		.map((line) => line.trim())
		.find((line) => line.includes('kind') || line.includes('agent_event')) ??
		'Emit x07.studio.agent_event@0.1.0 JSONL milestones.';
	return {
		agentLabel: handoff.agent_label,
		status: sourceOp ? `${sourceOp.op} / ${sourceOp.status}` : 'Handoff response ready',
		command: handoff.command.join(' '),
		promptPath: handoff.prompt_path,
		approval: handoff.approval_required
			? 'Human checkpoint before execute'
			: 'Execution allowed by profile',
		source: sourceOp?.op ?? 'latest handoff response',
		allowedVerbs: handoff.allowed_verbs,
		mcpTools: handoff.mcp_tools,
		writeRoots: handoff.write_roots,
		envContract: handoffEnvironmentContract(
			handoff.session_id,
			handoff.agent_id,
			handoff.prompt_path,
			handoff.allowed_verbs,
			handoff.mcp_tools,
			handoff.write_roots,
			handoff.approval_required
		),
		boundaries: boundaries.length ? boundaries : ['solve-pure default lane; widening requires approval.'],
		runbook: runbook.length ? runbook : ['Use x07 docs/MCP tools before selecting commands.'],
		eventProtocol,
		promptExcerpt: handoff.prompt.split('\n').slice(0, 10).join('\n').trim(),
		opId: sourceOp?.id ?? null
	};
}

function handoffEnvironmentContract(
	sessionId: string,
	agentId: string,
	handoffPath: string,
	allowedVerbs: string[],
	mcpTools: string[],
	writeRoots: string[],
	approvalRequired: boolean
): string[] {
	return [
		`X07_STUDIO_SESSION_ID=${sessionId}`,
		`X07_STUDIO_AGENT_ID=${agentId}`,
		`X07_STUDIO_HANDOFF_PATH=${handoffPath}`,
		`X07_STUDIO_ALLOWED_VERBS=${allowedVerbs.join(',')}`,
		`X07_STUDIO_MCP_TOOLS=${mcpTools.join(',')}`,
		`X07_STUDIO_WRITE_ROOTS=${writeRoots.join(',')}`,
		`X07_STUDIO_APPROVAL_REQUIRED=${String(approvalRequired)}`,
		'X07_STUDIO_EVENT_SCHEMA=x07.studio.agent_event@0.1.0'
	];
}

export function buildAgentExecutionTimeline(
	session: SessionSnapshot | null | undefined,
	agentId: string,
	agent?: AgentProfile | null
): AgentExecutionStep[] {
	const ops = session?.op_log ?? [];
	const handoff = latestExactOp(ops, `agent.handoff.${agentId}`);
	const plan = latestExactOp(ops, `agent.supervise.${agentId}`);
	const approval = latestExactOp(ops, `agent.approval.${agentId}`);
	const run = latestExactOp(ops, `agent.run.${agentId}`);
	const events = ops.filter((op) => op.op.startsWith(`agent.event.${agentId}.`));
	const eventKinds = agentEventKindCounts(events);
	const writeAudit = agentWriteAuditSummary(run);
	const approvalRequired = agent?.approval_required ?? true;
	const promptArtifact = handoff?.artifacts[0] ?? plan?.artifacts[0] ?? run?.artifacts[0] ?? '.x07/studio/handoffs/';

	return [
		{
			id: 'handoff',
			label: 'Handoff',
			state: handoff ? opStatusToAgentStepState(handoff.status) : session ? 'active' : 'blocked',
			evidence: handoff?.op ?? 'generate a session-contract handoff',
			detail: handoff?.notes ?? 'Compiles approved intent, allowed verbs, MCP tools, and write roots.',
			artifact: promptArtifact,
			opId: handoff?.id ?? null
		},
		{
			id: 'plan',
			label: 'Launch plan',
			state: plan ? opStatusToAgentStepState(plan.status) : handoff ? 'active' : 'blocked',
			evidence: plan?.op ?? (handoff ? 'ready to record supervised launch plan' : 'blocked before handoff'),
			detail: plan?.command.join(' ') ?? 'Dry-run the supervised command before execution.',
			artifact: plan?.artifacts[0] ?? promptArtifact,
			opId: plan?.id ?? null
		},
		{
			id: 'approval',
			label: 'Human checkpoint',
			state: approval
				? opStatusToAgentStepState(approval.status)
				: approvalRequired
					? plan || handoff
						? 'active'
						: 'blocked'
					: 'done',
			evidence: approval?.op ?? (approvalRequired ? 'approval required before execute' : 'profile allows autonomous execute'),
			detail: approval?.notes ?? 'Humans must approve policy, spec, architecture, world, budget, trust, or release widening.',
			artifact: approval?.artifacts[0] ?? promptArtifact,
			opId: approval?.id ?? null
		},
		{
			id: 'run',
			label: 'Supervised run',
			state: run
				? opStatusToAgentStepState(run.status)
				: approval?.status === 'succeeded' || (!approvalRequired && plan)
					? 'active'
					: 'blocked',
			evidence: run?.op ?? 'run waits for approval and command availability',
			detail: run?.notes ?? run?.command.join(' ') ?? 'Run under Loom supervision with captured stdout/stderr.',
			artifact: run?.artifacts[0] ?? promptArtifact,
			opId: run?.id ?? null
		},
		{
			id: 'events',
			label: 'Agent events',
			state: events.length ? 'done' : run ? 'active' : 'blocked',
			evidence: events.length ? `${events.length} events` : 'waiting for agent_event JSONL or classified output',
			detail: eventKinds || 'Artifacts, diagnostics, writes, and approval requests are promoted into worklog records.',
			artifact: events.flatMap((op) => op.artifacts).at(0) ?? 'x07.studio.agent_event@0.1.0',
			opId: events.at(-1)?.id ?? null
		},
		{
			id: 'write-audit',
			label: 'Write-root audit',
			state: writeAudit
				? writeAudit.violations
					? 'failed'
					: 'done'
				: run
					? 'active'
					: 'blocked',
			evidence: writeAudit
				? `${writeAudit.created} created / ${writeAudit.modified} modified / ${writeAudit.deleted} deleted`
				: 'audit recorded when supervised execution completes',
			detail: writeAudit
				? writeAudit.violations
					? `${writeAudit.violations} unapproved writes`
					: 'No out-of-contract writes'
				: 'Approved write roots are enforced by post-run evidence.',
			artifact: 'x07.studio.agent_write_audit@0.1.0',
			opId: run?.id ?? null
		}
	];
}

function latestAgentHandoffOp(ops: OpRecord[], agentId: string): OpRecord | null {
	const agentNeedles = agentId ? [`agent.handoff.${agentId}`, `agent.supervise.${agentId}`, `agent.run.${agentId}`] : [];
	const genericNeedles = ['agent.handoff.', 'agent.supervise.', 'agent.run.'];
	return latestMatchingOp(ops, agentNeedles.length ? agentNeedles : genericNeedles);
}

function latestExactOp(ops: OpRecord[], opName: string): OpRecord | null {
	return [...ops].reverse().find((op) => op.op === opName) ?? null;
}

function opStatusToAgentStepState(status: OperationStatus): AgentExecutionStepState {
	if (status === 'succeeded') return 'done';
	if (status === 'failed') return 'failed';
	if (status === 'running') return 'active';
	return 'active';
}

function agentEventKindCounts(events: OpRecord[]): string {
	const counts = new Map<string, number>();
	for (const event of events) {
		const kind = event.op.split('.').at(-1) ?? 'event';
		counts.set(kind, (counts.get(kind) ?? 0) + 1);
	}
	return [...counts.entries()].map(([kind, count]) => `${kind} x${count}`).join(' / ');
}

function agentWriteAuditSummary(op: OpRecord | null): {
	created: number;
	modified: number;
	deleted: number;
	violations: number;
} | null {
	const report = asPlainRecord(op?.report_json);
	const audit = asPlainRecord(report?.write_audit);
	if (!audit) return null;
	return {
		created: stringArray(audit.created).length,
		modified: stringArray(audit.modified).length,
		deleted: stringArray(audit.deleted).length,
		violations: stringArray(audit.violations).length
	};
}

function handoffFromOp(op: OpRecord | null): AgentHandoff | null {
	const report = asPlainRecord(op?.report_json);
	if (!report) return null;
	const direct = parseAgentHandoff(report);
	if (direct) return direct;
	const nested = asPlainRecord(report.handoff);
	return parseAgentHandoff(nested);
}

function parseAgentHandoff(value: Record<string, unknown> | null): AgentHandoff | null {
	if (!value || value.schema_version !== 'x07.studio.agent_handoff@0.1.0') return null;
	const command = stringArray(value.command);
	const allowedVerbs = stringArray(value.allowed_verbs);
	const mcpTools = stringArray(value.mcp_tools);
	const writeRoots = stringArray(value.write_roots);
	const artifacts = stringArray(value.artifacts);
	if (!command.length || !allowedVerbs.length) return null;
	return {
		schema_version: 'x07.studio.agent_handoff@0.1.0',
		session_id: String(value.session_id ?? ''),
		agent_id: String(value.agent_id ?? ''),
		agent_label: String(value.agent_label ?? value.agent_id ?? 'Agent'),
		command,
		prompt_path: String(value.prompt_path ?? command.at(-1) ?? '.x07/studio/handoffs/'),
		prompt: String(value.prompt ?? ''),
		allowed_verbs: allowedVerbs,
		mcp_tools: mcpTools,
		write_roots: writeRoots,
		approval_required: Boolean(value.approval_required),
		artifacts,
		created_at: String(value.created_at ?? '')
	};
}

function markdownSectionLines(markdown: string, heading: string): string[] {
	return markdownSectionText(markdown, heading)
		.split('\n')
		.map((line) => line.trim())
		.filter((line) => line.startsWith('- '))
		.map((line) => line.slice(2).trim())
		.filter(Boolean)
		.slice(0, 6);
}

function markdownSectionText(markdown: string, heading: string): string {
	const marker = `## ${heading}`;
	const start = markdown.indexOf(marker);
	if (start < 0) return '';
	const rest = markdown.slice(start + marker.length);
	const next = rest.indexOf('\n## ');
	return (next >= 0 ? rest.slice(0, next) : rest).trim();
}

function asPlainRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: null;
}

function stringArray(value: unknown): string[] {
	return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
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
	const polishOp = latestOpForOps(ops, ['intent.formalize']);
	const projectScaffoldCommand = projectScaffoldBindingId(template);
	const seededProject = projectScaffoldCommand.startsWith('project.seed.');
	const scaffoldOp = latestOpForOps(ops, [projectScaffoldCommand]);
	const setupSteps: AutomationPlanStep[] = [
		{
			label: 'Plan polish',
			command: 'intent.formalize',
			artifact: '.x07/studio/sessions/intent.json',
			gate: 'Human reviews the polished intent before spec approval',
			state: intentReady ? 'done' : session ? 'ready' : 'blocked',
			opId: polishOp?.id ?? null
		},
		{
			label: 'Human approval',
			command: 'approve_spec',
			artifact: '.x07/studio/sessions/session_contract.json',
			gate:
				approvalState === 'changes'
					? 'Blocked until the agent repolishes requested changes'
					: 'Locks write roots, docs, MCP tools, worlds, and budgets',
			state: approved ? 'done' : approvalState === 'changes' ? 'blocked' : intentReady ? 'ready' : 'blocked',
			opId: null
		},
		{
			label: 'Project scaffold',
			command: projectScaffoldCommand,
			artifact: 'x07.json',
			gate: 'Creates the x07 project only after approval',
			state: stateForOps(ops, [projectScaffoldCommand], approved),
			opId: scaffoldOp?.id ?? null
		}
	];

	if (taskType === 'brownfield_extract') {
		const op = latestOpForOps(ops, ['spec.extract']);
		setupSteps.push({
			label: 'Brownfield extraction',
			command: 'spec.extract',
			artifact: 'target/xtal/spec.extract.report.json',
			gate: 'Extract current behavior before implementation writes',
			state: stateForOps(ops, ['spec.extract'], approved),
			opId: op?.id ?? null
		});
	} else if (taskType === 'incident_repair') {
		const ingestOp = latestOpForOps(ops, ['xtal.ingest']);
		const improveOp = latestOpForOps(ops, ['xtal.improve']);
		setupSteps.push(
			{
				label: 'Incident normalization',
				command: 'xtal.ingest --normalize-only',
				artifact: 'target/xtal/ingest/summary.json',
				gate: 'Converts incident notes into canonical XTAL evidence',
				state: stateForOps(ops, ['xtal.ingest'], approved),
				opId: ingestOp?.id ?? null
			},
			{
				label: 'Improve from incident',
				command: 'xtal.improve',
				artifact: 'target/xtal/improve/summary.json',
				gate: 'Creates regression evidence before repair trust',
				state: stateForOps(ops, ['xtal.improve'], approved),
				opId: improveOp?.id ?? null
			}
		);
	} else if (!seededProject) {
		const specOp = latestOpForOps(ops, ['spec.scaffold']);
		const testsOp = latestOpForOps(ops, ['tests.gen.write']);
		setupSteps.push(
			{
				label: 'Spec scaffold',
				command: 'spec.scaffold',
				artifact: template.artifacts[0] ?? 'spec/',
				gate: 'Creates spec artifacts before implementation',
				state: stateForOps(ops, ['spec.scaffold'], approved),
				opId: specOp?.id ?? null
			},
			{
				label: 'Generated tests',
				command: 'tests.gen.write',
				artifact: template.artifacts[1] ?? 'gen/xtal/tests.json',
				gate: 'Examples become executable tests',
				state: stateForOps(ops, ['tests.gen.write'], approved),
				opId: testsOp?.id ?? null
			}
		);
	}

	if (!seededProject && taskType !== 'incident_repair') {
		const implOp = latestOpForOps(ops, ['impl.sync.write']);
		setupSteps.push({
			label: 'Implementation sync',
			command: 'impl.sync.write',
			artifact: 'target/xtal/impl-sync.patchset.json',
			gate: 'Implementation writes stay inside approved roots',
			state: stateForOps(ops, ['impl.sync.write'], approved),
			opId: implOp?.id ?? null
		});
	}

	const templateSteps = template.canonicalCommands.map((command, index) => {
		const op = latestOpForCommand(ops, command);
		return {
			label: `${template.label} evidence ${index + 1}`,
			command,
			artifact: template.artifacts[index] ?? template.artifacts.at(-1) ?? 'target/xtal/',
			gate: template.riskProfile,
			state: stateForCommand(ops, command, approved),
			opId: op?.id ?? null
		};
	});

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
	const agentOp = latestMatchingOp(ops, [
		'agent.handoff.',
		'agent.supervise.',
		'agent.run.',
		'agent.event.',
		'agent.approval.'
	]);
	const trustOp = latestMatchingOp(ops, [
		'xtal.certify',
		'wasm.provenance.verify',
		'wasm.deploy.plan',
		'lp.deploy.status.local',
		'lp.deploy.query.local'
	]);
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
			evidence: agentOp?.op ?? (session ? 'waiting for Codex/Claude handoff or supervised run evidence' : 'no session selected'),
			artifact: agentOp?.artifacts[0] ?? '.x07/studio/handoffs/',
			state: agentOp ? opStatusToCoverageState(agentOp.status) : session ? 'active' : 'blocked',
			op: agentOp
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

export function defaultVerifyRunOptions(): VerifyRunOptions {
	return {
		proofPolicy: 'balanced',
		allowOsWorld: false,
		unwind: '',
		maxBytesLen: '',
		inputLenBytes: ''
	};
}

export function normalizeVerifyRunOptions(options?: Partial<VerifyRunOptions>): VerifyRunOptions {
	const defaults = defaultVerifyRunOptions();
	return {
		proofPolicy: options?.proofPolicy === 'strict' ? 'strict' : defaults.proofPolicy,
		allowOsWorld: Boolean(options?.allowOsWorld),
		unwind: normalizePositiveIntegerText(options?.unwind),
		maxBytesLen: normalizePositiveIntegerText(options?.maxBytesLen),
		inputLenBytes: normalizePositiveIntegerText(options?.inputLenBytes)
	};
}

export function verifyRunVars(options?: Partial<VerifyRunOptions>): Record<string, string> {
	const normalized = normalizeVerifyRunOptions(options);
	return {
		proof_policy: normalized.proofPolicy,
		allow_os_world: String(normalized.allowOsWorld),
		unwind: normalized.unwind,
		max_bytes_len: normalized.maxBytesLen,
		input_len_bytes: normalized.inputLenBytes
	};
}

export function defaultRepairRunOptions(): RepairRunOptions {
	return {
		entry: '',
		strategy: 'semantic',
		write: false,
		allowEditNonStubs: false,
		maxRounds: '',
		maxCandidates: '',
		semanticMaxDepth: ''
	};
}

export function normalizeRepairRunOptions(options?: Partial<RepairRunOptions>): RepairRunOptions {
	const defaults = defaultRepairRunOptions();
	const strategy = options?.strategy;
	return {
		entry: (options?.entry ?? defaults.entry).trim(),
		strategy:
			strategy === 'semantic_only' || strategy === 'quickfix_only' || strategy === 'spec_patch'
				? strategy
				: defaults.strategy,
		write: Boolean(options?.write),
		allowEditNonStubs: Boolean(options?.allowEditNonStubs),
		maxRounds: normalizePositiveIntegerText(options?.maxRounds),
		maxCandidates: normalizePositiveIntegerText(options?.maxCandidates),
		semanticMaxDepth: normalizePositiveIntegerText(options?.semanticMaxDepth)
	};
}

export function repairRunVars(options?: Partial<RepairRunOptions>): Record<string, string> {
	const normalized = normalizeRepairRunOptions(options);
	return {
		repair_entry: normalized.entry,
		repair_strategy: normalized.strategy,
		repair_write: String(normalized.write),
		repair_allow_edit_non_stubs: String(normalized.allowEditNonStubs),
		repair_max_rounds: normalized.maxRounds,
		repair_max_candidates: normalized.maxCandidates,
		repair_semantic_max_depth: normalized.semanticMaxDepth
	};
}

export function defaultCertifyRunOptions(): CertifyRunOptions {
	return {
		specDir: 'spec',
		entry: '',
		allEntries: false,
		noPrechecks: false
	};
}

export function normalizeCertifyRunOptions(options?: Partial<CertifyRunOptions>): CertifyRunOptions {
	const defaults = defaultCertifyRunOptions();
	const allEntries = Boolean(options?.allEntries);
	return {
		specDir: (options?.specDir ?? defaults.specDir).trim() || defaults.specDir,
		entry: allEntries ? '' : (options?.entry ?? defaults.entry).trim(),
		allEntries,
		noPrechecks: Boolean(options?.noPrechecks)
	};
}

export function certifyRunVars(options?: Partial<CertifyRunOptions>): Record<string, string> {
	const normalized = normalizeCertifyRunOptions(options);
	return {
		cert_spec_dir: normalized.specDir,
		cert_entry: normalized.entry,
		cert_all: String(normalized.allEntries),
		cert_no_prechecks: String(normalized.noPrechecks)
	};
}

export function buildVerifyCommandPreview(options?: Partial<VerifyRunOptions>): string {
	const normalized = normalizeVerifyRunOptions(options);
	const args = ['x07', 'xtal', 'verify', '--proof-policy', normalized.proofPolicy];
	if (normalized.allowOsWorld) args.push('--allow-os-world');
	if (normalized.unwind) args.push('--unwind', normalized.unwind);
	if (normalized.maxBytesLen) args.push('--max-bytes-len', normalized.maxBytesLen);
	if (normalized.inputLenBytes) args.push('--input-len-bytes', normalized.inputLenBytes);
	return args.join(' ');
}

export function buildCertifyCommandPreview(options?: Partial<CertifyRunOptions>): string {
	const normalized = normalizeCertifyRunOptions(options);
	const args = ['x07', 'xtal', 'certify'];
	if (normalized.noPrechecks) args.push('--no-prechecks');
	if (normalized.specDir) args.push('--spec-dir', normalized.specDir);
	if (normalized.allEntries) args.push('--all');
	else if (normalized.entry) args.push('--entry', normalized.entry);
	return args.join(' ');
}

export function buildCertEvidenceBoard(
	op: OpRecord | null | undefined,
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate,
	options?: Partial<CertifyRunOptions>
): CertEvidenceBoard {
	const normalized = normalizeCertifyRunOptions(options);
	const report = certSummaryFromValue(op?.report_json);
	if (report) return certEvidenceBoardFromReport(report, normalized);
	return certEvidenceBoardFromOperation(op ?? null, session, template, normalized);
}

export function buildCertBundlePreview(op: OpRecord | null | undefined): CertBundlePreview | null {
	const bundle = certBundleFromValue(op?.report_json);
	if (!bundle) return null;
	const files = certBundleDigests(bundle.files);
	const externalFiles = certBundleDigests(bundle.external_files);
	const specDigests = certBundleDigests(bundle.spec_digests);
	const examplesDigests = certBundleDigests(bundle.examples_digests);
	const entries = certBundleEntries(bundle.entries);
	const byteTotal = [...files, ...externalFiles, ...specDigests, ...examplesDigests].reduce(
		(total, item) => total + (Number.parseInt(item.bytesLen, 10) || 0),
		0
	);
	return {
		schemaVersion: 'x07.xtal.cert_bundle@0.1.0',
		outcome: booleanOutcomeState(bundle.ok),
		outDir: stringValue(bundle.out_dir, 'target/xtal/cert'),
		specDir: stringValue(bundle.spec_dir, 'spec'),
		generatedAt: stringValue(bundle.generated_at, 'not generated'),
		entries,
		files,
		externalFiles,
		specDigests,
		examplesDigests,
		totals: [
			{ label: 'Entries', value: String(entries.length), detail: 'certified entry dirs' },
			{ label: 'Files', value: String(files.length), detail: `${byteTotal} bytes covered` },
			{ label: 'External', value: String(externalFiles.length), detail: 'external evidence files' },
			{ label: 'Spec digests', value: String(specDigests.length), detail: `${examplesDigests.length} example digests` }
		]
	};
}

export function buildRepairCommandPreview(options?: Partial<RepairRunOptions>): string {
	const normalized = normalizeRepairRunOptions(options);
	const args = ['x07', 'xtal', 'repair'];
	if (normalized.entry) args.push('--entry', normalized.entry);
	if (normalized.write) args.push('--write');
	if (normalized.maxRounds) args.push('--max-rounds', normalized.maxRounds);
	if (normalized.maxCandidates) args.push('--max-candidates', normalized.maxCandidates);
	if (normalized.semanticMaxDepth) args.push('--semantic-max-depth', normalized.semanticMaxDepth);
	if (normalized.allowEditNonStubs) args.push('--allow-edit-non-stubs');
	if (normalized.strategy === 'semantic_only') args.push('--semantic-only');
	if (normalized.strategy === 'quickfix_only') args.push('--quickfix-only');
	if (normalized.strategy === 'spec_patch') args.push('--suggest-spec-patch');
	return args.join(' ');
}

export function buildVerifyEvidenceBoard(
	op: OpRecord | null | undefined,
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate,
	options?: Partial<VerifyRunOptions>
): VerifyEvidenceBoard {
	const report = verifySummaryFromValue(op?.report_json);
	if (report) return verifyEvidenceBoardFromReport(report, template);
	return verifyEvidenceBoardFromOperation(op ?? null, session, template, options);
}

function certSummaryFromValue(value: unknown): Record<string, unknown> | null {
	const report = asPlainRecord(value);
	if (!report) return null;
	if (report.schema_version === 'x07.xtal.certify_summary@0.1.0') return report;
	const stdoutJson = asPlainRecord(report.stdout_json);
	if (stdoutJson?.schema_version === 'x07.xtal.certify_summary@0.1.0') return stdoutJson;
	const result = asPlainRecord(report.result);
	const resultStdoutJson = asPlainRecord(result?.stdout_json);
	if (resultStdoutJson?.schema_version === 'x07.xtal.certify_summary@0.1.0') {
		return resultStdoutJson;
	}
	const artifactPreview = asPlainRecord(report.artifact_preview);
	const previewJson = asPlainRecord(artifactPreview?.json);
	return previewJson?.schema_version === 'x07.xtal.certify_summary@0.1.0' ? previewJson : null;
}

function certBundleFromValue(value: unknown): Record<string, unknown> | null {
	const report = asPlainRecord(value);
	if (!report) return null;
	if (report.schema_version === 'x07.xtal.cert_bundle@0.1.0') return report;
	const artifactPreview = asPlainRecord(report.artifact_preview);
	const previewJson = asPlainRecord(artifactPreview?.json);
	if (previewJson?.schema_version === 'x07.xtal.cert_bundle@0.1.0') return previewJson;
	const stdoutJson = asPlainRecord(report.stdout_json);
	if (stdoutJson?.schema_version === 'x07.xtal.cert_bundle@0.1.0') return stdoutJson;
	const result = asPlainRecord(report.result);
	const resultStdoutJson = asPlainRecord(result?.stdout_json);
	return resultStdoutJson?.schema_version === 'x07.xtal.cert_bundle@0.1.0'
		? resultStdoutJson
		: null;
}

function certEvidenceBoardFromReport(
	report: Record<string, unknown>,
	options: CertifyRunOptions
): CertEvidenceBoard {
	const project = asPlainRecord(report.project);
	const settings = asPlainRecord(report.settings);
	const resultRows = Array.isArray(report.results)
		? report.results
				.map((entry) => certEntryFromReport(entry))
				.filter((entry): entry is CertEvidenceEntry => Boolean(entry))
		: [];
	const outDir = stringValue(settings?.out_dir, 'target/xtal/cert');
	const entriesRequested = stringList(settings?.entries);
	const reviewGates = stringList(settings?.review_gates);
	const allEntries = settings?.all_entries === true;
	const runPrechecks = settings?.run_prechecks !== false;
	const outcome = booleanOutcomeState(report.ok);
	const totalEntries = resultRows.length;
	const passedEntries = resultRows.filter((entry) => entry.state === 'pass').length;
	const failedEntries = resultRows.filter((entry) => entry.state === 'fail').length;
	const scope = certScopeLabel(allEntries, entriesRequested);
	const generatedAt = stringValue(report.generated_at, 'not generated');
	const fallbackEntry = entriesRequested[0] ?? scope;

	return {
		source: 'report',
		outcome,
		scope,
		specDir: options.specDir,
		outDir,
		prechecks: runPrechecks ? outcome : 'skip',
		generatedAt,
		reviewGates,
		entriesRequested,
		projectRefs: [
			certProjectRef('Manifest', project?.manifest_path, project?.manifest_sha256, 'x07.json'),
			certProjectRef(
				'XTAL manifest',
				project?.xtal_manifest_path,
				project?.xtal_manifest_sha256,
				'arch/xtal/xtal.json'
			),
			certProjectRef(
				'Trust profile',
				project?.trust_profile_path,
				project?.trust_profile_sha256,
				'arch/trust/profile.json'
			),
			certProjectRef('Baseline', project?.baseline_path, project?.baseline_sha256, 'no baseline')
		],
		summary: [
			{
				label: 'Prechecks',
				value: runPrechecks ? 'run' : 'skipped',
				detail: runPrechecks ? 'x07 xtal dev prechecks' : '--no-prechecks'
			},
			{
				label: 'Review gates',
				value: String(reviewGates.length),
				detail: reviewGates.join(', ') || 'no review gates reported'
			},
			{
				label: 'Entries',
				value: `${passedEntries}/${totalEntries || entriesRequested.length || 1} passed`,
				detail: failedEntries ? `${failedEntries} failed` : entriesRequested.join(', ') || scope
			},
			{
				label: 'Bundle',
				value: outcome === 'pass' ? 'ready' : outcome,
				detail: `${outDir}/bundle.json`
			}
		],
		entries: resultRows.length ? resultRows : [pendingCertEntry(fallbackEntry, outDir, outcome)],
		artifacts: [
			{
				label: 'Certify summary',
				kind: 'x07.xtal.certify_summary',
				path: `${outDir}/summary.json`,
				schemaVersion: 'x07.xtal.certify_summary@0.1.0'
			},
			{
				label: 'Certify bundle',
				kind: 'x07.xtal.cert_bundle',
				path: `${outDir}/bundle.json`,
				schemaVersion: 'x07.xtal.cert_bundle@0.1.0'
			}
		]
	};
}

function certEvidenceBoardFromOperation(
	op: OpRecord | null,
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate,
	options: CertifyRunOptions
): CertEvidenceBoard {
	const status = op ? opStatusToVerifyState(op.status) : 'pending';
	const outDir = 'target/xtal/cert';
	const entry = options.allEntries ? 'all entries' : options.entry || defaultCertEntry(session, template);
	const scope = options.allEntries ? 'all entries' : entry;
	const artifacts = op?.artifacts.length
		? op.artifacts.map((path) => certArtifactFromPath(path))
		: [certArtifactFromPath(`${outDir}/summary.json`), certArtifactFromPath(`${outDir}/bundle.json`)];

	return {
		source: op ? 'operation' : 'pending',
		outcome: status,
		scope,
		specDir: options.specDir,
		outDir,
		prechecks: options.noPrechecks ? 'skip' : status,
		generatedAt: op?.finished_at ?? 'not generated',
		reviewGates: [],
		entriesRequested: [scope],
		projectRefs: [
			certProjectRef('Manifest', 'x07.json', '', 'x07.json'),
			certProjectRef('XTAL manifest', 'arch/xtal/xtal.json', '', 'arch/xtal/xtal.json'),
			certProjectRef('Trust profile', 'arch/trust/profile.json', '', 'arch/trust/profile.json'),
			certProjectRef('Baseline', '', '', 'no baseline')
		],
		summary: [
			{
				label: 'Prechecks',
				value: options.noPrechecks ? 'skipped' : status,
				detail: options.noPrechecks ? '--no-prechecks' : 'x07 xtal dev prechecks'
			},
			{ label: 'Review gates', value: 'pending', detail: 'reported by certify summary' },
			{ label: 'Entries', value: scope, detail: options.allEntries ? '--all' : '--entry' },
			{ label: 'Bundle', value: status, detail: `${outDir}/bundle.json` }
		],
		entries: [pendingCertEntry(entry, outDir, status)],
		artifacts
	};
}

function normalizePositiveIntegerText(value: string | number | undefined): string {
	const trimmed = value === undefined ? '' : String(value).trim();
	if (!trimmed) return '';
	const parsed = Number.parseInt(trimmed, 10);
	if (!Number.isFinite(parsed) || parsed <= 0) return '';
	return String(parsed);
}

function certEntryFromReport(value: unknown): CertEvidenceEntry | null {
	const entry = asPlainRecord(value);
	if (!entry) return null;
	const entryName = stringValue(entry.entry, 'entry');
	const outDir = stringValue(entry.out_dir, `target/xtal/cert/${entryPathSegment(entryName)}`);
	const certificatePath = stringValue(entry.certificate_path, `${outDir}/certificate.json`);
	const trustReportPath = stringValue(entry.trust_report_path, `${outDir}/trust.report.json`);
	const certificateSha256 = stringValue(entry.certificate_sha256, '');
	const trustReportSha256 = stringValue(entry.trust_report_sha256, '');
	const state = booleanOutcomeState(entry.ok);
	return {
		entry: entryName,
		state,
		outDir,
		certificatePath,
		certificateSha256,
		trustReportPath,
		trustReportSha256,
		reviewDiffJsonPath: stringValue(entry.review_diff_json_path, `${outDir}/review.diff.json`),
		reviewDiffTxtPath: stringValue(entry.review_diff_txt_path, `${outDir}/review.diff.txt`),
		digestStatus: certificateSha256 && trustReportSha256 ? 'pass' : state === 'fail' ? 'fail' : 'warn'
	};
}

function pendingCertEntry(entry: string, outDir: string, state: CertEvidenceState): CertEvidenceEntry {
	const localPath = entryPathSegment(entry);
	const entryOutDir = `${outDir}/${localPath}`;
	return {
		entry,
		state,
		outDir: entryOutDir,
		certificatePath: `${entryOutDir}/certificate.json`,
		certificateSha256: '',
		trustReportPath: `${entryOutDir}/trust.report.json`,
		trustReportSha256: '',
		reviewDiffJsonPath: `${entryOutDir}/review.diff.json`,
		reviewDiffTxtPath: `${entryOutDir}/review.diff.txt`,
		digestStatus: state === 'pass' ? 'warn' : state
	};
}

function certProjectRef(
	label: string,
	path: unknown,
	sha256: unknown,
	fallbackPath: string
): CertEvidenceProjectRef {
	const pathText = stringValue(path, fallbackPath);
	const shaText = stringValue(sha256, label === 'Baseline' && !pathText ? 'not configured' : '');
	return {
		label,
		path: pathText,
		sha256: shaText || (label === 'Baseline' && pathText === 'no baseline' ? 'not configured' : 'missing digest'),
		state: digestEvidenceState(path, sha256)
	};
}

function certArtifactFromPath(path: string): VerifyEvidenceArtifact {
	if (path.endsWith('bundle.json')) {
		return {
			label: 'Certify bundle',
			kind: 'x07.xtal.cert_bundle',
			path,
			schemaVersion: 'x07.xtal.cert_bundle@0.1.0'
		};
	}
	if (path.endsWith('summary.json')) {
		return {
			label: 'Certify summary',
			kind: 'x07.xtal.certify_summary',
			path,
			schemaVersion: 'x07.xtal.certify_summary@0.1.0'
		};
	}
	return { label: 'Certify artifact', kind: 'operation_artifact', path, schemaVersion: '' };
}

function certScopeLabel(allEntries: boolean, entries: string[]): string {
	if (allEntries) return 'all entries';
	if (entries.length) return entries.join(', ');
	return 'manifest entry selection';
}

function defaultCertEntry(
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate
): string {
	const target = session?.intent?.targets[0];
	if (target?.entry) return `${target.module_id}.${target.entry}`;
	if (target?.module_id) return `${target.module_id}.run_v1`;
	return template.id;
}

function stringList(value: unknown): string[] {
	if (!Array.isArray(value)) return [];
	return value.map((item) => String(item).trim()).filter(Boolean);
}

function certBundleEntries(value: unknown): CertBundleEntry[] {
	if (!Array.isArray(value)) return [];
	return value
		.map((item) => asPlainRecord(item))
		.filter((item): item is Record<string, unknown> => Boolean(item))
		.map((item) => ({
			entry: stringValue(item.entry, 'entry'),
			dir: stringValue(item.dir, 'target/xtal/cert')
		}));
}

function certBundleDigests(value: unknown): CertBundleDigest[] {
	if (!Array.isArray(value)) return [];
	return value
		.map((item) => asPlainRecord(item))
		.filter((item): item is Record<string, unknown> => Boolean(item))
		.map((item) => ({
			path: stringValue(item.path, 'artifact'),
			sha256: stringValue(item.sha256, 'missing digest'),
			bytesLen: stringValue(item.bytes_len, '0')
		}));
}

function entryPathSegment(entry: string): string {
	return entry.replaceAll('.', '/').replaceAll(' ', '_');
}

function booleanOutcomeState(value: unknown): CertEvidenceState {
	if (value === true) return 'pass';
	if (value === false) return 'fail';
	return 'pending';
}

function digestEvidenceState(path: unknown, sha256: unknown): CertEvidenceState {
	const hasPath = typeof path === 'string' && path.trim().length > 0;
	const hasSha = typeof sha256 === 'string' && sha256.trim().length > 0;
	if (!hasPath && !hasSha) return 'skip';
	if (hasPath && hasSha) return 'pass';
	return 'warn';
}

function verifySummaryFromValue(value: unknown): Record<string, unknown> | null {
	const report = asPlainRecord(value);
	if (!report) return null;
	if (report.schema_version === 'x07.xtal.verify_summary@0.1.0') return report;
	const result = asPlainRecord(report.result);
	const stdoutJson = asPlainRecord(result?.stdout_json);
	if (stdoutJson?.schema_version === 'x07.xtal.verify_summary@0.1.0') return stdoutJson;
	const artifactPreview = asPlainRecord(report.artifact_preview);
	const previewJson = asPlainRecord(artifactPreview?.json);
	return previewJson?.schema_version === 'x07.xtal.verify_summary@0.1.0' ? previewJson : null;
}

function verifyEvidenceBoardFromReport(
	report: Record<string, unknown>,
	template: ProjectTemplate
): VerifyEvidenceBoard {
	const settings = asPlainRecord(report.settings);
	const results = asPlainRecord(report.results);
	const prechecks = asPlainRecord(results?.prechecks);
	const verification = asPlainRecord(results?.verification);
	const counts = asPlainRecord(verification?.counts);
	const tests = asPlainRecord(results?.tests);
	const diagnostics = asPlainRecord(results?.diagnostics);
	const artifacts = asPlainRecord(report.artifacts);
	const verifyDir = stringValue(artifacts?.verify_dir, 'target/xtal/verify');
	const artifactReports = Array.isArray(artifacts?.reports) ? artifacts.reports : [];
	const entries = Array.isArray(report.entries) ? report.entries : [];
	const bounds = asPlainRecord(settings?.verify_bounds);
	const proofBudget = asPlainRecord(settings?.proof_budget);
	const topCodes = Array.isArray(diagnostics?.top_codes)
		? diagnostics.top_codes
				.map((item) => asPlainRecord(item))
				.filter((item): item is Record<string, unknown> => Boolean(item))
				.map((item) => `${stringValue(item.code, 'diagnostic')} x${stringValue(item.count, '1')}`)
		: [];
	const artifactsList = artifactReports
		.map((item) => reportRef(item))
		.filter((item): item is VerifyEvidenceArtifact => Boolean(item));
	const testReport = reportRef(tests?.report);
	if (testReport && !artifactsList.some((item) => item.path === testReport.path)) {
		artifactsList.unshift(testReport);
	}

	return {
		source: 'report',
		outcome: outcomeState(results?.outcome),
		world: stringValue(settings?.world, 'solve-pure'),
		proofPolicy: stringValue(settings?.proof_policy, 'balanced'),
		bounds: verifyBoundsLabel(bounds, proofBudget),
		prechecks: [
			{ label: 'Spec', state: outcomeState(prechecks?.spec) },
			{ label: 'Generation', state: outcomeState(prechecks?.generation) },
			{ label: 'Implementation', state: outcomeState(prechecks?.impl) }
		],
		coverageOutcome: outcomeState(verification?.coverage_outcome),
		proveOutcome: outcomeState(verification?.prove_outcome),
		tests: {
			outcome: outcomeState(tests?.outcome),
			passed: stringValue(tests?.passed, '0'),
			failed: stringValue(tests?.failed, '0'),
			skipped: stringValue(tests?.skipped, '0'),
			report: testReport?.path ?? 'target/xtal/tests.report.json'
		},
		diagnostics: {
			outcome: outcomeState(diagnostics?.outcome),
			errors: stringValue(diagnostics?.error_count, '0'),
			warnings: stringValue(diagnostics?.warning_count, '0'),
			topCodes,
			report: reportRef(diagnostics?.report)?.path ?? 'target/xtal/xtal.verify.diag.json'
		},
		counts: verifyCounts(counts),
		entries: entries.length
			? entries
					.map((entry) => verifyEntryFromReport(entry))
					.filter((entry): entry is VerifyEvidenceEntry => Boolean(entry))
			: [pendingVerifyEntry(template, 'report has no entry rows')],
		artifacts: artifactsList,
		generatedTestManifest: 'gen/xtal/tests.json',
		verifyDir
	};
}

function verifyEvidenceBoardFromOperation(
	op: OpRecord | null,
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate,
	options?: Partial<VerifyRunOptions>
): VerifyEvidenceBoard {
	const normalized = normalizeVerifyRunOptions(options);
	const target = session?.intent?.targets[0];
	const entry = target?.entry ? `${target.module_id}.${target.entry}` : template.title;
	const status = op ? opStatusToVerifyState(op.status) : 'pending';
	const artifactPaths = op?.artifacts.length ? op.artifacts : ['target/xtal/verify/summary.json'];
	return {
		source: op ? 'operation' : 'pending',
		outcome: status,
		world: normalized.allowOsWorld ? 'OS-capable world allowed' : 'solve-* required',
		proofPolicy: normalized.proofPolicy,
		bounds: verifyBoundsLabel(
			{
				unwind: normalized.unwind,
				max_bytes_len: normalized.maxBytesLen,
				input_len_bytes: normalized.inputLenBytes
			},
			null
		),
		prechecks: [
			{ label: 'Spec', state: status },
			{ label: 'Generation', state: status },
			{ label: 'Implementation', state: status }
		],
		coverageOutcome: status,
		proveOutcome: status,
		tests: {
			outcome: status,
			passed: status === 'pass' ? 'demo' : '0',
			failed: status === 'fail' ? '1' : '0',
			skipped: '0',
			report: 'target/xtal/tests.report.json'
		},
		diagnostics: {
			outcome: status,
			errors: status === 'fail' ? '1' : '0',
			warnings: '0',
			topCodes: [],
			report: 'target/xtal/xtal.verify.diag.json'
		},
		counts: [
			{ label: 'entries', value: target ? '1' : '0' },
			{ label: 'proof status', value: status }
		],
		entries: [
			{
				entry,
				opId: target?.module_id ?? 'pending',
				specPath: template.artifacts[0] ?? 'spec/*.x07spec.json',
				coverage: status,
				prove: status,
				proveRaw: op ? 'report pending' : 'not run',
				coverageReport: 'target/xtal/verify/coverage/',
				proveReport: 'target/xtal/verify/prove/',
				proofObject: '',
				diagnostic: op ? 'operation has no parsed verify summary yet' : 'run xtal.verify to populate entry evidence'
			}
		],
		artifacts: artifactPaths.map((path) => ({
			label: path.endsWith('summary.json') ? 'Verify summary' : 'Verify artifact',
			kind: 'operation_artifact',
			path,
			schemaVersion: ''
		})),
		generatedTestManifest: 'gen/xtal/tests.json',
		verifyDir: 'target/xtal/verify'
	};
}

function verifyEntryFromReport(value: unknown): VerifyEvidenceEntry | null {
	const entry = asPlainRecord(value);
	if (!entry) return null;
	const coverage = asPlainRecord(entry.coverage);
	const prove = asPlainRecord(entry.prove);
	const coverageReport = reportRef(coverage?.report);
	const proveReport = reportRef(prove?.report);
	const proofObject = reportRef(prove?.proof_object);
	const diagnostic = asPlainRecord(prove?.first_diagnostic);
	return {
		entry: stringValue(entry.entry, 'entry'),
		opId: stringValue(entry.op_id, 'operation'),
		specPath: stringValue(entry.spec_path, 'spec/*.x07spec.json'),
		coverage: outcomeState(coverage?.outcome),
		prove: outcomeState(prove?.policy_outcome),
		proveRaw: stringValue(prove?.raw, 'not reported'),
		coverageReport: coverageReport?.path ?? '',
		proveReport: proveReport?.path ?? '',
		proofObject: proofObject?.path ?? '',
		diagnostic: diagnostic
			? `${stringValue(diagnostic.code, 'diagnostic')}: ${stringValue(diagnostic.message, '')}`
			: ''
	};
}

function reportRef(value: unknown): VerifyEvidenceArtifact | null {
	const record = asPlainRecord(value);
	if (!record) return null;
	return {
		label: reportKindLabel(stringValue(record.kind, 'report')),
		kind: stringValue(record.kind, 'report'),
		path: stringValue(record.path, ''),
		schemaVersion: stringValue(record.schema_version, '')
	};
}

function reportKindLabel(kind: string): string {
	return kind
		.split('_')
		.filter(Boolean)
		.map((part) => part[0]?.toUpperCase() + part.slice(1))
		.join(' ');
}

function verifyCounts(counts: Record<string, unknown> | null): Array<{ label: string; value: string }> {
	if (!counts) return [];
	return [
		['entries', 'entries_total'],
		['coverage fail', 'coverage_fail'],
		['proven', 'prove_proven'],
		['counterexample', 'prove_counterexample'],
		['inconclusive', 'prove_inconclusive'],
		['unsupported', 'prove_unsupported'],
		['timeout', 'prove_timeout'],
		['tool missing', 'prove_tool_missing']
	].map(([label, key]) => ({ label, value: stringValue(counts[key], '0') }));
}

function verifyBoundsLabel(
	bounds: Record<string, unknown> | null,
	proofBudget: Record<string, unknown> | null
): string {
	const items = [
		['unwind', bounds?.unwind],
		['max bytes', bounds?.max_bytes_len],
		['input bytes', bounds?.input_len_bytes],
		['z3 timeout', proofBudget?.z3_timeout_seconds],
		['z3 memory', proofBudget?.z3_memory_mb]
	]
		.filter(([, value]) => value !== undefined && value !== null && value !== '')
		.map(([label, value]) => `${label}: ${String(value)}`);
	return items.length ? items.join(' / ') : 'default bounded verification';
}

function pendingVerifyEntry(template: ProjectTemplate, diagnostic: string): VerifyEvidenceEntry {
	return {
		entry: template.title,
		opId: template.id,
		specPath: template.artifacts[0] ?? 'spec/*.x07spec.json',
		coverage: 'pending',
		prove: 'pending',
		proveRaw: 'not run',
		coverageReport: 'target/xtal/verify/coverage/',
		proveReport: 'target/xtal/verify/prove/',
		proofObject: '',
		diagnostic
	};
}

function outcomeState(value: unknown): VerifyEvidenceState {
	if (value === 'pass') return 'pass';
	if (value === 'warn') return 'warn';
	if (value === 'fail') return 'fail';
	if (value === 'skip') return 'skip';
	return 'pending';
}

function opStatusToVerifyState(status: OperationStatus): VerifyEvidenceState {
	if (status === 'succeeded') return 'pass';
	if (status === 'failed') return 'fail';
	return 'pending';
}

function stringValue(value: unknown, fallback: string): string {
	if (value === undefined || value === null || value === '') return fallback;
	return String(value);
}

export function buildProofCacheLedger(
	session: SessionSnapshot | null | undefined,
	template: ProjectTemplate,
	radar: WorkspaceRadarResponse | null | undefined,
	options?: Partial<VerifyRunOptions>
): ProofCacheItem[] {
	const ops = session?.op_log ?? [];
	const approved = Boolean(session?.contract) || (session ? phaseIndex(session.phase) >= phaseIndex('spec_approved') : false);
	const specOp = latestMatchingOp(ops, ['spec.check', 'spec.extract', 'spec.scaffold']);
	const implOp = latestMatchingOp(ops, ['impl.sync.write', 'impl.check', 'wasm.app.build.atlas_dev']);
	const verifyOp = latestMatchingOp(ops, [
		'xtal.verify',
		'gen.verify',
		'test.manifest',
		'wasm.app.verify.atlas_release',
		'wasm.app.test.'
	]);
	const certOp = latestMatchingOp(ops, [
		'xtal.certify',
		'wasm.provenance.verify',
		'wasm.provenance.attest',
		'lp.deploy.status.local'
	]);
	const target = session?.intent?.targets[0];
	const moduleId = target?.module_id ?? template.sourcePath.split('/').at(-1) ?? 'x07.project';
	const entry = target?.entry ?? 'main';
	const proofPolicy = proofPolicyForTemplate(template);
	const verifyOptions = normalizeVerifyRunOptions(options);
	const proofKeyParts = [
		proofPolicy.value,
		verifyOptions.proofPolicy,
		verifyOptions.allowOsWorld ? 'allow-os-world' : 'solve-world',
		verifyOptions.unwind ? `unwind-${verifyOptions.unwind}` : 'default-unwind',
		verifyOptions.maxBytesLen ? `bytes-${verifyOptions.maxBytesLen}` : 'default-bytes',
		verifyOptions.inputLenBytes ? `input-${verifyOptions.inputLenBytes}` : 'default-input'
	];
	const cacheKey = [
		'xtal-proof',
		moduleId,
		entry,
		approved ? 'contract-locked' : 'draft',
		...proofKeyParts
	].join(':');
	const verifyArtifact =
		radar?.latest_verify?.path ??
		verifyOp?.artifacts.find((artifact) => artifact.includes('verify') || artifact.includes('test')) ??
		template.artifacts.find((artifact) => artifact.includes('verify') || artifact.includes('test')) ??
		'target/xtal/verify/summary.json';
	const certArtifact =
		radar?.latest_certify?.path ??
		certOp?.artifacts.find((artifact) => artifact.includes('cert') || artifact.includes('provenance') || artifact.includes('deploy')) ??
		'target/xtal/cert/bundle.json';
	return [
		{
			label: 'Cache key preview',
			value: cacheKey,
			artifact: '.x07/studio/proof-cache/<key>.json',
			detail: 'Deterministic preview only; compiler-backed proof cache is not persisted yet.',
			state: approved ? 'ready' : 'blocked'
		},
		{
			label: 'Spec fingerprint',
			value: specOp?.op ?? (approved ? 'contract locked' : 'awaiting approval'),
			artifact: session?.contract?.task_doctrine.intent_ref ?? 'spec/*.x07spec.json',
			detail: 'Spec and examples must be stable before reusing proof evidence.',
			state: specOp || approved ? 'ready' : 'blocked',
			opId: specOp?.id
		},
		{
			label: 'Implementation hash',
			value: implOp?.op ?? (approved ? 'pending sync' : 'blocked'),
			artifact: 'target/xtal/impl-sync.patchset.json',
			detail: 'Implementation realization is part of the future proof-cache key.',
			state: implOp ? opStatusToProofCacheState(implOp.status) : approved ? 'pending' : 'blocked',
			opId: implOp?.id
		},
		{
			label: 'Proof policy',
			value: `${proofPolicy.value} / ${verifyOptions.proofPolicy}`,
			artifact: proofPolicy.artifact,
			detail: `${proofPolicy.detail} ${verifyOptions.allowOsWorld ? 'OS-capable worlds are explicitly allowed.' : 'Deterministic solve-* worlds remain required.'}`,
			state: approved ? 'ready' : 'blocked'
		},
		{
			label: 'Verify artifact',
			value: verifyOp?.op ?? (radar?.latest_verify ? 'workspace artifact' : 'not run'),
			artifact: verifyArtifact,
			detail: 'Coverage, proof, generated tests, app traces, or SLO results feed cache eligibility.',
			state: verifyOp
				? opStatusToProofCacheState(verifyOp.status)
				: radar?.latest_verify
					? 'ready'
					: approved
						? 'pending'
						: 'blocked',
			opId: verifyOp?.id
		},
		{
			label: 'Certification dependency',
			value: certOp?.op ?? (radar?.latest_certify ? 'workspace artifact' : 'not certified'),
			artifact: certArtifact,
			detail: 'Trust/cert evidence decides whether cached proof can support release-shaped work.',
			state: certOp
				? opStatusToProofCacheState(certOp.status)
				: radar?.latest_certify
					? 'ready'
					: approved
						? 'pending'
						: 'blocked',
			opId: certOp?.id
		}
	];
}

function proofPolicyForTemplate(template: ProjectTemplate): { value: string; artifact: string; detail: string } {
	const haystack = `${template.riskProfile} ${template.prompt} ${template.canonicalCommands.join(' ')}`.toLowerCase();
	if (haystack.includes('slo') || haystack.includes('budget')) {
		return {
			value: 'budgeted proof',
			artifact: 'arch/budgets/',
			detail: 'Budget/SLO evidence must be part of proof reuse decisions.'
		};
	}
	if (haystack.includes('wasm') || haystack.includes('provenance') || haystack.includes('release')) {
		return {
			value: 'release proof',
			artifact: 'dist/**/provenance',
			detail: 'Release/provenance proof evidence must stay tied to the pack artifact.'
		};
	}
	return {
		value: 'solve-pure proof',
		artifact: 'target/xtal/verify/',
		detail: 'Default cache policy is deterministic solve-pure verification.'
	};
}

function opStatusToProofCacheState(status: OperationStatus): ProofCacheState {
	if (status === 'succeeded') return 'ready';
	if (status === 'failed') return 'blocked';
	return 'pending';
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
	const op = latestOpForOps(ops, bindingIds);
	if (op) return opStatusToPlanState(op.status);
	return approved ? 'ready' : 'blocked';
}

function latestOpState(ops: OpRecord[], bindingIds: string[]): AutomationPlanState | null {
	const matched = latestOpForOps(ops, bindingIds);
	if (!matched) return null;
	return opStatusToPlanState(matched.status);
}

function latestOpForOps(ops: OpRecord[], bindingIds: string[]): OpRecord | null {
	return [...ops].reverse().find((op) => bindingIds.includes(op.op)) ?? null;
}

function stateForCommand(
	ops: OpRecord[],
	command: string,
	approved: boolean
): AutomationPlanState {
	const matched = latestOpForCommand(ops, command);
	if (matched) return opStatusToPlanState(matched.status);
	return approved ? 'ready' : 'blocked';
}

function latestOpForCommand(ops: OpRecord[], command: string): OpRecord | null {
	const normalizedCommand = normalizeCommand(command);
	return [...ops]
		.reverse()
		.find((op) => normalizedCommand.includes(normalizeCommand(op.op)) || normalizeCommand(op.command.join(' ')).includes(normalizedCommand.split(' ').slice(0, 5).join(' '))) ?? null;
}

function projectScaffoldBindingId(template: ProjectTemplate): string {
	if (template.sourcePath.includes('agent-gate/xtal/workflow-graph')) return 'project.seed.workflow-graph';
	if (template.sourcePath.includes('readiness-checks/x07-sm-arch-contracts-smoke')) return 'project.seed.state-machine-arch';
	if (template.sourcePath.includes('apps/x07-api-gateway')) return 'project.seed.x07-api-gateway';
	if (template.sourcePath.includes('apps/x07crawl')) return 'project.seed.x07crawl';
	if (template.sourcePath.includes('apps/x07dbguard')) return 'project.seed.x07dbguard';
	if (template.sourcePath.includes('wasm_showcases/x07_atlas')) return 'project.seed.x07_atlas';
	return 'project.init.xtal-pure';
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

export function previewSpecSource(raw: string): SpecSourcePreview {
	const normalized = raw.trim();
	if (!normalized) {
		return {
			state: 'empty',
			moduleId: 'awaiting spec',
			entry: 'operation',
			detail: 'Paste an x07 spec JSON object with module_id and at least one operation.'
		};
	}
	try {
		const { moduleId, operationName } = readSpecSourceTarget(normalized);
		if (!moduleId || !operationName) {
			return {
				state: 'invalid',
				moduleId: moduleId || 'missing module_id',
				entry: operationName ? entryFromSpecOperation(moduleId, operationName) : 'missing operation',
				detail: 'Existing Spec mode needs module_id and one operation name or id.'
			};
		}
		return {
			state: 'ready',
			moduleId,
			entry: entryFromSpecOperation(moduleId, operationName),
			detail: 'Spec source will be treated as already-authored behavior for human review.'
		};
	} catch {
		return {
			state: 'invalid',
			moduleId: 'invalid JSON',
			entry: 'operation',
			detail: 'Spec source is not valid JSON yet.'
		};
	}
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
		const { moduleId, operationName } = readSpecSourceTarget(raw);
		if (!moduleId || !operationName) return null;
		return {
			moduleId,
			entry: entryFromSpecOperation(moduleId, operationName)
		};
	} catch {
		return null;
	}
}

function readSpecSourceTarget(raw: string): { moduleId: string; operationName: string } {
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
	return { moduleId, operationName };
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
		revision_notes: [],
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
	artifacts?: string[],
	reportJson?: unknown
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
			report_json: reportJson,
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
			label: 'Intent review',
			state: session.intent
				? phaseIndex(session.phase) >= phaseIndex('spec_approved')
					? 'done'
					: 'active'
				: 'blocked'
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
