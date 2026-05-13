<script lang="ts">
	import type { AgentRole } from '$lib/studio';

	export let role: AgentRole = 'conductor';
</script>

<span class="role-badge {role}" data-testid={`role-${role}`}>{role}</span>

<style>
	.role-badge {
		display: inline-flex;
		align-items: center;
		border: 1px solid var(--border);
		border-radius: 999px;
		padding: 2px 7px;
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		color: var(--text);
	}
	.role-badge.conductor {
		border-color: rgba(85, 214, 231, 0.48);
		background: rgba(85, 214, 231, 0.12);
	}
	.role-badge.architect {
		border-color: rgba(114, 228, 180, 0.48);
		background: rgba(114, 228, 180, 0.12);
	}
	.role-badge.coder {
		border-color: rgba(255, 195, 91, 0.54);
		background: rgba(255, 195, 91, 0.14);
	}
	.role-badge.reviewer {
		border-color: rgba(168, 139, 250, 0.52);
		background: rgba(168, 139, 250, 0.14);
	}
</style>
