<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { RealizeProposal, RealizeQuorumRound, SemanticDiff as SemanticDiffType } from '$lib/studio';
	import SemanticDiff from './SemanticDiff.svelte';

	export let round: RealizeQuorumRound;
	export let busy = false;

	const dispatch = createEventDispatcher<{ pick: number }>();

	function body(value: unknown) {
		return JSON.stringify(value, null, 2);
	}

	function diffFor(proposal: RealizeProposal) {
		return `--- ${proposal.path}\n+++ ${proposal.agent_id}:${proposal.path}\n${body(proposal.body)
			.split('\n')
			.map((line) => `+${line}`)
			.join('\n')}`;
	}

	$: localDiff = {
		schema_version: 'x07.studio.semantic_diff@0.1.0',
		from: { kind: 'current' as const },
		to: { kind: 'current' as const },
		headline: round.proposals.some((proposal) => JSON.stringify(proposal.body).includes('os-net'))
			? 'adds os-net · review before picking'
			: 'stays solve-pure · proposals differ in code only',
		trust_delta_color: round.proposals.some((proposal) => JSON.stringify(proposal.body).includes('os-net')) ? 'red' : 'green',
		raw: round,
		world_changes: [],
		capability_changes: [],
		budget_changes: [],
		proof_changes: []
	} satisfies SemanticDiffType;
</script>

<section class="quorum-realize" data-testid="quorum-realize">
	<header>
		<h3>Compare both agents</h3>
		<span>{round.agreed ? 'agreed' : 'different proposals'}</span>
	</header>
	<SemanticDiff diff={localDiff} />
	<div class="proposal-grid">
		{#each round.proposals as proposal, index}
			<article class:failed={proposal.status !== 'ok'}>
				<header>
					<strong>{proposal.agent_id}</strong>
					<span>{proposal.status}</span>
				</header>
				<small>{proposal.path}</small>
				<pre>{diffFor(proposal)}</pre>
				{#if proposal.stderr_excerpt}
					<p class="stderr">{proposal.stderr_excerpt}</p>
				{/if}
				<button
					type="button"
					class="command-button primary"
					disabled={busy || proposal.status !== 'ok'}
					on:click={() => dispatch('pick', index)}
				>
					Pick this
				</button>
			</article>
		{/each}
	</div>
</section>

<style>
	.quorum-realize {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.quorum-realize > header,
	.proposal-grid article header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}
	.quorum-realize h3 {
		margin: 0;
		font-size: 0.95rem;
	}
	.proposal-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
		gap: 0.75rem;
	}
	.proposal-grid article {
		border: 1px solid rgba(148, 163, 184, 0.24);
		border-radius: 0.5rem;
		padding: 0.75rem;
		background: rgba(15, 23, 42, 0.3);
		min-width: 0;
	}
	.proposal-grid article.failed {
		border-color: rgba(248, 113, 113, 0.35);
	}
	.proposal-grid small,
	.quorum-realize span {
		color: var(--muted, #aab1c0);
	}
	.proposal-grid pre {
		white-space: pre-wrap;
		max-height: 18rem;
		overflow: auto;
		font-size: 0.74rem;
		line-height: 1.35;
	}
	.stderr {
		color: #fca5a5;
		font-size: 0.78rem;
	}
</style>
