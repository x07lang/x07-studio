import { describe, expect, it, vi } from 'vitest';

import { fadeUpOnMount, marquee, pulseOnce } from './motion';

describe('motion actions', () => {
	it('installs bounded style changes', () => {
		const node = document.createElement('div');
		vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb) => {
			cb(0);
			return 1;
		});
		vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => undefined);

		const action = fadeUpOnMount(node);

		expect(node.style.opacity).toBe('1');
		action.destroy();
	});

	it('keeps marquee non-invasive', () => {
		const node = document.createElement('div');
		marquee(node);
		expect(node.style.overflow).toBe('hidden');
	});

	it('runs pulse action without requiring app state', () => {
		const node = document.createElement('div');
		node.animate = vi.fn();
		pulseOnce(node);
		expect(node.animate).toHaveBeenCalled();
	});
});
