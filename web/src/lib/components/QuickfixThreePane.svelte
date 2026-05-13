<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { patchPulse } from '$lib/motion';
	import type { QuickfixRecord } from '$lib/studio';

	export let record: QuickfixRecord;
	export let busy = false;

	let applied = false;
	const dispatch = createEventDispatcher<{ apply: string }>();
	$: operation = typeof record.patch_ast === 'string' ? record.patch_ast : JSON.stringify(record.patch_ast, null, 2);
</script>

<section class="quickfix-three" data-testid="quickfix-three-pane" use:patchPulse={applied}>
	<header>
		<h3>{record.diagnostic_code}</h3>
		<span>{record.summary}</span>
	</header>
	<div class="panes">
		<section>
			<h4>Before</h4>
			<pre>{record.before_snippet ?? 'No source snippet available.'}</pre>
		</section>
		<section>
			<h4>Operation</h4>
			<pre>{operation}</pre>
		</section>
		<section>
			<h4>After</h4>
			<pre>{record.after_snippet ?? 'Apply the patch to materialize the after view.'}</pre>
		</section>
	</div>
	<button class="command-button primary" type="button" disabled={busy} on:click={() => { applied = true; dispatch('apply', record.diagnostic_code); }}>
		Apply
	</button>
</section>

<style>
	.quickfix-three {
		display: grid;
		gap: 10px;
	}
	header {
		display: grid;
		gap: 4px;
	}
	h3,
	h4 {
		margin: 0;
	}
	header span {
		color: var(--muted);
	}
	.panes {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 8px;
	}
	.panes section {
		min-width: 0;
		display: grid;
		gap: 6px;
	}
	pre {
		min-height: 160px;
		max-height: 280px;
		overflow: auto;
		margin: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 8px;
		font-size: 12px;
	}
</style>
