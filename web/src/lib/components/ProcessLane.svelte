<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { CanonicalStep, ProcessLane as ProcessLaneType, WhatIfForecast } from '$lib/studio';
	import RoleBadge from './RoleBadge.svelte';
	import StepNode from './StepNode.svelte';

	export let lane: ProcessLaneType | null = null;
	export let forecasts: Record<string, WhatIfForecast | null> = {};

	const dispatch = createEventDispatcher<{ step: CanonicalStep; forecast: string }>();

	$: current = lane?.current_index != null ? lane.steps[lane.current_index] : null;
	$: next = lane?.next_index != null ? lane.steps[lane.next_index] : null;
</script>

{#if lane}
	<section class="process-lane" data-testid="process-lane" aria-label="Process lane">
		<div class="lane-copy">
			<div>
				<span>Now</span>
				<strong>{current ? current.label : 'No active step'}</strong>
				{#if current}
					<RoleBadge role={current.actor} />
				{/if}
			</div>
			<div>
				<span>Next</span>
				<strong>{next ? next.label : 'Complete'}</strong>
				{#if next}
					<RoleBadge role={next.actor} />
				{/if}
			</div>
		</div>
		<div class="lane-strip">
			{#each lane.steps as step}
				<StepNode
					{step}
					forecast={forecasts[step.id] ?? null}
					on:select={(event) => dispatch('step', event.detail)}
					on:forecast={(event) => dispatch('forecast', event.detail)}
				/>
			{/each}
		</div>
	</section>
{/if}

<style>
	.process-lane {
		display: grid;
		gap: 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 12px;
		background: linear-gradient(180deg, rgba(20, 25, 32, 0.92), rgba(11, 15, 21, 0.94));
		box-shadow: var(--shadow);
	}
	.lane-copy {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 10px;
	}
	.lane-copy div {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.lane-copy span {
		color: var(--muted);
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
	}
	.lane-copy strong {
		min-width: 0;
		overflow-wrap: anywhere;
		font-size: 13px;
	}
	.lane-strip {
		display: flex;
		gap: 9px;
		overflow-x: auto;
		padding-block-end: 4px;
	}
	@media (max-width: 720px) {
		.lane-copy {
			grid-template-columns: 1fr;
		}
		.lane-strip {
			display: grid;
			grid-template-columns: 1fr;
		}
	}
</style>
