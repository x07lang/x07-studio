export interface Command {
	id: string;
	title: string;
	hint: string;
	group: string;
	action: string;
}

export const commands: Command[] = [
	{ id: 'compare-previous', title: 'Compare to previous turn', hint: 'semantic diff lens', group: 'Compare', action: 'compare' },
	{ id: 'build', title: 'Approve and build', hint: 'run XTAL lifecycle', group: 'Build', action: 'build' },
	{ id: 'autopilot', title: 'Run autopilot', hint: 'continue until human input is needed', group: 'Build', action: 'autopilot' },
	{ id: 'scan-incidents', title: 'Scan incidents', hint: 'load runtime violations', group: 'Repair', action: 'scan' },
	{ id: 'sync', title: 'Continue elsewhere', hint: 'mint a sync code', group: 'Session', action: 'sync' }
];

export function searchCommands(query: string, all: Command[] = commands): Command[] {
	const needle = query.trim().toLowerCase();
	if (!needle) return all;
	return all.filter((command) => `${command.title} ${command.hint} ${command.group}`.toLowerCase().includes(needle));
}
