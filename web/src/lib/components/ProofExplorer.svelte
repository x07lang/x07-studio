<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { ProofEvidence } from '$lib/studio';

	export let evidence: ProofEvidence | null = null;
	export let open = false;

	const dispatch = createEventDispatcher<{ close: void }>();
</script>

{#if open && evidence}
	<aside class="proof-explorer" data-testid="proof-explorer">
		<header>
			<div>
				<h2>Proof Explorer</h2>
				<span>{evidence.behavior_id}</span>
			</div>
			<button type="button" class="command-button" on:click={() => dispatch('close')}>Close</button>
		</header>
		<strong>{evidence.status}{evidence.z3_ms ? ` · Z3 ${evidence.z3_ms} ms` : ''}</strong>
		<section>
			<h3>Obligations</h3>
			{#each evidence.obligations as obligation}
				<details open>
					<summary>{obligation.id} · {obligation.status}</summary>
					<p>{obligation.goal}</p>
					{#if obligation.note}<small>{obligation.note}</small>{/if}
				</details>
			{/each}
		</section>
		<section>
			<h3>Citations</h3>
			<ul>
				{#each evidence.citations as citation}
					<li><code>{citation.file}</code>{citation.region ? ` · ${citation.region}` : ''}</li>
				{/each}
			</ul>
		</section>
	</aside>
{/if}

<style>
	.proof-explorer {
		position: fixed;
		top: 0;
		right: 0;
		z-index: 22;
		width: min(460px, 100vw);
		height: 100vh;
		overflow: auto;
		display: grid;
		align-content: start;
		gap: 14px;
		padding: 18px;
		border-left: 1px solid var(--border-strong);
		background: var(--surface-3, #0c1016);
		box-shadow: var(--shadow);
	}
	.proof-explorer header {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		align-items: start;
	}
	.proof-explorer h2,
	.proof-explorer h3 {
		margin: 0;
	}
	.proof-explorer header span,
	.proof-explorer small {
		color: var(--muted);
	}
	.proof-explorer details,
	.proof-explorer li {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 8px;
		background: rgba(255, 255, 255, 0.03);
	}
	.proof-explorer ul {
		display: grid;
		gap: 8px;
		padding: 0;
		margin: 0;
		list-style: none;
	}
	code {
		color: var(--mint);
	}
</style>
