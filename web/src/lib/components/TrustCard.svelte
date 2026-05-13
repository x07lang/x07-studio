<script lang="ts">
	import type { TrustPosture } from '$lib/studio';
	import PostureChip from './PostureChip.svelte';

	export let posture: TrustPosture | null = null;

	$: headline = posture
		? `${posture.worlds.join(', ')} · ${posture.capabilities.length} OS read${posture.capabilities.length === 1 ? '' : 's'} · ${Math.round(posture.proof_coverage.proved_pct)}% proof coverage`
		: 'Trust posture pending';
</script>

<section class="now-card trust-card {posture?.posture_color ?? 'amber'}" data-testid="trust-card">
	<header>
		<h2>Trust Card</h2>
		<PostureChip color={posture?.posture_color ?? 'amber'} label={posture?.trust_profile ?? 'pending'} />
	</header>
	<strong>{headline}</strong>
	{#if posture}
		<div class="posture-grid">
			<div>
				<span>Worlds</span>
				<code>{posture.worlds.join(', ') || 'none'}</code>
			</div>
			<div>
				<span>Budget</span>
				<code>{posture.budgets.prover_seconds_used}s / {posture.budgets.prover_seconds_cap ?? 'open'}s</code>
			</div>
			<div>
				<span>Proofs</span>
				<code>{posture.proof_coverage.proof_count} obligations</code>
			</div>
		</div>
		{#if posture.capabilities.length}
			<ul>
				{#each posture.capabilities as capability}
					<li><code>{capability.id}</code> {capability.justification}</li>
				{/each}
			</ul>
		{/if}
		{#if posture.deltas.length}
			<div class="delta-list">
				{#each posture.deltas.slice(0, 4) as delta}
					<span>{delta.summary}</span>
				{/each}
			</div>
		{/if}
	{:else}
		<p>Build or formalize a session to capture the first posture.</p>
	{/if}
</section>

<style>
	.trust-card {
		border-left-width: 4px;
	}
	.trust-card.green {
		border-left-color: var(--accent-pure, var(--mint));
	}
	.trust-card.amber {
		border-left-color: var(--accent-sandbox, var(--amber));
	}
	.trust-card.red {
		border-left-color: var(--accent-danger, var(--rose));
	}
	.trust-card strong {
		font-size: 15px;
		line-height: 1.35;
	}
	.posture-grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
	}
	.posture-grid div,
	.delta-list span {
		display: grid;
		gap: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 8px;
		background: rgba(255, 255, 255, 0.03);
	}
	.posture-grid span {
		color: var(--muted);
		font-size: 11px;
		text-transform: uppercase;
	}
	.posture-grid code,
	.trust-card li code {
		color: var(--mint);
	}
	.trust-card ul,
	.delta-list {
		display: grid;
		gap: 6px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.delta-list span {
		color: var(--muted);
		font-size: 12px;
	}
</style>
