import { describe, expect, it, vi } from 'vitest';

import { colorMorph, drawerExpand, fadeUpOnMount, marquee, patchPulse, pulseOnce } from './motion';

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

	it('wires posture color and drawer transitions', () => {
		const colorNode = document.createElement('div');
		colorMorph(colorNode);
		expect(colorNode.style.transition).toContain('border-color');

		const drawerNode = document.createElement('div');
		drawerNode.animate = vi.fn();
		Object.defineProperty(drawerNode, 'scrollHeight', { value: 42 });
		drawerExpand(drawerNode);
		expect(drawerNode.animate).toHaveBeenCalledWith(
			[{ height: '0px', opacity: 0 }, { height: '42px', opacity: 1 }],
			expect.objectContaining({ duration: 180 })
		);
	});

	it('runs quickfix pulse only when active', () => {
		const node = document.createElement('div');
		node.animate = vi.fn();
		patchPulse(node, false);
		expect(node.animate).not.toHaveBeenCalled();
		const action = patchPulse(node, true);
		expect(node.animate).toHaveBeenCalledTimes(1);
		action.update?.(true);
		expect(node.animate).toHaveBeenCalledTimes(2);
	});
});
