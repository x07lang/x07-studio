import { expect, request, test } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

const budgets = {
	landing_tti_ms: 5_000,
	process_step_render_p95_ms: 500,
	daemon_health_p95_ms: 200
};

test('connected performance budgets stay under GA limits', async ({ page }) => {
	const started = Date.now();
	await page.goto('/?perf=1');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	const landingTtiMs = Date.now() - started;

	await page.getByLabel('Auto').uncheck();
	const prompt = `Performance budget sorter ${Date.now()}`;
	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-agent_clarify')).toBeVisible({ timeout: 20_000 });

	const emptyInputQuestion = page.locator('[data-testid="clarify-card"]').first();
	await emptyInputQuestion.getByRole('button').first().click();
	await emptyInputQuestion.getByTestId('clarify-answer-submit').click();
	await expect(emptyInputQuestion.getByTestId('clarify-answer-locked')).toBeVisible({
		timeout: 10_000
	});

	await page.getByTestId('approve-build').click();
	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });

	const processMeasures = await page.evaluate(() =>
		performance
			.getEntriesByType('measure')
			.filter((entry) => entry.name.startsWith('x07-studio.step-node.'))
			.map((entry) => entry.duration)
	);
	const processStepRenderP95Ms = percentile(processMeasures, 95);
	const daemonHealthP95Ms = await daemonHealthP95();
	const measurements = {
		landing_tti_ms: landingTtiMs,
		process_step_render_p95_ms: processStepRenderP95Ms,
		daemon_health_p95_ms: daemonHealthP95Ms
	};
	const passed =
		measurements.landing_tti_ms <= budgets.landing_tti_ms &&
		measurements.process_step_render_p95_ms <= budgets.process_step_render_p95_ms &&
		measurements.daemon_health_p95_ms <= budgets.daemon_health_p95_ms;
	const report = {
		schema_version: 'x07.studio.perf_budget@0.1.0',
		captured_at: new Date().toISOString(),
		budgets,
		measurements,
		passed
	};
	const out = process.env.X07_STUDIO_PERF_BUDGET_OUT;
	if (out) {
		mkdirSync(dirname(out), { recursive: true });
		writeFileSync(out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
	}
	expect(report).toMatchObject({ passed: true });
});

function percentile(values: number[], pct: number) {
	if (values.length === 0) return 0;
	const sorted = [...values].sort((left, right) => left - right);
	const index = Math.min(sorted.length - 1, Math.ceil((pct / 100) * sorted.length) - 1);
	return sorted[index];
}

async function daemonHealthP95() {
	const context = await request.newContext({ baseURL: 'http://127.0.0.1:7729' });
	const timings: number[] = [];
	for (let index = 0; index < 25; index += 1) {
		const start = Date.now();
		const response = await context.get('/v1/health');
		expect(response.ok()).toBeTruthy();
		timings.push(Date.now() - start);
	}
	await context.dispose();
	return percentile(timings, 95);
}
