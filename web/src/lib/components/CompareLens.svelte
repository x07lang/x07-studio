<script lang="ts">
	import type { SemanticDiff } from '$lib/studio';
	import SemanticDiffView from './SemanticDiff.svelte';

	export let diff: SemanticDiff | null = null;
	export let open = false;
</script>

{#if open}
	<div class="compare-backdrop" role="presentation">
		<section class="compare-lens" data-testid="compare-lens">
			<slot name="actions" />
			<SemanticDiffView {diff} />
		</section>
	</div>
{/if}

<style>
	.compare-backdrop {
		position: fixed;
		inset: 0;
		z-index: 20;
		display: grid;
		place-items: center;
		padding: 24px;
		background: rgba(0, 0, 0, 0.52);
	}
	.compare-lens {
		width: min(820px, 100%);
		max-height: min(720px, 90vh);
		overflow: auto;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius);
		background: var(--surface-3, #0c1016);
		padding: 16px;
		box-shadow: var(--shadow);
	}
</style>
