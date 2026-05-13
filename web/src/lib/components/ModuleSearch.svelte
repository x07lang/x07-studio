<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { PkgProvidesResult } from '$lib/studio';

	export let result: PkgProvidesResult | null = null;
	export let busy = false;

	let moduleId = '';
	const dispatch = createEventDispatcher<{ search: string }>();
</script>

<section class="module-search" data-testid="module-search">
	<div class="inline-form">
		<input bind:value={moduleId} placeholder="text.normalize_v1" aria-label="Module id" />
		<button class="command-button" type="button" disabled={busy || !moduleId.trim()} on:click={() => dispatch('search', moduleId.trim())}>
			Find package
		</button>
	</div>
	{#if result}
		{#each result.candidates as candidate}
			<article>
				<strong>{candidate.package}@{candidate.version}</strong>
				<span>{candidate.source}</span>
				<code>{candidate.install_command}</code>
			</article>
		{/each}
	{/if}
</section>

<style>
	.module-search,
	article {
		display: grid;
		gap: 8px;
	}
	article {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 8px;
	}
	article span {
		color: var(--muted);
	}
	code {
		overflow-x: auto;
	}
</style>
