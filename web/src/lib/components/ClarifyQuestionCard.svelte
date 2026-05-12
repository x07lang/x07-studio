<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { ClarificationTurn, IntentWitnessKind } from '$lib/studio';

	export let turn: ClarificationTurn;
	export let disabled = false;
	let value = '';

	$: locked = Boolean(turn.answer_text);
	$: kindLabel = describeKind(turn.witness_kind);

	const dispatch = createEventDispatcher<{
		answer: { questionId: string; text: string; witnessKind: IntentWitnessKind };
	}>();

	function describeKind(kind: IntentWitnessKind): string {
		switch (kind) {
			case 'desired_behavior':
				return 'Behavior';
			case 'forbidden_behavior':
				return 'Forbidden';
			case 'policy_requirement':
				return 'Policy';
			case 'incident_report':
				return 'Incident';
			default:
				return kind;
		}
	}

	function chooseOption(option: string) {
		value = option;
	}

	function submit() {
		const trimmed = value.trim();
		if (!trimmed) return;
		dispatch('answer', {
			questionId: turn.question_id,
			text: trimmed,
			witnessKind: turn.witness_kind
		});
		value = '';
	}
</script>

<article class="clarify-card" data-testid="clarify-card" data-question-id={turn.question_id}>
	<header>
		<span class="kind" title={turn.witness_kind}>{kindLabel}</span>
		<span class="round">Round {turn.round}</span>
	</header>
	<p class="question">{turn.question_text}</p>

	{#if locked}
		<p class="answer-locked" data-testid="clarify-answer-locked">
			You said: <strong>{turn.answer_text}</strong>
		</p>
	{:else}
		{#if turn.options.length}
			<ul class="options" role="listbox" aria-label="Suggested answers">
				{#each turn.options as option}
					<li>
						<button
							type="button"
							class="option {value === option ? 'selected' : ''}"
							on:click={() => chooseOption(option)}
							{disabled}
							data-testid="clarify-option"
						>
							{option}
						</button>
					</li>
				{/each}
			</ul>
		{/if}
		<div class="answer">
			<input
				type="text"
				bind:value
				placeholder="Type your answer…"
				{disabled}
				data-testid="clarify-answer-input"
				on:keydown={(event) => {
					if (event.key === 'Enter') {
						event.preventDefault();
						submit();
					}
				}}
			/>
			<button
				type="button"
				class="submit"
				on:click={submit}
				disabled={disabled || !value.trim()}
				data-testid="clarify-answer-submit"
			>
				Answer
			</button>
		</div>
	{/if}
</article>

<style>
	.clarify-card {
		border: 1px solid var(--border, #d0d4dc);
		background: var(--surface, #ffffff);
		border-radius: 0.65rem;
		padding: 1rem 1.2rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: 0.8rem;
		color: var(--muted, #555);
	}
	.kind {
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.question {
		margin: 0;
		font-size: 1.05rem;
		font-weight: 500;
	}
	.options {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 0.45rem;
	}
	.option {
		font: inherit;
		font-size: 0.9rem;
		padding: 0.35rem 0.7rem;
		border-radius: 999px;
		border: 1px solid var(--border, #d0d4dc);
		background: var(--surface-2, #f1f3f8);
		cursor: pointer;
	}
	.option.selected {
		background: var(--accent-soft, #dde4ff);
		border-color: var(--accent, #4a6cf7);
		color: var(--accent-strong, #2944c8);
	}
	.option:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.answer {
		display: flex;
		gap: 0.5rem;
	}
	.answer input {
		flex: 1;
		font: inherit;
		padding: 0.5rem 0.7rem;
		border-radius: 0.4rem;
		border: 1px solid var(--border, #d0d4dc);
	}
	.submit {
		font: inherit;
		padding: 0.45rem 1rem;
		border-radius: 0.4rem;
		border: 1px solid transparent;
		background: var(--accent, #4a6cf7);
		color: white;
		font-weight: 600;
		cursor: pointer;
	}
	.submit:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.answer-locked {
		margin: 0;
		color: var(--muted, #555);
	}
</style>
