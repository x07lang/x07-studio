<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let id: string;
	export let label: string;
	export let x = 0;
	export let y = 0;

	const dispatch = createEventDispatcher<{ label: { id: string; label: string }; remove: string }>();
</script>

<div class="visual-node" style={`left:${x}px; top:${y}px;`}>
	<input
		value={label}
		aria-label={`Node ${id} label`}
		on:input={(event) => dispatch('label', { id, label: (event.currentTarget as HTMLInputElement).value })}
	/>
	<button type="button" class="link-button" on:click={() => dispatch('remove', id)}>Remove</button>
</div>

<style>
	.visual-node {
		position: absolute;
		width: 10.5rem;
		padding: 0.5rem;
		border: 1px solid rgba(148, 163, 184, 0.32);
		border-radius: 0.45rem;
		background: rgba(15, 23, 42, 0.9);
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.visual-node input {
		width: 100%;
		box-sizing: border-box;
	}
</style>
