<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { TryItRequest, TryItResult } from '$lib/studio';

	export let result: TryItResult | null = null;
	export let busy = false;

	let input = '';
	let argv = '';
	let mode: TryItRequest['input_kind'] = 'text';

	const dispatch = createEventDispatcher<{
		invoke: TryItRequest;
	}>();

	function run() {
		const req: TryItRequest = {
			input_kind: mode,
			input_text: mode === 'text' ? input : null,
			input_b64: mode === 'b64' ? input : null,
			input_path: mode === 'file' ? input : null,
			argv: mode === 'argv' ? argv.split(/\s+/u).filter(Boolean) : [],
			profile: 'sandbox'
		};
		dispatch('invoke', req);
	}

	function shellCommand() {
		const value = input || '<your input here>';
		return `printf "%s" "${value.replaceAll('"', '\\"')}" | x07 run --project x07.json --profile sandbox --stdin`;
	}
</script>

<section class="now-card" data-testid="try-it-panel">
	<header>
		<h2>Try It</h2>
	</header>
	<div class="segmented">
		{#each ['text', 'file', 'b64', 'argv'] as item}
			<button type="button" class:active={mode === item} on:click={() => (mode = item as TryItRequest['input_kind'])}>
				{item}
			</button>
		{/each}
	</div>
	{#if mode === 'argv'}
		<input bind:value={argv} placeholder="--flag value" />
	{:else}
		<textarea bind:value={input} rows="4" placeholder={mode === 'file' ? 'relative/input.bin' : 'input'}></textarea>
	{/if}
	<div class="button-row">
		<button type="button" class="command-button primary" on:click={run} disabled={busy}>Run it</button>
		<code>{shellCommand()}</code>
	</div>
	{#if result}
		<div class="try-output">
			<h3>Output</h3>
			<pre>{result.output_json ? JSON.stringify(result.output_json, null, 2) : result.output_text}</pre>
			{#if result.proof_citations.length}
				<ul>
					{#each result.proof_citations as citation}
						<li>{citation.summary} <code>{citation.clause_id}</code></li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</section>
