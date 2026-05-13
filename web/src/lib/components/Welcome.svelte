<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { recipes } from '$lib/recipes';
	import type { Recipe } from '$lib/studio';
	import PostureChip from './PostureChip.svelte';

	const dispatch = createEventDispatcher<{ start: Recipe }>();
</script>

<section class="welcome" data-testid="welcome-recipes">
	{#each recipes as recipe}
		<button type="button" on:click={() => dispatch('start', recipe)}>
			<header>
				<h2>{recipe.title}</h2>
				<PostureChip color={recipe.preview_posture.posture_color} label={`${recipe.preview_posture.worlds[0]} · 0 OS reads`} />
			</header>
			<p>{recipe.one_liner}</p>
		</button>
	{/each}
</section>

<style>
	.welcome {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 12px;
	}
	.welcome button {
		display: grid;
		gap: 10px;
		text-align: left;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(20, 25, 32, 0.74);
		color: var(--text);
		padding: 14px;
	}
	.welcome button:hover {
		border-color: var(--border-strong);
	}
	.welcome header {
		display: grid;
		gap: 8px;
	}
	.welcome h2,
	.welcome p {
		margin: 0;
	}
	.welcome p {
		color: var(--muted);
		line-height: 1.45;
	}
	@media (max-width: 900px) {
		.welcome {
			grid-template-columns: 1fr;
		}
	}
</style>
