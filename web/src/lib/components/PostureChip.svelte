<script lang="ts">
	export let color: string = 'amber';
	export let label = 'pending';
</script>

<span class="posture-chip {color}" data-testid="posture-chip">{label}</span>

<style>
	.posture-chip {
		display: inline-flex;
		align-items: center;
		min-height: 22px;
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 0 8px;
		font-family: var(--font-mono);
		font-size: 11px;
		color: var(--text);
		background: rgba(255, 255, 255, 0.04);
	}
	.green {
		border-color: color-mix(in srgb, var(--accent-pure, var(--mint)) 58%, transparent);
		color: var(--accent-pure, var(--mint));
	}
	.amber {
		border-color: color-mix(in srgb, var(--accent-sandbox, var(--amber)) 58%, transparent);
		color: var(--accent-sandbox, var(--amber));
	}
	.red {
		border-color: color-mix(in srgb, var(--accent-danger, var(--rose)) 58%, transparent);
		color: var(--accent-danger, var(--rose));
	}
</style>
