import { expect, test, type Page } from '@playwright/test';

const projects = [
	{
		difficulty: 'simple',
		label: 'Simple',
		title: 'XTAL toy sorter project',
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
		title: 'XTAL workflow graph project',
		taskType: 'behavior_change',
		mode: 'Voice Transcript',
		source: 'x07/docs/examples/agent-gate/xtal/workflow-graph',
		guard: 'solve-pure',
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
		guard: 'arch budget',
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
		guard: 'solve-rr',
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
		guard: 'network / OS',
		prompt:
			'Incident note: use docs/examples/apps/x07dbguard to build a DB migration and drift guard with deterministic fingerprints, policy-gated apply, solve-rr drift verification, and certification evidence.'
	},
	{
		difficulty: 'atlas',
		label: 'Atlas',
		title: 'x07 Atlas full-stack app project',
		taskType: 'new_behavior',
		mode: 'Written Plan',
		source: 'x07/docs/examples/wasm_showcases/x07_atlas',
		guard: 'wasm app',
		prompt:
			'Use docs/examples/wasm_showcases/x07_atlas to build the x07 Atlas full-stack WASM app with app profile validation, trace replay, release pack verification, provenance, deploy planning, and SLO evidence.'
	}
] as const;

const sorterSpec = JSON.stringify({
	schema_version: 'x07.x07spec@0.1.0',
	module_id: 'toy.sorter',
	operations: [{ id: 'op.sort_u8_asc.v1', name: 'toy.sorter.sort_u8_asc' }]
});

function inspectOperation(page: Page, op: string) {
	return page.getByRole('button', { name: new RegExp(`Inspect( operation)? ${escapeRegex(op)}`) }).first();
}

function escapeRegex(value: string) {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

test('user can create increasingly difficult x07 project sessions and exercise controls', async ({ page }) => {
	await page.setViewportSize({ width: 1728, height: 972 });
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.getByText('Demo projection active')).toBeVisible();
	await expect(page.getByLabel('Operation log')).toBeInViewport();
	await expect(page.getByLabel('Canonical command lane')).toContainText('x07 canonical command lane');
	await expect(page.getByLabel('XTAL automation plan')).toContainText('approval gated');
	await expect(page.getByLabel('XTAL automation plan')).toContainText('Project scaffold');
	await page.locator('#command-lane-input').fill('x07 run --workspace demo --from intent --to verify');
	await page.getByLabel('Command lane mode').selectOption('plan');
	await page.getByLabel('Command lane environment').selectOption('sandbox');
	await page.getByLabel('Command lane region').selectOption('us-east-1');
	await page.getByLabel('Canonical command lane').getByRole('button', { name: 'Plan' }).click();
	await expect(page.locator('footer')).toContainText('Planned');
	await page.getByLabel('Command lane mode').selectOption('execute');
	await expect(page.getByLabel('Command lane trust meter')).toContainText('Trust');
	await expect(page.getByText('Agent Lane')).toBeVisible();
	await expect(page.getByLabel('Trust review signals')).toContainText('No review signals recorded');
	await expect(page.getByRole('region', { name: 'Workspace radar' })).toContainText('XTAL');
	await expect(page.getByRole('region', { name: 'Workspace radar' })).toContainText('Sessions');
	await expect(page.getByRole('region', { name: 'Workspace radar' })).toContainText('Tests');
	await expect(page.getByRole('region', { name: 'Workspace radar' })).toContainText('Verify');
	await expect(page.getByRole('region', { name: 'Workspace radar' })).toContainText('Provider');
	await expect(page.getByLabel('Setup readiness')).toContainText('x07-wasm');
	await expect(page.getByLabel('Setup readiness')).toContainText('x07 platform');
	await expect(page.getByLabel('Onboarding setup plan')).toContainText('First-run defaults');
	await expect(page.getByLabel('Onboarding setup plan')).toContainText('bootstrap_components.py');
	await expect(page.getByLabel('Onboarding setup plan')).toContainText('OpenAI Codex agent');
	await expect(page.getByLabel('Counterexample theater')).toContainText('No counterexample captured');
	await expect(page.getByLabel('Provider intent polish')).toContainText('Deterministic polish only');
	await page.getByLabel('Provider intent polish').getByRole('checkbox').check();
	await expect(page.getByLabel('Provider profile')).toBeEnabled();
	await page.getByLabel('Provider profile').fill('ollama-local');
	await expect(page.getByLabel('Provider intent polish')).toContainText('Model suggestions');
	await page.getByLabel('Provider intent polish').getByRole('checkbox').uncheck();

	await page.getByRole('button', { name: 'Refresh Studio' }).click();
	await expect(page.getByText('Demo projection active')).toBeVisible();
	await page.getByRole('button', { name: 'Brownfield Extract' }).click();
	await expect(page.getByText('Brownfield extract intake prepared')).toBeVisible();
	await expect(page.getByLabel('Task type')).toHaveValue('brownfield_extract');
	await expect(page.getByLabel('Initial plan')).toHaveValue(/Extract the current project behavior/);
	await page.getByRole('button', { name: 'New Session', exact: true }).click();
	await expect(page.getByText('Created brownfield project: Brownfield XTAL extract')).toBeVisible();
	await page.getByRole('button', { name: 'Polish Intent' }).click();
	await expect(page.getByText('Awaiting Approval', { exact: true })).toBeVisible();
	await page.getByRole('button', { name: 'Approve Spec' }).click();
	await expect(inspectOperation(page, 'spec.extract')).toBeVisible();
	await expect(page.getByText('Spec approved; realization lane is unlocked')).toBeVisible();
	await page.getByRole('button', { name: 'Incident Improve' }).click();
	await expect(page.getByText('Incident improve intake prepared')).toBeVisible();
	await expect(page.getByLabel('Task type')).toHaveValue('incident_repair');
	await expect(page.getByLabel('Incident Note')).toBeChecked();
	await page.getByRole('button', { name: 'New Session', exact: true }).click();
	await expect(page.getByText('Created incident project: Incident improvement loop')).toBeVisible();
	await page.getByRole('button', { name: 'Polish Intent' }).click();
	await expect(page.getByText('Awaiting Approval', { exact: true })).toBeVisible();
	await page.getByRole('button', { name: 'Approve and Run' }).click();
	await expect(page.getByText('Incident ingest/improve evidence recorded')).toBeVisible();
	await expect(inspectOperation(page, 'xtal.ingest')).toBeVisible();
	await expect(inspectOperation(page, 'xtal.improve')).toBeVisible();
	await page.getByRole('button', { name: 'Intent', exact: true }).click();
	await expect(page.getByText('Intent intake prepared')).toBeVisible();
	await expect(page.getByLabel('Task type')).toHaveValue('new_behavior');
	await expect(page.getByLabel('Written Plan')).toBeChecked();
	await page.getByLabel('Existing Spec').click();
	await expect(page.getByLabel('Existing Spec')).toBeChecked();
	await page.getByLabel('Initial plan').fill(sorterSpec);
	await page.getByRole('button', { name: 'Polish Intent' }).click();
	await expect(page.getByLabel('Spec approval preview')).toContainText('toy.sorter');
	await page.getByLabel('Active room').selectOption('verify');
	await expect(page.getByRole('tab', { name: 'Verify' })).toHaveAttribute('aria-selected', 'true');
	await page.getByLabel('Active room').selectOption('intent');
	await expect(page.getByLabel('Example-backed XTAL template').getByText('x07/docs/examples/agent-gate/xtal/toy-sorter')).toBeVisible();

	for (const room of ['Spec', 'Realize', 'Verify', 'Repair', 'Trust', 'Ops', 'Agents', 'MCP', 'Intent']) {
		await page.getByRole('tab', { name: room }).click();
		await expect(page.getByRole('tab', { name: room })).toHaveAttribute('aria-selected', 'true');
	}
	await page.getByRole('tab', { name: 'MCP' }).click();
	await expect(page.getByLabel('Session doctrine')).toContainText('x07.search_v1');
	await expect(page.getByLabel('Session doctrine')).toContainText('x07/docs/getting-started/agent-quickstart.md');
	await page.getByRole('tab', { name: 'Agents' }).click();
	await expect(
		page.getByLabel('Configured coding agents').getByText('OpenAI Codex', { exact: true })
	).toBeVisible();
	await expect(
		page.getByLabel('Configured coding agents').getByText('Claude Code', { exact: true })
	).toBeVisible();
	await expect(page.getByLabel('OpenAI Codex readiness')).toContainText('Ready');
	await expect(page.getByLabel('OpenAI Codex readiness')).toContainText('Human checkpoint before execute');
	await expect(page.getByLabel('Claude Code readiness')).toContainText('Ready');
	await expect(page.getByLabel('Claude Code readiness')).toContainText('Human checkpoint before execute');
	await page.getByRole('tab', { name: 'Intent' }).click();

	await page.getByLabel('Active coding agent').selectOption({ label: 'Claude Code' });
	await expect(page.getByLabel('Active coding agent')).toHaveValue('claude-code');

	for (const project of projects) {
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
	await expect(inspectOperation(page, 'intent.formalize')).toBeVisible();
	await expect(page.getByRole('button', { name: 'Approve and Run' })).toBeEnabled();

	await page.getByLabel('Revision').fill('Add a deterministic repair witness before implementation.');
	await page.getByRole('button', { name: 'Request Changes' }).click();
	await expect(page.getByText('Revision routed back to intent review')).toBeVisible();
	await expect(page.getByLabel('Approval loop ledger')).toContainText('Revision 1');
	await expect(page.getByLabel('Approval loop ledger')).toContainText(
		'approval blocked until the agent repolishes revisions'
	);
	await expect(page.getByRole('button', { name: 'Approve Spec' })).toBeDisabled();
	await expect(page.getByRole('button', { name: 'Approve and Run' })).toBeDisabled();
	await page.getByRole('button', { name: 'Polish Intent' }).click();
	await expect(page.getByText('Awaiting Approval', { exact: true })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Approve Spec' })).toBeEnabled();
	await expect(page.getByRole('button', { name: 'Approve and Run' })).toBeEnabled();

	await page.getByRole('button', { name: 'Approve Spec' }).click();
	await expect(page.getByText('Spec approved; realization lane is unlocked')).toBeVisible();
	await expect(page.getByLabel('XTAL automation plan')).toContainText('contract locked');
	await expect(page.getByLabel('Session doctrine')).toContainText('x07.doc_v1');
	await expect(page.getByLabel('Session doctrine')).toContainText('x07/docs/getting-started/agent-quickstart.md');
	await page
		.getByLabel('Session doctrine')
		.getByRole('button', { name: 'Preview x07/docs/getting-started/agent-quickstart.md' })
		.click();
	await expect(page.getByLabel('Documentation preview')).toContainText('agent quickstart');
	await expect(page.getByLabel('Documentation preview')).toContainText('x07 run');

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
	await expect(inspectOperation(page, 'agent.event.claude-code.artifact')).toBeVisible();
	await expect(page.getByLabel('Trust review signals')).toContainText('Artifact surfaced');
	await page.getByLabel('Trust review signals').getByRole('button', { name: /Review Artifact surfaced/ }).click();
	await expect(page.getByLabel('Selected operation inspector')).toContainText(
		'agent.event.claude-code.artifact'
	);
	await page.getByRole('tab', { name: 'Intent' }).click();

	await page.getByRole('button', { name: 'Approve and Run' }).click();
	await expect(page.getByText(/Verify produced a repair session|Verify passed and trust review opened/)).toBeVisible();
	await expect(page.getByLabel('XTAL automation plan')).toContainText('done');
	await expect(page.getByLabel('XTAL automation plan')).toContainText('x07-wasm app build');
	await expect(page.getByLabel('Trust review signals')).toContainText('Local platform delivery');
	await expect(page.getByLabel('Trust review signals')).toContainText('SLO evidence');
	await expect(page.getByLabel('Trust review signals')).toContainText('Release evidence');
	await page.getByLabel('Trust review signals').getByRole('button', { name: /Review Local platform delivery/ }).click();
	await expect(page.getByLabel('Selected operation inspector')).toContainText('lp.deploy.status.local');
	await expect(page.getByLabel('Operation artifacts')).toContainText('.x07/platform');
	await expect(page.getByText('Agent Visible Worklog')).toBeVisible();
	await page.getByLabel('Worklog filter').selectOption('claude');
	await expect(inspectOperation(page, 'agent.run.claude-code')).toBeVisible();
	await expect(inspectOperation(page, 'agent.event.claude-code.artifact')).toBeVisible();
	await expect(inspectOperation(page, 'agent.approval.claude-code')).toBeVisible();
	await expect(inspectOperation(page, 'agent.supervise.claude-code')).toBeVisible();
	await expect(inspectOperation(page, 'agent.handoff.claude-code')).toBeVisible();
	await page.getByLabel('Worklog filter').selectOption('all');
	await expect(inspectOperation(page, 'wasm.app.verify.atlas_release')).toBeVisible();
	await expect(inspectOperation(page, 'lp.deploy.accept.local')).toBeVisible();
	await expect(inspectOperation(page, 'lp.deploy.status.local')).toBeVisible();
	await page.getByRole('button', { name: /Inspect lp\.deploy\.status\.local/ }).first().click();
	await expect(page.getByLabel('Selected operation inspector')).toContainText('lp.deploy.status.local');
	await expect(page.getByLabel('Operation artifacts')).toContainText(
		'.x07/platform'
	);
	await page.getByLabel('Trust review signals').getByRole('button', { name: /Review SLO evidence/ }).click();
	await expect(page.getByLabel('Selected operation inspector')).toContainText('wasm.slo.eval.atlas_canary_ok');

	await page.getByLabel('Worklog filter').selectOption('claude');
	await expect(inspectOperation(page, 'agent.run.claude-code')).toBeVisible();
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
