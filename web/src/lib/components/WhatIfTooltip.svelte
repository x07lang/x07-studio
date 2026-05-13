<script lang="ts">
	import type { WhatIfForecast } from '$lib/studio';

	export let forecast: WhatIfForecast | null = null;

	$: seconds = forecast ? (forecast.estimated_duration_ms / 1000).toFixed(1) : '0.0';
	$: confidence = forecast ? Math.round(forecast.confidence * 100) : 0;
</script>

{#if forecast}
	<div class="what-if-tooltip" data-testid="what-if-tooltip">
		<strong>Forecast · {confidence}%</strong>
		<span>Est. {seconds}s</span>
		{#if forecast.assumptions.length}
			<p>{forecast.assumptions[0]}</p>
		{/if}
		{#if forecast.predicted_delta}
			<small>Trust delta available</small>
		{/if}
	</div>
{/if}

<style>
	.what-if-tooltip {
		position: absolute;
		z-index: 12;
		inset-block-start: calc(100% + 8px);
		inset-inline-start: 0;
		width: min(240px, 72vw);
		display: grid;
		gap: 4px;
		padding: 9px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--panel, #111820);
		box-shadow: var(--shadow);
	}
	.what-if-tooltip strong,
	.what-if-tooltip span,
	.what-if-tooltip small {
		font-size: 11px;
	}
	.what-if-tooltip p {
		margin: 0;
		color: var(--muted);
		font-size: 12px;
		line-height: 1.4;
	}
</style>
