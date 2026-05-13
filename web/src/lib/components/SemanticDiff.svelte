<script lang="ts">
	import type { SemanticDiff } from '$lib/studio';
	import PostureChip from './PostureChip.svelte';

	export let diff: SemanticDiff | null = null;

	const tabs = ['World', 'Capabilities', 'Budget', 'Proof', 'Code'];
	let active = 'World';

	$: rows =
		active === 'World'
			? diff?.world_changes ?? []
			: active === 'Capabilities'
				? diff?.capability_changes ?? []
				: active === 'Budget'
					? diff?.budget_changes ?? []
					: active === 'Proof'
						? diff?.proof_changes ?? []
						: [JSON.stringify(diff?.raw ?? {}, null, 2)];
</script>

<section class="semantic-diff" data-testid="semantic-diff">
	<header>
		<h3>Semantic diff</h3>
		<PostureChip color={diff?.trust_delta_color ?? 'amber'} label={diff?.trust_delta_color ?? 'pending'} />
	</header>
	<strong>{diff?.headline ?? 'No comparison selected'}</strong>
	<div class="semantic-tabs" role="tablist">
		{#each tabs as tab}
			<button type="button" class:active={active === tab} on:click={() => (active = tab)}>{tab}</button>
		{/each}
	</div>
	{#if rows.length && diff}
		<ul class:code-tab={active === 'Code'}>
			{#each rows as row}
				<li>{row}</li>
			{/each}
		</ul>
	{:else}
		<p>No semantic changes in this lane.</p>
	{/if}
</section>

<style>
	.semantic-diff {
		display: grid;
		gap: 10px;
	}
	.semantic-diff header,
	.semantic-tabs {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		flex-wrap: wrap;
	}
	.semantic-diff h3 {
		margin: 0;
		font-size: 13px;
	}
	.semantic-diff strong {
		line-height: 1.35;
	}
	.semantic-tabs button {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(255, 255, 255, 0.04);
		color: var(--muted);
		padding: 5px 8px;
	}
	.semantic-tabs button.active {
		color: var(--text);
		border-color: var(--border-strong);
	}
	.semantic-diff ul {
		display: grid;
		gap: 6px;
		margin: 0;
		padding-left: 16px;
	}
	.semantic-diff .code-tab {
		list-style: none;
		padding: 8px;
		border-radius: var(--radius);
		background: rgba(0, 0, 0, 0.25);
		font-family: var(--font-mono);
		font-size: 12px;
		white-space: pre-wrap;
	}
</style>
