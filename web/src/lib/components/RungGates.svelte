<script lang="ts">
	import type { ArchCheckReport, RungGate } from '$lib/studio';
	import ArchCheckBadge from './ArchCheckBadge.svelte';

	export let gates: RungGate[] = [];
	export let archCheckReport: ArchCheckReport | null = null;
</script>

{#if gates.length}
	<div class="rung-gates" data-testid="rung-gates">
		{#each gates as gate}
			<div class:ok={gate.currently_satisfied}>
				<strong>{gate.label}</strong>
				<span>{gate.currently_satisfied ? 'satisfied' : 'needs evidence'}</span>
				{#if gate.id === 'arch-check'}
					<ArchCheckBadge report={archCheckReport} />
				{/if}
				<small>{gate.description}</small>
			</div>
		{/each}
	</div>
{/if}

<style>
	.rung-gates {
		grid-column: 1 / -1;
		display: grid;
		gap: 6px;
	}
	.rung-gates div {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: 3px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 7px;
		background: rgba(255, 255, 255, 0.025);
	}
	.rung-gates div.ok {
		border-color: rgba(114, 228, 180, 0.32);
	}
	.rung-gates small {
		grid-column: 1 / -1;
		color: var(--muted);
	}
</style>
