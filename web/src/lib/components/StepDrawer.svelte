<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { StepEvidence } from '$lib/studio';

	export let evidence: StepEvidence | null = null;
	export let open = false;

	const dispatch = createEventDispatcher<{ close: void }>();
</script>

{#if open && evidence}
	<div class="drawer-backdrop" data-testid="step-drawer">
		<section class="step-drawer" aria-label="Step evidence">
			<header>
				<div>
					<h2>{evidence.step_id}</h2>
					<span>{evidence.op?.op ?? 'No op linked'}</span>
				</div>
				<button type="button" class="command-button" on:click={() => dispatch('close')}>Close</button>
			</header>
			{#if evidence.op}
				<div class="evidence-block">
					<strong>{evidence.op.status}</strong>
					<p>{evidence.op.notes ?? evidence.op.stdout ?? 'No notes recorded.'}</p>
					{#if evidence.op.command.length}
						<code>{evidence.op.command.join(' ')}</code>
					{/if}
				</div>
			{/if}
			{#if evidence.artifacts.length}
				<div class="evidence-block">
					<strong>Artifacts</strong>
					<ul>
						{#each evidence.artifacts as artifact}
							<li><code>{artifact}</code></li>
						{/each}
					</ul>
				</div>
			{/if}
			{#if evidence.stream_events.length}
				<div class="evidence-block">
					<strong>Stream events</strong>
					<pre>{JSON.stringify(evidence.stream_events, null, 2)}</pre>
				</div>
			{/if}
			<details>
				<summary>Raw record</summary>
				<pre>{JSON.stringify(evidence, null, 2)}</pre>
			</details>
		</section>
	</div>
{/if}

<style>
	.drawer-backdrop {
		position: fixed;
		inset: 0;
		z-index: 31;
		display: grid;
		place-items: stretch end;
		background: rgba(0, 0, 0, 0.34);
	}
	.step-drawer {
		width: min(620px, 100vw);
		height: 100%;
		display: grid;
		align-content: start;
		gap: 12px;
		padding: 16px;
		border-left: 1px solid var(--border);
		background: var(--panel, #111820);
		overflow: auto;
	}
	header {
		display: flex;
		justify-content: space-between;
		gap: 12px;
	}
	h2,
	p {
		margin: 0;
	}
	header span,
	p {
		color: var(--muted);
	}
	.evidence-block {
		display: grid;
		gap: 7px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 10px;
		background: rgba(255, 255, 255, 0.03);
	}
	code,
	pre {
		min-width: 0;
		overflow: auto;
		font-family: var(--font-mono);
		font-size: 12px;
	}
	pre {
		max-height: 280px;
		margin: 0;
	}
</style>
