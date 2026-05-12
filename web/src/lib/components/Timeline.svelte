<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { plainOpLabel } from '$lib/plainEnglish';
	import type { IntentWitnessKind, OpRecord, SessionSnapshot, SessionTurn } from '$lib/studio';
	import ClarifyQuestionCard from './ClarifyQuestionCard.svelte';
	import ResultPreview from './ResultPreview.svelte';

	export let turns: SessionTurn[] = [];
	export let session: SessionSnapshot | null = null;
	export let detailsOpen = false;

	const dispatch = createEventDispatcher<{
		answer: { questionId: string; text: string; witnessKind: IntentWitnessKind };
		followup: string;
		repair: string;
	}>();

	$: opsById = new Map((session?.op_log ?? []).map((op) => [op.id, op]));

	function turnOps(ids: string[]): OpRecord[] {
		return ids.map((id) => opsById.get(id)).filter((op): op is OpRecord => Boolean(op));
	}
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
					<ResultPreview summary={turn.summary} on:followup={(event) => dispatch('followup', event.detail)} />
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
				{/if}

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
