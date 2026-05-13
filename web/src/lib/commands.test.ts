import { describe, expect, it } from 'vitest';

import { searchCommands } from './commands';

describe('command registry', () => {
	it('finds compare commands by title and hint', () => {
		const results = searchCommands('compare');

		expect(results[0].id).toBe('compare-previous');
		expect(results.some((command) => command.action === 'compare')).toBe(true);
	});
});
