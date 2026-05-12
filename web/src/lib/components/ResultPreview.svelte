<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { PlainEnglishSummary, TryItRequest, TryItResult } from '$lib/studio';

	export let summary: PlainEnglishSummary;
	export let tryResult: TryItResult | null = null;
	export let busy = false;

	let copied = false;
	let showDetails = false;
	let tryInput = '';

	const dispatch = createEventDispatcher<{
		followup: string;
		invoke: TryItRequest;
		realize: void;
	}>();

	async function copyInvocation() {
		if (!summary.run_invocation) return;
		try {
			await navigator.clipboard.writeText(summary.run_invocation);
			copied = true;
			setTimeout(() => (copied = false), 1600);
		} catch {
			copied = false;
		}
	}

	function runIt() {
		const text = tryInput.trim();
		if (!text) return;
		dispatch('invoke', {
			input_kind: 'text',
			input_text: text,
			input_b64: null,
			input_path: null,
			argv: [],
			profile: null
		});
	}

	function formatOutput(result: TryItResult): string {
		if (result.output_text) return result.output_text;
		if (result.output_json) return JSON.stringify(result.output_json, null, 2);
		return '(no output)';
	}
</script>

<section class="result-preview" data-testid="result-preview" data-scaffold-only={summary.scaffold_only ? 'true' : 'false'}>
	<header>
		<h2 data-testid="summary-headline">{summary.headline}</h2>
	</header>

	{#if summary.scaffold_only}
		<section class="realize-cta" data-testid="realize-cta">
			<p class="hint">
				The spec compiled and verify passed against a placeholder body, but the
				implementation under <code>src/</code> is still a stub. Have Claude Code
				fill it in — Studio will rerun <code>impl.check</code> + <code>xtal.verify</code>
				after the agent finishes and surface a fresh Verified turn.
			</p>
			{#if summary.stub_paths && summary.stub_paths.length}
				<ul class="stub-list">
					{#each summary.stub_paths as path}
						<li><code>{path}</code></li>
					{/each}
				</ul>
			{/if}
			<button
				type="button"
				class="command-button primary"
				on:click={() => dispatch('realize')}
				disabled={busy}
				data-testid="realize-cta-button"
			>
				{busy ? 'Claude Code is implementing…' : 'Implement with Claude Code'}
			</button>
		</section>
	{/if}

	{#if summary.behavior_promises.length}
		<div class="result-block">
			<h3>What it does</h3>
			<ul>
				{#each summary.behavior_promises as item}
					<li>{item}</li>
				{/each}
			</ul>
		</div>
	{/if}

	{#if summary.boundaries.length}
		<div class="result-block">
			<h3>Boundaries</h3>
			<ul>
				{#each summary.boundaries as item}
					<li>{item}</li>
				{/each}
			</ul>
		</div>
	{/if}

	{#if summary.evidence.length}
		<div class="result-block">
			<h3>Evidence</h3>
			<ul>
				{#each summary.evidence as item}
					<li>{item}</li>
				{/each}
			</ul>
		</div>
	{/if}

	{#if summary.run_invocation}
		<section class="run-it" data-testid="run-invocation">
			<div>
				<h3>Run it from your terminal</h3>
				<button class="command-button" type="button" on:click={copyInvocation}>
					{copied ? 'Copied' : 'Copy'}
				</button>
			</div>
			<code>{summary.run_invocation}</code>
			<button class="link-button" type="button" on:click={() => (showDetails = !showDetails)}>
				What does this do?
			</button>
			{#if showDetails}
				<p>
					It sends the example input to the verified x07 project through the sandbox profile.
				</p>
			{/if}
		</section>

		<section class="try-inline" data-testid="try-inline">
			<header>
				<h3>Or try it right here</h3>
			</header>
			<div class="try-row">
				<input
					type="text"
					bind:value={tryInput}
					placeholder="Type input and press Run"
					data-testid="try-inline-input"
					on:keydown={(event) => {
						if (event.key === 'Enter') {
							event.preventDefault();
							runIt();
						}
					}}
					disabled={busy}
				/>
				<button
					type="button"
					class="command-button primary"
					on:click={runIt}
					disabled={busy || !tryInput.trim()}
					data-testid="try-inline-run"
				>
					{busy ? 'Running…' : 'Run it'}
				</button>
			</div>
			{#if tryResult}
				<div
					class="try-output"
					data-testid="try-inline-output"
					data-kind={tryResult.output_kind}
				>
					<header>
						<span class="kind-label">{tryResult.output_kind}</span>
						{#if tryResult.proof_citations.length}
							<span class="proof-chip"
								>✓ proved by {tryResult.proof_citations.length} clause{tryResult
									.proof_citations.length === 1
									? ''
									: 's'}</span
							>
						{/if}
					</header>
					<pre>{formatOutput(tryResult)}</pre>
				</div>
			{/if}
		</section>
	{/if}

	{#if summary.followups.length}
		<section class="followups" data-testid="followups">
			<h3>Keep going</h3>
			<div>
				{#each summary.followups as followup}
					<button type="button" class="command-button" on:click={() => dispatch('followup', followup)}>
						{followup}
					</button>
				{/each}
			</div>
		</section>
	{/if}
</section>

<style>
	.realize-cta {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0.75rem 0.9rem;
		border-radius: 0.6rem;
		border: 1px solid rgba(245, 166, 35, 0.4);
		background: rgba(245, 166, 35, 0.08);
	}
	.realize-cta .hint {
		margin: 0;
		font-size: 0.9rem;
		line-height: 1.4;
		color: var(--text, #eef1f6);
	}
	.realize-cta .stub-list {
		margin: 0;
		padding-left: 1.1rem;
		font-size: 0.85rem;
		color: var(--muted, #aab1c0);
	}
	.realize-cta .stub-list code {
		font-family: var(--font-mono, ui-monospace, monospace);
	}
	@media (prefers-color-scheme: light) {
		.realize-cta {
			background: rgba(245, 166, 35, 0.14);
		}
		.realize-cta .hint {
			color: #1b1f2a;
		}
	}
	.try-inline {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		margin-top: 0.65rem;
		padding-top: 0.65rem;
		border-top: 1px solid rgba(148, 163, 184, 0.18);
	}
	.try-inline header h3 {
		font-size: 0.75rem;
		font-weight: 600;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		margin: 0;
		color: var(--muted, #aab1c0);
	}
	.try-row {
		display: flex;
		gap: 0.5rem;
		align-items: stretch;
	}
	.try-row input {
		flex: 1;
		font: inherit;
		padding: 0.5rem 0.7rem;
		border-radius: 0.4rem;
		border: 1px solid rgba(148, 163, 184, 0.35);
		background: rgba(15, 18, 24, 0.65);
		color: var(--text, #eef1f6);
	}
	.try-row input::placeholder {
		color: rgba(148, 163, 184, 0.6);
	}
	.try-row input:disabled {
		opacity: 0.55;
	}
	.try-output {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		padding: 0.55rem 0.75rem;
		border-radius: 0.5rem;
		background: rgba(15, 18, 24, 0.55);
		border: 1px solid rgba(148, 163, 184, 0.2);
	}
	.try-output header {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		font-size: 0.7rem;
	}
	.try-output .kind-label {
		color: var(--muted, #aab1c0);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.try-output .proof-chip {
		color: #2bbf6b;
	}
	.try-output[data-kind='not_verified'] .proof-chip {
		color: #f5a623;
	}
	.try-output pre {
		margin: 0;
		font-family: var(--font-mono, ui-monospace, monospace);
		font-size: 0.85rem;
		white-space: pre-wrap;
		word-break: break-word;
		color: var(--text, #eef1f6);
	}
	@media (prefers-color-scheme: light) {
		.try-row input {
			background: #ffffff;
			color: #1b1f2a;
			border-color: #d0d4dc;
		}
		.try-row input::placeholder {
			color: #6b7280;
		}
		.try-output {
			background: #f8fafc;
			border-color: #d0d4dc;
		}
		.try-output pre {
			color: #1b1f2a;
		}
	}
</style>
