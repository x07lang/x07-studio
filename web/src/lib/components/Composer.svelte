<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { createVoiceCapture } from '$lib/voice';
	import type { VoiceTranscript } from '$lib/studio';
	import VoiceTranscriptView from './VoiceTranscript.svelte';

	export let busy = false;
	export let placeholder = 'Describe what this project should do';
	export let incidentCount = 0;

	let text = '';
	let imageName = '';
	let auto = true;
	let listening = false;
	let voiceText = '';
	let voiceConfidence = 0;
	let voice = createVoiceCapture();

	const dispatch = createEventDispatcher<{
		compose: { text: string; auto: boolean; voiceTranscript?: VoiceTranscript | null };
		image: { file: File };
	}>();

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
		<p class="incident-badge" data-testid="incident-badge">{incidentCount} new incident{incidentCount === 1 ? '' : 's'}</p>
	{/if}
	<label>
		<span>Composer</span>
		<textarea
			bind:value={text}
			{placeholder}
			rows="3"
			on:keydown={onKeydown}
			data-testid="composer-input"
		></textarea>
	</label>
	<div class="composer-actions">
		<button
			type="button"
			class="command-button primary"
			class:recording={listening}
			on:click={toggleVoice}
			disabled={busy || !voice.supported}
			data-testid="composer-mic"
		>
			{listening ? 'Stop mic' : 'Mic'}
		</button>
		<label class="auto-toggle">
			<input type="checkbox" bind:checked={auto} />
			<span>Auto</span>
		</label>
		<label class="command-button file-button">
			<input type="file" accept="image/*" on:change={onFile} />
			{imageName || 'Add image'}
		</label>
		<button type="button" class="command-button primary" on:click={submit} disabled={busy || !text.trim()} data-testid="composer-submit">
			Send
		</button>
	</div>
</footer>

<style>
	.auto-toggle {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--muted, #aab1c0);
	}
	.recording {
		box-shadow: 0 0 0 3px rgba(248, 113, 113, 0.2);
	}
	.incident-badge {
		margin: 0;
		color: #fca5a5;
		font-size: 0.8rem;
	}
</style>
