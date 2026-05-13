<script lang="ts">
	import type { TrustPosture } from '$lib/studio';
	import { colorMorph } from '$lib/motion';
	import PostureChip from './PostureChip.svelte';

	export let posture: TrustPosture | null = null;
	export let isComputing = false;

	$: headline = posture
		? `${posture.worlds.join(', ')} · ${posture.capabilities.length} OS read${posture.capabilities.length === 1 ? '' : 's'} · ${Math.round(posture.proof_coverage.proved_pct)}% proof coverage`
		: isComputing
			? 'Computing trust posture...'
			: 'Trust posture pending';

	$: proverFraction = posture && posture.budgets.prover_seconds_cap
		? Math.min(1, posture.budgets.prover_seconds_used / posture.budgets.prover_seconds_cap)
		: 0;
</script>

<section class="trust-card {posture?.posture_color ?? 'amber'}" class:computing={!posture && isComputing} data-testid="trust-card" use:colorMorph>
	<header>
		<span class="eyebrow">Trust posture</span>
		<PostureChip color={posture?.posture_color ?? 'amber'} label={posture?.trust_profile ?? 'pending'} />
	</header>
	<h2 class="headline">{headline}</h2>
	{#if posture}
		<dl class="posture-grid">
			<div>
				<dt>Worlds</dt>
				<dd>{posture.worlds.join(' · ') || 'none'}</dd>
			</div>
			<div>
				<dt>Budget</dt>
				<dd>
					{posture.budgets.prover_seconds_used}s
					<span class="budget-cap">/ {posture.budgets.prover_seconds_cap ?? '∞'}s</span>
				</dd>
				{#if posture.budgets.prover_seconds_cap}
					<div class="budget-bar" aria-hidden="true">
						<span style="--fill: {proverFraction * 100}%"></span>
					</div>
				{/if}
			</div>
			<div>
				<dt>Proofs</dt>
				<dd>{posture.proof_coverage.proof_count} obligations</dd>
			</div>
		</dl>
		{#if posture.capabilities.length}
			<ul class="capability-chips">
				{#each posture.capabilities as capability}
					<li title={capability.justification}>{capability.id}</li>
				{/each}
			</ul>
		{/if}
		{#if posture.deltas.length}
			<div class="delta-list">
				{#each posture.deltas.slice(0, 4) as delta}
					<span>{delta.summary}</span>
				{/each}
			</div>
		{/if}
		{#if (posture.proof_support_notes ?? []).length}
			<details class="proof-notes" data-testid="proof-support-notes">
				<summary>
					<span class="proof-notes-eyebrow">Proof support</span>
					<span class="proof-notes-count">{(posture.proof_support_notes ?? []).length} note{(posture.proof_support_notes ?? []).length === 1 ? '' : 's'}</span>
				</summary>
				<ul>
					{#each posture.proof_support_notes ?? [] as note}
						<li class="proof-note severity-{note.severity || 'warning'}">
							<code class="proof-note-code">{note.code}</code>
							{#if note.target}<span class="proof-note-target">{note.target}</span>{/if}
							<span class="proof-note-message">{note.message}</span>
						</li>
					{/each}
				</ul>
			</details>
		{/if}
	{:else}
		<p class="pending-line">
			{isComputing
				? "I'm working on it - checking the build, generating proofs, capturing posture."
				: 'Build or formalize a session to capture the first posture.'}
		</p>
	{/if}
</section>

<style>
	.trust-card {
		position: relative;
		padding: 18px 20px 20px 22px;
		border: 1px solid var(--border);
		border-left-width: 8px;
		border-radius: var(--radius);
		background: var(--surface);
		display: grid;
		gap: 14px;
		box-shadow: 0 1px 0 rgba(255, 255, 255, 0.02) inset;
		transition: border-color var(--motion-base, 200ms) var(--ease-out, ease-out), box-shadow var(--motion-base, 200ms) var(--ease-out, ease-out);
	}
	.trust-card::after {
		content: '';
		position: absolute;
		inset: -1px -1px -1px -1px;
		border-radius: inherit;
		pointer-events: none;
		background: linear-gradient(135deg, transparent 60%, var(--accent, transparent) 240%);
		opacity: 0.18;
		mix-blend-mode: screen;
		transition: opacity var(--motion-base, 200ms) var(--ease-out, ease-out);
	}
	.trust-card.green { border-left-color: var(--accent-pure); --accent: var(--accent-pure); }
	.trust-card.amber { border-left-color: var(--accent-sandbox); --accent: var(--accent-sandbox); }
	.trust-card.red   { border-left-color: var(--accent-danger); --accent: var(--accent-danger); }
	.trust-card.computing {
		animation: trust-pulse 1.2s ease-in-out infinite;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}
	.eyebrow {
		color: var(--muted);
		font-size: 11px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		font-weight: 600;
	}
	.trust-card.computing .eyebrow,
	.trust-card.computing .headline {
		animation: trust-copy-pulse 1.2s ease-in-out infinite;
	}
	.headline {
		margin: 0;
		font-family: var(--font-mono);
		font-size: clamp(18px, 1.4vw + 14px, 22px);
		line-height: 1.3;
		color: var(--text);
		font-weight: 600;
		letter-spacing: -0.01em;
	}
	.posture-grid {
		margin: 0;
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 10px;
	}
	.posture-grid > div {
		display: grid;
		gap: 4px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: rgba(255, 255, 255, 0.025);
	}
	.posture-grid dt {
		margin: 0;
		color: var(--muted);
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		font-weight: 600;
	}
	.posture-grid dd {
		margin: 0;
		font-family: var(--font-mono);
		font-size: 13px;
		color: var(--accent);
		line-height: 1.2;
	}
	.budget-cap {
		color: var(--muted);
		font-size: 11px;
	}
	.budget-bar {
		margin-top: 6px;
		height: 4px;
		border-radius: 2px;
		background: rgba(255, 255, 255, 0.06);
		overflow: hidden;
	}
	.budget-bar span {
		display: block;
		height: 100%;
		width: var(--fill, 0%);
		background: var(--accent);
		opacity: 0.7;
		transition: width var(--motion-base, 200ms) var(--ease-out, ease-out);
	}
	.capability-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.capability-chips li {
		font-family: var(--font-mono);
		font-size: 11px;
		padding: 3px 8px;
		border-radius: 999px;
		border: 1px solid var(--border);
		background: rgba(255, 255, 255, 0.03);
		color: var(--text);
		cursor: help;
	}
	.delta-list {
		display: grid;
		gap: 4px;
	}
	.delta-list span {
		display: block;
		color: var(--muted);
		font-size: 12px;
		line-height: 1.4;
		padding-left: 10px;
		border-left: 2px solid var(--accent);
		opacity: 0.85;
	}
	.pending-line {
		margin: 0;
		color: var(--muted);
		font-size: 12px;
	}
	.proof-notes {
		border: 1px solid var(--border);
		border-radius: 6px;
		background: rgba(255, 255, 255, 0.025);
	}
	.proof-notes summary {
		display: flex;
		gap: 8px;
		align-items: baseline;
		padding: 8px 12px;
		cursor: pointer;
		list-style: none;
	}
	.proof-notes summary::-webkit-details-marker {
		display: none;
	}
	.proof-notes-eyebrow {
		color: var(--muted);
		font-size: 10px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		font-weight: 600;
	}
	.proof-notes-count {
		font-family: var(--font-mono);
		font-size: 11px;
		color: var(--text);
	}
	.proof-notes ul {
		margin: 0;
		padding: 0 12px 10px 12px;
		list-style: none;
		display: grid;
		gap: 6px;
	}
	.proof-note {
		display: grid;
		gap: 2px;
		padding: 6px 8px;
		border-radius: 4px;
		background: rgba(255, 255, 255, 0.025);
		font-size: 11px;
		line-height: 1.4;
	}
	.proof-note.severity-error {
		border-left: 2px solid var(--accent-danger);
	}
	.proof-note.severity-warning {
		border-left: 2px solid var(--accent-sandbox);
	}
	.proof-note-code {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--accent);
	}
	.proof-note-target {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--muted);
	}
	.proof-note-message {
		color: var(--text);
	}

	@keyframes trust-pulse {
		0%, 100% {
			box-shadow: 0 0 0 rgba(255, 195, 91, 0);
			border-left-color: rgba(255, 195, 91, 0.7);
		}
		50% {
			box-shadow: 0 0 18px rgba(255, 195, 91, 0.14);
			border-left-color: var(--accent-sandbox);
		}
	}

	@keyframes trust-copy-pulse {
		0%, 100% { opacity: 0.7; }
		50% { opacity: 1; }
	}

	@media (prefers-reduced-motion: reduce) {
		.trust-card.computing,
		.trust-card.computing .eyebrow,
		.trust-card.computing .headline {
			animation: none;
		}
	}
</style>
