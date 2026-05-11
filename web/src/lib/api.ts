import {
	appendDemoOp,
	createIntentPacket,
	defaultAgentProfiles,
	demoBindings,
	demoSession,
	reduceDemoEvent,
	type AgentApprovalResponse,
	type AgentHandoffResponse,
	type AgentProfile,
	type AgentRunMode,
	type AgentRunResponse,
	type ApprovalDecision,
	type ArtifactPreviewResponse,
	type BindingDescriptor,
	type FormalizeIntentResponse,
	type HealthResponse,
	type IntentInputMode,
	type IntentPacket,
	type SessionSnapshot,
	type TaskType
} from './studio';

export class StudioApi {
	private demoMode = false;
	private demoSessions: SessionSnapshot[] = [demoSession()];

	get isDemoMode() {
		return this.demoMode;
	}

	async health(): Promise<HealthResponse> {
		try {
			const health = await request<HealthResponse>('/v1/health');
			this.demoMode = false;
			return health;
		} catch {
			this.demoMode = true;
			return { ok: true, workspace_root: this.demoSessions[0]?.root ?? '/workspace/x07-project' };
		}
	}

	async listSessions(): Promise<SessionSnapshot[]> {
		if (!this.demoMode) {
			try {
				return await request<SessionSnapshot[]>('/v1/sessions');
			} catch {
				this.demoMode = true;
			}
		}
		return this.demoSessions;
	}

	async getSession(sessionId: string): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			try {
				return await request<SessionSnapshot>(`/v1/sessions/${sessionId}`);
			} catch {
				this.demoMode = true;
			}
		}
		return (
			this.demoSessions.find((session) => session.session_id === sessionId) ??
			this.demoSessions[0]
		);
	}

	async listBindings(): Promise<BindingDescriptor[]> {
		if (!this.demoMode) {
			try {
				return await request<BindingDescriptor[]>('/v1/bindings');
			} catch {
				this.demoMode = true;
			}
		}
		return demoBindings();
	}

	async listAgents(): Promise<AgentProfile[]> {
		if (!this.demoMode) {
			try {
				return await request<AgentProfile[]>('/v1/agents');
			} catch {
				this.demoMode = true;
			}
		}
		return defaultAgentProfiles;
	}

	async createSession(title: string, task_type: TaskType): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			try {
				return await request<SessionSnapshot>('/v1/sessions', {
					method: 'POST',
					body: JSON.stringify({ title, task_type })
				});
			} catch {
				this.demoMode = true;
			}
		}
		const session = {
			...demoSession(),
			session_id: `st-demo-${Date.now()}`,
			title,
			task_type
		};
		this.demoSessions = [session, ...this.demoSessions];
		return session;
	}

	async dispatch(session: SessionSnapshot, event: string, payload?: IntentPacket): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			try {
				return await request<SessionSnapshot>(`/v1/sessions/${session.session_id}/events`, {
					method: 'POST',
					body: JSON.stringify({
						event: payload ? { event, payload } : { event }
					})
				});
			} catch {
				this.demoMode = true;
			}
		}
		const next = reduceDemoEvent(session, event, payload);
		this.replaceDemo(next);
		return next;
	}

	async formalizeIntent(
		session: SessionSnapshot,
		raw: string,
		inputMode: IntentInputMode,
		revisionNotes: string[]
	): Promise<FormalizeIntentResponse> {
		if (!this.demoMode) {
			try {
				const response = await request<FormalizeIntentResponse>(
					`/v1/sessions/${session.session_id}/intent/formalize`,
					{
						method: 'POST',
						body: JSON.stringify({
							raw,
							input_mode: inputMode,
							revision_notes: revisionNotes
						})
					}
				);
				this.replaceDemo(response.session);
				return response;
			} catch {
				this.demoMode = true;
			}
		}

		let next = reduceDemoEvent(session, 'formalize_intent', createIntentPacket(session, raw, inputMode, revisionNotes));
		next = appendDemoOp(next, 'intent.formalize', 'succeeded', [
			'studio',
			'intent',
			'formalize',
			inputMode
		]);
		this.replaceDemo(next);
		return { intent: next.intent!, op: next.op_log.at(-1)!, session: next };
	}

	async runBinding(session: SessionSnapshot, binding_id: string): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			try {
				return await request<SessionSnapshot>(`/v1/sessions/${session.session_id}/bindings/run`, {
					method: 'POST',
					body: JSON.stringify({ binding_id, vars: bindingVars(session, binding_id) })
				});
			} catch {
				this.demoMode = true;
			}
		}
		const failed = binding_id === 'xtal.verify' && session.phase === 'verify_running';
		const next = appendDemoOp(session, binding_id, failed ? 'failed' : 'succeeded');
		this.replaceDemo(next);
		return next;
	}

	async previewArtifact(
		session: SessionSnapshot,
		artifact: string
	): Promise<ArtifactPreviewResponse> {
		if (!this.demoMode) {
			return await request<ArtifactPreviewResponse>(
				`/v1/sessions/${session.session_id}/artifacts/preview`,
				{
					method: 'POST',
					body: JSON.stringify({ artifact })
				}
			);
		}
		return demoArtifactPreview(artifact);
	}

	async runXtalWorkflow(session: SessionSnapshot): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			try {
				return await request<SessionSnapshot>(`/v1/sessions/${session.session_id}/xtal/run`, {
					method: 'POST'
				});
			} catch {
				this.demoMode = true;
			}
		}

		let current = session;
		for (const bindingId of [
			'project.init.xtal-pure',
			'spec.scaffold',
			'spec.check',
			'tests.gen.write'
		]) {
			current = appendDemoOp(current, bindingId, 'succeeded');
		}
		if (current.phase === 'spec_approved') {
			current = reduceDemoEvent(current, 'propose_realization');
		}
		for (const bindingId of ['impl.sync.write', 'impl.check']) {
			current = appendDemoOp(current, bindingId, 'succeeded');
		}
		if (current.phase === 'realization_proposed') {
			current = reduceDemoEvent(current, 'accept_realization');
		}
		current = appendDemoOp(current, 'xtal.verify', 'succeeded');
		current = reduceDemoEvent(current, 'verification_passed');
		this.replaceDemo(current);
		return current;
	}

	async createAgentHandoff(session: SessionSnapshot, agentId: string): Promise<AgentHandoffResponse> {
		if (!this.demoMode) {
			try {
				const response = await request<AgentHandoffResponse>(
					`/v1/sessions/${session.session_id}/agents/${agentId}/handoff`,
					{ method: 'POST' }
				);
				this.replaceDemo(response.session);
				return response;
			} catch {
				this.demoMode = true;
			}
		}

		const agent =
			defaultAgentProfiles.find((profile) => profile.id === agentId) ?? defaultAgentProfiles[0];
		const promptPath = `.x07/studio/handoffs/${session.session_id}-${agent.id}.md`;
		const handoff = {
			schema_version: 'x07.studio.agent_handoff@0.1.0' as const,
			session_id: session.session_id,
			agent_id: agent.id,
			agent_label: agent.label,
			command: [agent.command, ...agent.args, promptPath],
			prompt_path: promptPath,
			prompt: `# x07 Studio Agent Handoff\n\nAgent: ${agent.label}\nSession: ${session.title}\n`,
			allowed_verbs: agent.allowed_verbs,
			mcp_tools: agent.mcp_tools,
			write_roots: agent.write_roots,
			approval_required: agent.approval_required,
			artifacts: [promptPath],
			created_at: String(Date.now())
		};
		const next = appendDemoOp(session, `agent.handoff.${agent.id}`, 'succeeded');
		this.replaceDemo(next);
		return { handoff, session: next };
	}

	async runAgentHandoff(
		session: SessionSnapshot,
		agentId: string,
		mode: AgentRunMode
	): Promise<AgentRunResponse> {
		if (!this.demoMode) {
			try {
				const response = await request<AgentRunResponse>(
					`/v1/sessions/${session.session_id}/agents/${agentId}/run`,
					{
						method: 'POST',
						body: JSON.stringify({ mode, timeout_seconds: 30 })
					}
				);
				this.replaceDemo(response.session);
				return response;
			} catch {
				this.demoMode = true;
			}
		}

		const agent =
			defaultAgentProfiles.find((profile) => profile.id === agentId) ?? defaultAgentProfiles[0];
		const promptPath = `.x07/studio/handoffs/${session.session_id}-${agent.id}.md`;
		const handoff = {
			schema_version: 'x07.studio.agent_handoff@0.1.0' as const,
			session_id: session.session_id,
			agent_id: agent.id,
			agent_label: agent.label,
			command: [agent.command, ...agent.args, promptPath],
			prompt_path: promptPath,
			prompt: `# x07 Studio Agent Handoff\n\nAgent: ${agent.label}\nSession: ${session.title}\n`,
			allowed_verbs: agent.allowed_verbs,
			mcp_tools: agent.mcp_tools,
			write_roots: agent.write_roots,
			approval_required: agent.approval_required,
			artifacts: [promptPath],
			created_at: String(Date.now())
		};
		if (
			mode === 'execute' &&
			agent.approval_required &&
			!agentRunApproved(session, agent.id)
		) {
			const next = appendDemoOp(
				session,
				`agent.approval.${agent.id}`,
				'pending',
				['approve-agent', agent.id],
				[]
			);
			this.replaceDemo(next);
			return { handoff, op: next.op_log.at(-1)!, session: next };
		}
		const opId = mode === 'execute' ? `agent.run.${agent.id}` : `agent.supervise.${agent.id}`;
		let next = appendDemoOp(session, opId, 'succeeded', handoff.command, [promptPath]);
		const op = next.op_log.at(-1)!;
		if (mode === 'execute') {
			next = appendDemoOp(
				next,
				`agent.event.${agent.id}.artifact`,
				'succeeded',
				['observe-agent', agent.id, 'artifact'],
				[promptPath, 'target/xtal/verify/summary.json']
			);
		}
		this.replaceDemo(next);
		return { handoff, op, session: next };
	}

	async createAgentApproval(
		session: SessionSnapshot,
		agentId: string,
		reason: string
	): Promise<AgentApprovalResponse> {
		if (!this.demoMode) {
			try {
				const response = await request<AgentApprovalResponse>(
					`/v1/sessions/${session.session_id}/agents/${agentId}/approval`,
					{ method: 'POST', body: JSON.stringify({ reason }) }
				);
				this.replaceDemo(response.session);
				return response;
			} catch {
				this.demoMode = true;
			}
		}
		const next = appendDemoOp(session, `agent.approval.${agentId}`, 'pending', [
			'approve-agent',
			agentId
		]);
		this.replaceDemo(next);
		return { op: next.op_log.at(-1)!, session: next };
	}

	async resolveAgentApproval(
		session: SessionSnapshot,
		opId: string,
		decision: ApprovalDecision,
		notes: string
	): Promise<AgentApprovalResponse> {
		if (!this.demoMode) {
			try {
				const response = await request<AgentApprovalResponse>(
					`/v1/sessions/${session.session_id}/approvals/${opId}`,
					{ method: 'POST', body: JSON.stringify({ decision, notes }) }
				);
				this.replaceDemo(response.session);
				return response;
			} catch {
				this.demoMode = true;
			}
		}
		const next = structuredClone(session) as SessionSnapshot;
		const op = next.op_log.find((item) => item.id === opId);
		if (op) {
			op.status = decision === 'approve' ? 'succeeded' : 'failed';
			op.finished_at = String(Date.now());
			op.exit_code = decision === 'approve' ? 0 : 1;
			op.notes = `${decision === 'approve' ? 'Approved' : 'Rejected'}: ${notes}`;
		}
		this.replaceDemo(next);
		return { op: op ?? next.op_log.at(-1)!, session: next };
	}

	private replaceDemo(session: SessionSnapshot) {
		this.demoSessions = this.demoSessions.map((candidate) =>
			candidate.session_id === session.session_id ? session : candidate
		);
	}
}

function bindingVars(session: SessionSnapshot, bindingId: string): Record<string, string> {
	const target = session.intent?.targets[0];
	const moduleId = target?.module_id || 'app.main';
	const op = sanitizeOpName(target?.entry || 'run_v1');
	const result = op.includes('makespan') || op.includes('count') || op.includes('len') ? 'i32' : 'bytes';
	const specInput = `spec/${moduleId}.x07spec.json`;
	const common = {
		module_id: moduleId,
		op,
		param: 'payload:bytes',
		result,
		input: specInput,
		patchset_out: 'target/xtal/impl-sync.patchset.json'
	};
	if (bindingId === 'xtal.ingest' || bindingId === 'xtal.improve') {
		return { ...common, input: '.x07/studio/incidents/manual-note.jsonl' };
	}
	return common;
}

function demoArtifactPreview(artifact: string): ArtifactPreviewResponse {
	const beforeJson = {
		schema_version: 'x07.ast@0.1.0',
		decls: [],
		solve: ['bytes.lit', 'todo']
	};
	const afterJson = {
		schema_version: 'x07.ast@0.1.0',
		decls: [{ kind: 'export', names: ['main.run'] }],
		solve: ['bytes.lit', 'ok']
	};
	const json = artifact.includes('patchset')
		? {
				schema_version: 'x07.patchset@0.1.0',
				patches: [
					{
						path: 'src/main.x07.json',
						patch: [
							{
								op: 'add',
								path: '/decls/0',
								value: { kind: 'export', names: ['main.run'] }
							},
							{
								op: 'replace',
								path: '/solve',
								value: ['bytes.lit', 'ok']
							}
						],
						note: 'Demo implementation sync from approved spec'
					}
				]
			}
		: null;
	const text = json ? JSON.stringify(json, null, 2) : '';
	return {
		schema_version: 'x07.studio.artifact_preview@0.1.0',
		artifact,
		media_kind: json ? 'json' : 'text',
		bytes_read: text.length,
		truncated: false,
		text,
		json,
		patchset_preview: json
			? {
					schema_version: 'x07.studio.patchset_preview@0.1.0',
					targets: [
						{
							path: 'src/main.x07.json',
							note: 'Demo implementation sync from approved spec',
							operations: 2,
							before_json: beforeJson,
							after_json: afterJson,
							apply_error: null,
							truncated: false
						}
					]
				}
			: null
	};
}

function agentRunApproved(session: SessionSnapshot, agentId: string): boolean {
	const approvalOp = `agent.approval.${agentId}`;
	const handoffOp = `agent.handoff.${agentId}`;
	const planOp = `agent.supervise.${agentId}`;
	const runOp = `agent.run.${agentId}`;
	const latestAgentGate = [...session.op_log].reverse().find(
		(op) => op.op === approvalOp || op.op === handoffOp || op.op === planOp || op.op === runOp
	);
	return latestAgentGate?.op === approvalOp && latestAgentGate.status === 'succeeded';
}

function sanitizeOpName(value: string): string {
	const lowered = value.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '');
	return lowered || 'run_v1';
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
	const response = await fetch(path, {
		...init,
		headers: {
			'content-type': 'application/json',
			...(init.headers ?? {})
		}
	});
	if (!response.ok) {
		throw new Error(await response.text());
	}
	return response.json() as Promise<T>;
}
