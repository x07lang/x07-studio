<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { commands, searchCommands } from '$lib/commands';

	export let open = false;
	let query = '';
	let selected = 0;

	$: results = searchCommands(query, commands);
	const dispatch = createEventDispatcher<{ close: void; run: string }>();

	function run(action: string) {
		dispatch('run', action);
		dispatch('close');
		query = '';
		selected = 0;
	}
</script>

{#if open}
	<section class="command-palette" data-testid="command-palette">
		<input
			bind:value={query}
			placeholder="Command"
			on:keydown={(event) => {
				if (event.key === 'Escape') dispatch('close');
				if (event.key === 'ArrowDown') selected = Math.min(selected + 1, results.length - 1);
				if (event.key === 'ArrowUp') selected = Math.max(selected - 1, 0);
				if (event.key === 'Enter' && results[selected]) run(results[selected].action);
			}}
		/>
		<div>
			{#each results as command, index}
				<button type="button" class:active={index === selected} on:click={() => run(command.action)}>
					<strong>{command.title}</strong>
					<span>{command.group} · {command.hint}</span>
				</button>
			{/each}
		</div>
	</section>
{/if}

<style>
	.command-palette {
		position: fixed;
		top: 12vh;
		left: 50%;
		z-index: 30;
		width: min(620px, calc(100vw - 24px));
		transform: translateX(-50%);
		display: grid;
		gap: 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius);
		background: var(--surface-3, #0c1016);
		padding: 10px;
		box-shadow: var(--shadow);
	}
	input {
		width: 100%;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(255, 255, 255, 0.05);
		color: var(--text);
		padding: 10px;
	}
	.command-palette div {
		display: grid;
		gap: 4px;
	}
	button {
		display: grid;
		gap: 3px;
		text-align: left;
		border: 1px solid transparent;
		border-radius: var(--radius);
		background: transparent;
		color: var(--text);
		padding: 8px;
	}
	button.active,
	button:hover {
		border-color: var(--border);
		background: rgba(85, 214, 231, 0.08);
	}
	span {
		color: var(--muted);
		font-size: 12px;
	}
</style>
