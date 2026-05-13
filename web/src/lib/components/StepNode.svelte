<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { CanonicalStep, WhatIfForecast } from '$lib/studio';
	import RoleBadge from './RoleBadge.svelte';
	import WhatIfTooltip from './WhatIfTooltip.svelte';

	export let step: CanonicalStep;
	export let forecast: WhatIfForecast | null = null;

	const dispatch = createEventDispatcher<{ select: CanonicalStep; forecast: string }>();

	$: seconds = step.elapsed_ms ? `${Math.max(1, Math.round(step.elapsed_ms / 1000))}s` : '';
	$: budgetSeconds = step.budget?.wall_clock_ms ? Math.round(step.budget.wall_clock_ms / 1000) : null;
	$: accessibleName = `Process step ${step.label}: ${step.actor}, ${step.status}`;
</script>

<button
	type="button"
	class="step-node {step.actor} {step.status}"
	data-testid={`step-node-${step.id}`}
	aria-label={accessibleName}
	on:click={() => dispatch('select', step)}
	on:mouseenter={() => dispatch('forecast', step.id)}
	on:focus={() => dispatch('forecast', step.id)}
>
	<span class="status-dot" aria-hidden="true"></span>
	<span class="step-label">{step.label}</span>
	<RoleBadge role={step.actor} />
	{#if seconds}
		<span class="elapsed">{seconds}</span>
	{/if}
	{#if budgetSeconds && step.status === 'running'}
		<span class="budget">{seconds || '0s'} / {budgetSeconds}s</span>
	{/if}
	{#if step.round}
		<span class="round">round {step.round}</span>
	{/if}
	<WhatIfTooltip {forecast} />
</button>

<style>
	.step-node {
		position: relative;
		min-width: 168px;
		min-height: 78px;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 5px 8px;
		align-items: center;
		text-align: left;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 10px;
		background: rgba(255, 255, 255, 0.035);
		color: var(--text);
	}
	.step-node.pending {
		opacity: 0.58;
	}
	.step-node.running {
		animation: step-pulse 1.6s ease-in-out infinite;
	}
	.step-node.stalled {
		border-color: rgba(243, 111, 141, 0.58);
	}
	.status-dot {
		width: 9px;
		height: 9px;
		border-radius: 50%;
		background: var(--muted);
	}
	.step-node.done .status-dot {
		background: var(--mint);
	}
	.step-node.running .status-dot {
		background: var(--cyan);
	}
	.step-node.stalled .status-dot {
		background: var(--rose);
	}
	.step-label {
		min-width: 0;
		overflow-wrap: anywhere;
		font-size: 12px;
		font-weight: 700;
	}
	.role-badge,
	.elapsed,
	.budget,
	.round {
		grid-column: 2;
	}
	.elapsed,
	.budget,
	.round {
		color: var(--muted);
		font-family: var(--font-mono);
		font-size: 11px;
	}
	.budget {
		color: var(--amber);
	}
	@keyframes step-pulse {
		0%, 100% {
			box-shadow: 0 0 0 rgba(85, 214, 231, 0);
		}
		50% {
			box-shadow: 0 0 22px rgba(85, 214, 231, 0.22);
		}
	}
</style>
