// Plain-English labels for build pipeline stages and the canonical x07
// binding ops a non-engineer sees in Timeline turns. Edit here to tweak the
// vocabulary in one place rather than scattering strings across components.

import type { OpRecord, SessionPhase, SessionSnapshot } from './studio';

export const buildStageOrder: BuildStage[] = [
	'start',
	'design',
	'write',
	'test',
	'verify',
	'repair',
	'done',
	'needs_help'
];

export type BuildStage =
	| 'start'
	| 'design'
	| 'write'
	| 'test'
	| 'verify'
	| 'repair'
	| 'done'
	| 'needs_help';

const buildStageLabels: Record<BuildStage, string> = {
	start: 'Understanding what you want',
	design: 'Designing the structure',
	write: 'Writing the code',
	test: 'Generating tests',
	verify: 'Checking correctness',
	repair: 'Fixing an issue I found',
	done: 'Done',
	needs_help: 'I need your help'
};

const bindingStageMap: Record<string, BuildStage> = {
	'spec.scaffold': 'design',
	'spec.check': 'design',
	'spec.extract': 'design',
	'spec.lint': 'design',
	'spec.format': 'design',
	'tests.gen.write': 'test',
	'tests.gen.check': 'test',
	'impl.sync.write': 'write',
	'impl.sync.patchset': 'write',
	'impl.check': 'write',
	'xtal.verify': 'verify',
	'xtal.repair': 'repair'
};

export function stageLabel(stage: BuildStage): string {
	return buildStageLabels[stage];
}

export function stageFromOp(op: OpRecord): BuildStage | null {
	if (op.op.startsWith('build.stage.')) {
		const stage = op.op.replace('build.stage.', '') as BuildStage;
		if (stage in buildStageLabels) return stage;
		return null;
	}
	return bindingStageMap[op.op] ?? null;
}

export function currentBuildStage(session: SessionSnapshot): BuildStage | null {
	let latest: { index: number; stage: BuildStage } | null = null;
	for (let i = 0; i < session.op_log.length; i += 1) {
		const op = session.op_log[i];
		const stage = stageFromOp(op);
		if (!stage) continue;
		const index = buildStageOrder.indexOf(stage);
		if (index < 0) continue;
		latest = { index, stage };
	}
	return latest?.stage ?? null;
}

export function buildStageProgress(session: SessionSnapshot): {
	stage: BuildStage | null;
	completed: BuildStage[];
	pending: BuildStage[];
} {
	const stages: BuildStage[] = ['start', 'design', 'write', 'test', 'verify', 'done'];
	const current = currentBuildStage(session);
	if (!current) {
		return { stage: null, completed: [], pending: stages };
	}
	const currentIndex = stages.indexOf(current);
	if (currentIndex < 0) {
		return { stage: current, completed: [], pending: stages };
	}
	return {
		stage: current,
		completed: stages.slice(0, currentIndex),
		pending: stages.slice(currentIndex + 1)
	};
}

export function describePhase(phase: SessionPhase): string {
	switch (phase) {
		case 'intent_drafting':
			return 'Telling me what you want';
		case 'intent_ready':
			return 'Reviewing what we agreed on';
		case 'spec_draft':
		case 'spec_review':
			return 'Designing the structure';
		case 'spec_approved':
			return 'Ready to build';
		case 'realization_proposed':
			return 'Writing the code';
		case 'verify_running':
			return 'Checking correctness';
		case 'repair_eligible':
			return 'Found an issue — about to fix';
		case 'trust_review':
			return 'Built and verified';
		case 'certify_running':
			return 'Recording a trust certificate';
		case 'certified':
			return 'Certified';
		case 'incident_ingesting':
			return 'Looking at the incident';
		case 'human_intervention_required':
			return 'I need your help';
		default:
			return phase;
	}
}

export function plainOpLabel(op: OpRecord): string {
	if (op.op.startsWith('build.stage.')) {
		const stage = op.op.replace('build.stage.', '') as BuildStage;
		if (stage in buildStageLabels) return buildStageLabels[stage];
	}
	const stage = bindingStageMap[op.op];
	if (stage) return buildStageLabels[stage];
	if (op.op.startsWith('agent.event.') && op.op.endsWith('.clarify_question')) {
		return 'Asked a clarifying question';
	}
	if (op.op.startsWith('agent.event.') && op.op.endsWith('.clarify_done')) {
		return 'No more questions';
	}
	if (op.op.startsWith('agent.clarify.')) {
		if (op.status === 'failed') return 'Agent had nothing to ask';
		if (op.status === 'running') return 'Asking the agent for questions';
		return 'Agent finished asking';
	}
	if (op.op.startsWith('agent.run.')) return 'Agent working';
	if (op.op === 'intent.formalize') return 'Polished what you want';
	if (op.op === 'intent.clarify.answers') return 'Recorded your answers';
	if (op.op === 'summary.plain_english') return 'Wrote a summary';
	return op.op;
}
