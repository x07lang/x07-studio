<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { SessionSnapshot, VisualKind, VisualResponse } from '$lib/studio';
	import Canvas from './visual/Canvas.svelte';
	import VisualNode from './visual/Node.svelte';
	import VisualEdge from './visual/Edge.svelte';

	type VisualNode = { id: string; label: string };
	type VisualEdge = { from: string; to: string; label: string };
	type VisualGraph = { nodes: VisualNode[]; edges: VisualEdge[] };

	export let session: SessionSnapshot | null = null;
	export let busy = false;
	export let parseResult: VisualResponse | null = null;
	export let emitResult: VisualResponse | null = null;

	const kinds: Array<{ id: VisualKind; label: string }> = [
		{ id: 'streampipe', label: 'Pipe' },
		{ id: 'statemachine', label: 'State' },
		{ id: 'tasks', label: 'Tasks' }
	];

	let kind: VisualKind = 'streampipe';
	let sourceText = 'fetch input | normalize | verify';
	let graph: VisualGraph = normalizeGraph({
		nodes: [
			{ id: '1', label: 'fetch input' },
			{ id: '2', label: 'normalize' },
			{ id: '3', label: 'verify' }
		],
		edges: [
			{ from: '1', to: '2', label: 'next' },
			{ from: '2', to: '3', label: 'next' }
		]
	});
	let lastParse: VisualResponse | null = null;

	const dispatch = createEventDispatcher<{
		parse: { kind: VisualKind; source: unknown };
		emit: { kind: VisualKind; graph: VisualGraph };
	}>();

	$: if (parseResult && parseResult !== lastParse && parseResult.kind === kind) {
		graph = normalizeGraph(parseResult.value);
		lastParse = parseResult;
	}

	$: emittedText = emitResult?.kind === kind ? formatValue(emitResult.value) : '';

	function switchKind(next: VisualKind) {
		kind = next;
		if (next === 'streampipe') {
			sourceText = labelsFromGraph(graph).join(' | ') || 'fetch input | normalize | verify';
		} else {
			sourceText = JSON.stringify(graph, null, 2);
		}
	}

	function parseSource() {
		dispatch('parse', { kind, source: sourceValue() });
	}

	function emitGraph() {
		dispatch('emit', { kind, graph });
	}

	function addNode() {
		const id = String(nextNodeId(graph.nodes));
		graph = {
			nodes: [...graph.nodes, { id, label: `Step ${id}` }],
			edges: graph.edges
		};
	}

	function removeNode(id: string) {
		graph = {
			nodes: graph.nodes.filter((node) => node.id !== id),
			edges: graph.edges.filter((edge) => edge.from !== id && edge.to !== id)
		};
	}

	function renameNode(detail: { id: string; label: string }) {
		graph = {
			nodes: graph.nodes.map((node) => node.id === detail.id ? { ...node, label: detail.label } : node),
			edges: graph.edges
		};
	}

	function nodePosition(index: number) {
		return { x: 24 + (index % 2) * 170, y: 24 + Math.floor(index / 2) * 92 };
	}

	function nodeCenter(id: string) {
		const index = graph.nodes.findIndex((node) => node.id === id);
		const pos = nodePosition(Math.max(0, index));
		return { x: pos.x + 84, y: pos.y + 32 };
	}

	function connectSequential() {
		graph = {
			nodes: graph.nodes,
			edges: graph.nodes.slice(1).map((node, index) => ({
				from: graph.nodes[index].id,
				to: node.id,
				label: 'next'
			}))
		};
	}

	function sourceValue() {
		if (kind === 'streampipe') return sourceText;
		try {
			return JSON.parse(sourceText);
		} catch {
			return graph;
		}
	}

	function normalizeGraph(value: unknown): VisualGraph {
		if (!value || typeof value !== 'object') return { nodes: [], edges: [] };
		const input = value as { nodes?: unknown; edges?: unknown };
		const nodes = Array.isArray(input.nodes)
			? input.nodes.map((node, index) => {
					const item = node && typeof node === 'object' ? (node as Record<string, unknown>) : {};
					return {
						id: String(item.id ?? index + 1),
						label: String(item.label ?? item.name ?? `Step ${index + 1}`)
					};
				})
			: [];
		const edges = Array.isArray(input.edges)
			? input.edges
					.map((edge) => {
						const item = edge && typeof edge === 'object' ? (edge as Record<string, unknown>) : {};
						const from = String(item.from ?? item.source ?? '');
						const to = String(item.to ?? item.target ?? '');
						return {
							from,
							to,
							label: String(item.label ?? item.kind ?? 'next')
						};
					})
					.filter((edge) => edge.from && edge.to)
			: [];
		return { nodes, edges };
	}

	function nextNodeId(nodes: VisualNode[]) {
		return (
			nodes
				.map((node) => Number.parseInt(node.id, 10))
				.filter(Number.isFinite)
				.reduce((max, value) => Math.max(max, value), 0) + 1
		);
	}

	function labelsFromGraph(value: VisualGraph) {
		return value.nodes.map((node) => node.label.trim()).filter(Boolean);
	}

	function formatValue(value: unknown) {
		return typeof value === 'string' ? value : JSON.stringify(value, null, 2);
	}
</script>

<section class="now-card visual-editor" data-testid="visual-editor">
	<header>
		<h2>Visual editor</h2>
		<span>{kind}</span>
	</header>

	<div class="segmented" aria-label="Visual editor kind">
		{#each kinds as item}
			<button
				type="button"
				class:active={kind === item.id}
				aria-pressed={kind === item.id}
				on:click={() => switchKind(item.id)}
			>
				{item.label}
			</button>
		{/each}
	</div>

	<textarea bind:value={sourceText} rows="4" placeholder="Paste source or graph JSON"></textarea>
	<div class="button-row">
		<button type="button" class="command-button" disabled={!session || busy} on:click={parseSource}>
			Parse
		</button>
		<button type="button" class="command-button" disabled={!session || busy} on:click={addNode}>
			Add step
		</button>
		<button type="button" class="command-button" disabled={!session || busy || graph.nodes.length < 2} on:click={connectSequential}>
			Link sequence
		</button>
		<button type="button" class="command-button primary" disabled={!session || busy} on:click={emitGraph}>
			Emit
		</button>
	</div>

	<Canvas>
		{#each graph.edges as edge}
			{@const from = nodeCenter(edge.from)}
			{@const to = nodeCenter(edge.to)}
			<VisualEdge fromX={from.x} fromY={from.y} toX={to.x} toY={to.y} label={edge.label} />
		{/each}
		{#each graph.nodes as node, index}
			{@const pos = nodePosition(index)}
			<VisualNode id={node.id} label={node.label} x={pos.x} y={pos.y} on:label={(event) => renameNode(event.detail)} on:remove={(event) => removeNode(event.detail)} />
		{/each}
	</Canvas>

	{#if graph.edges.length}
		<div class="visual-edges">
			{#each graph.edges as edge}
				<span>{edge.from} -> {edge.to} {edge.label}</span>
			{/each}
		</div>
	{/if}

	{#if emittedText}
		<div class="try-output" data-testid="visual-output">
			<h3>Emitted</h3>
			<pre>{emittedText}</pre>
		</div>
	{/if}
</section>
