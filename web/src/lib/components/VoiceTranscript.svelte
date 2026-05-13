<script lang="ts">
	export let text = '';
	export let confidence = 0;
	export let supported = true;
</script>

{#if text || !supported}
	<section class="voice-transcript" data-testid="voice-transcript" class:low={confidence < 0.6}>
		<div class="confidence" style={`--confidence:${Math.max(0, Math.min(confidence, 1))}`}></div>
		<div>
			<strong>{supported ? 'Voice transcript' : 'Voice unavailable'}</strong>
			<p>{supported ? text : 'This browser does not expose Web Speech capture.'}</p>
			{#if supported}
				<small>{Math.round(confidence * 100)}% confidence{confidence < 0.6 ? ' - review before sending' : ''}</small>
			{/if}
		</div>
	</section>
{/if}

<style>
	.voice-transcript {
		display: flex;
		gap: 0.65rem;
		align-items: flex-start;
		padding: 0.6rem 0.7rem;
		border: 1px solid rgba(56, 189, 248, 0.3);
		border-radius: 0.5rem;
		background: rgba(8, 47, 73, 0.18);
	}
	.voice-transcript.low {
		border-color: rgba(245, 158, 11, 0.4);
		background: rgba(120, 53, 15, 0.18);
	}
	.voice-transcript p {
		margin: 0.15rem 0;
	}
	.voice-transcript small {
		color: var(--muted, #aab1c0);
	}
	.confidence {
		width: 1.9rem;
		height: 1.9rem;
		border-radius: 50%;
		background: conic-gradient(#38bdf8 calc(var(--confidence) * 360deg), rgba(148, 163, 184, 0.25) 0);
		flex: 0 0 auto;
	}
</style>
