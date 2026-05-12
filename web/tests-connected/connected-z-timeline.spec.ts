import { expect, test } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

test('connected timeline formalizes, clarifies, builds, tries, and scans incidents', async ({ page }) => {
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();

	await page
		.getByTestId('composer-input')
		.fill('Build a stable sorter for byte arrays. Equal items must keep order. Reject empty input.');
	await page.getByTestId('composer-submit').click();

	await expect(page.getByTestId('turn-user_intent')).toBeVisible({ timeout: 15_000 });
	await expect(page.getByTestId('turn-agent_clarify')).toBeVisible({ timeout: 20_000 });
	await expect(page.getByTestId('clarify-card')).toHaveCount(2, { timeout: 20_000 });

	const empty = page.locator('[data-testid="clarify-card"][data-question-id="q-claude-empty-input"]');
	await empty.getByRole('button', { name: 'Reject with an error' }).click();
	await empty.getByTestId('clarify-answer-submit').click();
	await expect(empty.getByTestId('clarify-answer-locked')).toBeVisible({ timeout: 10_000 });

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified')).toBeVisible({ timeout: 45_000 });
	await expect(page.getByTestId('run-invocation')).toContainText('x07 run');
	await expect(page.getByTestId('shipping-ladder')).toContainText('Local preview');

	await page.getByTestId('try-it-panel').getByRole('textbox').first().fill('[2,1]');
	await page.getByTestId('try-it-panel').getByRole('button', { name: 'Run it' }).click();
	await expect(page.getByTestId('try-it-panel')).toContainText('Latest verify evidence', {
		timeout: 15_000
	});

	await page.getByRole('button', { name: 'Scan incidents' }).click();
	await expect(page.getByTestId('turn-incident')).toContainText('demo-incident', {
		timeout: 15_000
	});
	await page.getByRole('button', { name: 'Repair this' }).click();
	await expect(page.getByTestId('turn-repair')).toContainText('latest', { timeout: 15_000 });
});

test('connected timeline runs the Atlas workflow lane', async ({ page }) => {
	const prompt =
		'Use docs/examples/wasm_showcases/x07_atlas to build x07 Atlas with profile validation, trace replay, release pack verification, provenance, deploy planning, and SLO evidence.';
	await page.goto('/?details=open');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();

	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText('wasm_showcases/x07_atlas', {
		timeout: 15_000
	});

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified')).toBeVisible({ timeout: 45_000 });
	await expect(page.getByTestId('shipping-ladder')).toContainText('Production');

	const response = await page.request.get('/v1/sessions');
	const sessions = (await response.json()) as Array<{ title: string; op_log: Array<{ op: string }> }>;
	const atlas = sessions.find((session) => session.title === prompt.slice(0, 80));
	expect(atlas?.op_log.some((op) => op.op === 'wasm.app.verify.atlas_release')).toBe(true);
	expect(atlas?.op_log.some((op) => op.op === 'wasm.provenance.verify.atlas_release')).toBe(true);
	expect(atlas?.op_log.some((op) => op.op === 'lp.deploy.status.local')).toBe(true);
});

test('connected handoff embeds detected service genpack schema', async ({ page }) => {
	const prompt = 'Build an API gateway service for account reads with request validation.';
	await page.goto('/?details=open');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();

	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText('API gateway', {
		timeout: 15_000
	});

	const sessionsResponse = await page.request.get('/v1/sessions');
	const sessions = (await sessionsResponse.json()) as Array<{ session_id: string; title: string }>;
	const service = sessions.find((session) => session.title === prompt.slice(0, 80));
	expect(service).toBeTruthy();

	const handoffResponse = await page.request.post(
		`/v1/sessions/${service?.session_id}/agents/openai-codex/handoff`
	);
	expect(handoffResponse.ok()).toBe(true);
	const handoff = (await handoffResponse.json()) as { handoff: { prompt: string } };
	expect(handoff.handoff.prompt).toContain('## Service Genpack Context');
	expect(handoff.handoff.prompt).toContain('Detected archetype: `api-cell`');
	expect(handoff.handoff.prompt).toContain('x07.service.genpack.schema_v1');
	expect(handoff.handoff.prompt).toContain('api-cell ::= service operations policy');
});

test('connected continuity tools run quorum, sync claims, cassette branches, and visual emit', async ({
	page
}) => {
	const prompt = 'Build an API gateway service with a replay cassette and visual task flow.';
	await page.goto('/?details=open');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();

	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText('API gateway', {
		timeout: 15_000
	});

	await page.getByTestId('run-quorum').click();
	await expect(page.getByTestId('timeline')).toContainText('openai-codex', { timeout: 30_000 });
	await expect(page.getByTestId('timeline')).toContainText('claude-code', { timeout: 30_000 });

	await page.getByRole('button', { name: 'Continue elsewhere' }).click();
	await expect(page.getByText('Sync code')).toBeVisible({ timeout: 10_000 });
	const radarText = await page.locator('.session-radar').innerText();
	const code = radarText.match(/[A-Z0-9]{8}/)?.[0];
	expect(code).toBeTruthy();
	await page.getByLabel('Sync code').fill(code ?? '');
	await page.getByRole('button', { name: 'Claim' }).click();
	await expect(page.getByText(`Claimed sync code ${code}`)).toBeVisible({ timeout: 10_000 });

	await page.getByRole('button', { name: 'Load cassettes' }).click();
	await expect(page.getByTestId('cassette-list')).toContainText('001-request.json', {
		timeout: 10_000
	});
	await page.getByTestId('cassette-list').getByRole('button', { name: 'Branch' }).first().click();
	await expect(page.locator('.session-radar')).toContainText('Replay .x07_rr/http/001-request.json', {
		timeout: 10_000
	});

	const visual = page.getByTestId('visual-editor');
	await visual.getByRole('button', { name: 'Parse' }).click();
	await expect(visual.getByLabel('Node 0 label')).toHaveValue('fetch input');
	await visual.getByRole('button', { name: 'Emit' }).click();
	await expect(page.getByTestId('visual-output')).toContainText('fetch input | normalize | verify', {
		timeout: 10_000
	});
});
