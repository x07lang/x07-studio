<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { HealthResponse, SyncCode } from '$lib/studio';
	import SyncQr from './SyncQr.svelte';

	export let health: HealthResponse;
	export let syncCode: SyncCode | null = null;
	export let detailsOpen = false;

	let syncOpen = false;

	const dispatch = createEventDispatcher<{
		refresh: void;
		toggleDetails: void;
		sync: void;
	}>();
</script>

<header class="app-header">
	<div>
		<h1>x07 Studio</h1>
		<p>{health.workspace_root}</p>
	</div>
	<div class="header-actions">
		<button class="command-button" type="button" on:click={() => dispatch('toggleDetails')} aria-pressed={detailsOpen}>
			Show details
		</button>
		<button class="command-button" type="button" on:click={() => dispatch('refresh')}>Refresh</button>
		<button
			class="command-button"
			type="button"
			on:click={() => {
				syncOpen = !syncOpen;
				dispatch('sync');
			}}
		>
			Continue on phone
		</button>
	</div>
	{#if syncOpen}
		<div class="sync-popover">
			<SyncQr {syncCode} />
		</div>
	{/if}
</header>

<style>
	.sync-popover {
		position: absolute;
		right: 1.25rem;
		top: 4.5rem;
		z-index: 20;
		max-width: min(24rem, calc(100vw - 2rem));
	}
</style>
