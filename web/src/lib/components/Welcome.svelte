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

	$: pathParts = (path: string) => {
		const trimmed = path.replace(/\/$/, '');
		const segments = trimmed.split('/');
		return {
			lead: segments.slice(0, -1).join('/'),
			last: segments[segments.length - 1] ?? trimmed
		};
	};
</script>

<section class="welcome" data-testid="welcome-recipes">
	<header class="intro">
		<span class="eyebrow">Canonical recipes</span>
		<h1>Pick a starter — built from x07's agent-gate examples.</h1>
		<p>Each card opens a real verified scaffold backed by <code>docs/examples/agent-gate/</code>.</p>
	</header>
	<div class="grid">
		{#each recipes as recipe}
			{@const parts = pathParts(recipe.canonical_example_path ?? '')}
			<a
				class="card {recipe.preview_posture.posture_color}"
				href={`?recipe=${encodeURIComponent(recipe.id)}`}
				role="button"
				data-recipe-id={recipe.id}
				data-recipe-color={recipe.preview_posture.posture_color}
				onclick={() => start(recipe)}
				onpointerdown={() => start(recipe)}
				onkeydown={(event) => {
					if (event.key === 'Enter' || event.key === ' ') start(recipe);
				}}
			>
				<header>
					<h2>{recipe.title}</h2>
					<PostureChip color={recipe.preview_posture.posture_color} label={recipe.preview_posture.worlds[0]} />
				</header>
				<p>{recipe.one_liner}</p>
				<footer>
					<code class="path">
						<span class="lead">{parts.lead}/</span><span class="last">{parts.last}</span>
					</code>
					<span class="start-cue" aria-hidden="true">Start →</span>
				</footer>
			</a>
		{/each}
	</div>
</section>

<style>
	.welcome {
		display: grid;
		gap: 18px;
	}
	.intro {
		display: grid;
		gap: 6px;
	}
	.eyebrow {
		color: var(--muted);
		font-size: 11px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		font-weight: 600;
	}
	.intro h1 {
		margin: 0;
		font-family: var(--font-mono);
		font-size: clamp(20px, 1.4vw + 14px, 26px);
		font-weight: 600;
		letter-spacing: -0.015em;
		line-height: 1.25;
		color: var(--text);
	}
	.intro p {
		margin: 0;
		color: var(--muted);
		font-size: 13px;
	}
	.intro code {
		font-family: var(--font-mono);
		color: var(--accent-pure);
		font-size: 12px;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(5, minmax(0, 1fr));
		gap: 12px;
	}
	.card {
		position: relative;
		display: grid;
		gap: 10px;
		text-align: left;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
		color: var(--text);
		padding: 14px;
		text-decoration: none;
		transition: transform var(--motion-fast, 120ms) var(--ease-out, ease-out),
			border-color var(--motion-fast, 120ms) var(--ease-out, ease-out),
			box-shadow var(--motion-base, 200ms) var(--ease-out, ease-out);
		isolation: isolate;
		overflow: hidden;
	}
	.card::before {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: inherit;
		pointer-events: none;
		opacity: 0;
		background: radial-gradient(120% 100% at 0% 0%, var(--card-accent, transparent) 0%, transparent 50%);
		transition: opacity var(--motion-base, 200ms) var(--ease-out, ease-out);
		z-index: -1;
	}
	.card.green { --card-accent: rgba(114, 228, 180, 0.22); }
	.card.amber { --card-accent: rgba(255, 195, 91, 0.22); }
	.card.red   { --card-accent: rgba(243, 111, 141, 0.22); }
	.card:hover,
	.card:focus-visible {
		border-color: var(--border-strong);
		transform: translateY(-2px);
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
		outline: none;
	}
	.card:hover::before,
	.card:focus-visible::before {
		opacity: 1;
	}
	.card:active {
		transform: translateY(-1px);
	}
	.card header {
		display: grid;
		gap: 6px;
	}
	.card h2 {
		margin: 0;
		font-family: var(--font-mono);
		font-size: 13px;
		font-weight: 600;
		letter-spacing: -0.005em;
		line-height: 1.3;
	}
	.card p {
		margin: 0;
		color: var(--muted);
		font-size: 12px;
		line-height: 1.45;
		min-height: 32px;
	}
	.card footer {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 6px;
		margin-top: 2px;
	}
	.path {
		font-family: var(--font-mono);
		font-size: 10px;
		overflow-wrap: anywhere;
		display: inline-flex;
		flex-wrap: wrap;
	}
	.path .lead { color: var(--faint); }
	.path .last { color: var(--mint); }
	.start-cue {
		color: var(--accent-pure);
		font-size: 11px;
		font-weight: 600;
		opacity: 0;
		transition: opacity var(--motion-fast, 120ms) var(--ease-out, ease-out), transform var(--motion-fast, 120ms) var(--ease-out, ease-out);
		transform: translateX(-4px);
	}
	.card:hover .start-cue,
	.card:focus-visible .start-cue {
		opacity: 1;
		transform: translateX(0);
	}
	@media (max-width: 1200px) {
		.grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (max-width: 700px) {
		.grid {
			grid-template-columns: 1fr;
		}
	}
</style>
