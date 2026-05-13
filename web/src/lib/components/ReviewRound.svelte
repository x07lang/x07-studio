<script lang="ts">
	import type { ReviewRound } from '$lib/studio';

	export let round: ReviewRound;
</script>

<section class="review-round" data-testid="review-round">
	<header>
		<span class="verdict {round.verdict}">{round.verdict}</span>
		<strong>{round.reviewer}</strong>
	</header>
	{#if round.concerns.length}
		<ul>
			{#each round.concerns as concern}
				<li>
					<strong>{concern.kind.replaceAll('_', ' ')}</strong>
					<span>{concern.message}</span>
				</li>
			{/each}
		</ul>
	{:else}
		<p>No blocking concerns recorded.</p>
	{/if}
</section>

<style>
	.review-round {
		display: grid;
		gap: 10px;
	}
	header {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.verdict {
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 3px 8px;
		font-size: 11px;
		font-weight: 800;
		text-transform: uppercase;
	}
	.verdict.accept {
		border-color: rgba(114, 228, 180, 0.52);
		color: var(--mint);
	}
	.verdict.revise {
		border-color: rgba(255, 195, 91, 0.52);
		color: var(--amber);
	}
	.verdict.block {
		border-color: rgba(243, 111, 141, 0.52);
		color: var(--rose);
	}
	ul {
		margin: 0;
		padding-left: 18px;
	}
	li {
		margin-bottom: 6px;
	}
	li span,
	p {
		color: var(--muted);
	}
</style>
