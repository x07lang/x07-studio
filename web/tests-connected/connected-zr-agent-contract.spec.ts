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

test('connected AGENT.md drawer saves and folds into handoff prompts', async ({ page }) => {
	const prompt = `Cycle 5 agent contract ${Date.now()}`;
	const session = await startSession(page, prompt);
	const constraint = `Cycle 5 contract constraint ${Date.now()}`;

	await page.getByRole('button', { name: 'AGENT.md', exact: true }).click();
	await expect(page.getByTestId('agent-contract-editor')).toBeVisible();
	const editor = page.getByLabel('AGENT.md markdown');
	await editor.fill(`# AGENT.md

## Purpose
Connected Cycle 5 test.

## Invariants
- ${constraint}
`);
	await page.getByRole('button', { name: 'Save' }).click();
	await expect(page.getByTestId('agent-contract-editor')).toContainText('Synced', {
		timeout: 10_000
	});

	const handoffResponse = await page.request.post(
		`/v1/sessions/${session.session_id}/agents/claude-code/handoff`
	);
	expect(handoffResponse.ok()).toBe(true);
	const handoff = (await handoffResponse.json()) as { handoff: { prompt: string } };
	expect(handoff.handoff.prompt).toContain('## Project AGENT.md');
	expect(handoff.handoff.prompt).toContain(constraint);
});
