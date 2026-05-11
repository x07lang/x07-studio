import { expect, test } from '@playwright/test';

const projects = [
	{
		difficulty: 'simple',
		label: 'Simple',
		title: 'Sorter smoke project',
		taskType: 'new_behavior',
		mode: 'Written Plan',
		prompt: 'Sort signed integers in ascending order, reject empty input, and keep the operation pure.'
	},
	{
		difficulty: 'intermediate',
		label: 'Intermediate',
		title: 'Workflow graph project',
		taskType: 'behavior_change',
		mode: 'Voice Transcript',
		prompt:
			'Transcript: compute workflow makespan from task durations and dependency edges, reject cycles, and expose reviewable examples.'
	},
	{
		difficulty: 'complex',
		label: 'Complex',
		title: 'Incident repair project',
		taskType: 'incident_repair',
		mode: 'Incident Note',
		prompt:
			'Incident note: verification failed after a policy change. Classify the repair, preserve the spec unless a witness changes, rerun verification, and certify evidence.'
	}
] as const;

test('user can create increasingly difficult x07 project sessions and exercise controls', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.getByText('Demo projection active')).toBeVisible();

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

	await page.getByRole('button', { name: 'spec.check' }).click();
	await expect(page.getByText('Ran spec.check')).toBeVisible();
});
