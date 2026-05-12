import { expect, test, type Page } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

const connectedLadder = [
	{
		difficulty: 'simple',
		label: 'Simple',
		title: 'Connected XTAL sorter ladder',
		taskType: 'new_behavior',
		mode: 'Written Plan',
		source: 'x07/docs/examples/agent-gate/xtal/toy-sorter',
		guard: 'solve-pure',
		prompt:
			'Use docs/examples/agent-gate/xtal/toy-sorter to create a spec-first sorter with generated tests and xtal.verify evidence.'
	},
	{
		difficulty: 'intermediate',
		label: 'Intermediate',
		title: 'Connected workflow graph ladder',
		taskType: 'behavior_change',
		mode: 'Voice Transcript',
		source: 'x07/docs/examples/agent-gate/xtal/workflow-graph',
		guard: 'solve-pure',
		prompt:
			'Transcript: follow docs/examples/agent-gate/xtal/workflow-graph, compute workflow makespan from task durations and dependency edges, reject cycles, generate tests from spec, and run xtal.verify.'
	},
	{
		difficulty: 'atlas',
		label: 'Atlas',
		title: 'Connected x07 Atlas project',
		taskType: 'new_behavior',
		mode: 'Written Plan',
		source: 'x07/docs/examples/wasm_showcases/x07_atlas',
		guard: 'wasm app',
		prompt:
			'Use docs/examples/wasm_showcases/x07_atlas to build the x07 Atlas full-stack WASM app with app profile validation, trace replay, release pack verification, provenance, deploy planning, and SLO evidence.'
	}
] as const;

function inspectOperation(page: Page, op: string) {
	return page.getByRole('button', { name: new RegExp(`Inspect( operation)? ${escapeRegex(op)}`) }).first();
}

function escapeRegex(value: string) {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

test('connected Studio creates simple-to-Atlas projects and runs the complex workflow through Loom', async ({ page }) => {
	await page.setViewportSize({ width: 1520, height: 940 });
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.locator('.rail-status')).toContainText('Loom online');
	await expect(page.getByText('Demo projection active')).toHaveCount(0);
	await expect(page.locator('footer')).toContainText('Connected to Loom daemon');
	await expect(page.getByRole('button', { name: 'Details' })).toHaveAttribute('aria-pressed', 'false');
	await page.getByRole('button', { name: 'Details' }).click();
	await expect(page.getByRole('button', { name: 'Details' })).toHaveAttribute('aria-pressed', 'true');
	await expect(page.getByLabel('Onboarding setup plan')).toContainText('First-run defaults');
	await expect(page.getByLabel('OpenAI Codex readiness')).toContainText('Ready');
	await expect(page.getByLabel('Claude Code readiness')).toContainText('Ready');
	await page.getByLabel('Active coding agent').selectOption({ label: 'Claude Code' });
	await expect(page.getByLabel('Active coding agent')).toHaveValue('claude-code');

	for (const project of connectedLadder) {
		await page.getByLabel('Project difficulty').selectOption(project.difficulty);
		await page.getByRole('button', { name: 'Load Brief', exact: true }).click();
		await expect(page.getByText(`${project.label} project brief loaded`)).toBeVisible();
		await expect(page.getByLabel('Example-backed XTAL template')).toContainText(project.source);
		await expect(page.getByLabel('World and budget guard')).toContainText(project.guard);

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
	await expect(page.getByLabel('Spec approval preview')).toContainText('atlas.app');
	await expect(page.getByLabel('XTAL automation plan')).toContainText('Human approval');
	await expect(page.getByLabel('XTAL automation plan')).toContainText('ready');

	await page.getByLabel('Revision').fill('Require explicit local platform delivery evidence before trust review.');
	await page.getByRole('button', { name: 'Request Changes' }).click();
	await expect(page.getByText('Revision routed back to intent review')).toBeVisible();
	await expect(page.getByRole('button', { name: 'Approve and Run' })).toBeDisabled();
	await page.getByRole('button', { name: 'Polish Intent' }).click();
	await expect(page.getByRole('button', { name: 'Approve and Run' })).toBeEnabled();

	await page.getByRole('tab', { name: 'Agents' }).click();
	await page.getByLabel('Active coding agent').selectOption({ label: 'OpenAI Codex' });
	await expect(page.getByLabel('Active coding agent')).toHaveValue('openai-codex');
	await page.getByRole('button', { name: 'Generate OpenAI Codex Handoff' }).click();
	await expect(page.locator('footer').getByText('OpenAI Codex handoff saved to')).toBeVisible();
	await expect(page.getByLabel('Agent handoff contract')).toContainText('OpenAI Codex');
	await expect(page.getByLabel('Agent handoff contract')).toContainText('Execution Boundary');
	await expect(page.getByLabel('Agent handoff contract')).toContainText('x07 run');
	await expect(page.getByLabel('Agent handoff contract')).toContainText('x07.studio.agent_event@0.1.0');
	await page.getByRole('button', { name: 'Plan OpenAI Codex Run' }).click();
	await expect(page.locator('footer').getByText('OpenAI Codex supervised launch plan recorded')).toBeVisible();
	await page.getByRole('button', { name: 'Run OpenAI Codex Command' }).click();
	await expect(page.locator('footer').getByText('OpenAI Codex approval required before supervised command')).toBeVisible();
	await page.getByRole('button', { name: 'Approve agent.approval.openai-codex' }).click();
	await expect(page.locator('footer').getByText('Agent checkpoint approved')).toBeVisible();
	await page.getByRole('button', { name: 'Run OpenAI Codex Command' }).click();
	await expect(page.locator('footer').getByText('OpenAI Codex supervised command succeeded')).toBeVisible();
	await expect(inspectOperation(page, 'agent.event.openai-codex.artifact')).toBeVisible();
	await page.getByLabel('Worklog filter').selectOption('codex');
	await expect(inspectOperation(page, 'agent.run.openai-codex')).toBeVisible();
	await expect(inspectOperation(page, 'agent.event.openai-codex.artifact')).toBeVisible();
	await page.getByLabel('Worklog filter').selectOption('all');

	await page.getByLabel('Active coding agent').selectOption({ label: 'Claude Code' });
	await expect(page.getByLabel('Active coding agent')).toHaveValue('claude-code');
	await page.getByRole('button', { name: 'Generate Claude Code Handoff' }).click();
	await expect(page.locator('footer').getByText('Claude Code handoff saved to')).toBeVisible();
	await expect(page.getByLabel('Agent handoff contract')).toContainText('Claude Code');
	await expect(page.getByLabel('Agent handoff contract')).toContainText('Execution Boundary');
	await expect(page.getByLabel('Agent handoff contract')).toContainText('x07 run');
	await expect(page.getByLabel('Agent handoff contract')).toContainText('x07.search_v1');
	await page.getByRole('button', { name: 'Plan Claude Code Run' }).click();
	await expect(page.locator('footer').getByText('Claude Code supervised launch plan recorded')).toBeVisible();
	await page.getByRole('button', { name: 'Run Claude Code Command' }).click();
	await expect(page.locator('footer').getByText('Claude Code approval required before supervised command')).toBeVisible();
	await page.getByRole('button', { name: 'Approve agent.approval.claude-code' }).click();
	await expect(page.locator('footer').getByText('Agent checkpoint approved')).toBeVisible();
	await page.getByRole('button', { name: 'Run Claude Code Command' }).click();
	await expect(page.locator('footer').getByText('Claude Code supervised command succeeded')).toBeVisible();
	await expect(inspectOperation(page, 'agent.event.claude-code.artifact')).toBeVisible();
	await page.getByRole('tab', { name: 'Intent' }).click();

	await page.getByRole('button', { name: 'Approve and Run' }).click();
	await expect(page.locator('footer')).toContainText('Verify passed and trust review opened', {
		timeout: 30_000
	});
	await expect(inspectOperation(page, 'project.seed.x07_atlas')).toBeVisible();
	await expect(inspectOperation(page, 'wasm.app.verify.atlas_release')).toBeVisible();
	await expect(inspectOperation(page, 'wasm.slo.eval.atlas_canary_ok')).toBeVisible();
	await expect(inspectOperation(page, 'lp.deploy.accept.local')).toBeVisible();
	await expect(inspectOperation(page, 'lp.deploy.status.local')).toBeVisible();
	await expect(page.getByLabel('XTAL automation plan')).toContainText('done');
	await expect(page.getByLabel('Trust review signals')).toContainText('Local platform delivery');
	await expect(page.getByLabel('Trust review signals')).toContainText('SLO evidence');
	await expect(page.getByLabel('x07 platform bridge')).toContainText('Platform delivery covered');
	await expect(page.getByLabel('Platform delivery lanes')).toContainText('lp.deploy.status.local');
	await page
		.getByLabel('x07 platform bridge')
		.getByRole('button', { name: /Inspect platform Platform delivery/ })
		.click();
	await expect(page.getByLabel('Selected operation inspector')).toContainText('lp.deploy.status.local');
	await page.getByLabel('Worklog filter').selectOption('claude');
	await expect(inspectOperation(page, 'agent.run.claude-code')).toBeVisible();
	await page.getByLabel('Worklog filter').selectOption('all');
});

test('connected Studio drives a simple XTAL session through Loom', async ({ page }) => {
	await page.setViewportSize({ width: 1440, height: 920 });
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.locator('.rail-status')).toContainText('Loom online');
	await expect(page.locator('footer')).toContainText('Connected to Loom daemon');
	await page.getByRole('button', { name: 'Details' }).click();
	const setupReadiness = page.getByLabel('Setup readiness');
	await expect(setupReadiness.locator('div', { hasText: 'x07 CLI' }).first()).toContainText('Ready');
	await expect(setupReadiness.locator('div', { hasText: 'x07-wasm' }).first()).toContainText('Ready');
	await expect(setupReadiness.locator('div', { hasText: 'x07 platform' }).first()).toContainText('Ready');
	await expect(page.getByLabel('Onboarding setup plan')).toContainText('First-run defaults');
	await expect(page.getByLabel('Onboarding setup plan')).toContainText('connected-e2e-bin');

	await page.getByLabel('Project title').fill('Connected XTAL sorter');
	await page.getByRole('button', { name: 'New Session', exact: true }).click();
	await expect(page.locator('footer')).toContainText('Created simple project: Connected XTAL sorter');

	await page.getByRole('button', { name: 'Polish Intent' }).click();
	await expect(page.getByText('Awaiting Approval', { exact: true })).toBeVisible();
	await expect(page.getByLabel('Spec approval preview')).toContainText('toy.sorter');

	await page.getByRole('button', { name: 'Approve and Run' }).click();
	await expect(page.locator('footer')).toContainText('Verify passed and trust review opened', {
		timeout: 30_000
	});
	await expect(inspectOperation(page, 'project.init.xtal-pure')).toBeVisible();
	await expect(inspectOperation(page, 'spec.scaffold')).toBeVisible();
	await expect(inspectOperation(page, 'tests.gen.write')).toBeVisible();
	await expect(inspectOperation(page, 'impl.sync.write')).toBeVisible();
	await expect(inspectOperation(page, 'xtal.verify')).toBeVisible();
	await expect(page.getByLabel('Operation artifacts')).toContainText('target/xtal/verify/summary.json');

	await page.getByLabel('Active binding').selectOption('spec.check');
	await page.getByRole('button', { name: 'Run Binding' }).click();
	await expect(page.locator('footer')).toContainText('Ran spec.check');
	await expect(inspectOperation(page, 'spec.check')).toBeVisible();
});
