<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { LadderState } from '$lib/studio';

	export let ladder: LadderState | null = null;
	export let busy = false;

	const dispatch = createEventDispatcher<{
		climb: string;
	}>();

	$: nextRung = ladder?.rungs.find((rung) => !rung.satisfied) ?? null;
</script>

<section class="now-card" data-testid="shipping-ladder">
	<header>
		<h2>Shipping Ladder</h2>
		<span>{ladder?.current_rung ?? 'pending'}</span>
	</header>
	{#if ladder}
		<div class="ladder-list">
			{#each ladder.rungs as rung}
				<div class:done={rung.satisfied}>
					<strong>{rung.label}</strong>
					<span>{rung.satisfied ? 'satisfied' : `${rung.missing.length} missing`}</span>
					{#if !rung.satisfied}
						<small>{rung.missing[0]}</small>
					{/if}
				</div>
			{/each}
		</div>
		{#if nextRung}
			<button type="button" class="command-button" disabled={busy} on:click={() => dispatch('climb', nextRung.id)}>
				Climb to {nextRung.label}
			</button>
		{/if}
	{:else}
		<p>Run a build to calculate the ladder.</p>
	{/if}
</section>
