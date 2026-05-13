<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { LintDiagnostic, LintReport as LintReportType, QuickfixRecord } from '$lib/studio';
	import QuickfixThreePane from './QuickfixThreePane.svelte';

	export let report: LintReportType | null = null;
	export let quickfix: QuickfixRecord | null = null;
	export let busy = false;

	const dispatch = createEventDispatcher<{ quickfix: string; close: void }>();
	$: diagnostics = report?.diagnostics ?? [];
	$: grouped = diagnostics.reduce<Record<string, LintDiagnostic[]>>((acc, diagnostic) => {
		(acc[diagnostic.severity] ??= []).push(diagnostic);
		return acc;
	}, {});
</script>

<section class="lint-report" data-testid="lint-report">
	<header>
		<div>
			<h2>Lint</h2>
			<span>{report ? `${report.diagnostics.length} diagnostics` : 'not run'}</span>
		</div>
		<button class="command-button" type="button" on:click={() => dispatch('close')}>Close</button>
	</header>
	{#if report}
		{#each Object.entries(grouped) as [severity, diagnostics]}
			<section class="severity-group">
				<h3>{severity}</h3>
				{#each diagnostics as diagnostic}
					<article>
						<strong>{diagnostic.id}</strong>
						<span><code>{diagnostic.file}:{diagnostic.line}:{diagnostic.column}</code></span>
						<p>{diagnostic.summary}</p>
						<button class="command-button" type="button" disabled={busy || !diagnostic.fixable} on:click={() => dispatch('quickfix', diagnostic.id)}>
							Apply quickfix
						</button>
					</article>
				{/each}
			</section>
		{/each}
		{#if quickfix}
			<QuickfixThreePane record={quickfix} {busy} on:apply={() => dispatch('quickfix', quickfix.diagnostic_code)} />
		{/if}
	{:else}
		<p>Run lint to load x07diag diagnostics.</p>
	{/if}
</section>

<style>
	.lint-report {
		display: grid;
		gap: 12px;
	}
	header,
	article {
		display: grid;
		gap: 6px;
	}
	header {
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
	}
	h2,
	h3,
	p {
		margin: 0;
	}
	header span,
	article span,
	p {
		color: var(--muted);
	}
	.severity-group {
		display: grid;
		gap: 8px;
	}
	article {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 10px;
		background: rgba(255, 255, 255, 0.025);
	}
</style>
