<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { HealthSnapshot } from '$lib/studio';

	export let snapshot: HealthSnapshot | null = null;
	export let busy = false;

	const dispatch = createEventDispatcher<{ migrate: void; refresh: void }>();

	$: doctorColor = snapshot ? (snapshot.doctor.ok ? 'green' : snapshot.doctor.blockers.length ? 'red' : 'amber') : 'amber';
	$: lockfileColor = snapshot ? (snapshot.lockfile.ok ? 'green' : snapshot.lockfile.stale ? 'amber' : 'red') : 'amber';
	$: migrateColor = snapshot ? (snapshot.migrate.needs_migration ? 'amber' : 'green') : 'amber';

	$: doctorValue = snapshot
		? snapshot.doctor.ok
			? 'ready'
			: (snapshot.doctor.blockers[0] ?? 'checking')
		: 'checking';
	$: lockfileValue = snapshot
		? snapshot.lockfile.ok
			? 'verified'
			: snapshot.lockfile.stale
				? 'stale'
				: 'blocked'
		: 'checking';
	$: migrateValue = snapshot?.migrate.needs_migration
		? `${snapshot.migrate.from_schema ?? 'schema'} → ${snapshot.migrate.to_schema ?? '0.5'}`
		: 'up to date';
</script>

<section class="health-row {snapshot?.overall_color ?? 'amber'}" data-testid="health-row">
	<button type="button" class="pill {doctorColor}" on:click={() => dispatch('refresh')} title="Re-run x07 doctor">
		<span class="dot" aria-hidden="true"></span>
		<span class="label">Doctor</span>
		<strong>{doctorValue}</strong>
	</button>
	<button type="button" class="pill {lockfileColor}" on:click={() => dispatch('refresh')} title="Re-check x07 pkg lock">
		<span class="dot" aria-hidden="true"></span>
		<span class="label">Lockfile</span>
		<strong>{lockfileValue}</strong>
	</button>
	<button
		type="button"
		class="pill {migrateColor}"
		class:resting={!snapshot?.migrate.needs_migration}
		disabled={busy || !snapshot?.migrate.needs_migration}
		on:click={() => dispatch('migrate')}
		title={snapshot?.migrate.needs_migration ? 'Apply x07 migrate' : 'No schema migration required'}
	>
		<span class="dot" aria-hidden="true"></span>
		<span class="label">Migrate</span>
		<strong>{migrateValue}</strong>
	</button>
</section>

<style>
	.health-row {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
	}
	.pill {
		position: relative;
		display: grid;
		grid-template-columns: auto 1fr;
		grid-template-rows: auto auto;
		grid-column-gap: 8px;
		grid-row-gap: 2px;
		align-items: center;
		text-align: left;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 8px 12px;
		background: rgba(255, 255, 255, 0.025);
		color: var(--text);
		cursor: pointer;
		transition: border-color var(--motion-fast, 120ms) var(--ease-out, ease-out),
			background var(--motion-fast, 120ms) var(--ease-out, ease-out);
	}
	.pill:hover:not(:disabled) {
		border-color: var(--border-strong);
	}
	.pill.resting {
		cursor: default;
		opacity: 0.96;
	}
	.pill:disabled {
		cursor: default;
	}
	.pill.resting:disabled {
		opacity: 0.96;
	}
	.dot {
		grid-row: 1 / span 2;
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--pill-color, var(--muted));
		box-shadow: 0 0 0 3px rgba(255, 255, 255, 0.02);
	}
	.pill.green { --pill-color: var(--accent-pure); }
	.pill.amber { --pill-color: var(--accent-sandbox); }
	.pill.red   { --pill-color: var(--accent-danger); }
	.pill.green .dot {
		box-shadow: 0 0 0 3px rgba(114, 228, 180, 0.18);
	}
	.pill.amber .dot {
		box-shadow: 0 0 0 3px rgba(255, 195, 91, 0.2);
	}
	.pill.red .dot {
		box-shadow: 0 0 0 3px rgba(243, 111, 141, 0.2);
	}
	.label {
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--muted);
	}
	.pill strong {
		min-width: 0;
		overflow-wrap: anywhere;
		font-size: 12px;
		font-weight: 600;
		color: var(--text);
		grid-column: 2;
	}
	.pill.green strong { color: var(--accent-pure); }
	.pill.amber strong { color: var(--accent-sandbox); }
	.pill.red strong { color: var(--accent-danger); }
</style>
