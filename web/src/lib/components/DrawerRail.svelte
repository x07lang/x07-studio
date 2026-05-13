<script lang="ts">
	import { drawerExpand } from '$lib/motion';
	import { loadDrawerState, saveDrawerState } from '$lib/drawerRailState';

	export let items: Array<{ id: string; title: string; open?: boolean }> = [];

	let open: Record<string, boolean> = loadDrawerState();
	$: {
		let changed = false;
		const next = { ...open };
		for (const item of items) {
			if (!(item.id in next)) {
				next[item.id] = item.open ?? false;
				changed = true;
			}
		}
		if (changed) open = next;
	}

	function setOpen(id: string, next: boolean) {
		open = { ...open, [id]: next };
		saveDrawerState(open);
	}
</script>

<section class="drawer-rail" data-testid="drawer-rail">
	{#each items as item}
		<section class="drawer">
			<button type="button" aria-expanded={open[item.id]} on:click={() => setOpen(item.id, !open[item.id])}>
				<span>{item.title}</span>
				<strong>{open[item.id] ? '-' : '+'}</strong>
			</button>
			{#if open[item.id]}
				<div class="drawer-body" use:drawerExpand>
					{#if item.id === 'now'}
						<slot name="now" />
					{:else if item.id === 'try'}
						<slot name="try" />
					{:else if item.id === 'ladder'}
						<slot name="ladder" />
					{:else if item.id === 'cassette'}
						<slot name="cassette" />
					{:else if item.id === 'time'}
						<slot name="time" />
					{:else if item.id === 'ask'}
						<slot name="ask" />
					{:else if item.id === 'visual'}
						<slot name="visual" />
					{/if}
				</div>
			{/if}
		</section>
	{/each}
</section>

<style>
	.drawer-rail,
	.drawer,
	.drawer-body {
		display: grid;
		gap: 8px;
	}
	.drawer button {
		display: flex;
		justify-content: space-between;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 9px 10px;
		background: rgba(255, 255, 255, 0.025);
		color: var(--text);
	}
</style>
