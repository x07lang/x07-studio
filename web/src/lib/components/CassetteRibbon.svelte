<script lang="ts">
	import type { CassetteRibbon } from '$lib/studio';

	export let ribbon: CassetteRibbon | null = null;
	let selected = 0;

	$: selectedBoundary = ribbon?.boundaries[selected] ?? null;
</script>

{#if ribbon?.boundaries.length}
	<section class="cassette-ribbon" data-testid="cassette-ribbon">
		<header>
			<h2>Cassette ribbon</h2>
			<span>{ribbon.boundaries.length} boundaries</span>
		</header>
		<div class="ribbon-strip">
			{#each ribbon.boundaries as boundary, index}
				<button
					type="button"
					class={boundary.kind}
					class:active={selected === index}
					title={boundary.summary}
					on:click={() => (selected = index)}
				>
					<span>{index + 1}</span>
				</button>
			{/each}
		</div>
		{#if selectedBoundary}
			<div class="boundary-detail">
				<strong>{selectedBoundary.kind} · {selectedBoundary.policy}</strong>
				<span>{selectedBoundary.summary}</span>
				<code>{selectedBoundary.cassette_path}</code>
			</div>
		{/if}
	</section>
{/if}

<style>
	.cassette-ribbon {
		display: grid;
		gap: 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 12px;
		background: rgba(255, 255, 255, 0.03);
	}
	.cassette-ribbon header {
		display: flex;
		justify-content: space-between;
		gap: 8px;
	}
	.cassette-ribbon h2 {
		margin: 0;
		font-size: 13px;
	}
	.ribbon-strip {
		display: flex;
		gap: 6px;
		overflow-x: auto;
		padding-bottom: 2px;
	}
	.ribbon-strip button {
		min-width: 30px;
		height: 30px;
		border: 1px solid var(--border);
		border-radius: 999px;
		color: var(--text);
		background: rgba(255, 255, 255, 0.05);
	}
	.ribbon-strip button.os-net,
	.ribbon-strip button.http {
		border-color: var(--amber);
	}
	.ribbon-strip button.os-fs {
		border-color: var(--violet);
	}
	.ribbon-strip button.active {
		background: rgba(85, 214, 231, 0.18);
		border-color: var(--cyan);
	}
	.boundary-detail {
		display: grid;
		gap: 4px;
	}
	.boundary-detail span {
		color: var(--muted);
	}
	.boundary-detail code {
		color: var(--mint);
		overflow-wrap: anywhere;
	}
</style>
