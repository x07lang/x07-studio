<script lang="ts">
	import type { ArchCheckReport } from '$lib/studio';

	export let report: ArchCheckReport | null = null;
</script>

<span class="arch-badge" class:ok={report?.passed} title={report?.violations.map((v) => `${v.rule}: ${v.summary}`).join('\n') || 'Repo-level architecture invariants pass'}>
	arch-check {report?.passed ? 'ok' : report ? `${report.violations.length} issues` : 'pending'}
</span>

<style>
	.arch-badge {
		display: inline-flex;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 3px 7px;
		color: var(--amber);
		font-size: 11px;
	}
	.arch-badge.ok {
		color: var(--mint);
		border-color: rgba(114, 228, 180, 0.32);
	}
</style>
