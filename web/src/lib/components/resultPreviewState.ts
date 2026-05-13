export function implementationReadyForSummary(scaffoldOnly: boolean, implementationInPlace: boolean) {
	return implementationInPlace && !scaffoldOnly;
}

export function implementationActionLabel(
	scaffoldOnly: boolean,
	implementationInPlace: boolean,
	busy: boolean
) {
	if (implementationReadyForSummary(scaffoldOnly, implementationInPlace)) return 'Implementation in place';
	if (busy) return 'Claude Code is implementing...';
	return 'Implement with Claude Code';
}
