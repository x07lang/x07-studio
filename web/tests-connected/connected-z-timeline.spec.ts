import { expect, test } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

async function useManualMode(page: import('@playwright/test').Page) {
	await page.getByLabel('Auto').uncheck();
}

async function openTryIt(page: import('@playwright/test').Page) {
	const button = page.getByRole('button', { name: /Try It/ });
	if ((await button.getAttribute('aria-expanded')) !== 'true') {
		await button.click();
	}
}

async function openDrawer(page: import('@playwright/test').Page, name: RegExp) {
	const button = page.getByRole('button', { name });
	if ((await button.getAttribute('aria-expanded')) !== 'true') {
		await button.click();
	}
}

test('connected timeline formalizes, clarifies, builds, tries, and scans incidents', async ({ page }) => {
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto('/');

	await expect(page.getByRole('heading', { name: 'x07 Studio' })).toBeVisible();
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await useManualMode(page);

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
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 45_000 });
	await expect(page.getByTestId('run-invocation')).toContainText('x07 run');
	await expect(page.getByTestId('shipping-ladder')).toContainText('Local preview');

	// The fake toolchain now writes the implementation to the same
	// intent-derived path that verify records, so Try-It should use the
	// verified artifact instead of tripping the old orphan-stub guard.
	await openTryIt(page);
	await page.getByTestId('try-it-panel').getByRole('textbox').first().fill('[2,1]');
	await page.getByTestId('try-it-panel').getByRole('button', { name: 'Run it' }).click();
	await expect(page.getByTestId('try-it-panel')).toContainText('"ok": true', {
		timeout: 15_000
	});
	await expect(page.getByTestId('try-it-panel')).toContainText('Latest verify evidence');

	await page.getByRole('button', { name: 'Scan incidents' }).click();
	await expect(page.getByTestId('turn-incident')).toContainText('demo-incident', {
		timeout: 15_000
	});
	await page.getByRole('button', { name: 'Repair this' }).click();
	await expect(page.getByTestId('turn-repair')).toContainText('latest', { timeout: 15_000 });
});

test('connected verify disables the realize CTA once template synthesis lands', async ({ page }) => {
	// The fake connected toolchain now writes to the intent-derived module
	// path, so template synthesis produces a non-stub implementation and
	// the realize CTA becomes a disabled "implementation in place" marker.
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await useManualMode(page);

	await page
		.getByTestId('composer-input')
		.fill('Build a small CLI greeting tool that prints hello for a name and refuses empty input.');
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toBeVisible({ timeout: 15_000 });

	const card = page.locator('[data-testid="clarify-card"]').first();
	await expect(card).toBeVisible({ timeout: 20_000 });
	await card.getByRole('button').nth(0).click();
	await card.getByTestId('clarify-answer-submit').click();

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });
	await expect(page.getByTestId('summary-headline')).not.toContainText('scaffolded');
	await expect(page.getByTestId('realize-cta')).toBeVisible();
	await expect(page.getByTestId('realize-cta')).toContainText('Implementation in place');
	await expect(page.getByTestId('realize-cta-button')).toBeDisabled();

	const response = await page.request.get('/v1/sessions');
	const sessions = (await response.json()) as Array<{ title: string; op_log: Array<{ op: string }> }>;
	const greeter = sessions.find((session) =>
		session.title.startsWith('Build a small CLI greeting tool')
	);
	expect(greeter?.op_log.some((op) => op.op === 'synthesis.template')).toBe(true);

	await openTryIt(page);
	await page.getByTestId('try-it-panel').getByRole('textbox').first().fill('Bodik');
	await page.getByTestId('try-it-panel').getByRole('button', { name: 'Run it' }).click();
	await expect(page.getByTestId('try-it-panel')).toContainText('"ok": true', {
		timeout: 15_000
	});
});

test('connected no-write realize failure exposes recovery actions', async ({ page }) => {
	await page.setViewportSize({ width: 1280, height: 900 });
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await useManualMode(page);

	await page
		.getByTestId('composer-input')
		.fill(
			'Build a no-write realize regression CLI calculator. It receives one-shot CLI arguments and supports integer add, subtract, multiply, and divide.'
		);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toBeVisible({ timeout: 15_000 });

	const card = page.locator('[data-testid="clarify-card"]').first();
	await expect(card).toBeVisible({ timeout: 20_000 });
	await card.getByRole('button').nth(0).click();
	await card.getByTestId('clarify-answer-submit').click();

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });
	await expect(page.getByTestId('summary-headline')).toContainText('scaffolded');
	await expect(page.getByTestId('realize-cta-button')).toBeEnabled();
	await expect(page.getByTestId('realize-cta-button')).toContainText('Implement with Claude Code');

	await page.getByTestId('realize-cta-button').click();
	const realizeTurn = page.getByTestId('turn-agent_realize');
	await expect(realizeTurn).toContainText('claude-code ran but reported issues', {
		timeout: 60_000
	});
	await expect(realizeTurn).toContainText('No file changes recorded by the write audit.');
	await expect(realizeTurn.getByRole('button', { name: 'Try Claude Code again' })).toBeEnabled();
	await expect(realizeTurn.getByRole('button', { name: 'Compare both agents' })).toBeEnabled();

	await realizeTurn.getByRole('button', { name: 'Compare both agents' }).click();
	await expect(page.getByTestId('turn-quorum_realize')).toBeVisible({ timeout: 60_000 });
	await expect(page.getByTestId('timeline')).toContainText('openai-codex', { timeout: 30_000 });
});

test('connected autopilot clarifies, answers, builds, and climbs local preview', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await expect(page.getByLabel('Auto')).toBeChecked();

	await page
		.getByTestId('composer-input')
		.fill('Build a small greeter that says hello to the supplied name and rejects empty input.');
	await page.getByTestId('composer-submit').click();

	await expect(page.getByTestId('turn-agent_clarify')).toBeVisible({ timeout: 30_000 });
	await expect(page.getByTestId('clarify-answer-locked').first()).toBeVisible({ timeout: 30_000 });
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });
	await expect(page.getByTestId('shipping-ladder')).toContainText('Local preview');
});

test('connected timeline runs the Atlas workflow lane', async ({ page }) => {
	const prompt =
		'Use docs/examples/wasm_showcases/x07_atlas to build x07 Atlas with profile validation, trace replay, release pack verification, provenance, deploy planning, and SLO evidence.';
	await page.goto('/?details=open');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await useManualMode(page);

	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText('wasm_showcases/x07_atlas', {
		timeout: 15_000
	});

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 45_000 });
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
	await useManualMode(page);

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
	await useManualMode(page);

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

	await openDrawer(page, /Time travel/);
	await page.getByRole('button', { name: 'Load cassettes' }).click();
	await expect(page.getByTestId('cassette-list')).toContainText('001-request.json', {
		timeout: 10_000
	});
	await page.getByTestId('cassette-list').getByRole('button', { name: 'Branch' }).first().click();
	await expect(page.locator('.session-radar')).toContainText('Replay .x07_rr/http/001-request.json', {
		timeout: 10_000
	});

	await openDrawer(page, /Visual editor/);
	const visual = page.getByTestId('visual-editor');
	await visual.getByRole('button', { name: 'Parse' }).click();
	await expect(visual.getByLabel('Node 0 label')).toHaveValue('fetch input');
	await visual.getByRole('button', { name: 'Emit' }).click();
	await expect(page.getByTestId('visual-output')).toContainText('fetch input | normalize | verify', {
		timeout: 10_000
	});
});
