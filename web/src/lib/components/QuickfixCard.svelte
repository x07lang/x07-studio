<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { QuickfixRecord } from '$lib/studio';
	import QuickfixThreePane from './QuickfixThreePane.svelte';

	export let incidentId: string;
	export let record: QuickfixRecord | null = null;
	export let busy = false;

	const dispatch = createEventDispatcher<{ load: string; apply: string }>();
</script>

<section class="quickfix-card" data-testid="quickfix-card">
	<header>
		<h3>Quickfix</h3>
		<span>{record?.diagnostic_code ?? incidentId}</span>
	</header>
	{#if record}
		<p><strong>{record.severity}</strong> · {record.summary}</p>
		<QuickfixThreePane {record} {busy} on:apply={() => dispatch('apply', incidentId)} />
		<div class="button-row">
			{#each record.citations as citation}
				<code>{citation.file}</code>
			{/each}
		</div>
	{:else}
		<button type="button" class="command-button" disabled={busy} on:click={() => dispatch('load', incidentId)}>
			Show diagnostic quickfix
		</button>
	{/if}
</section>

<style>
	.quickfix-card {
		display: grid;
		gap: 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 10px;
		background: rgba(255, 195, 91, 0.05);
	}
	.quickfix-card header {
		display: flex;
		justify-content: space-between;
		gap: 8px;
	}
	.quickfix-card h3,
	.quickfix-card p {
		margin: 0;
	}
	.quickfix-card span,
	.quickfix-card code {
		color: var(--amber);
	}
</style>
