<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { plainOpLabel } from '$lib/plainEnglish';
	import type {
		IntentWitnessKind,
		OpRecord,
		QuickfixRecord,
		SessionSnapshot,
		SessionTurn,
		TryItRequest,
		TryItResult
	} from '$lib/studio';
	import ClarifyQuestionCard from './ClarifyQuestionCard.svelte';
	import ResultPreview from './ResultPreview.svelte';
	import AgentStreamCard from './AgentStreamCard.svelte';
	import RealizePreview from './RealizePreview.svelte';
	import QuorumRealize from './QuorumRealize.svelte';
	import QuickfixCard from './QuickfixCard.svelte';
	import McpCallCard from './McpCallCard.svelte';

	export let turns: SessionTurn[] = [];
	export let session: SessionSnapshot | null = null;
	export let detailsOpen = false;
	export let tryResult: TryItResult | null = null;
	export let busy = false;
	export let realizeBusy = false;
	export let invokeBusy = false;
	export let quickfix: QuickfixRecord | null = null;

	const dispatch = createEventDispatcher<{
		answer: { questionId: string; text: string; witnessKind: IntentWitnessKind };
		followup: string;
		repair: string;
		invoke: TryItRequest;
		realize: void;
		quorum: void;
		pickProposal: number;
		proof: string;
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
</script>

<section class="timeline" aria-label="Session timeline" data-testid="timeline">
	{#if turns.length === 0}
		<div class="empty-turn" data-testid="timeline-empty">
			<h2>Start a session</h2>
			<p>The timeline will show the plan, questions, build, verification, and runtime follow-ups.</p>
		</div>
	{/if}

	{#each turns as turn (turn.id)}
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
					<ResultPreview
						summary={turn.summary}
						{tryResult}
						{busy}
						{realizeBusy}
						{invokeBusy}
						implementationInPlace={implementationDone()}
						on:followup={(event) => dispatch('followup', event.detail)}
						on:invoke={(event) => dispatch('invoke', event.detail)}
						on:realize={() => dispatch('realize')}
						on:quorum={() => dispatch('quorum')}
						on:proof={(event) => dispatch('proof', event.detail)}
					/>
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
									Compare both agents
								</button>
							</div>
						{/if}
					{/if}
					<RealizePreview events={streamEvents.filter((event) => 'agent_id' in event && event.agent_id === turn.agent_id)} />
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
				{:else if turn.kind === 'trust_posture_changed'}
					<header>
						<span>Trust posture</span>
						<time>{turn.at}</time>
					</header>
					<p>{turn.posture.worlds.join(', ')} · {Math.round(turn.posture.proof_coverage.proved_pct)}% proof coverage</p>
				{/if}

				<div class="turn-actions">
					<button type="button" class="link-button" on:click={() => dispatch('compare', turn.id)}>
						Compare to…
					</button>
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
