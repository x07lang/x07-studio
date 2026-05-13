<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type {
		AskAnswer,
		AutopilotState,
		CassetteEntry,
		CassetteRibbon as CassetteRibbonType,
		ArchCheckReport,
		HealthResponse,
		LadderState,
		PkgProvidesResult,
		ReleaseStatus,
		SessionSnapshot,
		TrustPosture,
		TryItRequest,
		TryItResult,
		VisualKind,
		VisualResponse
	} from '$lib/studio';
	import { describePhase } from '$lib/plainEnglish';
	import TryItPanel from './TryItPanel.svelte';
	import ShippingLadder from './ShippingLadder.svelte';
	import VisualEditor from './VisualEditor.svelte';
	import TrustCard from './TrustCard.svelte';
	import CassetteRibbon from './CassetteRibbon.svelte';
	import DrawerRail from './DrawerRail.svelte';
	import ModuleSearch from './ModuleSearch.svelte';

	export let session: SessionSnapshot | null = null;
	export let health: HealthResponse | null = null;
	export let ladder: LadderState | null = null;
	export let tryResult: TryItResult | null = null;
	export let askAnswer: AskAnswer | null = null;
	export let autopilot: AutopilotState | null = null;
	export let releaseStatus: ReleaseStatus | null = null;
	export let pkgProvidesResult: PkgProvidesResult | null = null;
	export let archCheckReport: ArchCheckReport | null = null;
	export let cassettes: CassetteEntry[] = [];
	export let cassetteRibbon: CassetteRibbonType | null = null;
	export let trustPosture: TrustPosture | null = null;
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
		certificate: void;
		exportReplay: void;
		cassetteLoad: void;
		cassetteBranch: { idx: number; title: string };
		visualParse: { kind: VisualKind; source: unknown };
		visualEmit: { kind: VisualKind; graph: unknown };
		pkgSearch: string;
	}>();

	function branchTitle(entry: CassetteEntry) {
		return `Replay ${entry.key}`;
	}

	function componentAvailable(id: string) {
		return (health?.components ?? []).some((component) => component.id === id && component.status === 'available');
	}

	$: compareAvailable = componentAvailable('claude-code') && componentAvailable('codex');
	$: drawerItems = [
		{ id: 'now', title: 'Now', open: !session },
		{ id: 'try', title: 'Try It', open: false },
		{ id: 'ladder', title: 'Shipping Ladder', open: true },
		{ id: 'cassette', title: 'Cassette ribbon', open: false },
		{ id: 'time', title: 'Time travel', open: false },
		{ id: 'ask', title: 'Ask the project', open: false },
		{ id: 'visual', title: 'Visual editor', open: false }
	];
</script>

<aside class="now-panel" data-testid="now-panel">
	<TrustCard posture={trustPosture} />

	<DrawerRail items={drawerItems}>
		<section slot="now" class="now-card status-card">
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
			<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('scan')}>Scan incidents</button>
			<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('sync')}>Continue elsewhere</button>
			<div class="inline-form">
				<input bind:value={syncClaim} placeholder="Sync code" aria-label="Sync code" />
				<button type="button" class="command-button" disabled={busy || !syncClaim.trim()} on:click={() => dispatch('claimSync', syncClaim)}>Claim</button>
			</div>
			<button type="button" class="command-button" disabled={!session || busy || !compareAvailable} on:click={() => dispatch('quorum')} data-testid="run-quorum">
				Compare both agents
			</button>
			<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('exportReplay')}>Export replay</button>
		</section>

		<div slot="try">
			<TryItPanel result={tryResult} {busy} on:invoke={(event) => dispatch('invoke', event.detail)} />
		</div>

		<div slot="ladder">
			<ShippingLadder
				{ladder}
				{busy}
				{releaseStatus}
				{archCheckReport}
				on:climb={(event) => dispatch('climb', event.detail)}
				on:release={(event) => dispatch('release', event.detail)}
				on:certificate={() => dispatch('certificate')}
			/>
		</div>

		<div slot="cassette">
			<CassetteRibbon ribbon={cassetteRibbon} />
		</div>

		<section slot="time" class="now-card cassette-card">
			<header>
				<h2>Time travel</h2>
				<span>{cassettes.length} entries</span>
			</header>
			<button type="button" class="command-button" disabled={!session || busy} on:click={() => dispatch('cassetteLoad')}>Load cassettes</button>
			{#if cassettes.length}
				<div class="cassette-list" data-testid="cassette-list">
					{#each cassettes.slice(0, 5) as entry}
						<div>
							<span><code>#{entry.idx}</code> {entry.key}</span>
							<button type="button" class="command-button" disabled={busy} on:click={() => dispatch('cassetteBranch', { idx: entry.idx, title: branchTitle(entry) })}>Branch</button>
							<small>{entry.kind} - {entry.size_bytes} bytes</small>
						</div>
					{/each}
				</div>
			{/if}
		</section>

		<section slot="ask" class="now-card ask-card">
			<header>
				<h2>Ask the project</h2>
			</header>
			<textarea bind:value={question} rows="3" placeholder="What did verify prove?"></textarea>
			<button type="button" class="command-button" disabled={!session || busy || !question.trim()} on:click={() => dispatch('ask', question)}>Ask</button>
			<ModuleSearch result={pkgProvidesResult} {busy} on:search={(event) => dispatch('pkgSearch', event.detail)} />
			{#if askAnswer}
				<p>{askAnswer.text}</p>
				<ul>
					{#each askAnswer.citations as citation}
						<li><code>{citation.path}</code> {citation.locator}</li>
					{/each}
				</ul>
			{/if}
		</section>

		<div slot="visual">
			<VisualEditor
				{session}
				{busy}
				parseResult={visualParseResult}
				emitResult={visualEmitResult}
				on:parse={(event) => dispatch('visualParse', event.detail)}
				on:emit={(event) => dispatch('visualEmit', event.detail)}
			/>
		</div>
	</DrawerRail>
</aside>
