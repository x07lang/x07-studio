<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { recipes } from '$lib/recipes';
	import type { Recipe } from '$lib/studio';
	import PostureChip from './PostureChip.svelte';

	export let recipeStart: ((recipe: Recipe) => void) | null = null;

	const dispatch = createEventDispatcher<{ start: Recipe }>();
	let startingRecipeId: string | null = null;

	function start(recipe: Recipe) {
		if (startingRecipeId === recipe.id) return;
		startingRecipeId = recipe.id;
		recipeStart?.(recipe);
		dispatch('start', recipe);
	}
</script>

<section class="welcome" data-testid="welcome-recipes">
	{#each recipes as recipe}
		<a
			href={`?recipe=${encodeURIComponent(recipe.id)}`}
			role="button"
			data-recipe-id={recipe.id}
			onclick={() => start(recipe)}
			onpointerdown={() => start(recipe)}
			onkeydown={(event) => {
				if (event.key === 'Enter' || event.key === ' ') start(recipe);
			}}
		>
			<header>
				<h2>{recipe.title}</h2>
				<PostureChip color={recipe.preview_posture.posture_color} label={`${recipe.preview_posture.worlds[0]} · 0 OS reads`} />
			</header>
			<p>{recipe.one_liner}</p>
			<code>{recipe.canonical_example_path}</code>
		</a>
	{/each}
</section>

<style>
	.welcome {
		display: grid;
		grid-template-columns: repeat(5, minmax(0, 1fr));
		gap: 12px;
	}
	.welcome a {
		display: grid;
		gap: 10px;
		text-align: left;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(20, 25, 32, 0.74);
		color: var(--text);
		padding: 14px;
		text-decoration: none;
	}
	.welcome a:hover {
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
	.welcome code {
		color: var(--mint);
		font-size: 11px;
		overflow-wrap: anywhere;
	}
	@media (max-width: 1200px) {
		.welcome {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (max-width: 900px) {
		.welcome {
			grid-template-columns: 1fr;
		}
	}
</style>
