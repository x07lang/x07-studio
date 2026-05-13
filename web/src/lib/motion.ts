export function fadeUpOnMount(node: HTMLElement) {
	node.style.opacity = '0';
	node.style.transform = 'translateY(6px)';
	const frame = requestAnimationFrame(() => {
		node.style.transition = `opacity var(--motion-base) var(--ease-out), transform var(--motion-base) var(--ease-out)`;
		node.style.opacity = '1';
		node.style.transform = 'translateY(0)';
	});
	return {
		destroy() {
			cancelAnimationFrame(frame);
		}
	};
}

export function pulseOnce(node: HTMLElement) {
	node.animate([{ opacity: 0.72 }, { opacity: 1 }], {
		duration: 200,
		easing: 'cubic-bezier(0.16, 1, 0.3, 1)'
	});
	return {};
}

export function marquee(node: HTMLElement) {
	node.style.overflow = 'hidden';
	return {};
}

export function colorMorph(node: HTMLElement) {
	node.style.transition = [
		node.style.transition,
		'border-color var(--motion-base) var(--ease-out)',
		'background-color var(--motion-base) var(--ease-out)'
	]
		.filter(Boolean)
		.join(', ');
	return {};
}

export function drawerExpand(node: HTMLElement) {
	const height = node.scrollHeight;
	node.animate([{ height: '0px', opacity: 0 }, { height: `${height}px`, opacity: 1 }], {
		duration: 180,
		easing: 'cubic-bezier(0.16, 1, 0.3, 1)'
	});
	return {};
}

export function patchPulse(node: HTMLElement, active = true) {
	if (!active) return {};
	node.animate(
		[
			{ boxShadow: '0 0 0 rgba(114, 228, 180, 0)' },
			{ boxShadow: '0 0 0 4px rgba(114, 228, 180, 0.24)' },
			{ boxShadow: '0 0 0 rgba(114, 228, 180, 0)' }
		],
		{
			duration: 200,
			easing: 'cubic-bezier(0.16, 1, 0.3, 1)'
		}
	);
	return {
		update(next: boolean) {
			if (next) patchPulse(node, true);
		}
	};
}
