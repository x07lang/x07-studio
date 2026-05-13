import { describe, expect, it } from 'vitest';

import { implementationActionLabel, implementationReadyForSummary } from './resultPreviewState';

describe('result preview implementation state', () => {
	it('keeps implementation actions available when the verified summary is still scaffolded', () => {
		expect(implementationReadyForSummary(true, true)).toBe(false);
		expect(implementationActionLabel(true, true, false)).toBe('Implement with Claude Code');
	});

	it('marks implementation ready only for a non-scaffolded implemented summary', () => {
		expect(implementationReadyForSummary(false, true)).toBe(true);
		expect(implementationActionLabel(false, true, false)).toBe('Implementation in place');
	});
});
