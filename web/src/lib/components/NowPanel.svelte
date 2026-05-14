<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type {
		AgentProfile,
		AgentRole,
		AskAnswer,
		AutopilotState,
		CassetteRibbon as CassetteRibbonType,
		ArchCheckReport,
		HealthResponse,
		LadderState,
		PkgProvidesResult,
		ReleaseStatus,
		RoleOverrides,
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
	export let cassetteRibbon: CassetteRibbonType | null = null;
	export let agents: AgentProfile[] = [];
	export let roleOverrides: RoleOverrides | null = null;
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
		visualParse: { kind: VisualKind; source: unknown };
		visualEmit: { kind: VisualKind; graph: unknown };
		pkgSearch: string;
		roleOverrides: RoleOverrides;
		proofReport: string;
		reproveTrust: void;
	}>();

	function componentAvailable(id: string) {
		return (health?.components ?? []).some((component) => component.id === id && component.status === 'available');
	}

	$: compareAvailable = componentAvailable('claude-code') && componentAvailable('codex');
	$: drawerItems = [
		{ id: 'now', title: 'Now', open: !session },
		{ id: 'try', title: 'Try It', open: false },
		{ id: 'ladder', title: 'Shipping Ladder', open: true },
		{ id: 'cassette', title: 'Cassette ribbon', open: false },
		{ id: 'ask', title: 'Ask the project', open: false },
		{ id: 'visual', title: 'Visual editor', open: false }
	];
	$: trustComputing = !!session && session.phase !== 'intent_drafting' && trustPosture == null;

	function resolved(role: AgentRole) {
		const override = roleOverrides?.[role];
		if (override) return override;
		return agents.find((agent) => agent.default_role === role || agent.eligible_roles.includes(role))?.id ?? '';
	}

	function updateOverride(role: AgentRole, agentId: string) {
		dispatch('roleOverrides', {
			schema_version: 'x07.studio.role_overrides@0.1.0',
			...(roleOverrides ?? {}),
			[role]: agentId || null
		});
	}
</script>

<aside class="now-panel" data-testid="now-panel">
	<TrustCard
		posture={trustPosture}
		isComputing={trustComputing}
		on:report={(event) => dispatch('proofReport', event.detail)}
		on:reprove={() => dispatch('reproveTrust')}
	/>

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
				Second opinion
			</button>
			<section class="role-overrides" data-testid="role-overrides">
				<header>
					<h3>Roles</h3>
					<span>Session routing</span>
				</header>
				{#each ['architect', 'coder', 'reviewer'] as role}
					<label>
						<span>{role}</span>
						<select
							value={resolved(role as AgentRole)}
							disabled={!session || busy}
							on:change={(event) => updateOverride(role as AgentRole, (event.currentTarget as HTMLSelectElement).value)}
						>
							<option value="">Default</option>
							{#each agents as agent}
								<option value={agent.id}>{agent.label}</option>
							{/each}
						</select>
					</label>
				{/each}
			</section>
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

<style>
	.role-overrides {
		display: grid;
		gap: 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 9px;
		background: rgba(255, 255, 255, 0.03);
	}
	.role-overrides header {
		display: flex;
		justify-content: space-between;
		gap: 8px;
	}
	.role-overrides h3 {
		margin: 0;
		font-size: 13px;
	}
	.role-overrides label {
		display: grid;
		grid-template-columns: 86px minmax(0, 1fr);
		gap: 8px;
		align-items: center;
	}
	.role-overrides span {
		color: var(--muted);
		font-size: 11px;
		text-transform: uppercase;
	}
	.role-overrides select {
		min-width: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(15, 23, 42, 0.8);
		color: var(--text);
		padding: 6px 8px;
	}
</style>
