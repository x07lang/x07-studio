<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let busy = false;
	export let placeholder = 'Describe what this project should do';

	let text = '';
	let imageName = '';

	const dispatch = createEventDispatcher<{
		compose: { text: string };
		image: { file: File };
	}>();

	function submit() {
		const value = text.trim();
		if (!value || busy) return;
		dispatch('compose', { text: value });
		text = '';
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
</script>

<footer class="composer" on:drop={onDrop} on:dragover|preventDefault data-testid="composer">
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
		<label class="command-button file-button">
			<input type="file" accept="image/*" on:change={onFile} />
			{imageName || 'Add image'}
		</label>
		<button type="button" class="command-button primary" on:click={submit} disabled={busy || !text.trim()} data-testid="composer-submit">
			Send
		</button>
	</div>
</footer>
