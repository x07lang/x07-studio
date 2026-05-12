<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import './+page.css';
	import { StudioApi } from '$lib/api';
	import type {
		AskAnswer,
		HealthResponse,
		IntentAnswer,
		IntentInputMode,
		LadderState,
		SessionSnapshot,
		SessionStreamEvent,
		SessionTurn,
		StudioMemory,
		SyncCode,
		TryItRequest,
		TryItResult
	} from '$lib/studio';
	import { demoHealth } from '$lib/studio';
	import Timeline from '$lib/components/Timeline.svelte';
	import Composer from '$lib/components/Composer.svelte';
	import NowPanel from '$lib/components/NowPanel.svelte';

	const api = new StudioApi();

	let health: HealthResponse = demoHealth();
	let sessions: SessionSnapshot[] = [];
	let selected: SessionSnapshot | null = null;
	let turns: SessionTurn[] = [];
	let ladder: LadderState | null = null;
	let tryResult: TryItResult | null = null;
	let askAnswer: AskAnswer | null = null;
	let memory: StudioMemory | null = null;
	let syncCode: SyncCode | null = null;
	let busy = false;
	let status = 'Starting Studio';
	let detailsOpen = false;
	let unsubscribe: (() => void) | null = null;

	onMount(async () => {
		const params = new URLSearchParams(window.location.search);
		detailsOpen = params.get('mode') === 'expert' || params.get('details') === 'open';
		await refresh();
		if (selected) subscribe(selected.session_id);
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	async function refresh() {
		health = await api.health();
		sessions = await api.listSessions();
		selected = selected
			? sessions.find((session) => session.session_id === selected?.session_id) ?? sessions[0] ?? null
			: sessions[0] ?? null;
		memory = await api.loadMemory().catch(() => null);
		await refreshDerived();
		status = api.isDemoMode ? 'Demo projection active' : 'Connected to Loom daemon';
	}

	async function refreshDerived() {
		if (!selected) {
			turns = [];
			ladder = null;
			return;
		}
		turns = await api.listTurns(selected.session_id);
		ladder = await api.ladderState(selected).catch(() => null);
	}

	function subscribe(sessionId: string) {
		unsubscribe?.();
		unsubscribe = api.subscribeSession(sessionId, (event) => {
			handleStreamEvent(event);
		});
	}

	function handleStreamEvent(event: SessionStreamEvent) {
		if (!selected) return;
		if (event.kind === 'snapshot') {
			replaceSelected(event.session);
		} else if (event.kind === 'op') {
			const op_log = [...selected.op_log];
			const index = op_log.findIndex((op) => op.id === event.op.id);
			if (index >= 0) op_log[index] = event.op;
			else op_log.push(event.op);
			replaceSelected({ ...selected, op_log });
		}
		void refreshDerived();
	}

	function replaceSelected(session: SessionSnapshot) {
		selected = session;
		sessions = [session, ...sessions.filter((item) => item.session_id !== session.session_id)];
	}

	async function compose(text: string) {
		busy = true;
		try {
			if (text.startsWith('/binding ')) {
				await runBindingShortcut(text);
				return;
			}
			const session = await api.createSession(text.slice(0, 80), 'new_behavior');
			replaceSelected(session);
			subscribe(session.session_id);
			const formalized = await api.formalizeIntent(session, text, 'text' as IntentInputMode, []);
			replaceSelected(formalized.session);
			status = 'Intent captured';
			await api.clarifyIntent(formalized.session, 'claude-code', { timeoutSeconds: 90 }).catch(() => null);
			selected = await api.getSession(formalized.session.session_id);
			await refreshDerived();
		} finally {
			busy = false;
		}
	}

	async function runBindingShortcut(text: string) {
		if (!selected) return;
		const binding = text.replace('/binding ', '').trim();
		if (!binding) return;
		selected = await api.runBinding(selected, binding);
		status = `Ran ${binding}`;
		await refreshDerived();
	}

	async function answer(detail: { questionId: string; text: string; witnessKind: IntentAnswer['witness_kind'] }) {
		if (!selected) return;
		busy = true;
		try {
			const response = await api.answerIntent(selected, [
				{
					question_id: detail.questionId,
					text: detail.text,
					witness_kind: detail.witnessKind
				}
			] as IntentAnswer[]);
			if (response) replaceSelected(response.session);
			await refreshDerived();
		} finally {
			busy = false;
		}
	}

	async function followup(text: string) {
		if (!selected) return;
		const questionId = `q-followup-${Date.now()}`;
		await answer({ questionId, text, witnessKind: 'desired_behavior' });
		status = 'Follow-up added';
	}

	async function build() {
		if (!selected) return;
		busy = true;
		try {
			let current = selected;
			if (current.phase === 'intent_ready') {
				current = await api.dispatch(current, 'draft_spec');
				current = await api.dispatch(current, 'approve_spec');
			}
			current = await api.runBuildPipeline(current, { maxRepairRounds: 3 });
			replaceSelected(current);
			status = 'Built and verified';
			await refreshDerived();
		} finally {
			busy = false;
		}
	}

	async function invoke(req: TryItRequest) {
		if (!selected) return;
		busy = true;
		try {
			tryResult = await api.invoke(selected, req);
			status = 'Try-It run finished';
		} finally {
			busy = false;
		}
	}

	async function climb(rung: string) {
		if (!selected) return;
		busy = true;
		try {
			const next = await api.climbRung(selected, rung);
			replaceSelected(next);
			await refreshDerived();
			status = `Climbed to ${rung}`;
		} finally {
			busy = false;
		}
	}

	async function scanIncidents() {
		if (!selected) return;
		await api.scanIncidents(selected);
		selected = await api.getSession(selected.session_id);
		await refreshDerived();
		status = 'Incident scan complete';
	}

	async function repairIncident(incidentId: string) {
		if (!selected) return;
		busy = true;
		try {
			const next = await api.repairIncident(selected, incidentId);
			replaceSelected(next);
			await refreshDerived();
			status = `Repair queued for ${incidentId}`;
		} finally {
			busy = false;
		}
	}

	async function ask(question: string) {
		if (!selected) return;
		askAnswer = await api.askProject(selected, question);
		status = 'Project answer ready';
	}

	async function mintSync() {
		syncCode = await api.mintSyncCode();
		status = syncCode ? `Sync code ${syncCode.code}` : 'Sync unavailable in demo mode';
	}

	async function uploadImage(detail: { file: File }) {
		if (!selected) return;
		const form = new FormData();
		form.append('file', detail.file);
		form.append('mime', detail.file.type || 'application/octet-stream');
		await fetch(`/v1/sessions/${selected.session_id}/intent/image`, {
			method: 'POST',
			body: form
		}).catch(() => undefined);
		status = `Image witness added: ${detail.file.name}`;
	}
</script>

<svelte:head>
	<title>x07 Studio</title>
</svelte:head>

<main class="timeline-shell">
	<header class="app-header">
		<div>
			<h1>x07 Studio</h1>
			<p>{health.workspace_root}</p>
		</div>
		<div class="header-actions">
			<button class="command-button" type="button" on:click={() => (detailsOpen = !detailsOpen)} aria-pressed={detailsOpen}>
				Show details
			</button>
			<button class="command-button" type="button" on:click={refresh}>Refresh</button>
		</div>
	</header>

	<section class="session-radar" aria-label="Session radar">
		<div>
			<span>Status</span>
			<strong>{status}</strong>
		</div>
		<div>
			<span>Session</span>
			<strong>{selected?.title ?? 'No session'}</strong>
		</div>
		<div>
			<span>Memory</span>
			<strong>{memory?.preferences.default_agent ?? 'default'}</strong>
		</div>
		<div>
			<span>Sync</span>
			<strong>{syncCode?.code ?? 'not minted'}</strong>
		</div>
	</section>

	<div class="main-grid">
		<Timeline
			{turns}
			session={selected}
			{detailsOpen}
			on:answer={(event) => answer(event.detail)}
			on:followup={(event) => followup(event.detail)}
			on:repair={(event) => repairIncident(event.detail)}
		/>
		<NowPanel
			session={selected}
			{ladder}
			{tryResult}
			{askAnswer}
			{busy}
			on:build={build}
			on:invoke={(event) => invoke(event.detail)}
			on:climb={(event) => climb(event.detail)}
			on:scan={scanIncidents}
			on:ask={(event) => ask(event.detail)}
			on:sync={mintSync}
		/>
	</div>

	<Composer {busy} on:compose={(event) => compose(event.detail.text)} on:image={(event) => uploadImage(event.detail)} />
</main>
