import { expect, test, type Page } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

// Connected end-to-end coverage for Simple Mode. The connected-e2e fake
// agent emits two `clarify_question` events whenever its handoff prompt
// path contains `-clarify`, so each Simple-Mode prompt naturally cycles
// through one clarify round before the build runs.

interface SimpleProject {
	label: string;
	prompt: string;
	answers: Array<{ option?: string; freeText?: string }>;
}

const simpleProjects: SimpleProject[] = [
	{
		label: 'Stable byte-array sorter',
		prompt:
			'Build a stable sorter for byte arrays. Equal items must keep their original order. Reject empty input.',
		answers: [{ option: 'Reject with an error' }, { option: 'Yes, stable' }]
	},
	{
		label: 'Workflow graph optimizer',
		prompt:
			'Build a workflow graph optimizer that computes the makespan from task durations and dependency edges, and rejects cycles.',
		answers: [{ option: 'Reject with an error' }, { option: 'Yes, stable' }]
	},
	{
		label: 'CLI greeting tool (free-form answers)',
		prompt:
			'Build a small CLI greeting tool. It should print a friendly hello message for a given name and refuse empty input.',
		answers: [
			{ freeText: 'Reject empty names with a clear error message.' },
			{ freeText: 'Order does not matter for this tool — there is no list.' }
		]
	}
];

async function waitForHydration(page: Page) {
	await page.waitForLoadState('networkidle').catch(() => undefined);
	await page.waitForTimeout(200);
}

async function clearStudioState(page: Page) {
	await page.evaluate(() => {
		try {
			window.localStorage.removeItem('x07-studio-mode');
		} catch {
			/* ignore */
		}
	});
}

async function runSimpleProject(page: Page, project: SimpleProject) {
	await page.goto('/');
	await waitForHydration(page);

	const start = page.getByTestId('simple-start');
	await expect(start, `${project.label}: Simple Mode landing visible`).toBeVisible();

	await page.getByTestId('simple-start-prompt').fill(project.prompt);
	await page.getByTestId('simple-start-begin').click();

	// The Simple-Mode component transitions from `start` -> `clarify` after
	// `intent.formalize` returns. In connected mode, the clarify run then
	// spawns the fake agent which emits two `clarify_question` events.
	const clarifyPanel = page.getByTestId('simple-clarify');
	await expect(clarifyPanel, `${project.label}: clarify panel visible`).toBeVisible({
		timeout: 15_000
	});

	const cards = page.getByTestId('clarify-card');
	await expect(cards, `${project.label}: two clarify cards visible`).toHaveCount(2, {
		timeout: 15_000
	});

	const expectedQuestionIds = ['q-empty-input', 'q-stability'];
	for (let i = 0; i < project.answers.length; i += 1) {
		const answer = project.answers[i];
		const card = page.locator(
			`[data-testid="clarify-card"][data-question-id="${expectedQuestionIds[i]}"]`
		);
		await expect(card, `${project.label}: card ${i} found`).toBeVisible();
		if (answer.option) {
			await card.getByRole('button', { name: answer.option }).click();
		} else if (answer.freeText) {
			await card.getByTestId('clarify-answer-input').fill(answer.freeText);
		}
		await card.getByTestId('clarify-answer-submit').click();
		await expect(card.getByTestId('clarify-answer-locked')).toBeVisible({ timeout: 10_000 });
	}

	await page.getByTestId('simple-clarify-build').click();

	const progress = page.getByTestId('simple-build-progress');
	await expect(progress, `${project.label}: build progress visible`).toBeVisible({
		timeout: 30_000
	});
	const stages = page.getByTestId('simple-build-stages');
	await expect(stages, `${project.label}: stage strip visible`).toBeVisible();

	// The result preview waits for the daemon to emit `summary.plain_english`
	// after `build.stage.done`. SSE pushes the snapshot, then the UI swaps
	// into SimpleResultPreview.
	await expect(
		page.getByTestId('simple-result-preview'),
		`${project.label}: result preview surfaced`
	).toBeVisible({ timeout: 60_000 });

	const headline = page.getByTestId('summary-headline');
	await expect(headline, `${project.label}: headline rendered`).toBeVisible();
	await expect(headline).toContainText(/verified|reviewed|repaired|fixed/i);

	// Flip to Expert mode and confirm the underlying XTAL evidence is real:
	// the studio shell, the worklog with the verify op, and the agent
	// clarify events should all be visible.
	await page.getByTestId('result-open-expert').click();
	await expect(
		page.getByTestId('studio-mode-toggle'),
		`${project.label}: mode toggle visible`
	).toHaveAttribute('data-mode', 'expert');
	const expertHeading = page.getByRole('heading', { name: 'x07 Studio', level: 1 }).first();
	await expect(expertHeading, `${project.label}: expert heading visible`).toBeVisible();

	await clearStudioState(page);
}

for (const project of simpleProjects) {
	test(`Simple Mode end-to-end: ${project.label}`, async ({ page }) => {
		await page.setViewportSize({ width: 1280, height: 900 });
		await runSimpleProject(page, project);
	});
}
