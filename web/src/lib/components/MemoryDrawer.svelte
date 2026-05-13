<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { StudioMemory } from '$lib/studio';

	export let memory: StudioMemory | null = null;
	export let open = false;

	let defaultAgent = '';
	let defaultTrustProfile = '';
	let namingStyle = '';
	let verbosity = '';
	let defaultArchitect = '';
	let defaultCoder = '';
	let defaultReviewer = '';
	let allowSelfReview = true;
	let maxReviewRounds = 2;

	const dispatch = createEventDispatcher<{ close: void; save: Partial<StudioMemory> }>();

	$: if (memory) {
		defaultAgent = memory.preferences.default_agent ?? '';
		defaultTrustProfile = memory.preferences.default_trust_profile ?? '';
		namingStyle = memory.preferences.naming_style ?? '';
		verbosity = memory.preferences.verbosity ?? '';
		defaultArchitect = memory.role_preferences?.default_architect ?? 'claude-code';
		defaultCoder = memory.role_preferences?.default_coder ?? 'openai-codex';
		defaultReviewer = memory.role_preferences?.default_reviewer ?? 'claude-code';
		allowSelfReview = memory.role_preferences?.allow_self_review ?? true;
		maxReviewRounds = memory.role_preferences?.default_max_review_rounds ?? 2;
	}

	function save() {
		dispatch('save', {
			preferences: {
				default_agent: defaultAgent || null,
				default_trust_profile: defaultTrustProfile || null,
				naming_style: namingStyle || null,
				verbosity: verbosity || null
			},
			role_preferences: {
				schema_version: 'x07.studio.role_preferences@0.1.0',
				default_architect: defaultArchitect || null,
				default_coder: defaultCoder || null,
				default_reviewer: defaultReviewer || null,
				allow_self_review: allowSelfReview,
				default_max_review_rounds: Number(maxReviewRounds) || 2
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
		<section class="role-memory" data-testid="memory-agent-roles">
			<h3>Agent roles</h3>
			<label>
				<span>Architect</span>
				<select bind:value={defaultArchitect}>
					<option value="claude-code">Claude Code</option>
					<option value="openai-codex">OpenAI Codex</option>
				</select>
			</label>
			<label>
				<span>Coder</span>
				<select bind:value={defaultCoder}>
					<option value="openai-codex">OpenAI Codex</option>
					<option value="claude-code">Claude Code</option>
				</select>
			</label>
			<label>
				<span>Reviewer</span>
				<select bind:value={defaultReviewer}>
					<option value="claude-code">Claude Code</option>
					<option value="openai-codex">OpenAI Codex</option>
				</select>
			</label>
			<label class="inline">
				<input type="checkbox" bind:checked={allowSelfReview} />
				<span>Allow self-review</span>
			</label>
			<label>
				<span>Review rounds</span>
				<input type="number" min="1" max="5" bind:value={maxReviewRounds} />
			</label>
		</section>
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
	.role-memory {
		display: grid;
		gap: 0.75rem;
		border-top: 1px solid rgba(148, 163, 184, 0.22);
		padding-top: 0.85rem;
	}
	.role-memory h3 {
		margin: 0;
		font-size: 0.9rem;
	}
	.memory-drawer label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.memory-drawer label.inline {
		flex-direction: row;
		align-items: center;
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
