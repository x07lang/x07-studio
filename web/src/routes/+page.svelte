<script lang="ts">
	import { onMount } from 'svelte';
	import './+page.css';
	import { StudioApi } from '$lib/api';
	import { buildPatchReview, buildReviewSignals, type ReviewSignal } from '$lib/review';
	import {
		agentLanes,
		canonicalDocRefs,
		canonicalMcpTools,
		defaultPrompt,
		lifecycle,
		nextPrimaryAction,
		phaseIndex,
		projectTemplates,
		providerCards,
		rooms,
		workflowChecklist,
		type AgentProfile,
		type ArtifactPreviewResponse,
		type BindingDescriptor,
		type HealthResponse,
		type IntentInputMode,
		type OpRecord,
		type ProjectDifficulty,
		type Room,
		type SessionSnapshot,
		type TaskType
	} from '$lib/studio';

	const api = new StudioApi();
	const initialProject = projectTemplates[0];

	let health: HealthResponse = { ok: true, workspace_root: '/workspace/x07-project' };
	let sessions: SessionSnapshot[] = [];
	let bindings: BindingDescriptor[] = [];
	let agentProfiles: AgentProfile[] = [];
	let selectedId = '';
	let selectedRoom: Room = 'intent';
	let selectedSessionForRoom = '';
	let projectTitle = initialProject.title;
	let projectTaskType: TaskType = initialProject.taskType;
	let projectDifficulty: ProjectDifficulty = initialProject.id;
	let promptText = initialProject.prompt || defaultPrompt;
	let inputMode: IntentInputMode = 'text';
	let revisionText = initialProject.revision;
	let revisionHistory: string[] = [];
	let statusLine = 'Starting Studio surface';
	let handoffStatus = 'No agent handoff generated';
	let busy = false;
	let approvalState: 'drafting' | 'awaiting' | 'changes' | 'approved' = 'drafting';
	let visibleAgent = 'Codex';
	let selectedBindingId = '';
	let selectedOpId = '';
	let worklogFilter: 'all' | 'codex' | 'claude' | 'xtal' = 'all';
	let autoScroll = true;
	let artifactPreviewKey = '';
	let artifactPreviewStatus = 'No artifact preview loaded';
	let selectedArtifactPreview: ArtifactPreviewResponse | null = null;

	const placeholderOp: OpRecord = {
		id: 'op-seed',
		op: 'intent.formalize',
		backend: 'demo',
		command: ['x07', 'flow', 'await-input'],
		started_at: '10:51:23',
		finished_at: null,
		status: 'pending',
		exit_code: null,
		artifacts: [],
		notes: 'visible agent operation record'
	};

	const roomStatus = {
		intent: {
			state: 'Draft',
			summary: 'Compile natural language into a reviewable intent packet.',
			owner: 'human'
		},
		spec: {
			state: 'Awaiting approval',
			summary: 'Convert the packet into specs, examples, and witness gates.',
			owner: 'human + agent'
		},
		realization: {
			state: 'Guarded',
			summary: 'Create implementation patchsets only after spec approval.',
			owner: 'agent'
		},
		verify: {
			state: 'Pending',
			summary: 'Run x07 checks, tests, proofs, and XTAL verification.',
			owner: 'x07'
		},
		repair: {
			state: 'Standby',
			summary: 'Classify failures before repair changes widen the contract.',
			owner: 'agent + human'
		},
		trust: {
			state: 'Not certified',
			summary: 'Review evidence before certification or runtime adoption.',
			owner: 'human'
		},
		ops: {
			state: 'Runtime ready',
			summary: 'Ingest incidents, run improvements, and preserve evidence.',
			owner: 'ops'
		},
		providers: {
			state: 'Approval gated',
			summary: 'Codex and Claude Code run through finite, visible verbs.',
			owner: 'human'
		},
		mcp: {
			state: 'Tooling',
			summary: 'x07 MCP tools expose search, exec, and context packs.',
			owner: 'x07'
		}
	} satisfies Record<Room, { state: string; summary: string; owner: string }>;

	const flowCommands = [
		'x07 flow init',
		'x07 intent submit',
		'x07 spec approve',
		'x07 realize run',
		'x07 verify run',
		'x07 repair run',
		'x07 trust certify',
		'x07 ops ingest',
		'x07 improve run'
	];

	$: selected = sessions.find((session) => session.session_id === selectedId) ?? sessions[0];
	$: allOps = selected?.op_log ?? [];
	$: if (selected && selected.session_id !== selectedSessionForRoom) {
		selectedRoom = selected.room;
		selectedSessionForRoom = selected.session_id;
		selectedOpId = '';
	}
	$: if (selectedOpId && !allOps.some((op) => op.id === selectedOpId)) {
		selectedOpId = '';
	}
	$: selectedProjectTemplate =
		projectTemplates.find((template) => template.id === projectDifficulty) ?? initialProject;
	$: progress = selected ? phaseIndex(selected.phase) : 0;
	$: primaryAction = selected ? nextPrimaryAction(selected.phase) : 'Create session';
	$: specOps =
		selected?.intent?.targets.map((target) => ({
			name: target.entry ?? 'main',
			module: target.module_id,
			status: selected.phase === 'intent_drafting' ? 'pending' : 'ready'
		})) ?? [];
	$: worklog = selected?.op_log.slice(-12).reverse() ?? [];
	$: visibleWorklog = worklog.filter((op) => {
		if (worklogFilter === 'all') return true;
		if (worklogFilter === 'codex') return op.op.includes('codex');
		if (worklogFilter === 'claude') return op.op.includes('claude-code');
		return isX07WorkflowOp(op.op);
	});
	$: selectedBindingId = selectedBindingId || bindings[0]?.id || '';
	$: pendingApprovals = worklog.filter(
		(op) => op.op.startsWith('agent.approval.') && op.status === 'pending'
	);
	$: selectedOp =
		(selectedOpId ? allOps.find((op) => op.id === selectedOpId) : undefined) ??
		allOps.at(-1) ??
		null;
	$: selectedPatchArtifact = patchsetArtifactForOp(selectedOp);
	$: selectedOpWithPreview = mergeArtifactPreview(selectedOp, selectedArtifactPreview);
	$: reviewSignals = buildReviewSignals(allOps);
	$: selectedPatchReview = buildPatchReview(selectedOpWithPreview);
	$: selectedOpDiagnostics = collectDiagnostics(selectedOp);
	$: selectedOpOutput = operationOutput(selectedOp);
	$: checklist = selected ? workflowChecklist(selected) : [];
	$: canApproveSpec = selected?.phase === 'intent_ready' || selected?.phase === 'spec_draft';
	$: canRunProject =
		Boolean(selected) &&
		(canApproveSpec ||
			selected?.phase === 'intent_drafting' ||
			selected?.phase === 'spec_approved' ||
			selected?.phase === 'realization_proposed');
	$: currentRoomStatus = roomStatus[selectedRoom];
	$: currentLifecycle = lifecycle[Math.min(progress, lifecycle.length - 1)] ?? lifecycle[0];
	$: consumedCredits = Math.min(42, Number((2.7 + progress * 3.6 + worklog.length * 0.9).toFixed(1)));
	$: budgetPercent = Math.min(98, Math.round((consumedCredits / 42) * 100));
	$: workspaceRoot = health.workspace_root || '/workspace/x07-project';
	$: workspaceName = workspaceLabel(workspaceRoot);
	$: readinessPercent = Math.round((progress / Math.max(lifecycle.length - 1, 1)) * 100);
	$: latestVerifyOp = latestOperation(allOps, ['xtal.verify', '.verify', 'test.']);
	$: latestCertifyOp = latestOperation(allOps, ['xtal.certify', 'certify', 'provenance.verify']);
	$: activeIncidentCount = sessions.filter((session) => session.task_type === 'incident_repair').length;
	$: availableAgentCount = agentProfiles.filter((profile) => profile.status === 'available').length;
	$: providerSummary = api.isDemoMode ? 'demo projection' : 'Loom daemon';
	$: radarMetrics = [
		{
			label: 'XTAL readiness',
			value: `${readinessPercent}%`,
			detail: currentLifecycle.label
		},
		{
			label: 'Active sessions',
			value: String(sessions.length),
			detail: selected?.phase.replaceAll('_', ' ') ?? 'none'
		},
		{
			label: 'Last verify',
			value: statusLabel(latestVerifyOp),
			detail: latestVerifyOp?.op ?? 'not run'
		},
		{
			label: 'Last certify',
			value: statusLabel(latestCertifyOp),
			detail: latestCertifyOp?.op ?? 'not run'
		},
		{
			label: 'Runtime incidents',
			value: String(activeIncidentCount),
			detail: activeIncidentCount ? 'repair lanes open' : 'none active'
		},
		{
			label: 'Agents',
			value: `${availableAgentCount}/${agentProfiles.length}`,
			detail: agentProfiles.length ? 'profiles configured' : 'loading profiles'
		}
	];
	$: worldState = [
		{ label: 'Services', value: selected ? '1 / 1' : '0 / 1', ok: Boolean(selected) },
		{ label: 'Specs', value: selected?.intent ? '2 / 2' : '0 / 2', ok: Boolean(selected?.intent) },
		{
			label: 'Policy',
			value: selected?.contract ? 'locked' : 'draft',
			ok: Boolean(selected?.contract)
		},
		{
			label: 'Evidence',
			value: selected && phaseIndex(selected.phase) >= phaseIndex('trust_review') ? 'ready' : 'pending',
			ok: Boolean(selected && phaseIndex(selected.phase) >= phaseIndex('trust_review'))
		}
	];
	$: operationRows = worklog.length ? worklog : [placeholderOp];
	$: doctrineDocRefs = selected?.contract?.global_doctrine.doc_refs.length
		? selected.contract.global_doctrine.doc_refs
		: canonicalDocRefs;
	$: doctrineMcpTools = selected?.contract?.global_doctrine.mcp_tools.length
		? selected.contract.global_doctrine.mcp_tools
		: canonicalMcpTools;
	$: doctrineAllowedVerbs = selected?.contract?.allowed_verbs.length
		? selected.contract.allowed_verbs
		: selected?.allowed_verbs ?? [];
	$: if (selected && selectedOp && selectedPatchArtifact) {
		void loadArtifactPreview(selected.session_id, selectedOp.id, selectedPatchArtifact);
	}

	onMount(async () => {
		await refresh();
	});

	async function refresh() {
		const activeSessionId = selected?.session_id || selectedId;
		if (activeSessionId) selectedSessionForRoom = activeSessionId;
		health = await api.health();
		sessions = await api.listSessions();
		bindings = await api.listBindings();
		agentProfiles = await api.listAgents();
		selectedId = selectedId || sessions[0]?.session_id || '';
		statusLine = api.isDemoMode ? 'Demo projection active' : 'Connected to Loom daemon';
	}

	async function createSession() {
		busy = true;
		try {
			const title = projectTitle.trim() || selectedProjectTemplate.title;
			if (!promptText.trim()) {
				promptText = selectedProjectTemplate.prompt;
			}
			const session = await api.createSession(`${selectedProjectTemplate.label}: ${title}`, projectTaskType);
			sessions = [session, ...sessions.filter((item) => item.session_id !== session.session_id)];
			selectedId = session.session_id;
			selectedRoom = 'intent';
			selectedSessionForRoom = session.session_id;
			approvalState = 'drafting';
			revisionHistory = [];
			statusLine = `Created ${selectedProjectTemplate.label.toLowerCase()} project: ${title}`;
		} finally {
			busy = false;
		}
	}

	function loadProjectBrief() {
		projectTitle = selectedProjectTemplate.title;
		projectTaskType = selectedProjectTemplate.taskType;
		promptText = selectedProjectTemplate.prompt;
		revisionText = selectedProjectTemplate.revision;
		revisionHistory = [];
		approvalState = 'drafting';
		statusLine = `${selectedProjectTemplate.label} project brief loaded`;
	}

	function focusRoom(room: Room) {
		const label = rooms.find((item) => item.id === room)?.label ?? room;
		selectedRoom = room;
		const focusedSessionId = selected?.session_id || selectedId;
		if (focusedSessionId) selectedSessionForRoom = focusedSessionId;
		statusLine = `Focused ${label} room`;
	}

	function primeRadarAction(action: 'intent' | 'brownfield' | 'incident') {
		selectedRoom = 'intent';
		const focusedSessionId = selected?.session_id || selectedId;
		if (focusedSessionId) selectedSessionForRoom = focusedSessionId;
		approvalState = 'drafting';
		revisionHistory = [];
		if (action === 'brownfield') {
			projectTitle = 'Brownfield XTAL extract';
			projectTaskType = 'brownfield_extract';
			inputMode = 'text';
			promptText =
				'Extract the current project behavior into XTAL specs, examples, write-scope doctrine, generated tests, and a human approval gate before any implementation changes.';
			revisionText = 'Keep implementation writes blocked until the extracted spec is approved.';
			statusLine = 'Brownfield extract intake prepared';
			return;
		}
		if (action === 'incident') {
			projectTitle = 'Incident improvement loop';
			projectTaskType = 'incident_repair';
			inputMode = 'incident';
			promptText =
				'Incident note: ingest the failure as read-only evidence, classify the violated spec clause, propose a spec-preserving repair first, and require human review before widening behavior or policy.';
			revisionText = 'Require regression evidence tied to the incident artifact.';
			statusLine = 'Incident improve intake prepared';
			return;
		}
		projectTitle = selectedProjectTemplate.title;
		projectTaskType = selectedProjectTemplate.taskType;
		inputMode = 'text';
		promptText = selectedProjectTemplate.prompt;
		revisionText = selectedProjectTemplate.revision;
		statusLine = 'Intent intake prepared';
	}

	function isX07WorkflowOp(op: string) {
		return [
			'project.',
			'spec.',
			'tests.',
			'test.',
			'gen.',
			'sm.',
			'arch.',
			'pkg.',
			'run.',
			'bundle.',
			'impl.',
			'xtal.'
		].some((prefix) => op.startsWith(prefix));
	}

	function latestOperation(ops: OpRecord[], needles: string[]) {
		return [...ops].reverse().find((op) => needles.some((needle) => op.op.includes(needle))) ?? null;
	}

	function statusLabel(op: OpRecord | null) {
		return op?.status ? op.status.replaceAll('_', ' ') : 'pending';
	}

	function workspaceLabel(root: string) {
		const parts = root.split('/').filter(Boolean);
		return parts.at(-1) ?? root;
	}

	function focusRoomFromKey(event: KeyboardEvent, room: Room) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			focusRoom(room);
		}
	}

	function selectSession(sessionId: string) {
		selectedId = sessionId;
		const session = sessions.find((candidate) => candidate.session_id === sessionId);
		if (session) {
			selectedRoom = session.room;
			selectedSessionForRoom = session.session_id;
			statusLine = `Selected ${session.title}`;
		}
	}

	function selectRoomFromControl(event: Event) {
		const target = event.currentTarget as HTMLSelectElement;
		focusRoom(target.value as Room);
	}

	function selectOperation(op: OpRecord) {
		selectedOpId = op.id;
		statusLine = `Inspecting ${op.op}`;
	}

	function selectReviewSignal(signal: ReviewSignal) {
		selectedOpId = signal.opId;
		if (signal.op.startsWith('xtal.verify')) selectedRoom = 'verify';
		if (signal.op.startsWith('xtal.certify')) selectedRoom = 'trust';
		if (signal.op.startsWith('impl.')) selectedRoom = 'realization';
		if (signal.op.startsWith('agent.')) selectedRoom = 'providers';
		statusLine = `Reviewing ${signal.label.toLowerCase()}: ${signal.op}`;
	}

	async function loadArtifactPreview(sessionId: string, opId: string, artifact: string) {
		const key = `${sessionId}|${opId}|${artifact}`;
		if (artifactPreviewKey === key) return;
		artifactPreviewKey = key;
		selectedArtifactPreview = null;
		artifactPreviewStatus = `Loading ${artifact}`;
		const session = sessions.find((item) => item.session_id === sessionId);
		if (!session) return;
		try {
			const preview = await api.previewArtifact(session, artifact);
			if (artifactPreviewKey !== key) return;
			selectedArtifactPreview = preview;
			artifactPreviewStatus = preview.truncated
				? `Preview truncated at ${preview.bytes_read} bytes`
				: `Preview loaded from ${preview.artifact}`;
		} catch (error) {
			if (artifactPreviewKey !== key) return;
			selectedArtifactPreview = null;
			artifactPreviewStatus = error instanceof Error ? error.message : 'Artifact preview failed';
		}
	}

	function patchsetArtifactForOp(op: OpRecord | null): string {
		return op?.artifacts.find((artifact) => artifact.includes('patchset') && artifact.endsWith('.json')) ?? '';
	}

	function mergeArtifactPreview(
		op: OpRecord | null,
		preview: ArtifactPreviewResponse | null
	): OpRecord | null {
		if (!op || !preview?.json || !op.artifacts.includes(preview.artifact)) return op;
		return {
			...op,
			report_json: {
				...(asRecord(op.report_json) ?? {}),
				artifact_preview: {
					artifact: preview.artifact,
					json: preview.json,
					patchset_preview: preview.patchset_preview
				}
			}
		};
	}

	function collectDiagnostics(op: OpRecord | null): Array<{ code: string; severity: string; message: string }> {
		const report = asRecord(op?.report_json);
		const topLevel = diagnosticsFrom(report?.diagnostics);
		if (topLevel.length) return topLevel;
		const result = asRecord(report?.result);
		const stdoutJson = asRecord(result?.stdout_json);
		return diagnosticsFrom(stdoutJson?.diagnostics);
	}

	function diagnosticsFrom(value: unknown): Array<{ code: string; severity: string; message: string }> {
		if (!Array.isArray(value)) return [];
		return value.flatMap((item) => {
			const record = asRecord(item);
			if (!record) return [];
			return [
				{
					code: String(record.code ?? 'diagnostic'),
					severity: String(record.severity ?? 'info'),
					message: String(record.message ?? '')
				}
			];
		});
	}

	function operationOutput(op: OpRecord | null): string {
		if (!op) return '';
		const direct = [op.stdout, op.stderr].filter(Boolean).join('\n').trim();
		if (direct) return direct.slice(0, 1200);
		const report = asRecord(op.report_json);
		const result = asRecord(report?.result);
		const stdout = asRecord(result?.stdout);
		const stderr = asRecord(result?.stderr);
		return [textValue(stdout?.text), textValue(stderr?.text)]
			.filter(Boolean)
			.join('\n')
			.trim()
			.slice(0, 1200);
	}

	function textValue(value: unknown): string {
		return typeof value === 'string' ? value : '';
	}

	function asRecord(value: unknown): Record<string, unknown> | null {
		return value && typeof value === 'object' && !Array.isArray(value)
			? (value as Record<string, unknown>)
			: null;
	}

	async function runSelectedBinding() {
		if (!selectedBindingId) return;
		await runBinding(selectedBindingId);
	}

	async function replaceSession(snapshot: SessionSnapshot) {
		sessions = sessions.map((session) =>
			session.session_id === snapshot.session_id ? snapshot : session
		);
		if (!sessions.some((session) => session.session_id === snapshot.session_id)) {
			sessions = [snapshot, ...sessions];
		}
		selectedId = snapshot.session_id;
		selectedRoom = snapshot.room;
	}

	async function refreshSession(sessionId: string) {
		const snapshot = await api.getSession(sessionId);
		await replaceSession(snapshot);
	}

	async function polishIntent() {
		if (!selected) return;
		busy = true;
		try {
			const response = await api.formalizeIntent(selected, promptText, inputMode, revisionHistory);
			await replaceSession(response.session);
			approvalState = 'awaiting';
			statusLine = `${visibleAgent} polished the plan into an auditable intent packet`;
		} finally {
			busy = false;
		}
	}

	async function requestChanges() {
		approvalState = 'changes';
		revisionHistory = [...revisionHistory, revisionText];
		promptText = `${promptText}\n\nRevision request: ${revisionText}`;
		statusLine = 'Revision routed back to intent review';
	}

	async function dispatch(event: string, label: string) {
		if (!selected) return;
		const snapshot = await api.dispatch(selected, event);
		await replaceSession(snapshot);
		statusLine = label;
	}

	async function runBinding(bindingId: string) {
		if (!selected) return;
		const snapshot = await api.runBinding(selected, bindingId);
		await replaceSession(snapshot);
		statusLine = `Ran ${bindingId}`;
	}

	async function generateAgentHandoff(agentId: string) {
		if (!selected) return;
		busy = true;
		try {
			const response = await api.createAgentHandoff(selected, agentId);
			await replaceSession(response.session);
			handoffStatus = `${response.handoff.agent_label} handoff saved to ${response.handoff.prompt_path}`;
			statusLine = handoffStatus;
		} finally {
			busy = false;
		}
	}

	async function runAgentHandoff(agentId: string, mode: 'plan' | 'execute') {
		if (!selected) return;
		const sessionId = selected.session_id;
		const agentLabel =
			agentProfiles.find((profile) => profile.id === agentId)?.label ?? 'Agent';
		busy = true;
		let poll: ReturnType<typeof setInterval> | undefined;
		try {
			const pending = api.runAgentHandoff(selected, agentId, mode);
			if (mode === 'execute') {
				handoffStatus = `${agentLabel} supervised command running`;
				statusLine = handoffStatus;
				poll = setInterval(() => {
					void refreshSession(sessionId);
				}, 500);
			}
			const response = await pending;
			await replaceSession(response.session);
			const label = response.handoff.agent_label;
			if (response.op.op.startsWith('agent.approval.') && response.op.status === 'pending') {
				handoffStatus = `${label} approval required before supervised command`;
			} else {
				handoffStatus =
					mode === 'execute'
						? `${label} supervised command ${response.op.status}`
						: `${label} supervised launch plan recorded`;
			}
			statusLine = handoffStatus;
		} finally {
			if (poll) clearInterval(poll);
			busy = false;
		}
	}

	async function resolveApproval(opId: string, decision: 'approve' | 'reject') {
		if (!selected) return;
		busy = true;
		try {
			const response = await api.resolveAgentApproval(
				selected,
				opId,
				decision,
				'Studio human checkpoint'
			);
			await replaceSession(response.session);
			handoffStatus =
				decision === 'approve' ? 'Agent checkpoint approved' : 'Agent checkpoint rejected';
			statusLine = handoffStatus;
		} finally {
			busy = false;
		}
	}

	async function approveAndRun() {
		if (!selected) return;
		busy = true;
		try {
			let current = selected;
			if (current.phase === 'intent_drafting' || current.phase === 'intent_ready' || current.phase === 'spec_draft') {
				current = await approveSpecSnapshot(current);
			}
			current = await api.runXtalWorkflow(current);
			const failed = current.op_log.at(-1)?.status === 'failed';
			await replaceSession(current);
			statusLine = failed
				? `${current.op_log.at(-1)?.op ?? 'XTAL workflow'} failed; repair review required`
				: 'Verify passed and trust review opened';
		} finally {
			busy = false;
		}
	}

	async function approveSpec() {
		if (!selected) return;
		busy = true;
		try {
			const snapshot = await approveSpecSnapshot(selected);
			await replaceSession(snapshot);
			approvalState = 'approved';
			statusLine = 'Spec approved; realization lane is unlocked';
		} finally {
			busy = false;
		}
	}

	async function approveSpecSnapshot(session: SessionSnapshot): Promise<SessionSnapshot> {
		let current = session;
		if (!current.intent) {
			const response = await api.formalizeIntent(current, promptText, inputMode, revisionHistory);
			current = response.session;
		}
		if (current.phase === 'intent_ready') {
			current = await api.dispatch(current, 'draft_spec');
		}
		if (current.phase === 'spec_draft') {
			current = await api.dispatch(current, 'approve_spec');
		}
		approvalState = 'approved';
		return current;
	}
</script>

<svelte:head>
	<title>x07 Studio</title>
</svelte:head>

<main class="studio-shell">
	<aside class="rail" aria-label="x07 Studio rooms">
		<div class="brand">
			<div class="brand-mark">x07</div>
			<div>
				<strong>Studio</strong>
				<span>XTAL surface</span>
			</div>
		</div>

		<div class="room-list" role="tablist" aria-label="Studio rooms">
			{#each rooms as room}
				<button
					class:active={selectedRoom === room.id}
					type="button"
					role="tab"
					data-room={room.id}
					aria-label={room.label}
					aria-selected={selectedRoom === room.id}
					on:pointerdown={() => focusRoom(room.id)}
					on:keydown={(event) => focusRoomFromKey(event, room.id)}
				>
					<span>{room.label}</span>
					<small>{roomStatus[room.id].state}</small>
				</button>
			{/each}
		</div>

		<section class="rail-card" aria-label="Created projects">
			<div class="rail-card-head">
				<strong>Rooms</strong>
				<span>{sessions.length}</span>
			</div>
			<div class="session-stack">
				{#each sessions.slice(0, 5) as session}
					<button
						type="button"
						class:active={selected?.session_id === session.session_id}
						on:click={() => selectSession(session.session_id)}
					>
						<strong>{session.title}</strong>
						<small>{session.phase.replaceAll('_', ' ')}</small>
					</button>
				{/each}
			</div>
		</section>

		<section class="rail-card room-status-card" aria-label="Room status">
			<div class="rail-card-head">
				<strong>Room Status</strong>
				<span>{currentRoomStatus.state}</span>
			</div>
			<div class="room-meter">
				<span>Progress</span>
				<strong>{Math.round((progress / Math.max(lifecycle.length - 1, 1)) * 100)}%</strong>
				<div><i style={`width: ${Math.round((progress / Math.max(lifecycle.length - 1, 1)) * 100)}%`}></i></div>
			</div>
			<dl>
				<div>
					<dt>Owner</dt>
					<dd>{currentRoomStatus.owner}</dd>
				</div>
				<div>
					<dt>Updated</dt>
					<dd>live</dd>
				</div>
			</dl>
		</section>

		<div class="rail-status">
			<span class:online={!api.isDemoMode}></span>
			{api.isDemoMode ? 'Demo mode' : 'Loom online'}
		</div>
	</aside>

	<section class="workspace">
		<header class="topbar">
			<div class="active-room">
				<label for="active-room">Active Room</label>
				<select id="active-room" value={selectedRoom} on:change={selectRoomFromControl} aria-label="Active room">
					{#each rooms as room}
						<option value={room.id}>{room.label}</option>
					{/each}
				</select>
			</div>
			<div class="flow-select">
				<label for="active-binding">Canonical x07 Flow</label>
				<select id="active-binding" bind:value={selectedBindingId} aria-label="Active binding">
					{#each bindings as binding}
						<option value={binding.id}>{binding.id}</option>
					{/each}
				</select>
			</div>
			<div class="flow-select">
				<label for="active-agent">Agent Lane</label>
				<select id="active-agent" bind:value={visibleAgent} aria-label="Active coding agent">
					<option>Codex</option>
					<option>Claude Code</option>
				</select>
			</div>
			<div class="top-actions">
				<button class="icon-button" type="button" aria-label="Refresh Studio" on:click={refresh}>
					R
				</button>
				<button class="command-button" type="button" on:click={runSelectedBinding} disabled={!selected || !selectedBindingId}>
					Run Binding
				</button>
			</div>
		</header>

		<section class="workspace-radar panel" aria-label="Workspace radar">
			<div class="radar-identity">
				<p class="eyebrow">Workspace Radar</p>
				<h1>{workspaceName}</h1>
				<code>{workspaceRoot}</code>
			</div>
			<div class="radar-grid">
				{#each radarMetrics as metric}
					<div class="radar-metric">
						<span>{metric.label}</span>
						<strong>{metric.value}</strong>
						<small>{metric.detail}</small>
					</div>
				{/each}
			</div>
			<div class="radar-actions" aria-label="Radar quick actions">
				<div>
					<span>Provider</span>
					<strong>{providerSummary}</strong>
				</div>
				<button class="command-button primary" type="button" on:click={() => primeRadarAction('intent')}>
					Intent
				</button>
				<button class="command-button" type="button" on:click={() => primeRadarAction('brownfield')}>
					Brownfield Extract
				</button>
				<button class="command-button warning" type="button" on:click={() => primeRadarAction('incident')}>
					Incident Improve
				</button>
			</div>
		</section>

		<section class="project-form panel" aria-label="Project intake">
			<div class="form-title">
				<p class="eyebrow">Intent Intake</p>
				<h1>x07 Studio</h1>
			</div>
			<div class="field">
				<label for="project-title">Project title</label>
				<input id="project-title" bind:value={projectTitle} />
			</div>
			<div class="field">
				<label for="project-task-type">Task type</label>
				<select id="project-task-type" bind:value={projectTaskType}>
					<option value="new_behavior">New behavior</option>
					<option value="bug_fix">Bug fix</option>
					<option value="behavior_change">Behavior change</option>
					<option value="incident_repair">Incident repair</option>
					<option value="explanation">Explanation</option>
					<option value="brownfield_extract">Brownfield extract</option>
				</select>
			</div>
			<div class="field">
				<label for="project-difficulty">Project difficulty</label>
				<select id="project-difficulty" bind:value={projectDifficulty}>
					{#each projectTemplates as template}
						<option value={template.id}>{template.label}</option>
					{/each}
				</select>
			</div>
			<div class="intake-actions">
				<button class="command-button" type="button" on:click={loadProjectBrief} disabled={busy}>
					Load Brief
				</button>
				<button class="command-button primary" type="button" on:click={createSession} disabled={busy}>
					New Session
				</button>
			</div>
			<div class="template-meta" aria-label="Example-backed XTAL template">
				<div>
					<span>Source example</span>
					<strong>{selectedProjectTemplate.sourcePath}</strong>
				</div>
				<div>
					<span>Risk profile</span>
					<strong>{selectedProjectTemplate.riskProfile}</strong>
				</div>
				<div class="template-commands">
					<span>Canonical loop</span>
					{#each selectedProjectTemplate.canonicalCommands.slice(0, 4) as command}
						<code>{command}</code>
					{/each}
				</div>
				<div class="template-commands">
					<span>Expected artifacts</span>
					{#each selectedProjectTemplate.artifacts as artifact}
						<code>{artifact}</code>
					{/each}
				</div>
			</div>
		</section>

		<section class="lifecycle" aria-label="XTAL lifecycle">
			{#each lifecycle as step, index}
				<div class:done={index < progress} class:current={index === progress} class="life-step">
					<span>{index + 1}</span>
					<div>
						<strong>{step.label}</strong>
						<small>{step.binding ?? step.room}</small>
					</div>
					<em>{index < progress ? 'Complete' : index === progress ? currentRoomStatus.state : 'Pending'}</em>
				</div>
			{/each}
		</section>

		<section class="main-grid">
			<section id="room-intent" class="intent-panel panel" class:focused={selectedRoom === 'intent'}>
				<div class="panel-head">
					<div>
						<p class="eyebrow">Intent</p>
						<h2>Initial plan to approved spec</h2>
					</div>
					<span class="badge">{primaryAction}</span>
				</div>
				<div class="intent-spec-grid">
					<div class="intent-editor">
						<div class="mode-row" aria-label="Intent input mode">
							<label class:active={inputMode === 'text'}>
								<input type="radio" bind:group={inputMode} value="text" />
								Written Plan
							</label>
							<label class:active={inputMode === 'voice'}>
								<input type="radio" bind:group={inputMode} value="voice" />
								Voice Transcript
							</label>
							<label class:active={inputMode === 'incident'}>
								<input type="radio" bind:group={inputMode} value="incident" />
								Incident Note
							</label>
						</div>
						<textarea bind:value={promptText} aria-label="Initial plan"></textarea>
						<div class="revision-lane">
							<label for="revision">Revision</label>
							<input id="revision" bind:value={revisionText} />
						</div>
					</div>
					<div class="spec-preview" aria-label="Spec approval preview">
						<div class="approval-banner">{approvalState === 'awaiting' ? 'Awaiting Approval' : currentLifecycle.label}</div>
						<dl>
							<div>
								<dt>Session</dt>
								<dd>{selected?.session_id ?? 'none'}</dd>
							</div>
							<div>
								<dt>Module</dt>
								<dd>{specOps[0]?.module ?? 'awaiting intent'}</dd>
							</div>
							<div>
								<dt>Entry</dt>
								<dd>{specOps[0]?.name ?? 'operation'}</dd>
							</div>
							<div>
								<dt>Scope</dt>
								<dd>{selected?.contract?.task_doctrine.focus_paths.join(', ') ?? 'spec, src, tests'}</dd>
							</div>
							<div>
								<dt>Acceptance</dt>
								<dd>Human-reviewed witnesses, tests, verify evidence.</dd>
							</div>
						</dl>
						<div class="button-row">
							<button class="command-button" type="button" on:click={polishIntent} disabled={busy || !selected}>
								Polish Intent
							</button>
							<button class="command-button warning" type="button" on:click={requestChanges} disabled={busy || !selected}>
								Request Changes
							</button>
							<button class="command-button" type="button" on:click={approveSpec} disabled={busy || !canApproveSpec}>
								Approve Spec
							</button>
							<button class="command-button primary" type="button" on:click={approveAndRun} disabled={busy || !canRunProject}>
								Approve and Run
							</button>
						</div>
					</div>
				</div>
				<div class="witnesses">
					{#each selected?.intent?.witnesses ?? [] as witness}
						<span>{witness.kind.replaceAll('_', ' ')}: {witness.text}</span>
					{/each}
				</div>
				<div class="approval-list" aria-label="Approval checklist">
					{#each checklist as item}
						<div class={item.state}>
							<span></span>
							<strong>{item.label}</strong>
							<small>{item.state}</small>
						</div>
					{/each}
				</div>
			</section>

			<section id="room-spec" class="graph-panel panel" class:focused={selectedRoom === 'spec'}>
				<div class="panel-head">
					<div>
						<p class="eyebrow">Lineage Graph</p>
						<h2>Intent to evidence</h2>
					</div>
					<div class="graph-tools" aria-label="Lineage tools">
						<button type="button" class="icon-button" aria-label="Zoom in">+</button>
						<button type="button" class="icon-button" aria-label="Zoom out">-</button>
						<button type="button" class="icon-button" aria-label="Fit graph">Fit</button>
					</div>
				</div>
				<div class="lineage-map" aria-label="XTAL lineage graph">
					<div class="map-node incident">INTENT<br /><small>{inputMode}</small></div>
					<div class="map-node spec">SPEC<br /><small>{approvalState}</small></div>
					<div class="map-node arch">ARCH<br /><small>{selected?.contract ? 'locked' : 'draft'}</small></div>
					<div class="map-node impl">REALIZE<br /><small>{canRunProject ? 'ready' : 'gated'}</small></div>
					<div class="map-node verify">VERIFY<br /><small>{currentLifecycle.binding ?? 'pending'}</small></div>
					<div class="map-node repair">REPAIR<br /><small>conditional</small></div>
					<div class="map-node trust">TRUST<br /><small>{selected?.phase === 'certified' ? 'certified' : 'review'}</small></div>
				</div>
				<div class="spec-grid">
					{#each specOps.length ? specOps : [{ name: 'operation', module: 'awaiting intent', status: 'pending' }] as op}
						<div>
							<strong>{op.name}</strong>
							<span>{op.module}</span>
							<small>{op.status}</small>
						</div>
					{/each}
				</div>
			</section>

			<section id="room-providers" class="agent-panel panel" class:focused={selectedRoom === 'providers'}>
				<div class="panel-head">
					<div>
						<p class="eyebrow">Agent Providers</p>
						<h2>Codex and Claude Code lanes</h2>
					</div>
					<span class="badge">{visibleAgent}</span>
				</div>
				<div class="providers">
					{#each providerCards as provider}
						<div class="provider">
							<strong>{provider.label}</strong>
							<span>{provider.model}</span>
							<small>{provider.bridge}</small>
						</div>
					{/each}
				</div>
				<div class="agent-profiles" aria-label="Configured coding agents">
					{#each agentProfiles as agent}
						<div>
							<strong>{agent.label}</strong>
							<span>{agent.command} / {agent.status.replaceAll('_', ' ')}</span>
							<small>{agent.allowed_verbs.join(' -> ')}</small>
							<em>{agent.approval_required ? 'Approval gated' : 'Autonomous'} / {agent.write_roots.join(', ')}</em>
							<div class="agent-actions">
								<button
									class="segmented-button"
									type="button"
									on:click={() => generateAgentHandoff(agent.id)}
									disabled={busy || !selected}
								>
									Generate {agent.label} Handoff
								</button>
								<button
									class="segmented-button"
									type="button"
									on:click={() => runAgentHandoff(agent.id, 'plan')}
									disabled={busy || !selected}
								>
									Plan {agent.label} Run
								</button>
								<button
									class="segmented-button"
									type="button"
									on:click={() => runAgentHandoff(agent.id, 'execute')}
									disabled={busy || !selected}
								>
									Run {agent.label} Command
								</button>
							</div>
						</div>
					{/each}
				</div>
				<p class="handoff-status">{handoffStatus}</p>
				{#if pendingApprovals.length}
					<div class="approval-queue" aria-label="Agent approval checkpoints">
						{#each pendingApprovals as approval}
							<div>
								<strong>{approval.op}</strong>
								<span>{approval.notes ?? 'Human approval required'}</span>
								<button
									class="segmented-button"
									type="button"
									on:click={() => resolveApproval(approval.id, 'approve')}
									disabled={busy}
								>
									Approve {approval.op}
								</button>
								<button
									class="segmented-button"
									type="button"
									on:click={() => resolveApproval(approval.id, 'reject')}
									disabled={busy}
								>
									Reject {approval.op}
								</button>
							</div>
						{/each}
					</div>
				{/if}
				<div class="agent-lanes">
					{#each agentLanes as lane}
						<div>
							<strong>{lane.label}</strong>
							<span>{lane.role}</span>
							<small>{lane.verbs.join(' -> ')}</small>
							<em>{lane.reviewGate}</em>
						</div>
					{/each}
				</div>
			</section>

			<section class="worklog-panel panel" aria-label="Agent Visible Worklog">
				<div class="panel-head">
					<div>
						<p class="eyebrow">Agent Visible Worklog</p>
						<h2>Every agent step is inspectable</h2>
					</div>
					<div class="worklog-controls">
						<select bind:value={worklogFilter} aria-label="Worklog filter">
							<option value="all">All Agents</option>
							<option value="codex">Codex</option>
							<option value="claude">Claude Code</option>
							<option value="xtal">XTAL</option>
						</select>
						<label class:active={autoScroll}>
							<input type="checkbox" bind:checked={autoScroll} />
							Auto-scroll
						</label>
					</div>
				</div>
				<div class="worklog">
					{#each visibleWorklog.length ? visibleWorklog : [placeholderOp] as op}
						<button
							type="button"
							class:active={selectedOp?.id === op.id}
							aria-label={`Inspect ${op.op}`}
							on:click={() => selectOperation(op)}
						>
							<span class:failed={op.status === 'failed'} class:succeeded={op.status === 'succeeded'}></span>
							<code>{op.op}</code>
							<small>{op.command.join(' ')}</small>
						</button>
					{/each}
				</div>
			</section>
		</section>

		<section class="operation-log panel" aria-label="Operation log">
			<div class="panel-head">
				<div>
					<p class="eyebrow">Operation Log</p>
					<h2>Canonical command stream</h2>
				</div>
				<span class="badge">{busy ? 'Streaming' : 'Idle'}</span>
			</div>
			<div class="terminal-log">
				{#each operationRows as op}
					<button
						type="button"
						class:active={selectedOp?.id === op.id}
						aria-label={`Inspect operation ${op.op}`}
						on:click={() => selectOperation(op)}
					>
						<time>{op.started_at}</time>
						<span class="terminal-op">{op.op}</span>
						<span>{op.status}</span>
						<small>{op.command.join(' ')}</small>
					</button>
				{/each}
			</div>
			{#if selectedOp}
				<div class="op-inspector" aria-label="Selected operation inspector">
					<div class="inspector-head">
						<div>
							<span>Selected operation</span>
							<strong>{selectedOp.op}</strong>
						</div>
						<em>{selectedOp.backend} / {selectedOp.status}</em>
					</div>
					<dl>
						<div>
							<dt>Exit</dt>
							<dd>{selectedOp.exit_code ?? 'pending'}</dd>
						</div>
						<div>
							<dt>Report</dt>
							<dd>{selectedOp.report_path ?? 'inline or unavailable'}</dd>
						</div>
						<div>
							<dt>Notes</dt>
							<dd>{selectedOp.notes ?? 'none'}</dd>
						</div>
					</dl>
					<div class="artifact-list" aria-label="Operation artifacts">
						<span>Artifacts</span>
						{#each selectedOp.artifacts.length ? selectedOp.artifacts : ['No artifacts recorded'] as artifact}
							<code>{artifact}</code>
						{/each}
					</div>
					{#if selectedPatchReview}
						<div class="patch-review" aria-label="Visual patch review">
							<div class="patch-review-head">
								<div>
									<span>Patch review</span>
									<strong>{selectedPatchReview.gate}</strong>
								</div>
								<em>{selectedPatchReview.status}</em>
							</div>
							<div class="patch-file-list">
								{#each selectedPatchReview.files as file}
									<div class={`patch-file ${file.risk}`}>
										<strong>{file.path}</strong>
										<span>{file.action}</span>
										<small>{file.note}</small>
										<em>{file.operations ? `${file.operations} ops` : file.source}</em>
										{#if file.applyError}
											<p class="patch-error">{file.applyError}</p>
										{/if}
										{#if file.before || file.after}
											<div class="patch-diff" aria-label={`Patch before and after ${file.path}`}>
												<div>
													<span>Before</span>
													<pre>{file.before ?? 'unavailable'}</pre>
												</div>
												<div>
													<span>After</span>
													<pre>{file.after ?? 'unavailable'}</pre>
												</div>
											</div>
										{/if}
									</div>
								{/each}
							</div>
							<div class="patch-command">
								<span>Command</span>
								<code>{selectedPatchReview.command}</code>
							</div>
							<div class="patch-command">
								<span>Artifact preview</span>
								<code>{artifactPreviewStatus}</code>
							</div>
						</div>
					{/if}
					<div class="diagnostic-list" aria-label="Operation diagnostics">
						<span>Diagnostics</span>
						{#if selectedOpDiagnostics.length}
							{#each selectedOpDiagnostics.slice(0, 4) as diagnostic}
								<div>
									<strong>{diagnostic.code}</strong>
									<em>{diagnostic.severity}</em>
									<p>{diagnostic.message}</p>
								</div>
							{/each}
						{:else}
							<p>No diagnostics recorded for this operation.</p>
						{/if}
					</div>
					{#if selectedOpOutput}
						<pre aria-label="Operation output">{selectedOpOutput}</pre>
					{/if}
				</div>
			{/if}
		</section>

		<footer class="statusbar">
			<span>{statusLine}</span>
			<span>{busy ? 'Running' : 'Idle'}</span>
		</footer>
	</section>

	<aside class="right-rail" aria-label="Trust and canonical flow">
		<section class="panel trust-panel" class:focused={selectedRoom === 'trust'}>
			<div class="panel-head">
				<div>
					<p class="eyebrow">Trust / Certify</p>
					<h2>{selected?.phase === 'certified' ? 'Certified' : 'Not certified'}</h2>
				</div>
			</div>
			<p>{currentRoomStatus.summary}</p>
			<button class="command-button primary wide" type="button" on:click={approveAndRun} disabled={busy || !canRunProject}>
				Run Verification
			</button>
		</section>

		<section class="panel review-panel" aria-label="Trust review signals">
			<div class="panel-head">
				<div>
					<p class="eyebrow">Review Queue</p>
					<h2>{reviewSignals.length ? `${reviewSignals.length} signals` : 'No signals'}</h2>
				</div>
			</div>
			<div class="review-signal-list">
				{#if reviewSignals.length}
					{#each reviewSignals as signal}
						<button
							type="button"
							class={`review-signal ${signal.tone}`}
							aria-label={`Review ${signal.label} ${signal.op}`}
							on:click={() => selectReviewSignal(signal)}
						>
							<span>{signal.label}</span>
							<strong>{signal.detail}</strong>
							{#if signal.artifact}
								<code>{signal.artifact}</code>
							{/if}
							<em>{signal.op}</em>
						</button>
					{/each}
				{:else}
					<div class="review-empty">No review signals recorded</div>
				{/if}
			</div>
		</section>

		<section class="panel evidence-panel" aria-label="World state evidence">
			<div class="panel-head">
				<div>
					<p class="eyebrow">World State</p>
					<h2>Evidence</h2>
				</div>
			</div>
			<div class="evidence-list">
				{#each worldState as item}
					<div class:ok={item.ok}>
						<span>{item.label}</span>
						<strong>{item.value}</strong>
					</div>
				{/each}
			</div>
		</section>

		<section class="panel doctrine-panel" aria-label="Session doctrine">
			<div class="panel-head">
				<div>
					<p class="eyebrow">Session Doctrine</p>
					<h2>{selected?.contract ? 'Locked' : 'Prepared'}</h2>
				</div>
			</div>
			<div class="doctrine-block">
				<span>Allowed verbs</span>
				{#each doctrineAllowedVerbs.slice(0, 5) as verb}
					<code>{verb}</code>
				{/each}
			</div>
			<div class="doctrine-block">
				<span>MCP tools</span>
				{#each doctrineMcpTools.slice(0, 5) as tool}
					<code>{tool}</code>
				{/each}
			</div>
			<div class="doctrine-block">
				<span>Canonical docs</span>
				{#each doctrineDocRefs.slice(0, 5) as docRef}
					<code>{docRef}</code>
				{/each}
			</div>
		</section>

		<section class="panel budget-panel" aria-label="Budget">
			<div class="panel-head">
				<div>
					<p class="eyebrow">Budget</p>
					<h2>42.0 credits</h2>
				</div>
			</div>
			<div class="budget-row">
				<span>Consumed</span>
				<strong>{consumedCredits.toFixed(1)}</strong>
			</div>
			<div class="budget-meter"><i style={`width: ${budgetPercent}%`}></i></div>
			<div class="budget-row">
				<span>Remaining</span>
				<strong>{(42 - consumedCredits).toFixed(1)}</strong>
			</div>
		</section>

		<section class="panel flow-panel" aria-label="Canonical x07 flow operations">
			<div class="panel-head">
				<div>
					<p class="eyebrow">Canonical x07 Flow</p>
					<h2>{currentLifecycle.label}</h2>
				</div>
			</div>
			<ol class="flow-list">
				{#each flowCommands as command, index}
					<li class:done={index < progress} class:current={index === progress}>
						<code>{command}</code>
						<span>{index < progress ? 'done' : index === progress ? 'active' : 'pending'}</span>
					</li>
				{/each}
			</ol>
			<div class="binding-strip">
				{#each bindings.slice(0, 8) as binding}
					<button type="button" class="segmented-button" on:click={() => runBinding(binding.id)}>
						{binding.id}
					</button>
				{/each}
			</div>
		</section>
	</aside>
</main>
