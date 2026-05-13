<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { HealthResponse, SyncCode } from '$lib/studio';
	import { openCommandPalette } from '$lib/store/commandPalette';
	import SyncQr from './SyncQr.svelte';

	export let health: HealthResponse;
	export let syncCode: SyncCode | null = null;
	export let detailsOpen = false;
	export let onCommand: (() => void) | null = null;

	let syncOpen = false;

	const dispatch = createEventDispatcher<{
		refresh: void;
		toggleDetails: void;
		sync: void;
		command: void;
		agentContract: void;
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
		<button class="command-button" type="button" on:click={() => dispatch('agentContract')}>
			AGENT.md
		</button>
		<button
			class="key-hint"
			type="button"
			aria-label="Open command palette"
			on:click={() => {
				openCommandPalette();
				onCommand?.();
				dispatch('command');
			}}
		>⌘K</button>
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
	.key-hint {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(255, 255, 255, 0.03);
		padding: 7px 9px;
		color: var(--muted);
		font-family: var(--font-mono);
		font-size: 12px;
		cursor: pointer;
	}
</style>
