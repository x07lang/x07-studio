<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { PlainEnglishSummary } from '$lib/studio';

	export let summary: PlainEnglishSummary;

	let copied = false;
	let showDetails = false;

	const dispatch = createEventDispatcher<{
		followup: string;
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
</script>

<section class="result-preview" data-testid="result-preview">
	<header>
		<h2 data-testid="summary-headline">{summary.headline}</h2>
	</header>

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
