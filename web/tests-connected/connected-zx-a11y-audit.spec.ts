import AxeBuilder from '@axe-core/playwright';
import { expect, test, type Page } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

test('landing and verified-session views have no serious or critical axe violations', async ({ page }) => {
	const reports: Array<{ view: string; violations: unknown[] }> = [];
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	reports.push({ view: 'landing', violations: await seriousCriticalViolations(page) });

	await page.getByLabel('Auto').uncheck();
	const prompt = `Accessibility audit sorter ${Date.now()}`;
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
	reports.push({ view: 'verified-session', violations: await seriousCriticalViolations(page) });

	const violations = reports.flatMap((report) =>
		report.violations.map((violation) => ({ view: report.view, violation }))
	);
	const out = process.env.X07_STUDIO_A11Y_AUDIT_OUT;
	if (out) {
		mkdirSync(dirname(out), { recursive: true });
		writeFileSync(
			out,
			`${JSON.stringify(
				{
					schema_version: 'x07.studio.a11y_audit@0.1.0',
					captured_at: new Date().toISOString(),
					views: reports,
					serious_critical_count: violations.length,
					passed: violations.length === 0
				},
				null,
				2
			)}\n`,
			'utf8'
		);
	}
	expect(violations).toEqual([]);
});

async function seriousCriticalViolations(page: Page) {
	const results = await new AxeBuilder({ page })
		.withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
		.analyze();
	return results.violations.filter((violation) => violation.impact === 'serious' || violation.impact === 'critical');
}
