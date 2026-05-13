<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { PbtRound } from '$lib/studio';

	export let round: PbtRound | null = null;
	export let busy = false;

	const dispatch = createEventDispatcher<{ run: void; regression: string }>();
</script>

<section class="pbt-panel" data-testid="pbt-panel">
	<header>
		<h3>PBT</h3>
		<button class="command-button" type="button" disabled={busy} on:click={() => dispatch('run')}>Run PBT</button>
	</header>
	{#if round}
		<p>{round.properties_run} properties ran, {round.counterexamples.length} counterexamples.</p>
		{#each round.counterexamples as example}
			<article>
				<strong>{example.property}</strong>
				<code>{example.repro_path || example.repro_id}</code>
				<button class="command-button" type="button" disabled={busy} on:click={() => dispatch('regression', example.repro_id)}>
					Lock as regression test
				</button>
			</article>
		{/each}
	{:else}
		<p>Property checks have not run for this verified turn yet.</p>
	{/if}
</section>

<style>
	.pbt-panel,
	article {
		display: grid;
		gap: 8px;
	}
	header {
		display: flex;
		justify-content: space-between;
		gap: 8px;
	}
	h3,
	p {
		margin: 0;
	}
	p,
	code {
		color: var(--muted);
	}
	article {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 8px;
	}
</style>
