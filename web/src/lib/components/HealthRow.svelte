<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { HealthSnapshot } from '$lib/studio';

	export let snapshot: HealthSnapshot | null = null;
	export let busy = false;

	const dispatch = createEventDispatcher<{ migrate: void; refresh: void }>();
</script>

<section class="health-row {snapshot?.overall_color ?? 'amber'}" data-testid="health-row">
	<button type="button" class="pill" on:click={() => dispatch('refresh')}>
		<span class:ok={snapshot?.doctor.ok}>Doctor</span>
		<strong>{snapshot?.doctor.ok ? 'ok' : snapshot?.doctor.blockers[0] ?? 'checking'}</strong>
	</button>
	<button type="button" class="pill" on:click={() => dispatch('refresh')}>
		<span class:ok={snapshot?.lockfile.ok}>Lockfile</span>
		<strong>{snapshot?.lockfile.ok ? 'ok' : snapshot?.lockfile.stale ? 'stale' : 'blocked'}</strong>
	</button>
	<button type="button" class="pill" disabled={busy || !snapshot?.migrate.needs_migration} on:click={() => dispatch('migrate')}>
		<span class:ok={!snapshot?.migrate.needs_migration}>Migrate</span>
		<strong>{snapshot?.migrate.needs_migration ? `${snapshot.migrate.from_schema ?? 'schema'} -> ${snapshot.migrate.to_schema ?? '0.5'}` : 'clean'}</strong>
	</button>
</section>

<style>
	.health-row {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
	}
	.pill {
		display: grid;
		gap: 3px;
		text-align: left;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 9px;
		background: rgba(255, 255, 255, 0.025);
		color: var(--text);
	}
	.pill span {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		color: var(--amber);
	}
	.pill span.ok {
		color: var(--mint);
	}
	.pill strong {
		min-width: 0;
		overflow-wrap: anywhere;
		font-size: 12px;
	}
</style>
