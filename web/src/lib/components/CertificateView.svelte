<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { CertificateSummary } from '$lib/studio';

	export let certificate: CertificateSummary | null = null;
	export let open = false;

	let tab = 'Summary';
	const dispatch = createEventDispatcher<{ close: void; refresh: void }>();
</script>

{#if open && certificate}
	<section class="certificate-view" data-testid="certificate-view">
		<header>
			<div>
				<h2>Certificate</h2>
				<span>{certificate.profile} · {certificate.operational_entry}</span>
			</div>
			<div class="button-row">
				<button type="button" class="command-button" on:click={() => dispatch('refresh')}>Refresh</button>
				<button type="button" class="command-button" on:click={() => dispatch('close')}>Close</button>
			</div>
		</header>
		<div class="cert-tabs">
			{#each ['Summary', 'Proof', 'Trust report'] as item}
				<button type="button" class:active={tab === item} on:click={() => (tab = item)}>{item}</button>
			{/each}
		</div>
		{#if tab === 'Summary'}
			<dl>
				<dt>Issued</dt><dd>{certificate.issued_at}</dd>
				<dt>Signature</dt><dd><code>{certificate.signature}</code></dd>
				<dt>HTML</dt><dd><code>{certificate.html_summary_path}</code></dd>
			</dl>
		{:else if tab === 'Proof'}
			<pre>{JSON.stringify(certificate.proof_summary, null, 2)}</pre>
		{:else}
			<pre>{JSON.stringify(certificate.trust_report, null, 2)}</pre>
		{/if}
	</section>
{/if}

<style>
	.certificate-view {
		position: fixed;
		inset: 6vh 8vw;
		z-index: 23;
		display: grid;
		align-content: start;
		gap: 14px;
		overflow: auto;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius);
		background: var(--surface-3, #0c1016);
		padding: 16px;
		box-shadow: var(--shadow);
	}
	.certificate-view header,
	.cert-tabs {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		flex-wrap: wrap;
	}
	.certificate-view h2 {
		margin: 0;
	}
	.certificate-view span,
	.certificate-view dt {
		color: var(--muted);
	}
	.cert-tabs button {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(255, 255, 255, 0.04);
		color: var(--muted);
		padding: 6px 10px;
	}
	.cert-tabs button.active {
		color: var(--text);
		border-color: var(--border-strong);
	}
	dl {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		gap: 8px 12px;
	}
	pre {
		max-height: 55vh;
		overflow: auto;
	}
</style>
