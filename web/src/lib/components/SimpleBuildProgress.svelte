<script lang="ts">
	import {
		buildStageOrder,
		stageLabel,
		stageFromOp,
		buildStageProgress,
		plainOpLabel,
		type BuildStage
	} from '$lib/plainEnglish';
	import type { SessionSnapshot } from '$lib/studio';

	export let session: SessionSnapshot;
	export let running = false;

	$: progress = buildStageProgress(session);
	$: stages = buildStageOrder.filter(
		(stage) => stage !== 'needs_help' && stage !== 'repair'
	);
	$: liveEvents = session.op_log
		.slice(-12)
		.reverse()
		.map((op) => ({ id: op.id, label: plainOpLabel(op), status: op.status }));

	function stageState(stage: BuildStage): 'done' | 'current' | 'pending' {
		if (progress.completed.includes(stage)) return 'done';
		if (progress.stage === stage) return 'current';
		return 'pending';
	}
</script>

<section class="build-progress" data-testid="simple-build-progress">
	<header>
		<h2>Building your project</h2>
		<p class="hint">
			{#if running}Working through the canonical XTAL lifecycle. You can keep watching.
			{:else if progress.stage === 'done'}Done — see the summary below.
			{:else if progress.stage === 'needs_help'}I'm stuck and need your help.
			{:else}Ready to begin.{/if}
		</p>
	</header>

	<ol class="stages" data-testid="simple-build-stages">
		{#each stages as stage}
			{@const state = stageState(stage)}
			<li class={state} data-testid="build-stage-{stage}" data-state={state}>
				<span class="dot" aria-hidden="true"></span>
				<span class="label">{stageLabel(stage)}</span>
			</li>
		{/each}
	</ol>

	<details class="activity" open>
		<summary>Live activity</summary>
		<ul data-testid="simple-build-activity">
			{#each liveEvents as event (event.id)}
				<li class="activity-row" data-status={event.status}>
					<span class="status" aria-hidden="true"></span>
					<span class="label">{event.label}</span>
				</li>
			{/each}
		</ul>
	</details>
</section>

<style>
	.build-progress {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		padding: 1.5rem;
		border-radius: 0.75rem;
		background: var(--surface, #ffffff);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
	}
	header h2 {
		font-size: 1.25rem;
		margin: 0 0 0.25rem;
	}
	.hint {
		margin: 0;
		color: var(--muted, #555);
		font-size: 0.9rem;
	}
	.stages {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 0.6rem 1rem;
	}
	.stages li {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.95rem;
		color: var(--muted, #777);
	}
	.stages .dot {
		width: 0.6rem;
		height: 0.6rem;
		border-radius: 999px;
		background: var(--border, #d0d4dc);
		display: inline-block;
	}
	.stages li.done {
		color: var(--accent-strong, #2944c8);
	}
	.stages li.done .dot {
		background: var(--accent, #4a6cf7);
	}
	.stages li.current {
		color: var(--text, #1b1f2a);
		font-weight: 600;
	}
	.stages li.current .dot {
		background: #f5a623;
		box-shadow: 0 0 0 3px rgba(245, 166, 35, 0.2);
	}
	.activity ul {
		list-style: none;
		padding: 0;
		margin: 0.5rem 0 0;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		font-size: 0.9rem;
	}
	.activity-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	.activity-row .status {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 999px;
		background: var(--border, #d0d4dc);
	}
	.activity-row[data-status='succeeded'] .status {
		background: #2bbf6b;
	}
	.activity-row[data-status='failed'] .status {
		background: #d04848;
	}
	.activity-row[data-status='running'] .status {
		background: #f5a623;
		box-shadow: 0 0 0 2px rgba(245, 166, 35, 0.25);
	}
</style>
