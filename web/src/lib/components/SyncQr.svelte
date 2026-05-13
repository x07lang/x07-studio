<script lang="ts">
	import QRCode from 'qrcode';
	import type { SyncCode } from '$lib/studio';

	export let syncCode: SyncCode | null = null;

	let qrUrl = '';
	$: claimUrl = syncCode ? `${location.origin}${location.pathname}?claim=${syncCode.code}` : '';
	$: void renderQr(claimUrl);

	async function renderQr(value: string) {
		qrUrl = value ? await QRCode.toDataURL(value, { margin: 1, width: 160 }) : '';
	}
</script>

{#if syncCode}
	<section class="sync-qr" data-testid="sync-qr">
		<img alt={`Sync code ${syncCode.code}`} src={qrUrl} />
		<div>
			<strong>{syncCode.code}</strong>
			<small>{claimUrl}</small>
		</div>
	</section>
{/if}

<style>
	.sync-qr {
		display: flex;
		gap: 0.75rem;
		align-items: center;
		padding: 0.75rem;
		border: 1px solid rgba(148, 163, 184, 0.24);
		border-radius: 0.5rem;
		background: rgba(15, 23, 42, 0.36);
	}
	.sync-qr img {
		width: 5rem;
		height: 5rem;
		background: white;
	}
	.sync-qr div {
		min-width: 0;
	}
	.sync-qr small {
		display: block;
		color: var(--muted, #aab1c0);
		overflow-wrap: anywhere;
	}
</style>
