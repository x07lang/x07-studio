import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { PlainEnglishSummary } from '$lib/studio';
import ResultPreview from './ResultPreview.svelte';

const summary: PlainEnglishSummary = {
	schema_version: 'x07.studio.plain_english_summary@0.1.0',
	headline: 'Verified behavior',
	behavior_promises: [],
	boundaries: [],
	evidence: [],
	run_invocation: null,
	followups: []
};

describe('ResultPreview examples', () => {
	it('distinguishes architect-sourced examples from user examples', () => {
		render(ResultPreview, {
			props: {
				summary,
				examples: [
					{ text: '[3,1,2] -> [1,2,3]', source: 'user' },
					{ text: '[10,20,30] -> [ewma per-byte]', source: 'architect' }
				]
			}
		});

		const items = screen.getAllByRole('listitem');

		expect(items).toHaveLength(2);
		expect(within(items[0]).queryByText('Architect')).not.toBeInTheDocument();
		expect(within(items[0]).getByText('[3,1,2] -> [1,2,3]')).toBeInTheDocument();
		expect(within(items[1]).getByText('Architect')).toBeInTheDocument();
		expect(within(items[1]).getByText('[10,20,30] -> [ewma per-byte]')).toBeInTheDocument();
	});
});
