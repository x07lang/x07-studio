<script lang="ts">
	import type { AgentStreamEvent } from '$lib/studio';

	export let event: AgentStreamEvent;
</script>

{#if event.kind === 'mcp_call'}
	<section class="mcp-call-card" data-testid="mcp-call-card">
		<header>
			<h3>{event.server}</h3>
			<code>{event.tool}</code>
		</header>
		<pre>{JSON.stringify(event.input, null, 2)}</pre>
		{#if event.output}
			<pre>{JSON.stringify(event.output, null, 2)}</pre>
		{/if}
	</section>
{/if}

<style>
	.mcp-call-card {
		display: grid;
		gap: 8px;
		border: 1px solid rgba(85, 214, 231, 0.28);
		border-radius: var(--radius);
		padding: 10px;
		background: rgba(85, 214, 231, 0.05);
	}
	.mcp-call-card header {
		display: flex;
		justify-content: space-between;
		gap: 8px;
	}
	.mcp-call-card h3 {
		margin: 0;
		font-size: 13px;
	}
	.mcp-call-card code {
		color: var(--cyan);
	}
	.mcp-call-card pre {
		max-height: 160px;
		overflow: auto;
		margin: 0;
		font-size: 12px;
		white-space: pre-wrap;
	}
</style>
