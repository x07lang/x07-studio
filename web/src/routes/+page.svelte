<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import './+page.css';
	import { StudioApi } from '$lib/api';
	import type {
		AgentContract,
		AgentProfile,
		ArtifactPreviewResponse,
		AskAnswer,
		ArchCheckReport,
		AutopilotState,
		CassetteRibbon,
		CertificateSummary,
		HealthResponse,
		HealthSnapshot,
		IntentAnswer,
		IntentInputMode,
		LadderState,
		LintReport,
		PbtRound,
		PkgProvidesResult,
		ProcessLane as ProcessLaneType,
		ProofEvidence,
		QuickfixRecord,
		ReleaseStatus,
		ReplayExportResponse,
		RoleOverrides,
		SemanticDiff,
		SessionSummary,
		SessionSnapshot,
		SessionStreamEvent,
		SessionTurn,
		StepEvidence,
		StudioMemory,
		SyncCode,
		TrustPosture,
		TryItRequest,
		TryItResult,
		WhatIfForecast,
		VoiceTranscript,
		VisualKind,
		VisualResponse
	} from '$lib/studio';
	import { demoHealth } from '$lib/studio';
	import Timeline from '$lib/components/Timeline.svelte';
	import ProcessLane from '$lib/components/ProcessLane.svelte';
	import StepDrawer from '$lib/components/StepDrawer.svelte';
	import Composer from '$lib/components/Composer.svelte';
	import NowPanel from '$lib/components/NowPanel.svelte';
	import Header from '$lib/components/Header.svelte';
	import MemoryChip from '$lib/components/MemoryChip.svelte';
	import MemoryDrawer from '$lib/components/MemoryDrawer.svelte';
	import ProofExplorer from '$lib/components/ProofExplorer.svelte';
	import CompareLens from '$lib/components/CompareLens.svelte';
	import CertificateView from '$lib/components/CertificateView.svelte';
	import Welcome from '$lib/components/Welcome.svelte';
	import HealthRow from '$lib/components/HealthRow.svelte';
	import SessionSummaryCard from '$lib/components/SessionSummaryCard.svelte';
	import AgentContractEditor from '$lib/components/AgentContractEditor.svelte';
	import LintReportDrawer from '$lib/components/LintReport.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import { insertOptimistic, reconcile, type OptimisticTurn } from '$lib/store/optimistic';
	import {
		closeCommandPalette,
		commandPaletteOpen,
		openCommandPalette
	} from '$lib/store/commandPalette';
	import type { Recipe } from '$lib/studio';
	import { recipes } from '$lib/recipes';

	const api = new StudioApi();

	let health: HealthResponse = demoHealth();
	let healthSnapshot: HealthSnapshot | null = null;
	let sessions: SessionSnapshot[] = [];
	let selected: SessionSnapshot | null = null;
	let turns: SessionTurn[] = [];
	let processLane: ProcessLaneType | null = null;
	let stepEvidence: StepEvidence | null = null;
	let stepDrawerOpen = false;
	let forecasts: Record<string, WhatIfForecast | null> = {};
	let ladder: LadderState | null = null;
	let tryResult: TryItResult | null = null;
	let askAnswer: AskAnswer | null = null;
	let cassetteRibbon: CassetteRibbon | null = null;
	let visualParseResult: VisualResponse | null = null;
	let visualEmitResult: VisualResponse | null = null;
	let memory: StudioMemory | null = null;
	let agents: AgentProfile[] = [];
	let roleOverrides: RoleOverrides | null = null;
	let syncCode: SyncCode | null = null;
	let autopilot: AutopilotState | null = null;
	let releaseStatus: ReleaseStatus | null = null;
	let replayExport: ReplayExportResponse | null = null;
	let trustPosture: TrustPosture | null = null;
	let proofEvidence: ProofEvidence | null = null;
	let artifactPreview: ArtifactPreviewResponse | null = null;
	let quickfix: QuickfixRecord | null = null;
	let lintReport: LintReport | null = null;
	let lintOpen = false;
	let pbtRound: PbtRound | null = null;
	let archCheckReport: ArchCheckReport | null = null;
	let pkgProvidesResult: PkgProvidesResult | null = null;
	let agentContract: AgentContract | null = null;
	let agentContractOpen = false;
	let semanticDiff: SemanticDiff | null = null;
	let certificate: CertificateSummary | null = null;
	let certificateOpen = false;
	let compareOpen = false;
	let incidentNotice = 0;
	let memoryOpen = false;
	let busy = false;
	let autopilotRunning = false;
	let realizeBusy = false;
	let invokeBusy = false;
	let status = 'Starting Studio';
	let detailsOpen = false;
	let unsubscribe: (() => void) | null = null;
	let optimisticTurns: OptimisticTurn[] = [];
	let recipeStartInFlight: string | null = null;
	let sessionSummaryStatus = '';

	onMount(() => {
		void (async () => {
			const params = new URLSearchParams(window.location.search);
			detailsOpen = params.get('mode') === 'expert' || params.get('details') === 'open';
			await refresh();
			const recipeId = params.get('recipe');
			const recipe = recipes.find((item) => item.id === recipeId);
			if (recipe && !selected) {
				window.history.replaceState(null, '', window.location.pathname);
				await startRecipe(recipe);
			}
			const claim = params.get('claim');
			if (claim) await claimSync(claim);
			if (selected) subscribe(selected.session_id);
		})();
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	function handleGlobalKeydown(event: KeyboardEvent) {
		const key = event.key.toLowerCase();
		if ((event.metaKey || event.ctrlKey) && (key === 'k' || event.code === 'KeyK')) {
			event.preventDefault();
			openCommandPalette();
		}
	}

	async function refresh() {
		health = await api.health();
		healthSnapshot = await api.healthSnapshot().catch(() => null);
		sessions = await api.listSessions();
		agents = await api.listAgents().catch(() => []);
		selected = selected
			? sessions.find((session) => session.session_id === selected?.session_id) ?? sessions[0] ?? null
			: sessions[0] ?? null;
		memory = await api.loadMemory().catch(() => null);
		await refreshDerived();
		status = api.isDemoMode ? 'Demo projection active' : 'Connected to Loom daemon';
	}

	async function refreshDerived() {
		if (!selected) {
			turns = [];
			ladder = null;
			return;
		}
		turns = await api.listTurns(selected.session_id);
		processLane = await api.getProcessLane(selected).catch(() => null);
		for (const turn of turns) optimisticTurns = reconcile(optimisticTurns, turn);
		if (optimisticTurns.length) turns = [...turns, ...optimisticTurns.filter((turn) => turn.optimistic)];
		ladder = await api.ladderState(selected).catch(() => null);
		trustPosture = await api.trustPosture(selected).catch(() => null);
		cassetteRibbon = await api.cassetteRibbon(selected).catch(() => null);
		roleOverrides = await api.getRoleOverrides(selected).catch(() => null);
	}

	function subscribe(sessionId: string) {
		unsubscribe?.();
		unsubscribe = api.subscribeSession(sessionId, (event) => {
			handleStreamEvent(event);
		});
		const session = sessions.find((item) => item.session_id === sessionId) ?? selected;
		if (session) void api.watchIncidents(session).catch(() => null);
	}

	function handleStreamEvent(event: SessionStreamEvent) {
		if (!selected) return;
		if (event.kind === 'snapshot') {
			replaceSelected(event.session);
		} else if (event.kind === 'op') {
			const op_log = [...selected.op_log];
			const index = op_log.findIndex((op) => op.id === event.op.id);
			if (index >= 0) op_log[index] = event.op;
			else op_log.push(event.op);
			if (event.op.op.includes('.incident.')) incidentNotice += 1;
			replaceSelected({ ...selected, op_log });
		}
		void refreshDerived();
	}

	function replaceSelected(session: SessionSnapshot) {
		selected = session;
		sessions = [session, ...sessions.filter((item) => item.session_id !== session.session_id)];
	}

	async function compose(detail: { text: string; auto?: boolean; voiceTranscript?: VoiceTranscript | null }) {
		busy = true;
		try {
			const text = detail.text;
			if (text.startsWith('/binding ')) {
				await runBindingShortcut(text);
				return;
			}
			optimisticTurns = insertOptimistic(optimisticTurns, {
				kind: 'user_intent',
				id: `optimistic-${Date.now()}`,
				at: new Date().toISOString(),
				raw: text,
				source_kind: detail.voiceTranscript ? 'voice' : 'text'
			});
			const session = await api.createSession(text.slice(0, 80), 'new_behavior');
			replaceSelected(session);
			subscribe(session.session_id);
			const inputMode = detail.voiceTranscript ? ('voice' as IntentInputMode) : ('text' as IntentInputMode);
			const formalized = await api.formalizeIntent(
				session,
				text,
				inputMode,
				[],
				undefined,
				detail.voiceTranscript ?? null
			);
			replaceSelected(formalized.session);
			status = 'Intent captured';
			if (detail.auto) {
				autopilotRunning = true;
				try {
					const response = await api.startAutopilot(formalized.session, {
						allow_quorum: false,
						auto_climb_to: 'local_preview'
					});
					if (response) {
						autopilot = response.state;
						replaceSelected(response.session);
					}
				} finally {
					autopilotRunning = false;
				}
			} else {
				await api.clarifyIntent(formalized.session, 'claude-code', { timeoutSeconds: 90 }).catch(() => null);
				selected = await api.getSession(formalized.session.session_id);
			}
			await refreshDerived();
		} finally {
			busy = false;
		}
	}

	async function startRecipe(recipe: Recipe) {
		if (recipeStartInFlight) return;
		recipeStartInFlight = recipe.id;
		try {
			await compose({ text: recipe.intent_text, auto: true, voiceTranscript: null });
			if (selected) {
				agentContract = await api.getAgentContract(selected.session_id).catch(() => null);
				agentContractOpen = agentContract != null;
			}
		} finally {
			recipeStartInFlight = null;
		}
	}

	function startRecipeFromWelcome(event: MouseEvent) {
		const target = event.target instanceof Element ? event.target : null;
		const button = target?.closest<HTMLElement>('[data-recipe-id]');
		const recipe = recipes.find((item) => item.id === button?.dataset.recipeId);
		if (recipe) void startRecipe(recipe);
	}

	async function runBindingShortcut(text: string) {
		if (!selected) return;
		const binding = text.replace('/binding ', '').trim();
		if (!binding) return;
		selected = await api.runBinding(selected, binding);
		status = `Ran ${binding}`;
		await refreshDerived();
	}

	async function answer(detail: { questionId: string; text: string; witnessKind: IntentAnswer['witness_kind'] }) {
		if (!selected) return;
		busy = true;
		try {
			const response = await api.answerIntent(selected, [
				{
					question_id: detail.questionId,
					text: detail.text,
					witness_kind: detail.witnessKind
				}
			] as IntentAnswer[]);
			if (response) replaceSelected(response.session);
			await refreshDerived();
		} finally {
			busy = false;
		}
	}

	async function followup(text: string) {
		if (!selected) return;
		const questionId = `q-followup-${Date.now()}`;
		await answer({ questionId, text, witnessKind: 'desired_behavior' });
		status = 'Follow-up added';
	}

	async function build() {
		if (!selected) return;
		busy = true;
		try {
			let current = selected;
			if (current.phase === 'intent_ready') {
				current = await api.dispatch(current, 'draft_spec');
				current = await api.dispatch(current, 'approve_spec');
			}
			current = await api.runBuildPipeline(current, { maxRepairRounds: 3 });
			replaceSelected(current);
			status = 'Built and verified';
			await refreshDerived();
			lintReport = await api.getLintReport(current).catch(() => null);
			pbtRound = await api.runPbt(current).catch(() => null);
		} finally {
			busy = false;
		}
	}

	async function invoke(req: TryItRequest) {
		if (!selected) return;
		busy = true;
		invokeBusy = true;
		try {
			tryResult = await api.invoke(selected, req);
			status = 'Try-It run finished';
		} finally {
			invokeBusy = false;
			busy = false;
		}
	}

	async function realize() {
		if (!selected) return;
		busy = true;
		realizeBusy = true;
		try {
			status = 'Asking Claude Code to fill in the implementation…';
			const response = await api.realize(selected, { timeoutSeconds: 240 });
			replaceSelected(response.session);
			await refreshDerived();
			status = response.ok
				? `Implementation filled in (${response.wrote_files.length} file${
						response.wrote_files.length === 1 ? '' : 's'
				  } written) and verified.`
				: 'Implementation attempt completed but verify still failed — see the timeline.';
		} catch (error) {
			status = `Realize failed: ${(error as Error).message ?? error}`;
		} finally {
			realizeBusy = false;
			busy = false;
		}
	}

	async function climb(rung: string) {
		if (!selected) return;
		busy = true;
		try {
			if (['shareable', 'team', 'production'].includes(rung)) {
				archCheckReport = await api.archCheck(selected).catch(() => null);
			}
			const next = await api.climbRung(selected, rung);
			replaceSelected(next);
			await refreshDerived();
			status = `Climbed to ${rung}`;
		} finally {
			busy = false;
		}
	}

	async function scanIncidents() {
		if (!selected) return;
		await api.scanIncidents(selected);
		incidentNotice = 0;
		selected = await api.getSession(selected.session_id);
		await refreshDerived();
		status = 'Incident scan complete';
	}

	async function repairIncident(incidentId: string) {
		if (!selected) return;
		busy = true;
		try {
			const next = await api.repairIncident(selected, incidentId);
			replaceSelected(next);
			await refreshDerived();
			status = `Repair queued for ${incidentId}`;
		} finally {
			busy = false;
		}
	}

	async function loadQuickfix(incidentId: string) {
		if (!selected) return;
		quickfix = await api.incidentQuickfix(selected, incidentId);
		status = `Quickfix ${quickfix.diagnostic_code}`;
	}

	async function openLintReport() {
		if (!selected) return;
		lintReport = await api.getLintReport(selected);
		lintOpen = true;
		status = `Lint loaded (${lintReport.diagnostics.length} diagnostics)`;
	}

	async function applyLintQuickfix(diagnosticId: string) {
		if (!selected) return;
		quickfix = await api.applyLintQuickfix(selected, diagnosticId);
		lintReport = await api.getLintReport(selected).catch(() => lintReport);
		status = `Quickfix ${quickfix.diagnostic_code}`;
	}

	async function runPbt() {
		if (!selected) return;
		pbtRound = await api.runPbt(selected);
		status = `PBT ran ${pbtRound.properties_run} properties`;
	}

	async function pbtRegression(reproId: string) {
		if (!selected) return;
		quickfix = await api.pbtRegressionFrom(selected, reproId);
		status = `Locked ${reproId} as regression`;
		await refreshDerived();
	}

	async function runArchCheck() {
		if (!selected) return;
		archCheckReport = await api.archCheck(selected);
		status = archCheckReport.passed ? 'Architecture check passed' : 'Architecture check has violations';
		await refreshDerived();
	}

	async function searchPackage(moduleId: string) {
		pkgProvidesResult = await api.pkgProvides(moduleId);
		status = `Package search ${pkgProvidesResult.module_id}`;
	}

	async function applyHealthMigrate() {
		await api.applyMigrate(healthSnapshot?.migrate.to_schema ?? '0.5');
		healthSnapshot = await api.healthSnapshot().catch(() => null);
		status = 'Migration check refreshed';
	}

	async function openAgentContract() {
		if (!selected) return;
		agentContract = await api.getAgentContract(selected.session_id);
		agentContractOpen = true;
	}

	async function saveAgentContract(detail: { markdown: string; priorHash: string | null }) {
		if (!selected) return;
		agentContract = await api.saveAgentContract(selected.session_id, detail.markdown, detail.priorHash);
		status = 'AGENT.md saved';
	}

	async function openProof(behaviorId: string) {
		if (!selected) return;
		proofEvidence = await api.proofEvidence(selected, behaviorId);
	}

	async function openArtifactReport(path: string) {
		if (!selected) return;
		artifactPreview = await api.previewArtifact(selected, path);
	}

	async function reprovePatient() {
		if (!selected) return;
		busy = true;
		status = 'Re-proving with patient policy';
		try {
			selected = await api.runXtalWorkflow(selected, { proofPolicy: 'patient' });
			replaceSelected(selected);
			trustPosture = await api.trustPosture(selected).catch(() => trustPosture);
			status = 'Patient proof pass finished';
		} finally {
			busy = false;
		}
	}

	async function compareTurn(turnId: string) {
		if (!selected) return;
		semanticDiff = await api.semanticDiff(selected, {
			from: { kind: 'turn_id', turn_id: turnId },
			to: { kind: 'current' },
			mode: 'project'
		});
		compareOpen = true;
	}

	async function ask(question: string) {
		if (!selected) return;
		askAnswer = await api.askProject(selected, question);
		status = 'Project answer ready';
	}

	async function mintSync() {
		syncCode = await api.mintSyncCode();
		if (syncCode) {
			syncCode = await api.saveSyncState(syncCode.code, {
				selected: selected?.session_id,
				status,
				turns
			});
		}
		status = syncCode ? `Sync code ${syncCode.code}` : 'Sync unavailable in demo mode';
	}

	async function claimSync(code: string) {
		const claimed = await api.claimSyncCode(code);
		if (!claimed) {
			status = 'Sync claim unavailable in demo mode';
			return;
		}
		replaceSelected(claimed.session);
		subscribe(claimed.session.session_id);
		await refreshDerived();
		status = claimed.state_blob
			? `Claimed sync code ${code.trim().toUpperCase()} with saved state`
			: `Claimed sync code ${code.trim().toUpperCase()}`;
	}

	async function runQuorum() {
		if (!selected) return;
		busy = true;
		try {
			const round = await api.realizeQuorum(selected, ['claude-code', 'openai-codex'], {
				timeoutSeconds: 240
			});
			selected = await api.getSession(selected.session_id);
			await refreshDerived();
			status = round ? `Realize quorum compared ${round.proposals.length} proposal(s)` : 'Quorum unavailable in demo mode';
		} finally {
			busy = false;
		}
	}

	async function pickProposal(index: number) {
		if (!selected) return;
		busy = true;
		try {
			const response = await api.pickRealizeProposal(selected, index);
			if (response) {
				replaceSelected(response.session);
				await refreshDerived();
				status = `Picked proposal ${index + 1}`;
			}
		} finally {
			busy = false;
		}
	}

	async function startAutopilot() {
		if (!selected) return;
		busy = true;
		autopilotRunning = true;
		try {
			const response = await api.startAutopilot(selected, {
				allow_quorum: false,
				auto_climb_to: null,
				allow_repair_iters: 3
			});
			if (response) {
				autopilot = response.state;
				replaceSelected(response.session);
				await refreshDerived();
				status = response.state.last_decision?.reason ?? 'Autopilot stopped';
			}
		} finally {
			autopilotRunning = false;
			busy = false;
		}
	}

	async function pauseAutopilot() {
		if (!selected) return;
		const response = await api.pauseAutopilot(selected);
		if (response) {
			autopilot = response.state;
			replaceSelected(response.session);
			await refreshDerived();
			status = 'Autopilot paused';
		}
	}

	async function visualParse(detail: { kind: VisualKind; source: unknown }) {
		if (!selected) return;
		visualParseResult = await api.visualParse(selected, detail.kind, detail.source);
		status = `Parsed ${detail.kind} graph`;
	}

	async function visualEmit(detail: { kind: VisualKind; graph: unknown }) {
		if (!selected) return;
		visualEmitResult = await api.visualEmit(selected, detail.kind, detail.graph);
		status = `Emitted ${detail.kind} source`;
	}

	async function uploadImage(detail: { file: File }) {
		if (!selected) return;
		try {
			await api.uploadIntentImage(selected, detail.file);
			status = `Image witness added: ${detail.file.name}`;
		} catch (error) {
			status = error instanceof Error ? error.message : 'Image witness upload failed';
		}
	}

	async function submitRelease(rung: string) {
		if (!selected) return;
		busy = true;
		try {
			releaseStatus = await api.submitRelease(selected, rung);
			status = releaseStatus ? `Release ${releaseStatus.release_id}: ${releaseStatus.status}` : 'Release unavailable';
		} finally {
			busy = false;
		}
	}

	async function openCertificate() {
		if (!selected) return;
		certificate = await api.certificateSummary(selected);
		certificateOpen = true;
	}

	async function refreshCertificate() {
		if (!selected) return;
		certificate = await api.refreshCertificate(selected);
		status = 'Certificate refreshed';
	}

	async function runCommand(action: string) {
		if (action === 'compare' && turns.length) await compareTurn(turns.at(-1)?.id ?? turns[0].id);
		else if (action === 'build') await build();
		else if (action === 'autopilot') await startAutopilot();
		else if (action === 'scan') await scanIncidents();
		else if (action === 'sync') await mintSync();
	}

	async function exportReplay() {
		if (!selected) return;
		replayExport = await api.exportReplay(selected);
		status = replayExport ? `Replay capsule ${replayExport.capsule_id} exported` : 'Replay export unavailable';
	}

	async function saveMemory(patch: Partial<StudioMemory>) {
		memory = await api.saveMemory(patch);
		memoryOpen = false;
		status = 'Memory updated';
	}

	async function openStep(step: ProcessLaneType['steps'][number]) {
		if (!selected || !step.op_id) return;
		stepEvidence = await api.getStepEvidence(selected, step.op_id);
		stepDrawerOpen = Boolean(stepEvidence);
	}

	async function loadForecast(stepId: string) {
		if (!selected || forecasts[stepId]) return;
		forecasts = {
			...forecasts,
			[stepId]: await api.getWhatIf(selected, stepId).catch(() => null)
		};
	}

	async function saveRoleOverrides(overrides: RoleOverrides) {
		if (!selected) return;
		roleOverrides = await api.setRoleOverrides(selected, overrides);
		status = 'Role overrides saved';
		await refreshDerived();
	}

	async function saveAgentRole(detail: {
		agentId: string;
		defaultRole: AgentProfile['default_role'];
		eligibleRoles: AgentProfile['eligible_roles'];
	}) {
		const saved = await api.setAgentRole(detail.agentId, detail.defaultRole, detail.eligibleRoles);
		if (saved) {
			agents = agents.map((agent) => (agent.id === saved.id ? saved : agent));
			status = `${saved.label} is now ${saved.default_role}`;
		}
	}

	async function submitSessionSummary(summary: SessionSummary) {
		const response = await api.submitSessionSummary(summary);
		sessionSummaryStatus = response.accepted ? `Saved locally (${response.retained})` : 'Not saved';
	}
</script>

<svelte:head>
	<title>x07 Studio</title>
</svelte:head>

<svelte:document on:keydown|capture={handleGlobalKeydown} />

<main class="timeline-shell">
	<Header
		{health}
		{syncCode}
		{detailsOpen}
		onCommand={openCommandPalette}
		on:toggleDetails={() => (detailsOpen = !detailsOpen)}
		on:command={openCommandPalette}
		on:refresh={refresh}
		on:sync={mintSync}
		on:agentContract={openAgentContract}
	/>

	<section class="session-radar" aria-label="Session radar">
		<div>
			<span>Status</span>
			<strong>{status}</strong>
		</div>
		<div>
			<span>Session</span>
			<strong>{selected?.title ?? 'No session'}</strong>
		</div>
		<div>
			<span>Memory</span>
			<strong>{memory?.preferences.default_agent ?? 'default'}</strong>
		</div>
		<div>
			<span>Sync</span>
			<strong>{syncCode?.code ?? 'not minted'}</strong>
		</div>
		{#if replayExport}
			<div>
				<span>Replay</span>
				<strong>{replayExport.capsule_id}</strong>
			</div>
		{/if}
	</section>
	<MemoryChip ops={selected?.op_log ?? []} on:edit={() => (memoryOpen = true)} />

	{#if sessions.length === 0 && selected === null}
		<div role="presentation" on:click={startRecipeFromWelcome}>
			<Welcome recipeStart={startRecipe} />
		</div>
	{/if}

	<div class="main-grid">
		<div class="left-stack">
			<ProcessLane
				lane={processLane}
				{forecasts}
				on:step={(event) => openStep(event.detail)}
				on:forecast={(event) => loadForecast(event.detail)}
			/>
			<Timeline
				{turns}
				session={selected}
				{detailsOpen}
				{tryResult}
				{busy}
				{realizeBusy}
				{invokeBusy}
				{quickfix}
				{trustPosture}
				{pbtRound}
				on:answer={(event) => answer(event.detail)}
				on:followup={(event) => followup(event.detail)}
				on:repair={(event) => repairIncident(event.detail)}
				on:quickfix={(event) => loadQuickfix(event.detail)}
				on:proof={(event) => openProof(event.detail)}
				on:lint={openLintReport}
				on:pbt={runPbt}
				on:pbtRegression={(event) => pbtRegression(event.detail)}
				on:compare={(event) => compareTurn(event.detail)}
				on:invoke={(event) => invoke(event.detail)}
				on:realize={realize}
				on:quorum={runQuorum}
				on:pickProposal={(event) => pickProposal(event.detail)}
			/>
		</div>
		<div class="right-rail">
			<HealthRow
				snapshot={healthSnapshot}
				{busy}
				on:refresh={refresh}
				on:migrate={applyHealthMigrate}
			/>
			<NowPanel
				session={selected}
				{health}
				{ladder}
				{tryResult}
				{askAnswer}
				{cassetteRibbon}
				{agents}
				{roleOverrides}
				{trustPosture}
				{visualParseResult}
				{visualEmitResult}
				{autopilot}
				{releaseStatus}
				{pkgProvidesResult}
				{archCheckReport}
				{busy}
				on:build={build}
				on:invoke={(event) => invoke(event.detail)}
				on:climb={(event) => climb(event.detail)}
				on:scan={scanIncidents}
				on:ask={(event) => ask(event.detail)}
				on:pkgSearch={(event) => searchPackage(event.detail)}
				on:sync={mintSync}
				on:claimSync={(event) => claimSync(event.detail)}
				on:quorum={runQuorum}
				on:autopilot={startAutopilot}
				on:pauseAutopilot={pauseAutopilot}
				on:roleOverrides={(event) => saveRoleOverrides(event.detail)}
				on:release={(event) => submitRelease(event.detail)}
				on:certificate={openCertificate}
				on:exportReplay={exportReplay}
				on:visualParse={(event) => visualParse(event.detail)}
				on:visualEmit={(event) => visualEmit(event.detail)}
				on:proofReport={(event) => openArtifactReport(event.detail)}
				on:reproveTrust={reprovePatient}
			/>
			<SessionSummaryCard
				session={selected}
				{busy}
				status={sessionSummaryStatus}
				on:submit={(event) => submitSessionSummary(event.detail)}
			/>
		</div>
	</div>

	<Composer
		{busy}
		autopilotActive={autopilotRunning}
		incidentCount={incidentNotice}
		on:compose={(event) => compose(event.detail)}
		on:image={(event) => uploadImage(event.detail)}
		on:pauseAutopilot={pauseAutopilot}
	/>
	<MemoryDrawer {memory} open={memoryOpen} on:close={() => (memoryOpen = false)} on:save={(event) => saveMemory(event.detail)} />
	<StepDrawer evidence={stepEvidence} open={stepDrawerOpen} on:close={() => (stepDrawerOpen = false)} />
	<AgentContractEditor
		contract={agentContract}
		open={agentContractOpen}
		{busy}
		{agents}
		on:close={() => (agentContractOpen = false)}
		on:save={(event) => saveAgentContract(event.detail)}
		on:role={(event) => saveAgentRole(event.detail)}
	/>
	{#if lintOpen}
		<div class="modal-sheet">
			<LintReportDrawer
				report={lintReport}
				{quickfix}
				{busy}
				on:quickfix={(event) => applyLintQuickfix(event.detail)}
				on:close={() => (lintOpen = false)}
			/>
		</div>
	{/if}
	<ProofExplorer evidence={proofEvidence} open={Boolean(proofEvidence)} on:close={() => (proofEvidence = null)} />
	{#if artifactPreview}
		<div class="modal-sheet">
			<header>
				<h2>{artifactPreview.artifact}</h2>
				<button type="button" class="command-button" on:click={() => (artifactPreview = null)}>Close</button>
			</header>
			{#if artifactPreview.media_kind === 'json'}
				<pre>{JSON.stringify(artifactPreview.json, null, 2)}</pre>
			{:else}
				<pre>{artifactPreview.text ?? ''}</pre>
			{/if}
		</div>
	{/if}
	<CompareLens diff={semanticDiff} open={compareOpen}>
		<button slot="actions" type="button" class="command-button" on:click={() => (compareOpen = false)}>Close</button>
	</CompareLens>
	<CertificateView
		{certificate}
		open={certificateOpen}
		on:close={() => (certificateOpen = false)}
		on:refresh={refreshCertificate}
	/>
	<CommandPalette
		open={$commandPaletteOpen}
		on:close={closeCommandPalette}
		on:run={(event) => runCommand(event.detail)}
	/>
</main>
