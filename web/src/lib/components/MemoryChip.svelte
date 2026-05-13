<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { OpRecord } from '$lib/studio';

	export let ops: OpRecord[] = [];

	const dispatch = createEventDispatcher<{ edit: void }>();

	$: applied = [...ops].reverse().find((op) => op.op === 'preferences.apply');
	$: report = (applied?.report_json ?? {}) as Record<string, unknown>;
	$: labels = ['default_agent', 'default_trust_profile', 'naming_style', 'verbosity']
		.map((key) => [key, report[key]] as const)
		.filter(([, value]) => typeof value === 'string' && value);
</script>

{#if applied && labels.length}
	<section class="memory-chip" data-testid="memory-chip">
		<div>
			<strong>Memory applied</strong>
			<span>{labels.map(([key, value]) => `${key}: ${value}`).join(' / ')}</span>
		</div>
		<button type="button" class="link-button" on:click={() => dispatch('edit')}>Edit memory</button>
	</section>
{/if}

<style>
	.memory-chip {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		align-items: center;
		padding: 0.55rem 0.7rem;
		border-radius: 0.5rem;
		border: 1px solid rgba(56, 189, 248, 0.26);
		background: rgba(8, 47, 73, 0.16);
		font-size: 0.82rem;
	}
	.memory-chip div {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		min-width: 0;
	}
	.memory-chip span {
		color: var(--muted, #aab1c0);
		overflow-wrap: anywhere;
	}
</style>
