<script lang="ts">
	import { onMount } from 'svelte';
	import './+page.css';
	import { StudioApi } from '$lib/api';
	import {
		agentLanes,
		defaultPrompt,
		lifecycle,
		nextPrimaryAction,
		phaseIndex,
		projectTemplates,
		providerCards,
		rooms,
		workflowChecklist,
		type AgentProfile,
		type BindingDescriptor,
		type HealthResponse,
		type IntentInputMode,
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

	$: selected = sessions.find((session) => session.session_id === selectedId) ?? sessions[0];
	$: if (selected && selected.session_id !== selectedSessionForRoom) {
		selectedRoom = selected.room;
		selectedSessionForRoom = selected.session_id;
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
	$: pendingApprovals = worklog.filter(
		(op) => op.op.startsWith('agent.approval.') && op.status === 'pending'
	);
	$: checklist = selected ? workflowChecklist(selected) : [];
	$: canApproveSpec = selected?.phase === 'intent_ready' || selected?.phase === 'spec_draft';
	$: canRunProject =
		selected?.phase === 'spec_approved' || selected?.phase === 'realization_proposed';

	onMount(async () => {
		await refresh();
	});

	async function refresh() {
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
		statusLine = `Focused ${label} room`;
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
			const intent = api.formalizeLocal(selected, promptText, inputMode, revisionHistory);
			const snapshot = await api.dispatch(selected, 'formalize_intent', intent);
			await replaceSession(snapshot);
			approvalState = 'awaiting';
			statusLine = `${visibleAgent} polished the plan into an intent packet`;
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
			const intent = api.formalizeLocal(current, promptText, inputMode, revisionHistory);
			current = await api.dispatch(current, 'formalize_intent', intent);
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
					aria-selected={selectedRoom === room.id}
					on:click={() => focusRoom(room.id)}
				>
					<span>{room.label}</span>
				</button>
			{/each}
		</div>
		<div class="rail-status">
			<span class:online={!api.isDemoMode}></span>
			{api.isDemoMode ? 'Demo mode' : 'Loom online'}
		</div>
	</aside>

	<section class="workspace">
		<header class="topbar">
			<div>
				<p class="eyebrow">Canonical x07 Flow</p>
				<h1>x07 Studio</h1>
			</div>
			<div class="top-actions">
				<select bind:value={visibleAgent} aria-label="Active coding agent">
					<option>Codex</option>
					<option>Claude Code</option>
				</select>
				<button class="command-button" type="button" on:click={refresh}>Refresh</button>
			</div>
		</header>

		<section class="project-form panel" aria-label="Project intake">
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
			<div class="session-stack" aria-label="Created projects">
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

		<section class="radar">
			<div>
				<span>Workspace</span>
				<strong>{health.workspace_root}</strong>
			</div>
			<div>
				<span>Session</span>
				<strong>{selected?.title ?? 'None'}</strong>
			</div>
			<div>
				<span>Phase</span>
				<strong>{selected?.phase.replaceAll('_', ' ') ?? 'loading'}</strong>
			</div>
			<div>
				<span>Approval</span>
				<strong>{approvalState === 'awaiting' ? 'Awaiting Approval' : approvalState}</strong>
			</div>
			<div>
				<span>Room Focus</span>
				<strong>{selectedRoom}</strong>
			</div>
		</section>

		<section class="lifecycle" aria-label="XTAL lifecycle">
			{#each lifecycle as step, index}
				<div class:done={index < progress} class:current={index === progress} class="life-step">
					<span>{index + 1}</span>
					<strong>{step.label}</strong>
					<small>{step.binding ?? step.room}</small>
				</div>
			{/each}
		</section>

		<section class="main-grid">
			<section id="room-intent" class="intent-panel panel" class:focused={selectedRoom === 'intent'}>
				<div class="panel-head">
					<div>
						<p class="eyebrow">Intent Room</p>
						<h2>Initial plan to approved spec</h2>
					</div>
					<span class="badge">{primaryAction}</span>
				</div>
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
				<div class="revision-lane">
					<label for="revision">Revision</label>
					<input id="revision" bind:value={revisionText} />
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
				</div>
				<div class="lineage">
					<div class="node intent">Intent</div>
					<div class="edge"></div>
					<div class="node spec">Spec</div>
					<div class="edge"></div>
					<div class="node impl">Impl</div>
					<div class="edge"></div>
					<div class="node verify">Verify</div>
					<div class="edge"></div>
					<div class="node trust">Trust</div>
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
						<p class="eyebrow">Agent Visible Worklog</p>
						<h2>Codex and Claude Code lanes</h2>
					</div>
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
							<span>{agent.command} · {agent.status.replaceAll('_', ' ')}</span>
							<small>{agent.allowed_verbs.join(' -> ')}</small>
							<em>{agent.approval_required ? 'Approval gated' : 'Autonomous'} · {agent.write_roots.join(', ')}</em>
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
				<div class="worklog">
					{#each worklog.length ? worklog : [{ op: 'intent.formalize', status: 'pending', command: ['awaiting', 'approval'], artifacts: [] }] as op}
						<div>
							<span class:failed={op.status === 'failed'} class:succeeded={op.status === 'succeeded'}></span>
							<code>{op.op}</code>
							<small>{op.command.join(' ')}</small>
						</div>
					{/each}
				</div>
			</section>

			<section id="room-trust" class="evidence-panel panel" class:focused={selectedRoom === 'trust'}>
				<div class="panel-head">
					<div>
						<p class="eyebrow">Trust Border</p>
						<h2>World, budget, policy</h2>
					</div>
				</div>
				<div class="evidence-list">
					<div><span>World</span><strong>solve-* default</strong></div>
					<div><span>Spec writes</span><strong>{selected?.contract?.project_doctrine.write_policy.agent_write_specs === false ? 'human gated' : 'drafting'}</strong></div>
					<div><span>Arch writes</span><strong>{selected?.contract?.project_doctrine.write_policy.agent_write_arch === false ? 'human gated' : 'drafting'}</strong></div>
					<div><span>Bindings</span><strong>{bindings.length}</strong></div>
				</div>
				<div class="binding-strip">
					{#each bindings.slice(0, 8) as binding}
						<button type="button" class="segmented-button" on:click={() => runBinding(binding.id)}>
							{binding.id}
						</button>
					{/each}
				</div>
			</section>
		</section>

		<footer class="statusbar">
			<span>{statusLine}</span>
			<span>{busy ? 'Running' : 'Idle'}</span>
		</footer>
	</section>
</main>
