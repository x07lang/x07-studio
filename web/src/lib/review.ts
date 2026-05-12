import type { OpRecord, OperationStatus } from './studio';

export type ReviewTone = 'info' | 'warn' | 'ok';
export type PatchReviewRisk = 'low' | 'medium' | 'high';
export type PatchReviewSource = 'patchset' | 'artifact' | 'write-root' | 'evidence';

export interface ReviewSignal {
	opId: string;
	op: string;
	label: string;
	detail: string;
	tone: ReviewTone;
	artifact?: string;
}

export interface PatchReviewFile {
	path: string;
	action: string;
	note: string;
	operations: number;
	source: PatchReviewSource;
	risk: PatchReviewRisk;
	before?: string;
	after?: string;
	applyError?: string;
}

export interface PatchReview {
	opId: string;
	op: string;
	gate: string;
	status: OperationStatus;
	command: string;
	files: PatchReviewFile[];
	artifacts: string[];
}

export interface WriteAuditReview {
	allowedRoots: string[];
	created: string[];
	modified: string[];
	deleted: string[];
	violations: string[];
	truncated: boolean;
}

export type CounterexampleTone = 'empty' | 'failed' | 'repair' | 'incident';

export interface CounterexampleDiagnostic {
	code: string;
	severity: string;
	message: string;
}

export interface CounterexampleTheater {
	tone: CounterexampleTone;
	title: string;
	summary: string;
	clause: string;
	counterexample: string;
	route: string;
	evidence: string[];
	diagnostics: CounterexampleDiagnostic[];
	opId?: string;
	op?: string;
	command?: string;
}

export function buildReviewSignals(ops: OpRecord[]): ReviewSignal[] {
	const seen = new Set<string>();
	const signals: ReviewSignal[] = [];
	for (const op of [...ops].reverse()) {
		const signal = reviewSignalFromOp(op);
		if (!signal) continue;
		const key = `${signal.label}|${signal.detail}|${signal.artifact ?? ''}`;
		if (seen.has(key)) continue;
		seen.add(key);
		signals.push(signal);
		if (signals.length >= 8) break;
	}
	return signals;
}

export function buildCounterexampleTheater(ops: OpRecord[]): CounterexampleTheater {
	const op = [...ops].reverse().find(isCounterexampleCandidate);
	if (!op) {
		return {
			tone: 'empty',
			title: 'No counterexample captured',
			summary: 'Verification failures, incidents, and repair candidates will appear here.',
			clause: 'Awaiting failed clause',
			counterexample: 'No failing witness is recorded yet.',
			route: 'Run verify from an approved spec.',
			evidence: [],
			diagnostics: []
		};
	}
	const diagnostics = diagnosticsFromOp(op);
	const evidence = op.artifacts.filter(isCounterexampleArtifact);
	const tone = counterexampleTone(op);
	return {
		tone,
		title: counterexampleTitle(op, tone),
		summary: diagnostics[0]?.message || shortReviewText(op, evidence[0]),
		clause: counterexampleClause(op, diagnostics),
		counterexample: counterexampleBody(op),
		route: counterexampleRoute(op, tone),
		evidence: evidence.length ? evidence.slice(0, 5) : op.artifacts.slice(0, 5),
		diagnostics: diagnostics.slice(0, 3),
		opId: op.id,
		op: op.op,
		command: op.command.join(' ')
	};
}

export function buildPatchReview(op: OpRecord | null): PatchReview | null {
	if (!op) return null;
	const patchFiles = patchsetFilesFromOp(op);
	const artifactFiles = op.artifacts.flatMap((artifact) => patchReviewFileFromArtifact(artifact));
	const files = dedupePatchFiles([...patchFiles, ...artifactFiles]);
	const isPatchReviewOp =
		op.op === 'impl.sync.write' ||
		op.op === 'impl.sync.patchset' ||
		op.op.startsWith('xtal.repair') ||
		op.artifacts.some((artifact) => artifact.includes('patchset')) ||
		patchFiles.length > 0;
	if (!isPatchReviewOp || files.length === 0) return null;
	return {
		opId: op.id,
		op: op.op,
		gate: patchReviewGate(op),
		status: op.status,
		command: op.command.join(' '),
		files,
		artifacts: op.artifacts
	};
}

function reviewSignalFromOp(op: OpRecord): ReviewSignal | null {
	const artifact = primaryArtifact(op);
	const detail = shortReviewText(op, artifact);
	const writeAudit = writeAuditFromOp(op);
	if (writeAudit?.violations.length) {
		return reviewSignal(
			op,
			'Write-root audit',
			`${writeAudit.violations.length} out-of-contract path${writeAudit.violations.length === 1 ? '' : 's'}: ${writeAudit.violations[0]}`,
			'warn',
			writeAudit.violations[0]
		);
	}
	if (op.op.startsWith('agent.event.')) {
		if (op.op.endsWith('.approval')) {
			return reviewSignal(op, 'Approval request', detail, 'warn', artifact);
		}
		if (op.op.endsWith('.diagnostic')) {
			return reviewSignal(op, 'Diagnostic classified', detail, 'warn', artifact);
		}
		if (op.op.endsWith('.write')) {
			return reviewSignal(op, 'Write activity', detail, 'info', artifact);
		}
		if (op.op.endsWith('.artifact')) {
			return reviewSignal(op, 'Artifact surfaced', detail, 'info', artifact);
		}
	}
	if (op.op === 'impl.sync.patchset') {
		return reviewSignal(op, 'Patchset review', detail, 'warn', artifact);
	}
	if (op.op === 'impl.sync.write') {
		return reviewSignal(op, 'Implementation write', detail, 'info', artifact);
	}
	if (op.artifacts.some((item) => item.includes('patchset'))) {
		return reviewSignal(op, 'Patchset review', detail, 'warn', artifact);
	}
	if (op.op.startsWith('xtal.verify')) {
		return reviewSignal(
			op,
			'Verify evidence',
			op.status === 'succeeded' ? 'Verification succeeded' : detail,
			op.status === 'succeeded' ? 'ok' : 'warn',
			artifact
		);
	}
	if (op.op.startsWith('xtal.certify')) {
		return reviewSignal(
			op,
			'Trust evidence',
			op.status === 'succeeded' ? 'Certification evidence ready' : detail,
			op.status === 'succeeded' ? 'ok' : 'warn',
			artifact
		);
	}
	if (op.op.startsWith('wasm.app.verify') || op.op.startsWith('wasm.provenance.verify')) {
		return reviewSignal(
			op,
			'Release evidence',
			op.status === 'succeeded' ? 'Release verification succeeded' : detail,
			op.status === 'succeeded' ? 'ok' : 'warn',
			artifact
		);
	}
	if (op.op.startsWith('wasm.slo.eval')) {
		return reviewSignal(
			op,
			'SLO evidence',
			op.status === 'succeeded' ? 'SLO evaluation succeeded' : detail,
			op.status === 'succeeded' ? 'ok' : 'warn',
			artifact
		);
	}
	if (op.op.startsWith('wasm.deploy.plan')) {
		return reviewSignal(
			op,
			'Deploy plan',
			op.status === 'succeeded' ? 'Deploy plan generated' : detail,
			op.status === 'succeeded' ? 'ok' : 'warn',
			artifact
		);
	}
	if (op.op.startsWith('lp.deploy.')) {
		return reviewSignal(
			op,
			'Local platform delivery',
			op.status === 'succeeded' ? 'Local platform step succeeded' : detail,
			op.status === 'succeeded' ? 'ok' : 'warn',
			artifact
		);
	}
	return null;
}

export function writeAuditFromOp(op: OpRecord | null): WriteAuditReview | null {
	const audit = asRecord(asRecord(op?.report_json)?.write_audit);
	if (!audit) return null;
	return {
		allowedRoots: stringArray(audit.allowed_roots),
		created: stringArray(audit.created),
		modified: stringArray(audit.modified),
		deleted: stringArray(audit.deleted),
		violations: stringArray(audit.violations),
		truncated: Boolean(audit.truncated)
	};
}

function isCounterexampleCandidate(op: OpRecord): boolean {
	const hasRepairArtifact = op.artifacts.some((artifact) => artifact.includes('/repair/'));
	const hasIncidentArtifact = op.artifacts.some(
		(artifact) => artifact.includes('/violations/') || artifact.includes('/ingest/')
	);
	return (
		op.status === 'failed' ||
		op.op.startsWith('xtal.repair') ||
		op.op.startsWith('xtal.ingest') ||
		op.op.includes('incident') ||
		hasRepairArtifact ||
		hasIncidentArtifact ||
		diagnosticsFromOp(op).length > 0
	);
}

function isCounterexampleArtifact(artifact: string): boolean {
	return (
		artifact.includes('/verify/') ||
		artifact.includes('/repair/') ||
		artifact.includes('/violations/') ||
		artifact.includes('/ingest/') ||
		artifact.includes('counterexample') ||
		artifact.includes('incident')
	);
}

function counterexampleTone(op: OpRecord): CounterexampleTone {
	if (op.op.startsWith('xtal.repair') || op.artifacts.some((artifact) => artifact.includes('/repair/'))) {
		return 'repair';
	}
	if (
		op.op.startsWith('xtal.ingest') ||
		op.op.includes('incident') ||
		op.artifacts.some((artifact) => artifact.includes('/violations/') || artifact.includes('/ingest/'))
	) {
		return 'incident';
	}
	if (op.status === 'failed') return 'failed';
	return 'repair';
}

function counterexampleTitle(op: OpRecord, tone: CounterexampleTone): string {
	if (tone === 'repair') return 'Repair candidate ready';
	if (tone === 'incident') return 'Incident witness linked';
	if (op.op.startsWith('xtal.verify')) return 'Verification counterexample';
	return 'Failure witness captured';
}

function counterexampleClause(op: OpRecord, diagnostics: CounterexampleDiagnostic[]): string {
	const fromReport = findTextByKeys(op.report_json, [
		'clause_id',
		'clause',
		'property',
		'requires',
		'ensures'
	]);
	return fromReport || diagnostics[0]?.code || 'Clause pending classification';
}

function counterexampleBody(op: OpRecord): string {
	const fromReport = findTextByKeys(op.report_json, [
		'smallest_counterexample',
		'counterexample',
		'failing_input',
		'input',
		'witness',
		'repro'
	]);
	const fromOutput = [op.stderr, op.stdout]
		.filter(Boolean)
		.join('\n')
		.trim();
	const value = fromReport || fromOutput || op.notes || 'No failing input was recorded in the operation payload.';
	return value.length > 220 ? `${value.slice(0, 217)}...` : value;
}

function counterexampleRoute(op: OpRecord, tone: CounterexampleTone): string {
	if (tone === 'repair') return 'Review patchset and rerun verify.';
	if (tone === 'incident') return 'Open a repair session from the incident witness.';
	if (op.op.startsWith('xtal.verify')) return 'Run xtal.repair before widening the spec.';
	return 'Classify the failure, then choose repair or spec review.';
}

function diagnosticsFromOp(op: OpRecord): CounterexampleDiagnostic[] {
	return [
		...diagnosticsFromValue(op.report_json),
		...diagnosticsFromValue(op.stdout_json),
		...diagnosticsFromValue(op.stderr_json)
	];
}

function reviewSignal(
	op: OpRecord,
	label: ReviewSignal['label'],
	detail: string,
	tone: ReviewTone,
	artifact?: string
): ReviewSignal {
	return { opId: op.id, op: op.op, label, detail, tone, artifact };
}

function primaryArtifact(op: OpRecord): string | undefined {
	return (
		op.artifacts.find((artifact) => !artifact.includes('.x07/studio/handoffs/')) ??
		op.artifacts[0]
	);
}

function shortReviewText(op: OpRecord, artifact?: string): string {
	const notes = op.notes === 'visible agent operation record' ? '' : op.notes;
	const value =
		[op.stdout, op.stderr, notes, op.command.join(' '), artifact, op.op].find((item) =>
			item?.trim()
		) ?? op.op;
	return value.length > 118 ? `${value.slice(0, 115)}...` : value;
}

function patchsetFilesFromOp(op: OpRecord): PatchReviewFile[] {
	const values = [op.report_json, op.stdout_json, op.stderr_json];
	const patchFiles = values.flatMap((value) => patchsetFilesFromValue(value));
	const previewFiles = values.flatMap((value) => patchsetPreviewFilesFromValue(value));
	return mergePatchsetPreviewFiles(patchFiles, previewFiles);
}

function patchsetFilesFromValue(value: unknown): PatchReviewFile[] {
	const patchset = findPatchset(value);
	if (!patchset) return [];
	const patches = Array.isArray(patchset.patches) ? patchset.patches : [];
	return patches.flatMap((item) => {
		const record = asRecord(item);
		const path = textValue(record?.path);
		if (!path) return [];
		const patch = Array.isArray(record?.patch) ? record.patch : [];
		const actions = summarizePatchActions(patch);
		return [
			{
				path,
				action: actions || 'json patch',
				note: textValue(record?.note) || 'Patchset entry',
				operations: patch.length,
				source: 'patchset' as const,
				risk: riskForPath(path)
			}
		];
	});
}

function findPatchset(value: unknown, depth = 0): { patches?: unknown } | null {
	if (depth > 6) return null;
	const record = asRecord(value);
	if (!record) {
		if (!Array.isArray(value)) return null;
		for (const item of value) {
			const found = findPatchset(item, depth + 1);
			if (found) return found;
		}
		return null;
	}
	const schema = textValue(record.schema_version);
	if (
		(schema === 'x07.patchset@0.1.0' || schema === 'x07.arch.patchset@0.1.0') &&
		Array.isArray(record.patches)
	) {
		return record;
	}
	for (const item of Object.values(record)) {
		const found = findPatchset(item, depth + 1);
		if (found) return found;
	}
	return null;
}

function patchsetPreviewFilesFromValue(value: unknown): PatchReviewFile[] {
	const preview = findPatchsetPreview(value);
	if (!preview) return [];
	const targets = Array.isArray(preview.targets) ? preview.targets : [];
	return targets.flatMap((item) => {
		const record = asRecord(item);
		const path = textValue(record?.path);
		if (!path) return [];
		const applyError = textValue(record?.apply_error);
		return [
			{
				path,
				action: applyError ? 'preview error' : 'before/after JSON preview',
				note: textValue(record?.note) || 'Patchset preview target',
				operations: numericValue(record?.operations),
				source: 'patchset' as const,
				risk: riskForPath(path),
				before: jsonSnippet(record?.before_json),
				after: jsonSnippet(record?.after_json),
				applyError: applyError || undefined
			}
		];
	});
}

function findPatchsetPreview(value: unknown, depth = 0): { targets?: unknown } | null {
	if (depth > 6) return null;
	const record = asRecord(value);
	if (!record) {
		if (!Array.isArray(value)) return null;
		for (const item of value) {
			const found = findPatchsetPreview(item, depth + 1);
			if (found) return found;
		}
		return null;
	}
	if (
		textValue(record.schema_version) === 'x07.studio.patchset_preview@0.1.0' &&
		Array.isArray(record.targets)
	) {
		return record;
	}
	for (const item of Object.values(record)) {
		const found = findPatchsetPreview(item, depth + 1);
		if (found) return found;
	}
	return null;
}

function mergePatchsetPreviewFiles(
	patchFiles: PatchReviewFile[],
	previewFiles: PatchReviewFile[]
): PatchReviewFile[] {
	if (!previewFiles.length) return patchFiles;
	const files = [...patchFiles];
	for (const preview of previewFiles) {
		const existing = files.find((file) => file.path === preview.path && file.source === 'patchset');
		if (!existing) {
			files.push(preview);
			continue;
		}
		existing.note = preview.note || existing.note;
		existing.operations = preview.operations || existing.operations;
		existing.before = preview.before;
		existing.after = preview.after;
		existing.applyError = preview.applyError;
	}
	return files;
}

function summarizePatchActions(patch: unknown[]): string {
	const counts = new Map<string, number>();
	for (const item of patch) {
		const op = textValue(asRecord(item)?.op) || 'op';
		counts.set(op, (counts.get(op) ?? 0) + 1);
	}
	return [...counts.entries()].map(([op, count]) => `${op} ${count}`).join(', ');
}

function patchReviewFileFromArtifact(artifact: string): PatchReviewFile[] {
	if (!artifact) return [];
	if (artifact.endsWith('/')) {
		return [
			{
				path: artifact,
				action: 'write root',
				note: writeRootNote(artifact),
				operations: 0,
				source: 'write-root',
				risk: riskForPath(artifact)
			}
		];
	}
	if (artifact.includes('patchset')) {
		return [
			{
				path: artifact,
				action: 'patchset artifact',
				note: 'Deterministic x07 patchset',
				operations: 0,
				source: 'artifact',
				risk: riskForPath(artifact)
			}
		];
	}
	if (artifact.includes('/verify/') || artifact.includes('/cert/') || artifact.includes('/repair/')) {
		return [
			{
				path: artifact,
				action: 'evidence artifact',
				note: 'Review evidence',
				operations: 0,
				source: 'evidence',
				risk: 'low'
			}
		];
	}
	return [];
}

function dedupePatchFiles(files: PatchReviewFile[]): PatchReviewFile[] {
	const seen = new Set<string>();
	const result: PatchReviewFile[] = [];
	for (const file of files) {
		const key = `${file.path}|${file.source}`;
		if (seen.has(key)) continue;
		seen.add(key);
		result.push(file);
	}
	return result.slice(0, 8);
}

function diagnosticsFromValue(value: unknown, depth = 0): CounterexampleDiagnostic[] {
	if (depth > 6) return [];
	const record = asRecord(value);
	if (!record) {
		if (!Array.isArray(value)) return [];
		return value.flatMap((item) => diagnosticsFromValue(item, depth + 1));
	}
	if (Array.isArray(record.diagnostics)) {
		return record.diagnostics.flatMap((item) => {
			const diagnostic = asRecord(item);
			if (!diagnostic) return [];
			return [
				{
					code: textValue(diagnostic.code) || 'diagnostic',
					severity: textValue(diagnostic.severity) || 'info',
					message: textValue(diagnostic.message) || textValue(diagnostic.detail)
				}
			];
		});
	}
	return Object.values(record).flatMap((item) => diagnosticsFromValue(item, depth + 1));
}

function findTextByKeys(value: unknown, keys: string[], depth = 0): string {
	if (depth > 6) return '';
	const record = asRecord(value);
	if (!record) {
		if (!Array.isArray(value)) return '';
		for (const item of value) {
			const found = findTextByKeys(item, keys, depth + 1);
			if (found) return found;
		}
		return '';
	}
	for (const key of keys) {
		const direct = stringifyReviewValue(record[key]);
		if (direct) return direct;
	}
	for (const item of Object.values(record)) {
		const found = findTextByKeys(item, keys, depth + 1);
		if (found) return found;
	}
	return '';
}

function stringifyReviewValue(value: unknown): string {
	if (typeof value === 'string') return value;
	if (typeof value === 'number' || typeof value === 'boolean') return String(value);
	if (value === undefined || value === null) return '';
	try {
		const body = JSON.stringify(value);
		return body.length > 220 ? `${body.slice(0, 217)}...` : body;
	} catch {
		return '';
	}
}

function patchReviewGate(op: OpRecord): string {
	if (op.op === 'impl.sync.write') return 'Write gate: implementation paths';
	if (op.op === 'impl.sync.patchset') return 'Patchset gate: apply after review';
	if (op.op.startsWith('xtal.repair')) return 'Repair gate: spec changes require approval';
	return 'Review gate: canonical x07 patchset';
}

function writeRootNote(path: string): string {
	if (path.startsWith('spec/')) return 'Spec writes require human approval';
	if (path.startsWith('arch/')) return 'Architecture writes require policy review';
	if (path.startsWith('tests/') || path.startsWith('gen/')) return 'Test evidence surface';
	if (path.startsWith('src/')) return 'Implementation write surface';
	return 'Workspace write surface';
}

function riskForPath(path: string): PatchReviewRisk {
	if (path.startsWith('spec/') || path.startsWith('arch/')) return 'high';
	if (path.startsWith('src/') || path.includes('patchset')) return 'medium';
	return 'low';
}

function asRecord(value: unknown): Record<string, unknown> | null {
	return value && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: null;
}

function textValue(value: unknown): string {
	return typeof value === 'string' ? value : '';
}

function numericValue(value: unknown): number {
	return typeof value === 'number' && Number.isFinite(value) ? value : 0;
}

function stringArray(value: unknown): string[] {
	return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
}

function jsonSnippet(value: unknown): string | undefined {
	if (value === undefined || value === null) return undefined;
	try {
		const body = JSON.stringify(value, null, 2);
		return body.length > 900 ? `${body.slice(0, 897)}...` : body;
	} catch {
		return undefined;
	}
}
