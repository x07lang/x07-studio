<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { StudioMemory } from '$lib/studio';

	export let memory: StudioMemory | null = null;
	export let open = false;

	let defaultAgent = '';
	let defaultTrustProfile = '';
	let namingStyle = '';
	let verbosity = '';

	const dispatch = createEventDispatcher<{ close: void; save: Partial<StudioMemory> }>();

	$: if (memory) {
		defaultAgent = memory.preferences.default_agent ?? '';
		defaultTrustProfile = memory.preferences.default_trust_profile ?? '';
		namingStyle = memory.preferences.naming_style ?? '';
		verbosity = memory.preferences.verbosity ?? '';
	}

	function save() {
		dispatch('save', {
			preferences: {
				default_agent: defaultAgent || null,
				default_trust_profile: defaultTrustProfile || null,
				naming_style: namingStyle || null,
				verbosity: verbosity || null
			}
		});
	}
</script>

{#if open}
	<aside class="memory-drawer" data-testid="memory-drawer">
		<header>
			<h2>Memory</h2>
			<button type="button" class="link-button" on:click={() => dispatch('close')}>Close</button>
		</header>
		<label>
			<span>Default agent</span>
			<select bind:value={defaultAgent}>
				<option value="">Default</option>
				<option value="claude-code">Claude Code</option>
				<option value="openai-codex">OpenAI Codex</option>
			</select>
		</label>
		<label>
			<span>Trust profile</span>
			<input bind:value={defaultTrustProfile} placeholder="shareable" />
		</label>
		<label>
			<span>Naming</span>
			<select bind:value={namingStyle}>
				<option value="">Default</option>
				<option value="snake_case">snake_case</option>
				<option value="camelCase">camelCase</option>
			</select>
		</label>
		<label>
			<span>Verbosity</span>
			<select bind:value={verbosity}>
				<option value="">Default</option>
				<option value="concise">Concise</option>
				<option value="detailed">Detailed</option>
			</select>
		</label>
		<button type="button" class="command-button primary" on:click={save}>Save memory</button>
	</aside>
{/if}

<style>
	.memory-drawer {
		position: fixed;
		right: 1rem;
		top: 1rem;
		bottom: 1rem;
		z-index: 30;
		width: min(24rem, calc(100vw - 2rem));
		padding: 1rem;
		border: 1px solid rgba(148, 163, 184, 0.28);
		border-radius: 0.6rem;
		background: #111827;
		box-shadow: 0 18px 60px rgba(0, 0, 0, 0.35);
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.memory-drawer header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.memory-drawer h2 {
		margin: 0;
	}
	.memory-drawer label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.memory-drawer input,
	.memory-drawer select {
		color: var(--text, #eef1f6);
		background: rgba(15, 23, 42, 0.8);
		border: 1px solid rgba(148, 163, 184, 0.35);
		border-radius: 0.4rem;
		padding: 0.45rem 0.55rem;
	}
</style>
