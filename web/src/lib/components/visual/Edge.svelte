<script lang="ts">
	export let fromX = 0;
	export let fromY = 0;
	export let toX = 120;
	export let toY = 80;
	export let label = '';
</script>

<svg class="visual-edge">
	<defs>
		<marker id="arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto">
			<path d="M0,0 L8,4 L0,8 Z" fill="currentColor" />
		</marker>
	</defs>
	<line x1={fromX} y1={fromY} x2={toX} y2={toY} marker-end="url(#arrow)" />
	{#if label}
		<text x={(fromX + toX) / 2} y={(fromY + toY) / 2 - 6}>{label}</text>
	{/if}
</svg>

<style>
	.visual-edge {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		color: rgba(125, 211, 252, 0.9);
		pointer-events: none;
	}
	line {
		stroke: currentColor;
		stroke-width: 2;
	}
	text {
		fill: currentColor;
		font-size: 11px;
	}
</style>
