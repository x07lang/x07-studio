<script lang="ts">
	import type { AgentStreamEvent, LiveDiff } from '$lib/studio';

	export let events: AgentStreamEvent[] = [];

	$: diffs = events.map(extractLiveDiff).filter((item): item is LiveDiff => Boolean(item));

	function extractLiveDiff(event: AgentStreamEvent): LiveDiff | null {
		if (event.kind !== 'tool_use' || !event.input || typeof event.input !== 'object') return null;
		return ((event.input as { live_diff?: LiveDiff }).live_diff ?? null) as LiveDiff | null;
	}
</script>

{#if diffs.length}
	<section class="realize-preview" data-testid="realize-preview">
		<header>
			<h3>Live implementation preview</h3>
			<span>{diffs.length} diff{diffs.length === 1 ? '' : 's'}</span>
		</header>
		{#each diffs as diff}
			<article>
				<strong>{diff.path}</strong>
				<pre>{diff.unified_diff}</pre>
			</article>
		{/each}
	</section>
{/if}

<style>
	.realize-preview {
		margin: 0.65rem 0;
		padding: 0.75rem;
		border: 1px solid rgba(34, 197, 94, 0.28);
		border-radius: 0.45rem;
		background: rgba(20, 83, 45, 0.12);
	}
	.realize-preview header {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		align-items: center;
	}
	.realize-preview h3 {
		margin: 0;
		font-size: 0.85rem;
	}
	.realize-preview span {
		font-size: 0.76rem;
		color: var(--muted, #aab1c0);
	}
	.realize-preview article {
		margin-top: 0.6rem;
	}
	.realize-preview pre {
		margin: 0.35rem 0 0;
		white-space: pre-wrap;
		max-height: 18rem;
		overflow: auto;
		font-size: 0.76rem;
		line-height: 1.35;
	}
</style>
