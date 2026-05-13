<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let turnId: string;
	export let compareTurn: ((turnId: string) => void) | null = null;
	let open = false;

	const dispatch = createEventDispatcher<{ compare: string }>();

	function compare() {
		if (compareTurn) compareTurn(turnId);
		else dispatch('compare', turnId);
	}
</script>

<div
	class="compare-menu"
	role="group"
	aria-label="Compare turn"
	on:mouseenter={() => (open = true)}
	on:mouseleave={() => (open = false)}
	on:focusin={() => (open = true)}
	on:focusout={() => (open = false)}
>
	<button type="button" aria-haspopup="menu" aria-expanded={open} on:click={() => (open = true)}>Compare</button>
	{#if open}
		<div role="menu">
			<button type="button" role="menuitem" on:pointerdown={compare} on:click={compare}>With current</button>
			<button type="button" role="menuitem" on:pointerdown={compare} on:click={compare}>With previous turn</button>
			<button type="button" role="menuitem" on:pointerdown={compare} on:click={compare}>With quorum proposal</button>
		</div>
	{/if}
</div>

<style>
	.compare-menu {
		position: relative;
	}
	.compare-menu > button {
		opacity: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(255, 255, 255, 0.03);
		color: var(--muted);
		font-size: 12px;
	}
	:global(.turn-body:hover) .compare-menu > button,
	.compare-menu > button:focus {
		opacity: 1;
	}
	[role='menu'] {
		position: absolute;
		right: 0;
		z-index: 10;
		display: grid;
		min-width: 180px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--panel, #111820);
		box-shadow: var(--shadow);
	}
	[role='menu'] button {
		border: 0;
		background: transparent;
		color: var(--text);
		padding: 8px;
		text-align: left;
	}
</style>
