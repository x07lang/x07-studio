<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let busy = false;
	export let voiceSupported = false;
	export let recording = false;
	export let prompt = '';

	const dispatch = createEventDispatcher<{
		begin: { prompt: string };
		'start-voice': void;
		'stop-voice': void;
		'open-expert': void;
	}>();

	function onBegin() {
		const trimmed = prompt.trim();
		if (!trimmed) return;
		dispatch('begin', { prompt: trimmed });
	}

	function onKey(event: KeyboardEvent) {
		if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			onBegin();
		}
	}
</script>

<section class="simple-start" data-testid="simple-start">
	<header>
		<h1>What do you want to build?</h1>
		<p class="hint">
			Describe your idea in plain language. I'll ask if I have questions, then I'll build it
			and show you what I made.
		</p>
	</header>

	<label class="prompt">
		<span class="visually-hidden">Initial plan</span>
		<textarea
			bind:value={prompt}
			placeholder="Example: Build a stable sorter for byte arrays that rejects empty input and keeps equal items in their original order."
			rows="6"
			disabled={busy}
			data-testid="simple-start-prompt"
			on:keydown={onKey}
		></textarea>
	</label>

	<div class="actions">
		{#if voiceSupported}
			<button
				type="button"
				class="voice {recording ? 'recording' : ''}"
				disabled={busy}
				on:click={() => dispatch(recording ? 'stop-voice' : 'start-voice')}
				data-testid="simple-start-mic"
				aria-pressed={recording}
			>
				{recording ? '⏺ Stop' : '🎤 Speak'}
			</button>
		{/if}
		<button
			type="button"
			class="primary"
			disabled={busy || !prompt.trim()}
			on:click={onBegin}
			data-testid="simple-start-begin"
		>
			{busy ? 'Working…' : 'Begin Building'}
		</button>
		<button
			type="button"
			class="link"
			on:click={() => dispatch('open-expert')}
			data-testid="simple-start-expert"
		>
			Open Expert mode
		</button>
	</div>
</section>

<style>
	.simple-start {
		max-width: 720px;
		margin: 4rem auto;
		padding: 2.5rem;
		border-radius: 1rem;
		background: var(--surface, #ffffff);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04), 0 8px 24px rgba(0, 0, 0, 0.06);
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}
	h1 {
		font-size: 1.75rem;
		margin: 0 0 0.5rem;
		font-weight: 600;
	}
	.hint {
		margin: 0;
		color: var(--muted, #555);
		font-size: 0.95rem;
		line-height: 1.5;
	}
	.prompt textarea {
		width: 100%;
		font: inherit;
		font-size: 1rem;
		padding: 0.85rem 1rem;
		border-radius: 0.5rem;
		border: 1px solid var(--border, #d0d4dc);
		background: var(--input, #fafbfd);
		resize: vertical;
		min-height: 7.5rem;
	}
	.prompt textarea:focus {
		outline: 2px solid var(--accent, #4a6cf7);
		outline-offset: 1px;
	}
	.actions {
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}
	button {
		font: inherit;
		padding: 0.6rem 1.1rem;
		border-radius: 0.45rem;
		border: 1px solid transparent;
		cursor: pointer;
	}
	button:disabled {
		cursor: not-allowed;
		opacity: 0.55;
	}
	.primary {
		background: var(--accent, #4a6cf7);
		color: white;
		font-weight: 600;
	}
	.voice {
		background: var(--surface-2, #f1f3f8);
		border-color: var(--border, #d0d4dc);
	}
	.voice.recording {
		background: #f7d4d4;
		border-color: #d04848;
		color: #6e1f1f;
	}
	.link {
		background: transparent;
		color: var(--accent, #4a6cf7);
		margin-left: auto;
		text-decoration: underline;
	}
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		border: 0;
	}
</style>
