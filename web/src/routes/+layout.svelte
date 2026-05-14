<script lang="ts">
	import { onMount } from 'svelte';
	import '../app.css';
	import { installErrorReporter } from '$lib/errorReporter';

	onMount(() => {
		installErrorReporter();
	});
</script>

<slot />
