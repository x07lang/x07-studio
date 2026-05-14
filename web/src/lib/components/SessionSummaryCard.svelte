<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import { setTelemetryEnabled, telemetryEnabled } from '$lib/errorReporter';
	import type { SessionSnapshot, SessionSummary } from '$lib/studio';

	export let session: SessionSnapshot | null = null;
	export let busy = false;
	export let status = '';

	const dispatch = createEventDispatcher<{ submit: SessionSummary }>();

	let consent = false;
	let archetype = 'sort';
	let success = true;
	let built = '';
	let confusing = '';
	let shipped = 'yes';

	onMount(() => {
		consent = telemetryEnabled();
	});

	$: repairRounds = session?.op_log.filter((op) => op.op.includes('repair')).length ?? 0;
	$: agentMinutes = estimateAgentMinutes(session);
	$: timeToVerified = estimateTimeToVerified(session);

	function submit() {
		if (!session) return;
		setTelemetryEnabled(consent);
		dispatch('submit', {
			schema_version: 'x07.studio.session_summary@0.1.0',
			session_id: session.session_id,
			consent,
			archetype,
			time_to_verified_ms: timeToVerified,
			repair_rounds: repairRounds,
			agent_minutes: agentMinutes,
			success,
			friction_notes: [
				built ? `built: ${built}` : '',
				confusing ? `confusing: ${confusing}` : '',
				`shipped: ${shipped}`
			].filter(Boolean)
		});
	}

	function estimateTimeToVerified(current: SessionSnapshot | null) {
		if (!current) return null;
		const first = current.op_log[0]?.started_at;
		const verified = current.op_log.find(
			(op) => op.status === 'succeeded' && (op.op.includes('verify') || op.op.includes('certify'))
		)?.finished_at;
		if (!first || !verified) return null;
		const start = Date.parse(first);
		const finish = Date.parse(verified);
		return Number.isFinite(start) && Number.isFinite(finish) && finish >= start ? finish - start : null;
	}

	function estimateAgentMinutes(current: SessionSnapshot | null) {
		if (!current) return 0;
		const totalMs = current.op_log
			.filter((op) => op.backend.includes('claude') || op.backend.includes('codex') || op.op.includes('agent'))
			.reduce((total, op) => {
				const start = Date.parse(op.started_at);
				const finish = op.finished_at ? Date.parse(op.finished_at) : start;
				return Number.isFinite(start) && Number.isFinite(finish) && finish >= start ? total + finish - start : total;
			}, 0);
		return Math.round((totalMs / 60000) * 10) / 10;
	}
</script>

{#if session}
	<section class="now-card session-summary-card" aria-label="Session summary">
		<header>
			<div>
				<span>Session summary</span>
				<h2>Feedback</h2>
			</div>
		</header>
		<label>
			<span>Archetype</span>
			<select bind:value={archetype}>
				<option value="sort">Sort</option>
				<option value="parser">Parser</option>
				<option value="service">Service</option>
				<option value="validator">Validator</option>
				<option value="calculator">Calculator</option>
				<option value="other">Other</option>
			</select>
		</label>
		<label>
			<span>Built</span>
			<input bind:value={built} placeholder="CSV parser, release service, stable sorter" />
		</label>
		<label>
			<span>Confusing</span>
			<textarea bind:value={confusing} rows="3" placeholder="One blocker or rough edge"></textarea>
		</label>
		<div class="summary-row">
			<label>
				<span>Shipped</span>
				<select bind:value={shipped}>
					<option value="yes">Yes</option>
					<option value="not-yet">Not yet</option>
					<option value="blocked">Blocked</option>
				</select>
			</label>
			<label>
				<span>Result</span>
				<select bind:value={success}>
					<option value={true}>Succeeded</option>
					<option value={false}>Failed</option>
				</select>
			</label>
		</div>
		<label class="inline-check">
			<input type="checkbox" bind:checked={consent} />
			<span>Share locally</span>
		</label>
		<button type="button" class="command-button" disabled={busy} on:click={submit}>Submit</button>
		{#if status}
			<p class="summary-status">{status}</p>
		{/if}
	</section>
{/if}

<style>
	.session-summary-card {
		display: grid;
		gap: 10px;
	}
	.session-summary-card label {
		display: grid;
		gap: 6px;
	}
	.session-summary-card span {
		color: var(--muted);
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
	}
	.session-summary-card input,
	.session-summary-card select,
	.session-summary-card textarea {
		width: 100%;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(255, 255, 255, 0.045);
		color: var(--text);
		padding: 8px 10px;
	}
	.summary-row {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 8px;
	}
	.inline-check {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.inline-check input {
		width: auto;
	}
	.summary-status {
		margin: 0;
		color: var(--muted);
		font-size: 12px;
	}
</style>
