<script lang="ts">
	import type { AgentStreamEvent, LiveDiff } from '$lib/studio';

	export let event: AgentStreamEvent;

	let expanded = false;

	$: liveDiff = extractLiveDiff(event);
	$: title = streamTitle(event);
	$: body = streamBody(event);

	function streamTitle(value: AgentStreamEvent) {
		if (value.kind === 'tool_use') return `${value.tool} requested`;
		if (value.kind === 'tool_result') return `${value.tool} ${value.success ? 'succeeded' : 'failed'}`;
		if (value.kind === 'done') return `Done (${value.exit_code})`;
		if (value.kind === 'reasoning') return 'Reasoning';
		return 'Message';
	}

	function streamBody(value: AgentStreamEvent) {
		if (value.kind === 'reasoning') return value.text;
		if (value.kind === 'agent_message') return value.text;
		if (value.kind === 'tool_result') return value.snippet ?? '';
		if (value.kind === 'tool_use') return JSON.stringify(value.input, null, 2);
		return '';
	}

	function extractLiveDiff(value: AgentStreamEvent): LiveDiff | null {
		if (value.kind !== 'tool_use' || !value.input || typeof value.input !== 'object') return null;
		const diff = (value.input as { live_diff?: unknown }).live_diff;
		if (!diff || typeof diff !== 'object') return null;
		return diff as LiveDiff;
	}
</script>

<section class="agent-stream-card" data-testid="agent-stream-card">
	<header>
		<span>{title}</span>
		<time>{event.at}</time>
	</header>
	{#if body}
		<pre class:collapsed={!expanded}>{body}</pre>
		{#if body.length > 220}
			<button type="button" class="link-button" on:click={() => (expanded = !expanded)}>
				{expanded ? 'Collapse' : 'Expand'}
			</button>
		{/if}
	{/if}
	{#if liveDiff}
		<div class="diff-chip">{liveDiff.path}</div>
	{/if}
</section>

<style>
	.agent-stream-card {
		border: 1px solid rgba(148, 163, 184, 0.24);
		border-radius: 0.45rem;
		padding: 0.65rem;
		background: rgba(15, 23, 42, 0.34);
	}
	.agent-stream-card header {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		font-size: 0.78rem;
		color: var(--muted, #aab1c0);
	}
	.agent-stream-card pre {
		margin: 0.45rem 0 0;
		white-space: pre-wrap;
		font-size: 0.76rem;
		line-height: 1.35;
		max-height: 18rem;
		overflow: auto;
	}
	.agent-stream-card pre.collapsed {
		max-height: 4.5rem;
		overflow: hidden;
	}
	.diff-chip {
		display: inline-flex;
		margin-top: 0.45rem;
		padding: 0.15rem 0.45rem;
		border-radius: 999px;
		background: rgba(56, 189, 248, 0.12);
		color: #7dd3fc;
		font-size: 0.72rem;
	}
</style>
