<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { AskAnswer, LadderState, SessionSnapshot, TryItRequest, TryItResult } from '$lib/studio';
	import { describePhase } from '$lib/plainEnglish';
	import TryItPanel from './TryItPanel.svelte';
	import ShippingLadder from './ShippingLadder.svelte';

	export let session: SessionSnapshot | null = null;
	export let ladder: LadderState | null = null;
	export let tryResult: TryItResult | null = null;
	export let askAnswer: AskAnswer | null = null;
	export let busy = false;

	let question = '';

	const dispatch = createEventDispatcher<{
		build: void;
		invoke: TryItRequest;
		climb: string;
		scan: void;
		ask: string;
		sync: void;
	}>();
</script>

<aside class="now-panel" data-testid="now-panel">
	<section class="now-card status-card">
		<header>
			<h2>Now</h2>
			<span>{session ? describePhase(session.phase) : 'No session'}</span>
		</header>
		<button type="button" class="command-button primary" disabled={!session || busy} on:click={() => dispatch('build')} data-testid="approve-build">
			Approve &amp; Build
		</button>
		<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('scan')}>
			Scan incidents
		</button>
		<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('sync')}>
			Continue elsewhere
		</button>
	</section>

	<TryItPanel result={tryResult} {busy} on:invoke={(event) => dispatch('invoke', event.detail)} />
	<ShippingLadder {ladder} {busy} on:climb={(event) => dispatch('climb', event.detail)} />

	<section class="now-card ask-card">
		<header>
			<h2>Ask the project</h2>
		</header>
		<textarea bind:value={question} rows="3" placeholder="What did verify prove?"></textarea>
		<button type="button" class="command-button" disabled={!session || busy || !question.trim()} on:click={() => dispatch('ask', question)}>
			Ask
		</button>
		{#if askAnswer}
			<p>{askAnswer.text}</p>
			<ul>
				{#each askAnswer.citations as citation}
					<li><code>{citation.path}</code> {citation.locator}</li>
				{/each}
			</ul>
		{/if}
	</section>
</aside>
