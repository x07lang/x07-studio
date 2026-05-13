import { expect, test, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const shotDir = resolve(process.cwd(), 'target', 'cycle4-walkthrough-shots');

test.beforeAll(() => {
	mkdirSync(shotDir, { recursive: true });
});

async function snap(page: Page, name: string) {
	await page.screenshot({ path: resolve(shotDir, `${name}.png`), fullPage: true });
}

test('cycle 4 walkthrough captures every new surface', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await expect(page.getByLabel('Auto')).toBeChecked();
	await snap(page, '01-landing');

	const prompt = `Walkthrough sentiment sort ${Date.now()}`;
	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText(prompt, { timeout: 20_000 });
	await snap(page, '02-after-intent');

	await expect(page.getByTestId('trust-card')).toContainText('solve-pure', { timeout: 20_000 });
	await snap(page, '03-trust-card-visible');

	await expect(page.getByTestId('turn-verified').first()).toBeVisible({ timeout: 60_000 });
	await snap(page, '04-after-autopilot-verified');

	const sessionsResponse = await page.request.get('/v1/sessions');
	const sessions = (await sessionsResponse.json()) as Array<{ session_id: string; title: string }>;
	const session = sessions.find((s) => s.title === prompt.slice(0, 80));
	expect(session).toBeTruthy();

	await expect(page.getByTestId('shipping-ladder')).toContainText('Local preview');
	await snap(page, '05-ladder-with-gates');

	const diffResponse = await page.request.post(`/v1/sessions/${session!.session_id}/diff`, {
		data: {
			from: { kind: 'current' },
			to: { kind: 'current' },
			mode: 'project'
		}
	});
	expect(diffResponse.ok()).toBe(true);
	const diff = (await diffResponse.json()) as { headline: string; trust_delta_color: string };
	expect(diff.headline).toMatch(/solve-pure|trust|no trust delta/i);

	const certResponse = await page.request.post(
		`/v1/sessions/${session!.session_id}/certificate/refresh`,
		{ data: {} }
	);
	expect(certResponse.ok()).toBe(true);
	await snap(page, '06-after-certificate-refresh');

	await page.keyboard.press('Meta+k');
	const palette = page.getByTestId('command-palette');
	if (await palette.isVisible().catch(() => false)) {
		await snap(page, '07-command-palette');
		await page.keyboard.press('Escape');
	}

	await snap(page, '08-final-state');
});
