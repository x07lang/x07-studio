<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { LadderState, ReleaseStatus } from '$lib/studio';
	import RungGates from './RungGates.svelte';

	export let ladder: LadderState | null = null;
	export let releaseStatus: ReleaseStatus | null = null;
	export let busy = false;

	const dispatch = createEventDispatcher<{
		climb: string;
		release: string;
		certificate: void;
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
					<RungGates gates={rung.gates ?? []} />
				</div>
			{/each}
		</div>
		{#if nextRung}
			<button type="button" class="command-button" disabled={busy} on:click={() => dispatch('climb', nextRung.id)}>
				Climb to {nextRung.label}
			</button>
		{/if}
		{#if ladder.current_rung !== 'local_preview'}
			<button type="button" class="command-button primary" disabled={busy} on:click={() => dispatch('release', ladder.current_rung)}>
				Submit release
			</button>
		{/if}
		{#if ladder.current_rung === 'production' || ladder.current_rung === 'team'}
			<button type="button" class="command-button" disabled={busy} on:click={() => dispatch('certificate')} data-testid="view-certificate">
				View certificate
			</button>
		{/if}
		{#if releaseStatus}
			<div class="release-status" data-testid="release-status">
				<strong>{releaseStatus.release_id}</strong>
				<span>{releaseStatus.status} / {releaseStatus.environment}</span>
				<small>{releaseStatus.message}</small>
			</div>
		{/if}
	{:else}
		<p>Run a build to calculate the ladder.</p>
	{/if}
</section>
