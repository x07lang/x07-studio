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
