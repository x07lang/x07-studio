<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { PlainEnglishSummary, SessionSnapshot } from '$lib/studio';

	export let session: SessionSnapshot;
	export let summary: PlainEnglishSummary | null = null;

	const dispatch = createEventDispatcher<{
		'open-expert': void;
		restart: void;
	}>();

	$: phaseSucceeded =
		session.phase === 'trust_review' ||
		session.phase === 'certify_running' ||
		session.phase === 'certified';
</script>

<section class="result-preview" data-testid="simple-result-preview">
	{#if summary}
		<header>
			<h2 data-testid="summary-headline">{summary.headline}</h2>
		</header>
		{#if summary.behavior_promises.length}
			<div class="block">
				<h3>What it does</h3>
				<ul>
					{#each summary.behavior_promises as item}
						<li>{item}</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if summary.boundaries.length}
			<div class="block">
				<h3>What it won't do</h3>
				<ul>
					{#each summary.boundaries as item}
						<li>{item}</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if summary.evidence.length}
			<div class="block">
				<h3>Evidence</h3>
				<ul>
					{#each summary.evidence as item}
						<li>{item}</li>
					{/each}
				</ul>
			</div>
		{/if}
	{:else if phaseSucceeded}
		<header>
			<h2>Build complete</h2>
		</header>
		<p class="hint">The summary is being generated. Open Expert mode for the full evidence.</p>
	{:else}
		<header>
			<h2>I need your help</h2>
		</header>
		<p class="hint">
			The pipeline paused before verification completed. Open Expert mode to inspect the
			worklog and decide the next move.
		</p>
	{/if}

	<div class="actions">
		<button
			type="button"
			class="link"
			on:click={() => dispatch('open-expert')}
			data-testid="result-open-expert"
		>
			See full evidence (Expert mode)
		</button>
		<button
			type="button"
			class="link secondary"
			on:click={() => dispatch('restart')}
			data-testid="result-restart"
		>
			Start a new build
		</button>
	</div>
</section>

<style>
	.result-preview {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		padding: 1.5rem;
		border-radius: 0.75rem;
		background: var(--surface, #ffffff);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
	}
	header h2 {
		margin: 0;
		font-size: 1.35rem;
	}
	.hint {
		margin: 0;
		color: var(--muted, #555);
	}
	.block {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.block h3 {
		margin: 0;
		font-size: 0.95rem;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--muted, #555);
	}
	.block ul {
		margin: 0;
		padding-left: 1.25rem;
		font-size: 0.95rem;
	}
	.actions {
		display: flex;
		gap: 0.75rem;
		margin-top: 0.5rem;
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
	.link.secondary {
		color: var(--muted, #555);
	}
</style>
