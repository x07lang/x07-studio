<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { AgentContract, AgentProfile, AgentRole } from '$lib/studio';
	import AgentRoleSettings from './AgentRoleSettings.svelte';

	export let open = false;
	export let contract: AgentContract | null = null;
	export let busy = false;
	export let agents: AgentProfile[] = [];

	let draft = '';
	let lastHash: string | null = null;

	$: if (contract && contract.hash !== lastHash) {
		draft = contract.markdown;
		lastHash = contract.hash;
	}
	$: unsaved = Boolean(contract && draft !== contract.markdown);

	const dispatch = createEventDispatcher<{
		close: void;
		save: { markdown: string; priorHash: string | null };
		role: { agentId: string; defaultRole: AgentRole; eligibleRoles: AgentRole[] };
	}>();
</script>

{#if open}
	<div class="drawer-backdrop" data-testid="agent-contract-editor">
		<section class="drawer-panel" aria-label="AGENT.md editor">
			<header>
				<div>
					<h2>AGENT.md</h2>
					<span>{contract?.exists ? 'Synced' : 'Template'} · {contract?.hash.slice(0, 8) ?? 'pending'}</span>
				</div>
				<div class="button-row">
					<button class="command-button primary" type="button" disabled={busy || !unsaved} on:click={() => dispatch('save', { markdown: draft, priorHash: contract?.hash ?? null })}>
						Save
					</button>
					<button class="command-button" type="button" on:click={() => dispatch('close')}>Close</button>
				</div>
			</header>
			<div class="contract-grid">
				<nav aria-label="Contract sections">
					{#each contract?.sections ?? [] as section}
						<a href={`#contract-${section.title.toLowerCase().replaceAll(' ', '-')}`}>{section.title}</a>
					{/each}
				</nav>
				<textarea bind:value={draft} spellcheck="false" aria-label="AGENT.md markdown"></textarea>
				<div class="preview">
					<AgentRoleSettings {agents} on:save={(event) => dispatch('role', event.detail)} />
					{#each draft.split('\n') as line}
						{#if line.startsWith('## ')}
							<h3 id={`contract-${line.slice(3).toLowerCase().replaceAll(' ', '-')}`}>{line.slice(3)}</h3>
						{:else if line.startsWith('# ')}
							<h2>{line.slice(2)}</h2>
						{:else if line.startsWith('- ')}
							<p class="bullet">{line}</p>
						{:else if line.trim()}
							<p>{line}</p>
						{/if}
					{/each}
				</div>
			</div>
		</section>
	</div>
{/if}

<style>
	.drawer-backdrop {
		position: fixed;
		inset: 0;
		z-index: 30;
		display: grid;
		place-items: stretch end;
		background: rgba(0, 0, 0, 0.36);
	}
	.drawer-panel {
		width: min(920px, 100vw);
		height: 100%;
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		gap: 12px;
		padding: 16px;
		border-left: 1px solid var(--border);
		background: var(--panel, #111820);
	}
	header {
		display: flex;
		justify-content: space-between;
		gap: 12px;
	}
	header h2,
	header span {
		margin: 0;
	}
	header span,
	.preview p,
	.contract-grid nav a {
		color: var(--muted);
	}
	.contract-grid {
		min-height: 0;
		display: grid;
		grid-template-columns: 150px minmax(0, 1fr) minmax(0, 1fr);
		gap: 12px;
	}
	.contract-grid nav,
	.preview {
		overflow: auto;
	}
	.contract-grid nav {
		display: grid;
		align-content: start;
		gap: 8px;
	}
	.contract-grid nav a {
		text-decoration: none;
		font-size: 12px;
	}
	textarea {
		min-height: 0;
		resize: none;
		font-family: var(--font-mono);
	}
	.preview {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 12px;
		background: rgba(255, 255, 255, 0.025);
	}
	.preview h2,
	.preview h3,
	.preview p {
		margin: 0 0 8px;
	}
	.bullet {
		font-family: var(--font-mono);
	}
</style>
