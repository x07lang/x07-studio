<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let zoom = 1;
	export let panX = 0;
	export let panY = 0;

	let dragging = false;
	let lastX = 0;
	let lastY = 0;

	const dispatch = createEventDispatcher<{ pan: { x: number; y: number }; zoom: number }>();

	function pointerDown(event: PointerEvent) {
		dragging = true;
		lastX = event.clientX;
		lastY = event.clientY;
	}

	function pointerMove(event: PointerEvent) {
		if (!dragging) return;
		panX += event.clientX - lastX;
		panY += event.clientY - lastY;
		lastX = event.clientX;
		lastY = event.clientY;
		dispatch('pan', { x: panX, y: panY });
	}

	function pointerUp() {
		dragging = false;
	}

	function wheel(event: WheelEvent) {
		event.preventDefault();
		zoom = Math.max(0.5, Math.min(2.2, zoom + (event.deltaY < 0 ? 0.1 : -0.1)));
		dispatch('zoom', zoom);
	}
</script>

<div
	class="visual-canvas-frame"
	role="application"
	aria-label="Visual graph canvas"
	on:pointerdown={pointerDown}
	on:pointermove={pointerMove}
	on:pointerup={pointerUp}
	on:pointerleave={pointerUp}
	on:wheel={wheel}
>
	<div class="visual-canvas-content" style={`transform: translate(${panX}px, ${panY}px) scale(${zoom});`}>
		<slot />
	</div>
</div>

<style>
	.visual-canvas-frame {
		position: relative;
		min-height: 14rem;
		overflow: hidden;
		border: 1px solid rgba(148, 163, 184, 0.25);
		border-radius: 0.5rem;
		background:
			linear-gradient(rgba(148, 163, 184, 0.08) 1px, transparent 1px),
			linear-gradient(90deg, rgba(148, 163, 184, 0.08) 1px, transparent 1px);
		background-size: 24px 24px;
		touch-action: none;
	}
	.visual-canvas-content {
		position: relative;
		min-height: 14rem;
		transform-origin: 0 0;
	}
</style>
