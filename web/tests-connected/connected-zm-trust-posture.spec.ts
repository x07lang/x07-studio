import { expect, test, type Page } from '@playwright/test';

async function useManualMode(page: Page) {
	await page.getByLabel('Auto').uncheck();
}

async function startSession(page: Page, prompt: string) {
	await page.goto('/');
	await expect(page.getByText('Connected to Loom daemon')).toBeVisible();
	await useManualMode(page);
	await page.getByTestId('composer-input').fill(prompt);
	await page.getByTestId('composer-submit').click();
	await expect(page.getByTestId('turn-user_intent')).toContainText(prompt, { timeout: 20_000 });

	const sessionsResponse = await page.request.get('/v1/sessions');
	const sessions = (await sessionsResponse.json()) as Array<{ session_id: string; title: string }>;
	const session = sessions.find((item) => item.title === prompt.slice(0, 80));
	expect(session).toBeTruthy();
	return session!;
}

test('connected trust posture uses compatibility session fields and renders quickfix evidence', async ({
	page
}) => {
	const compat = await page.request.post('/v1/sessions', {
		data: {
			intent_text: 'Compatibility create session request',
			mode: 'bug_fix'
		}
	});
	expect(compat.ok()).toBe(true);
	const compatSession = (await compat.json()) as { title: string; task_type: string };
	expect(compatSession.title).toBe('Compatibility create session request');
	expect(compatSession.task_type).toBe('bug_fix');

	const prompt = `Cycle 4 trust posture ${Date.now()}`;
	const session = await startSession(page, prompt);

	await expect(page.getByTestId('trust-card')).toContainText('solve-pure');
	await expect(page.getByTestId('posture-badge')).toContainText('solve-pure');

	const postureResponse = await page.request.get(`/v1/sessions/${session.session_id}/trust/posture`);
	expect(postureResponse.ok()).toBe(true);
	const posture = (await postureResponse.json()) as { worlds: string[]; posture_color: string };
	expect(posture.worlds).toContain('solve-pure');
	expect(['green', 'amber', 'red']).toContain(posture.posture_color);

	await page.getByRole('button', { name: 'Scan incidents' }).click();
	await expect(page.getByTestId('turn-incident')).toContainText('demo-incident', {
		timeout: 15_000
	});
	await page.getByRole('button', { name: 'Show diagnostic quickfix' }).click();
	await expect(page.getByTestId('quickfix-card')).toContainText('runtime_violation');
});
