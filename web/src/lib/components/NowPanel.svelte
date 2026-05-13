<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type {
		AskAnswer,
		AutopilotState,
		CassetteEntry,
		LadderState,
		ReleaseStatus,
		SessionSnapshot,
		TryItRequest,
		TryItResult,
		VisualKind,
		VisualResponse
	} from '$lib/studio';
	import { describePhase } from '$lib/plainEnglish';
	import TryItPanel from './TryItPanel.svelte';
	import ShippingLadder from './ShippingLadder.svelte';
	import VisualEditor from './VisualEditor.svelte';

	export let session: SessionSnapshot | null = null;
	export let ladder: LadderState | null = null;
	export let tryResult: TryItResult | null = null;
	export let askAnswer: AskAnswer | null = null;
	export let autopilot: AutopilotState | null = null;
	export let releaseStatus: ReleaseStatus | null = null;
	export let cassettes: CassetteEntry[] = [];
	export let visualParseResult: VisualResponse | null = null;
	export let visualEmitResult: VisualResponse | null = null;
	export let busy = false;

	let question = '';
	let syncClaim = '';

	const dispatch = createEventDispatcher<{
		build: void;
		invoke: TryItRequest;
		climb: string;
		scan: void;
		ask: string;
		sync: void;
		claimSync: string;
		quorum: void;
		autopilot: void;
		pauseAutopilot: void;
		release: string;
		exportReplay: void;
		cassetteLoad: void;
		cassetteBranch: { idx: number; title: string };
		visualParse: { kind: VisualKind; source: unknown };
		visualEmit: { kind: VisualKind; graph: unknown };
	}>();

	function branchTitle(entry: CassetteEntry) {
		return `Replay ${entry.key}`;
	}
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
		<button type="button" class="command-button primary" disabled={!session || busy} on:click={() => dispatch('autopilot')} data-testid="start-autopilot">
			Run autopilot
		</button>
		{#if autopilot?.last_decision}
			<div class="autopilot-state" data-testid="autopilot-state">
				<strong>{autopilot.last_decision.stage}</strong>
				<span>{autopilot.last_decision.reason}</span>
				<button type="button" class="link-button" disabled={busy} on:click={() => dispatch('pauseAutopilot')}>Pause</button>
			</div>
		{/if}
		<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('scan')}>
			Scan incidents
		</button>
		<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('sync')}>
			Continue elsewhere
		</button>
		<div class="inline-form">
			<input bind:value={syncClaim} placeholder="Sync code" aria-label="Sync code" />
			<button type="button" class="command-button" disabled={busy || !syncClaim.trim()} on:click={() => dispatch('claimSync', syncClaim)}>
				Claim
			</button>
		</div>
		<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('quorum')} data-testid="run-quorum">
			Compare both agents
		</button>
		<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('exportReplay')}>
			Export replay
		</button>
	</section>

	<TryItPanel result={tryResult} {busy} on:invoke={(event) => dispatch('invoke', event.detail)} />
	<ShippingLadder
		{ladder}
		{busy}
		{releaseStatus}
		on:climb={(event) => dispatch('climb', event.detail)}
		on:release={(event) => dispatch('release', event.detail)}
	/>

	<section class="now-card cassette-card">
		<header>
			<h2>Time travel</h2>
			<span>{cassettes.length} entries</span>
		</header>
		<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('cassetteLoad')}>
			Load cassettes
		</button>
		{#if cassettes.length}
			<div class="cassette-list" data-testid="cassette-list">
				{#each cassettes.slice(0, 5) as entry}
					<div>
						<span><code>#{entry.idx}</code> {entry.key}</span>
						<button
							type="button"
							class="command-button"
							disabled={busy}
							on:click={() => dispatch('cassetteBranch', { idx: entry.idx, title: branchTitle(entry) })}
						>
							Branch
						</button>
						<small>{entry.kind} - {entry.size_bytes} bytes</small>
					</div>
				{/each}
			</div>
		{/if}
	</section>

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

	<VisualEditor
		{session}
		{busy}
		parseResult={visualParseResult}
		emitResult={visualEmitResult}
		on:parse={(event) => dispatch('visualParse', event.detail)}
		on:emit={(event) => dispatch('visualEmit', event.detail)}
	/>
</aside>
