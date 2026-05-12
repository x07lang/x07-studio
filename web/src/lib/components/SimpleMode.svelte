<script lang="ts">
	import { createEventDispatcher, onDestroy, onMount } from 'svelte';
	import { StudioApi } from '$lib/api';
	import type {
		AgentProfile,
		ClarificationTurn,
		IntentAnswer,
		IntentWitnessKind,
		PlainEnglishSummary,
		SessionSnapshot
	} from '$lib/studio';
	import SimpleStart from './SimpleStart.svelte';
	import SimpleBuildProgress from './SimpleBuildProgress.svelte';
	import SimpleResultPreview from './SimpleResultPreview.svelte';
	import ClarifyQuestionCard from './ClarifyQuestionCard.svelte';

	export let api: StudioApi;
	export let session: SessionSnapshot | null = null;

	type Stage = 'start' | 'clarify' | 'building' | 'done' | 'needs_help';
	let stage: Stage = session ? deriveStage(session) : 'start';
	let prompt = '';
	let busy = false;
	let recording = false;
	let voiceSupported = false;
	let speechController: { stop: () => void } | null = null;
	let agentId = 'claude-code';
	let agents: AgentProfile[] = [];
	let pendingAnswers: Record<string, string> = {};
	let unsubscribeStream: (() => void) | null = null;
	let lastSummary: PlainEnglishSummary | null = null;

	const dispatch = createEventDispatcher<{
		'open-expert': { sessionId: string | null };
		'session-updated': { session: SessionSnapshot };
	}>();

	$: voiceSupported = typeof window !== 'undefined' && (
		'SpeechRecognition' in window || 'webkitSpeechRecognition' in window
	);

	$: clarificationTurns = (session?.intent?.clarification_history ?? []) as ClarificationTurn[];
	$: latestRound = clarificationTurns.reduce(
		(max, turn) => (turn.round > max ? turn.round : max),
		0
	);
	$: currentRoundTurns = clarificationTurns.filter((turn) => turn.round === latestRound);
	$: pendingTurns = clarificationTurns.filter((turn) => !turn.answer_text);
	$: lastSummary = extractPlainEnglishSummary(session);

	onMount(async () => {
		try {
			agents = await api.listAgents();
			const enabled = agents.find(
				(profile) => profile.id === 'claude-code' || profile.id === 'openai-codex'
			);
			if (enabled) agentId = enabled.id;
		} catch {
			// keep default agent id
		}
		if (session) subscribeToStream(session.session_id);
	});

	onDestroy(() => {
		unsubscribeStream?.();
		speechController?.stop();
	});

	function deriveStage(snapshot: SessionSnapshot): Stage {
		if (snapshot.phase === 'trust_review'
			|| snapshot.phase === 'certify_running'
			|| snapshot.phase === 'certified') {
			return 'done';
		}
		if (snapshot.phase === 'human_intervention_required') return 'needs_help';
		const lastBuildStage = [...snapshot.op_log]
			.reverse()
			.find((op) => op.op.startsWith('build.stage.'));
		if (lastBuildStage) {
			const id = lastBuildStage.op.replace('build.stage.', '');
			if (id === 'done') return 'done';
			if (id === 'needs_help') return 'needs_help';
			return 'building';
		}
		if (snapshot.intent) return 'clarify';
		return 'start';
	}

	function extractPlainEnglishSummary(
		snapshot: SessionSnapshot | null
	): PlainEnglishSummary | null {
		if (!snapshot) return null;
		const op = [...snapshot.op_log].reverse().find((entry) => entry.op === 'summary.plain_english');
		if (!op) return null;
		const payload = op.report_json as PlainEnglishSummary | undefined;
		if (!payload || payload.schema_version !== 'x07.studio.plain_english_summary@0.1.0') {
			return null;
		}
		return payload;
	}

	function subscribeToStream(sessionId: string) {
		unsubscribeStream?.();
		unsubscribeStream = api.subscribeSession(sessionId, (event) => {
			if (event.kind === 'snapshot') {
				replaceSession(event.session);
			} else if (event.kind === 'op') {
				if (!session) return;
				const merged = mergeOp(session, event.op);
				replaceSession(merged);
			}
		});
	}

	function mergeOp(snapshot: SessionSnapshot, op: SessionSnapshot['op_log'][number]): SessionSnapshot {
		const existing = snapshot.op_log.findIndex((entry) => entry.id === op.id);
		const op_log = [...snapshot.op_log];
		if (existing >= 0) op_log[existing] = op;
		else op_log.push(op);
		return { ...snapshot, op_log };
	}

	function replaceSession(next: SessionSnapshot) {
		session = next;
		stage = deriveStage(next);
		dispatch('session-updated', { session: next });
	}

	async function onBegin(detail: { prompt: string }) {
		busy = true;
		try {
			const created = await api.createSession(detail.prompt.slice(0, 80), 'new_behavior');
			const formalized = await api.formalizeIntent(created, detail.prompt, 'text', []);
			replaceSession(formalized.session);
			subscribeToStream(formalized.session.session_id);
			stage = 'clarify';
			await tryClarify();
		} finally {
			busy = false;
		}
	}

	async function tryClarify() {
		if (!session) return;
		try {
			const response = await api.clarifyIntent(session, agentId, { timeoutSeconds: 90 });
			if (response) replaceSession(response.session);
		} catch (error) {
			console.warn('clarify failed', error);
		}
	}

	async function onAnswer(detail: { questionId: string; text: string; witnessKind: IntentWitnessKind }) {
		if (!session) return;
		pendingAnswers = { ...pendingAnswers, [detail.questionId]: detail.text };
		const answer: IntentAnswer = {
			question_id: detail.questionId,
			text: detail.text,
			witness_kind: detail.witnessKind
		};
		busy = true;
		try {
			const response = await api.answerIntent(session, [answer]);
			if (response) replaceSession(response.session);
		} finally {
			busy = false;
		}
	}

	async function onAskMore() {
		if (!session) return;
		busy = true;
		try {
			await tryClarify();
		} finally {
			busy = false;
		}
	}

	async function onBuild() {
		if (!session) return;
		busy = true;
		stage = 'building';
		try {
			if (session.phase === 'intent_ready') {
				// move IntentReady -> SpecDraft -> SpecApproved so build can run
				await api.dispatch(session, 'draft_spec');
				const approved = await api.dispatch(
					(await api.getSession(session.session_id)),
					'approve_spec'
				);
				replaceSession(approved);
			}
			const next = await api.runBuildPipeline(session, { maxRepairRounds: 3 });
			replaceSession(next);
		} finally {
			busy = false;
		}
	}

	type RecognitionLike = {
		continuous: boolean;
		interimResults: boolean;
		lang: string;
		onresult: ((event: { resultIndex: number; results: ArrayLike<ArrayLike<{ transcript: string }>> }) => void) | null;
		onend: (() => void) | null;
		start: () => void;
		stop: () => void;
	};

	function startVoice() {
		const winLike = window as unknown as {
			SpeechRecognition?: new () => RecognitionLike;
			webkitSpeechRecognition?: new () => RecognitionLike;
		};
		const ctor = winLike.SpeechRecognition ?? winLike.webkitSpeechRecognition;
		if (!ctor) return;
		const recognition = new ctor();
		recognition.continuous = true;
		recognition.interimResults = true;
		recognition.lang = 'en-US';
		recognition.onresult = (event) => {
			let transcript = '';
			for (let i = event.resultIndex; i < event.results.length; i += 1) {
				transcript += event.results[i][0].transcript;
			}
			prompt = `${prompt} ${transcript}`.trim();
		};
		recognition.onend = () => {
			recording = false;
		};
		recognition.start();
		recording = true;
		speechController = { stop: () => recognition.stop() };
	}

	function stopVoice() {
		speechController?.stop();
		speechController = null;
		recording = false;
	}

	function openExpert() {
		dispatch('open-expert', { sessionId: session?.session_id ?? null });
	}

	function restart() {
		unsubscribeStream?.();
		unsubscribeStream = null;
		session = null;
		stage = 'start';
		prompt = '';
		pendingAnswers = {};
	}
</script>

<div class="simple-mode" data-testid="simple-mode">
	{#if stage === 'start'}
		<SimpleStart
			bind:prompt
			{busy}
			{voiceSupported}
			{recording}
			on:begin={(event) => onBegin(event.detail)}
			on:start-voice={startVoice}
			on:stop-voice={stopVoice}
			on:open-expert={openExpert}
		/>
	{:else if stage === 'clarify' && session}
		<section class="clarify" data-testid="simple-clarify">
			<header>
				<h2>A couple of questions</h2>
				<p class="hint">
					I'll keep my questions short. Answer what fits, then say <em>Approve & Build</em>
					when I have enough.
				</p>
			</header>
			{#if currentRoundTurns.length}
				<div class="card-list">
					{#each currentRoundTurns as turn (turn.question_id)}
						<ClarifyQuestionCard
							{turn}
							disabled={busy}
							on:answer={(event) => onAnswer(event.detail)}
						/>
					{/each}
				</div>
				{#if pendingTurns.length === 0}
					<p class="hint" data-testid="clarify-all-answered">
						All answered. Approve &amp; Build whenever you're ready.
					</p>
				{/if}
			{:else}
				<p class="hint" data-testid="clarify-empty">
					No questions yet — I might just have enough to begin.
				</p>
			{/if}
			<div class="clarify-actions">
				<button
					type="button"
					class="link"
					on:click={onAskMore}
					disabled={busy}
					data-testid="simple-clarify-ask-more"
				>
					Ask another round
				</button>
				<button
					type="button"
					class="primary"
					on:click={onBuild}
					disabled={busy}
					data-testid="simple-clarify-build"
				>
					Approve &amp; Build
				</button>
				<button type="button" class="link" on:click={openExpert}>Open Expert mode</button>
			</div>
		</section>
	{:else if stage === 'building' && session}
		<SimpleBuildProgress {session} running={busy} />
	{:else if (stage === 'done' || stage === 'needs_help') && session}
		<SimpleBuildProgress {session} running={false} />
		<SimpleResultPreview
			{session}
			summary={lastSummary}
			on:open-expert={openExpert}
			on:restart={restart}
		/>
	{/if}
</div>

<style>
	.simple-mode {
		max-width: 760px;
		margin: 0 auto;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.clarify {
		background: var(--surface, #ffffff);
		border-radius: 0.75rem;
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
	}
	.clarify header h2 {
		margin: 0 0 0.25rem;
	}
	.hint {
		margin: 0;
		color: var(--muted, #555);
	}
	.card-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.clarify-actions {
		display: flex;
		gap: 0.75rem;
		align-items: center;
		flex-wrap: wrap;
	}
	.primary {
		background: var(--accent, #4a6cf7);
		color: white;
		font-weight: 600;
		font: inherit;
		padding: 0.55rem 1.1rem;
		border-radius: 0.45rem;
		border: 1px solid transparent;
		cursor: pointer;
	}
	.primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.link {
		background: transparent;
		border: none;
		color: var(--accent, #4a6cf7);
		cursor: pointer;
		font: inherit;
		text-decoration: underline;
		padding: 0;
	}
</style>
