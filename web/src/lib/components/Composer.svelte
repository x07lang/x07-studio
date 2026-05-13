<script lang="ts">
	import { onDestroy } from 'svelte';
	import { createEventDispatcher } from 'svelte';
	import { createVoiceCapture } from '$lib/voice';
	import type { VoiceTranscript } from '$lib/studio';
	import VoiceTranscriptView from './VoiceTranscript.svelte';

	export let busy = false;
	export let placeholder = 'Tell me what to build. Mic for voice. ⌘↵ to send.';
	export let incidentCount = 0;
	export let autopilotActive = false;

	let text = '';
	let imageName = '';
	let auto = true;
	let listening = false;
	let passiveListening = false;
	let voiceText = '';
	let voiceConfidence = 0;
	let voice = createVoiceCapture();
	let interruptVoice = createVoiceCapture('en-US', true);

	const dispatch = createEventDispatcher<{
		compose: { text: string; auto: boolean; voiceTranscript?: VoiceTranscript | null };
		image: { file: File };
		pauseAutopilot: void;
	}>();

	interruptVoice.onMatch(['wait stop', 'pause autopilot'], () => {
		dispatch('pauseAutopilot');
		passiveListening = false;
		interruptVoice.stop();
	});

	voice.onPartial((value, confidence) => {
		voiceText = value;
		voiceConfidence = confidence;
		text = value;
	});
	voice.onFinal((value, confidence) => {
		voiceText = value;
		voiceConfidence = confidence;
		text = value;
		listening = false;
	});

	$: if (autopilotActive && interruptVoice.supported && !passiveListening) {
		passiveListening = true;
		interruptVoice.start();
	}

	$: if (!autopilotActive && passiveListening) {
		passiveListening = false;
		interruptVoice.stop();
	}

	onDestroy(() => {
		voice.stop();
		interruptVoice.stop();
	});

	function submit() {
		const value = text.trim();
		if (!value || busy) return;
		dispatch('compose', {
			text: value,
			auto,
			voiceTranscript: voiceText
				? {
						schema_version: 'x07.studio.voice_transcript@0.1.0',
						text: voiceText,
						confidence: voiceConfidence,
						language: navigator.language || 'en-US',
						recorded_at: String(Date.now())
				  }
				: null
		});
		text = '';
		voiceText = '';
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
			event.preventDefault();
			submit();
		}
	}

	function onDrop(event: DragEvent) {
		event.preventDefault();
		const file = event.dataTransfer?.files?.[0];
		if (!file) return;
		imageName = file.name;
		dispatch('image', { file });
	}

	function onFile(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		imageName = file.name;
		dispatch('image', { file });
		input.value = '';
	}

	function toggleVoice() {
		if (!voice.supported || busy) return;
		if (listening) {
			voice.stop();
			listening = false;
		} else {
			listening = true;
			voice.start();
		}
	}
</script>

<footer class="composer" on:drop={onDrop} on:dragover|preventDefault data-testid="composer">
	<VoiceTranscriptView text={voiceText} confidence={voiceConfidence} supported={voice.supported} />
	{#if incidentCount > 0}
		<p class="incident-badge" data-testid="incident-badge">
			<span class="dot" aria-hidden="true"></span>
			{incidentCount} new incident{incidentCount === 1 ? '' : 's'}
		</p>
	{/if}
	{#if passiveListening}
		<p class="listen-badge" data-testid="passive-listen">Listening for stop words</p>
	{/if}
	<label class="composer-field">
		<span class="sr-only">Composer</span>
		<textarea
			bind:value={text}
			{placeholder}
			rows="2"
			on:keydown={onKeydown}
			data-testid="composer-input"
			disabled={busy}
		></textarea>
	</label>
	<div class="composer-actions">
		<button
			type="button"
			class="mic-button"
			class:recording={listening}
			class:idle={!listening}
			on:click={toggleVoice}
			disabled={busy || !voice.supported}
			data-testid="composer-mic"
			title={listening ? 'Stop voice' : voice.supported ? 'Voice input' : 'Voice unsupported'}
		>
			<span class="mic-dot" aria-hidden="true"></span>
			{listening ? 'Stop' : 'Mic'}
		</button>
		<label class="auto-toggle" title="Run autopilot — drive verify, repair, climb without prompting">
			<input type="checkbox" bind:checked={auto} />
			<span class="auto-track" aria-hidden="true">
				<span class="auto-knob"></span>
			</span>
			<span class="auto-label">Auto</span>
		</label>
		<label class="image-button" title="Attach a screenshot or sketch">
			<input type="file" accept="image/*" on:change={onFile} />
			<span>{imageName || 'Image'}</span>
		</label>
		<button
			type="button"
			class="send-button"
			on:click={submit}
			disabled={busy || !text.trim()}
			data-testid="composer-submit"
			title="⌘↵ Send"
		>
			Send
		</button>
	</div>
</footer>

<style>
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}

	.composer-field textarea {
		font-family: var(--font-mono);
		font-size: 14px;
		line-height: 1.45;
		min-height: 56px;
		transition: border-color var(--motion-fast, 120ms) var(--ease-out, ease-out),
			box-shadow var(--motion-fast, 120ms) var(--ease-out, ease-out);
	}
	.composer-field textarea::placeholder {
		color: var(--muted);
		opacity: 0.85;
	}
	.composer-field textarea:focus {
		outline: none;
		border-color: var(--accent-pure);
		box-shadow: 0 0 0 3px rgba(114, 228, 180, 0.12);
	}

	.composer-actions {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.mic-button,
	.send-button,
	.image-button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.04);
		color: var(--text);
		padding: 7px 14px;
		font-size: 12px;
		font-weight: 600;
		letter-spacing: 0.01em;
		cursor: pointer;
		transition: border-color var(--motion-fast, 120ms) var(--ease-out, ease-out),
			background var(--motion-fast, 120ms) var(--ease-out, ease-out),
			transform var(--motion-fast, 120ms) var(--ease-out, ease-out);
	}
	.mic-button:hover:not(:disabled),
	.send-button:hover:not(:disabled),
	.image-button:hover {
		border-color: var(--border-strong);
	}
	.mic-button:active:not(:disabled),
	.send-button:active:not(:disabled) {
		transform: translateY(1px);
	}

	.mic-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--accent-pure);
		opacity: 0.55;
		transition: opacity var(--motion-fast, 120ms) var(--ease-out, ease-out),
			box-shadow var(--motion-base, 200ms) var(--ease-out, ease-out);
	}
	.mic-button.recording {
		border-color: var(--accent-danger);
		background: rgba(243, 111, 141, 0.16);
		color: var(--accent-danger);
	}
	.mic-button.recording .mic-dot {
		background: var(--accent-danger);
		opacity: 1;
		box-shadow: 0 0 0 4px rgba(243, 111, 141, 0.22);
		animation: pulse 1.4s ease-in-out infinite;
	}
	@keyframes pulse {
		0%, 100% { box-shadow: 0 0 0 0 rgba(243, 111, 141, 0.4); }
		50%      { box-shadow: 0 0 0 7px rgba(243, 111, 141, 0); }
	}

	.send-button {
		background: var(--accent-pure);
		color: #0b0d10;
		border-color: var(--accent-pure);
	}
	.send-button:hover:not(:disabled) {
		filter: brightness(1.08);
	}
	.send-button:disabled {
		background: rgba(255, 255, 255, 0.04);
		border-color: var(--border);
		color: var(--muted);
		filter: none;
	}

	.image-button {
		position: relative;
	}
	.image-button input {
		position: absolute;
		inset: 0;
		opacity: 0;
		cursor: pointer;
	}
	.image-button span {
		font-family: var(--font-mono);
		font-size: 11px;
	}

	.auto-toggle {
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 8px;
		cursor: pointer;
		user-select: none;
	}
	.auto-toggle input {
		position: absolute;
		inset: 0;
		z-index: 1;
		width: 100%;
		height: 100%;
		margin: 0;
		opacity: 0;
		cursor: pointer;
	}
	.auto-track {
		position: relative;
		width: 30px;
		height: 16px;
		border-radius: 999px;
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid var(--border);
		transition: background var(--motion-fast, 120ms) var(--ease-out, ease-out);
		pointer-events: none;
	}
	.auto-knob {
		position: absolute;
		top: 1px;
		left: 1px;
		width: 12px;
		height: 12px;
		border-radius: 50%;
		background: var(--muted);
		transition: transform var(--motion-fast, 120ms) var(--ease-out, ease-out),
			background var(--motion-fast, 120ms) var(--ease-out, ease-out);
	}
	.auto-toggle input:checked ~ .auto-track {
		background: rgba(114, 228, 180, 0.22);
		border-color: var(--accent-pure);
	}
	.auto-toggle input:checked ~ .auto-track .auto-knob {
		transform: translateX(14px);
		background: var(--accent-pure);
	}
	.auto-toggle input:focus-visible ~ .auto-track {
		outline: 2px solid var(--accent-pure);
		outline-offset: 2px;
	}
	.auto-label {
		font-size: 12px;
		font-weight: 600;
		color: var(--muted);
		pointer-events: none;
	}
	.auto-toggle input:checked ~ .auto-label {
		color: var(--text);
	}

	.incident-badge {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		margin: 0 0 2px;
		padding: 4px 10px;
		border: 1px solid rgba(243, 111, 141, 0.42);
		border-radius: 999px;
		background: rgba(243, 111, 141, 0.08);
		color: var(--accent-danger);
		font-size: 12px;
		justify-self: start;
	}
	.listen-badge {
		margin: 0;
		color: var(--cyan);
		font-size: 12px;
	}
	.incident-badge .dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--accent-danger);
		animation: pulse 1.6s ease-in-out infinite;
	}
</style>
