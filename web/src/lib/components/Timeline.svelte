<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { plainOpLabel } from '$lib/plainEnglish';
	import type {
		IntentWitnessKind,
		OpRecord,
		PbtRound,
		QuickfixRecord,
		SessionSnapshot,
		SessionTurn,
		TrustPosture,
		TryItRequest,
		TryItResult
	} from '$lib/studio';
	import ClarifyQuestionCard from './ClarifyQuestionCard.svelte';
	import ResultPreview from './ResultPreview.svelte';
	import AgentStreamCard from './AgentStreamCard.svelte';
	import RealizePreview from './RealizePreview.svelte';
	import QuorumRealize from './QuorumRealize.svelte';
	import ReviewRound from './ReviewRound.svelte';
	import QuickfixCard from './QuickfixCard.svelte';
	import McpCallCard from './McpCallCard.svelte';
	import PostureBadge from './PostureBadge.svelte';
	import CompareMenu from './CompareMenu.svelte';

	export let turns: SessionTurn[] = [];
	export let session: SessionSnapshot | null = null;
	export let detailsOpen = false;
	export let tryResult: TryItResult | null = null;
	export let busy = false;
	export let realizeBusy = false;
	export let invokeBusy = false;
	export let quickfix: QuickfixRecord | null = null;
	export let trustPosture: TrustPosture | null = null;
	export let pbtRound: PbtRound | null = null;

	const dispatch = createEventDispatcher<{
		answer: { questionId: string; text: string; witnessKind: IntentWitnessKind };
		followup: string;
		repair: string;
		invoke: TryItRequest;
		realize: void;
		quorum: void;
		pickProposal: number;
		proof: string;
		lint: void;
		pbt: void;
		pbtRegression: string;
		quickfix: string;
		compare: string;
	}>();

	$: opsById = new Map((session?.op_log ?? []).map((op) => [op.id, op]));

	function turnOps(ids: string[]): OpRecord[] {
		return ids.map((id) => opsById.get(id)).filter((op): op is OpRecord => Boolean(op));
	}

	function implementationDone() {
		return (session?.op_log ?? []).some(
			(op) =>
				(op.op === 'synthesis.template' || op.op.startsWith('agent.realize.')) &&
				op.status === 'succeeded'
		);
	}

	$: streamEvents = turns
		.filter((turn): turn is Extract<SessionTurn, { kind: 'agent_stream' }> => turn.kind === 'agent_stream')
		.map((turn) => turn.event);
	$: latestPosture =
		trustPosture ??
		[...turns].reverse().find((turn): turn is Extract<SessionTurn, { kind: 'trust_posture_changed' }> => turn.kind === 'trust_posture_changed')?.posture ??
		null;
	$: visibleTurns = turns.filter(
		(turn): turn is Exclude<SessionTurn, { kind: 'trust_posture_changed' }> =>
			turn.kind !== 'trust_posture_changed'
	);
</script>

<section class="timeline" aria-label="Session timeline" data-testid="timeline">
	<PostureBadge posture={latestPosture} />

	{#if turns.length === 0}
		<div class="empty-turn" data-testid="timeline-empty">
			<h2>Start a session</h2>
			<p>The timeline will show the plan, questions, build, verification, and runtime follow-ups.</p>
		</div>
	{/if}

	{#each visibleTurns as turn (turn.id)}
		<article class="turn {turn.kind}" data-testid={`turn-${turn.kind}`}>
			<div class="turn-marker"></div>
			<div class="turn-body">
				{#if turn.kind === 'user_intent'}
					<header>
						<span>{turn.source_kind}</span>
						<time>{turn.at}</time>
					</header>
					<h2>Intent</h2>
					<p>{turn.raw}</p>
				{:else if turn.kind === 'agent_clarify'}
					<header>
						<span>{turn.agent_id}</span>
						<time>{turn.at}</time>
					</header>
					<h2>Questions</h2>
					<div class="question-stack">
						{#each turn.questions as question}
							<ClarifyQuestionCard
								turn={{
									question_id: question.id,
									question_text: question.text,
									witness_kind: question.witness_kind,
									round: 0,
									agent_id: turn.agent_id,
									options: question.options,
									question_recorded_at: turn.at,
									answer_text: question.answer,
									answer_recorded_at: null
								}}
								on:answer={(event) => dispatch('answer', event.detail)}
							/>
						{/each}
					</div>
				{:else if turn.kind === 'user_answer'}
					<header>
						<span>Answer</span>
						<time>{turn.at}</time>
					</header>
					<p>{turn.text}</p>
				{:else if turn.kind === 'agent_draft'}
					<header>
						<span>{turn.agent_id}</span>
						<time>{turn.at}</time>
					</header>
					<h2>Agent draft</h2>
					<p>{turn.summary}</p>
					{#if turn.evidence.length}
						<ul class="evidence-list">
							{#each turn.evidence as item}
								<li>{item.label}</li>
							{/each}
						</ul>
					{/if}
				{:else if turn.kind === 'user_approved'}
					<header>
						<span>Approved</span>
						<time>{turn.at}</time>
					</header>
					<p>{turn.by}</p>
				{:else if turn.kind === 'build_stage'}
					<header>
						<span>Build</span>
						<time>{turn.at}</time>
					</header>
					<h2>{turn.stage.replaceAll('_', ' ')}</h2>
					<div class="stage-strip">
						{#each turnOps(turn.op_ids) as op}
							<span class={op.status}>{plainOpLabel(op)}</span>
						{/each}
					</div>
				{:else if turn.kind === 'verified'}
					<header>
						<span>Verified</span>
						<time>{turn.at}</time>
					</header>
					{#if turn.refined_from_scaffold}
						<p class="hint">Refined from scaffold.</p>
					{/if}
					<ResultPreview
						summary={turn.summary}
						{tryResult}
						{busy}
						{realizeBusy}
						{invokeBusy}
						implementationInPlace={implementationDone()}
						{pbtRound}
						examples={session?.intent?.examples ?? []}
						on:followup={(event) => dispatch('followup', event.detail)}
						on:invoke={(event) => dispatch('invoke', event.detail)}
						on:realize={() => dispatch('realize')}
						on:quorum={() => dispatch('quorum')}
						on:proof={(event) => dispatch('proof', event.detail)}
						on:pbt={() => dispatch('pbt')}
						on:pbtRegression={(event) => dispatch('pbtRegression', event.detail)}
					/>
					<button class="command-button" type="button" disabled={busy} on:click={() => dispatch('quorum')}>
						Second opinion
					</button>
				{:else if turn.kind === 'review'}
					<header>
						<span>Review</span>
						<time>{turn.at}</time>
					</header>
					<ReviewRound round={turn.round} />
				{:else if turn.kind === 'incident'}
					<header>
						<span>Incident</span>
						<time>{turn.at}</time>
					</header>
					<h2>{turn.incident_id}</h2>
					<p>{turn.summary}</p>
					{#if turn.repair_available}
						<button class="command-button primary" type="button" on:click={() => dispatch('repair', turn.incident_id)}>
							Repair this
						</button>
					{/if}
					<QuickfixCard
						incidentId={turn.incident_id}
						record={quickfix?.citations.some((citation) => citation.file.includes(turn.incident_id)) ? quickfix : null}
						{busy}
						on:load={(event) => dispatch('quickfix', event.detail)}
						on:apply={(event) => dispatch('repair', event.detail)}
					/>
				{:else if turn.kind === 'repair'}
					<header>
						<span>Repair</span>
						<time>{turn.at}</time>
					</header>
					<h2>{turn.incident_id}</h2>
					<div class="stage-strip">
						{#each turnOps(turn.op_ids) as op}
							<span class={op.status}>{plainOpLabel(op)}</span>
						{/each}
					</div>
				{:else if turn.kind === 'agent_realize'}
					<header>
						<span>{turn.agent_id}</span>
						<time>{turn.at}</time>
					</header>
					<h2>{turn.ok ? `${turn.agent_id} filled in the implementation` : `${turn.agent_id} ran but reported issues`}</h2>
					{#if turn.wrote_files.length}
						<p class="hint">Wrote / edited:</p>
						<ul class="evidence-list">
							{#each turn.wrote_files as path}
								<li><code>{path}</code></li>
							{/each}
						</ul>
					{:else}
						<p class="hint">No file changes recorded by the write audit.</p>
						{#if !turn.ok}
							<div class="button-row" aria-label="Implementation recovery actions">
								<button class="command-button primary" type="button" disabled={busy} on:click={() => dispatch('realize')}>
									{busy ? 'Claude Code is implementing...' : 'Try Claude Code again'}
								</button>
								<button class="command-button" type="button" disabled={busy} on:click={() => dispatch('quorum')}>
									Second opinion
								</button>
							</div>
						{/if}
					{/if}
					<RealizePreview events={streamEvents.filter((event) => 'agent_id' in event && event.agent_id === turn.agent_id)} />
					{#if turn.ok || turn.wrote_files.length}
						<button class="command-button" type="button" disabled={busy} on:click={() => dispatch('quorum')}>
							Second opinion
						</button>
					{/if}
				{:else if turn.kind === 'agent_stream'}
					<header>
						<span>{turn.agent_id}</span>
						<time>{turn.at}</time>
					</header>
					<AgentStreamCard event={turn.event} />
				{:else if turn.kind === 'mcp_call'}
					<header>
						<span>MCP</span>
						<time>{turn.at}</time>
					</header>
					<McpCallCard event={turn.call} />
				{:else if turn.kind === 'quorum_realize'}
					<header>
						<span>Realize quorum</span>
						<time>{turn.at}</time>
					</header>
					<QuorumRealize round={turn.round} {busy} on:pick={(event) => dispatch('pickProposal', event.detail)} />
				{:else if turn.kind === 'lint'}
					<header>
						<span>Lint</span>
						<time>{turn.at}</time>
					</header>
					<h2>x07 lint</h2>
					<p>{Object.entries(turn.count_by_severity).map(([key, value]) => `${value} ${key}`).join(', ') || 'no diagnostics'}</p>
					{#if turn.diagnostic_ids.length}
						<ul class="evidence-list">
							{#each turn.diagnostic_ids.slice(0, 5) as diagnosticId}
								<li><code>{diagnosticId}</code></li>
							{/each}
						</ul>
					{/if}
					<button class="command-button" type="button" on:click={() => dispatch('lint')}>Open lint report</button>
				{/if}

				<div class="turn-actions">
					<CompareMenu
						turnId={turn.id}
						compareTurn={(turnId) => dispatch('compare', turnId)}
						on:compare={(event) => dispatch('compare', event.detail)}
					/>
				</div>

				{#if detailsOpen}
					<details open class="turn-evidence">
						<summary>Show evidence</summary>
						<pre>{JSON.stringify(turn, null, 2)}</pre>
					</details>
				{:else}
					<details class="turn-evidence">
						<summary>Show evidence</summary>
						<pre>{JSON.stringify(turn, null, 2)}</pre>
					</details>
				{/if}
			</div>
		</article>
	{/each}
</section>
