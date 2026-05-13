<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { AgentProfile, AgentRole } from '$lib/studio';
	import RoleBadge from './RoleBadge.svelte';

	export let agents: AgentProfile[] = [];

	const roles: AgentRole[] = ['architect', 'coder', 'reviewer', 'conductor'];
	const dispatch = createEventDispatcher<{
		save: { agentId: string; defaultRole: AgentRole; eligibleRoles: AgentRole[] };
	}>();

	function save(agent: AgentProfile, role: AgentRole) {
		const eligible = Array.from(new Set([role, ...agent.eligible_roles]));
		dispatch('save', { agentId: agent.id, defaultRole: role, eligibleRoles: eligible });
	}
</script>

<section class="agent-role-settings" data-testid="agent-role-settings">
	<header>
		<h3>Agent roles</h3>
	</header>
	{#each agents as agent}
		<div class="role-row">
			<div>
				<strong>{agent.label}</strong>
				<RoleBadge role={agent.default_role} />
			</div>
			<select
				aria-label={`${agent.label} role`}
				value={agent.default_role}
				on:change={(event) => save(agent, (event.currentTarget as HTMLSelectElement).value as AgentRole)}
			>
				{#each roles as role}
					<option value={role}>{role}</option>
				{/each}
			</select>
		</div>
	{/each}
</section>

<style>
	.agent-role-settings {
		display: grid;
		gap: 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 10px;
		background: rgba(255, 255, 255, 0.03);
	}
	h3 {
		margin: 0;
		font-size: 13px;
	}
	.role-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(120px, auto);
		gap: 8px;
		align-items: center;
	}
	.role-row div {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.role-row strong {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	select {
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: rgba(15, 23, 42, 0.8);
		color: var(--text);
		padding: 6px 8px;
	}
</style>
