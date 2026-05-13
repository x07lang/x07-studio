<script lang="ts">
	import type { TrustPosture } from '$lib/studio';

	export let posture: TrustPosture | null = null;
</script>

<div class="posture-badge {posture?.posture_color ?? 'amber'}" data-testid="posture-badge">
	<strong>{posture?.trust_profile ?? 'posture pending'}</strong>
	<span>{posture ? `${posture.worlds.join(', ')} · ${Math.round(posture.proof_coverage.proved_pct)}% proofs` : 'Run verify to capture trust posture'}</span>
</div>

<style>
	.posture-badge {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		border: 1px solid var(--border);
		border-left: 4px solid var(--amber);
		border-radius: var(--radius);
		padding: 8px 10px;
		background: rgba(255, 255, 255, 0.025);
	}
	.posture-badge.green {
		border-left-color: var(--mint);
	}
	.posture-badge.red {
		border-left-color: var(--rose);
	}
	.posture-badge span {
		color: var(--muted);
		font-size: 12px;
	}
</style>
