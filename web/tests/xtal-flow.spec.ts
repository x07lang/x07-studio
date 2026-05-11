import { expect, test } from '@playwright/test';

const projects = [
	{
		difficulty: 'simple',
		label: 'Simple',
		title: 'XTAL toy sorter project',
		taskType: 'new_behavior',
		mode: 'Written Plan',
		source: 'x07/docs/examples/agent-gate/xtal/toy-sorter',
		prompt:
			'Use docs/examples/agent-gate/xtal/toy-sorter to create a spec-first sorter with generated tests and xtal.verify evidence.'
	},
	{
		difficulty: 'intermediate',
		label: 'Intermediate',
		title: 'XTAL workflow graph project',
		taskType: 'behavior_change',
		mode: 'Voice Transcript',
		source: 'x07/docs/examples/agent-gate/xtal/workflow-graph',
		prompt:
			'Transcript: follow docs/examples/agent-gate/xtal/workflow-graph, compute workflow makespan from task durations and dependency edges, reject cycles, generate tests from spec, and run xtal.verify.'
	},
	{
		difficulty: 'advanced',
		label: 'Advanced',
		title: 'State machine contracts project',
		taskType: 'new_behavior',
		mode: 'Written Plan',
		source: 'x07/docs/examples/readiness-checks/x07-sm-arch-contracts-smoke',
		prompt:
			'Use docs/examples/readiness-checks/x07-sm-arch-contracts-smoke to generate a lifecycle step function with x07 sm gen, arch contracts, drift checks, and budget-scoped tests.'
	},
	{
		difficulty: 'complex',
		label: 'Complex',
		title: 'Replayable API gateway project',
		taskType: 'new_behavior',
		mode: 'Voice Transcript',
		source: 'x07/docs/examples/apps/x07-api-gateway',
		prompt:
			'Transcript: use docs/examples/apps/x07-api-gateway to build a replayable API gateway with solve-pure routing, solve-rr upstream replay, sandbox policy, cassettes, and trust scripts.'
	},
	{
		difficulty: 'expert',
		label: 'Expert',
		title: 'DB drift guard project',
		taskType: 'incident_repair',
		mode: 'Incident Note',
		source: 'x07/docs/examples/apps/x07dbguard',
		prompt:
			'Incident note: use docs/examples/apps/x07dbguard to build a DB migration and drift guard with deterministic fingerprints, policy-gated apply, solve-rr drift verification, and certification evidence.'
	}
] as const;

test('user can create increasingly difficult x07 project sessions and exercise controls', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.getByText('Demo projection active')).toBeVisible();

	await page.getByRole('button', { name: 'Refresh Studio' }).click();
	await expect(page.getByText('Demo projection active')).toBeVisible();
	await page.getByLabel('Active room').selectOption('verify');
	await expect(page.getByRole('tab', { name: 'Verify' })).toHaveAttribute('aria-selected', 'true');
	await page.getByLabel('Active room').selectOption('intent');
	await expect(page.getByLabel('Example-backed XTAL template').getByText('x07/docs/examples/agent-gate/xtal/toy-sorter')).toBeVisible();

	for (const room of ['Spec', 'Realize', 'Verify', 'Repair', 'Trust', 'Ops', 'Agents', 'Intent']) {
		await page.getByRole('tab', { name: room }).click();
		await expect(page.getByRole('tab', { name: room })).toHaveAttribute('aria-selected', 'true');
	}
	await page.getByRole('tab', { name: 'Agents' }).click();
	await expect(
		page.getByLabel('Configured coding agents').getByText('OpenAI Codex', { exact: true })
	).toBeVisible();
	await expect(
		page.getByLabel('Configured coding agents').getByText('Claude Code', { exact: true })
	).toBeVisible();
	await page.getByRole('tab', { name: 'Intent' }).click();

	await page.getByLabel('Active coding agent').selectOption('Claude Code');

	for (const project of projects) {
		await page.getByLabel('Project difficulty').selectOption(project.difficulty);
		await page.getByRole('button', { name: 'Load Brief', exact: true }).click();
		await expect(page.getByText(`${project.label} project brief loaded`)).toBeVisible();
		await expect(page.getByLabel('Example-backed XTAL template')).toContainText(project.source);

		await page.getByLabel('Project title').fill(project.title);
		await page.getByLabel('Task type').selectOption(project.taskType);
		await page.getByLabel('Initial plan').fill(project.prompt);
		await page.getByLabel(project.mode).click();
		await page.getByRole('button', { name: 'New Session', exact: true }).click();

		await expect(
			page.getByText(`Created ${project.label.toLowerCase()} project: ${project.title}`)
		).toBeVisible();
		await expect(
			page.getByLabel('Created projects').getByText(`${project.label}: ${project.title}`)
		).toBeVisible();
	}

	await page.getByRole('button', { name: 'Polish Intent' }).click();
	await expect(page.getByText('Awaiting Approval', { exact: true })).toBeVisible();
	await expect(page.getByText('incident report:', { exact: false })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Approve and Run' })).toBeEnabled();

	await page.getByLabel('Revision').fill('Add a deterministic repair witness before implementation.');
	await page.getByRole('button', { name: 'Request Changes' }).click();
	await expect(page.getByText('Revision routed back to intent review')).toBeVisible();

	await page.getByRole('button', { name: 'Approve Spec' }).click();
	await expect(page.getByText('Spec approved; realization lane is unlocked')).toBeVisible();

	await page.getByRole('tab', { name: 'Agents' }).click();
	await page.getByRole('button', { name: 'Generate Claude Code Handoff' }).click();
	await expect(page.locator('footer').getByText('Claude Code handoff saved to')).toBeVisible();
	await page.getByRole('button', { name: 'Plan Claude Code Run' }).click();
	await expect(page.locator('footer').getByText('Claude Code supervised launch plan recorded')).toBeVisible();
	await page.getByRole('button', { name: 'Run Claude Code Command' }).click();
	await expect(page.locator('footer').getByText('Claude Code approval required before supervised command')).toBeVisible();
	await page.getByRole('button', { name: 'Approve agent.approval.claude-code' }).click();
	await expect(page.locator('footer').getByText('Agent checkpoint approved')).toBeVisible();
	await page.getByRole('button', { name: 'Run Claude Code Command' }).click();
	await expect(page.locator('footer').getByText('Claude Code supervised command succeeded')).toBeVisible();
	await page.getByRole('tab', { name: 'Intent' }).click();

	await page.getByRole('button', { name: 'Approve and Run' }).click();
	await expect(page.getByText(/Verify produced a repair session|Verify passed and trust review opened/)).toBeVisible();
	await expect(page.getByText('Agent Visible Worklog')).toBeVisible();
	await expect(page.locator('code').filter({ hasText: 'agent.run.claude-code' })).toBeVisible();
	await expect(page.locator('code').filter({ hasText: 'agent.approval.claude-code' })).toBeVisible();
	await expect(page.locator('code').filter({ hasText: 'agent.supervise.claude-code' })).toBeVisible();
	await expect(page.locator('code').filter({ hasText: 'agent.handoff.claude-code' })).toBeVisible();
	await expect(page.locator('code').filter({ hasText: 'project.init.xtal-pure' })).toBeVisible();
	await expect(page.locator('code').filter({ hasText: 'impl.sync.write' })).toBeVisible();
	await expect(page.locator('code').filter({ hasText: 'xtal.verify' })).toBeVisible();
	await page.getByRole('button', { name: /Inspect xtal\.verify/ }).first().click();
	await expect(page.getByLabel('Selected operation inspector')).toContainText('xtal.verify');
	await expect(page.getByLabel('Operation artifacts')).toContainText(
		'target/xtal/verify/summary.json'
	);

	await page.getByLabel('Worklog filter').selectOption('claude');
	await expect(page.locator('code').filter({ hasText: 'agent.run.claude-code' })).toBeVisible();
	await page.getByLabel('Worklog filter').selectOption('all');
	await page.getByLabel('Auto-scroll').uncheck();
	await expect(page.getByLabel('Auto-scroll')).not.toBeChecked();
	await page.getByLabel('Auto-scroll').check();
	await expect(page.getByLabel('Auto-scroll')).toBeChecked();

	await page.getByLabel('Active binding').selectOption('spec.check');
	await page.getByRole('button', { name: 'Run Binding' }).click();
	await expect(page.getByText('Ran spec.check')).toBeVisible();
	await page.getByRole('button', { name: 'spec.check', exact: true }).click();
	await expect(page.getByText('Ran spec.check')).toBeVisible();
});
