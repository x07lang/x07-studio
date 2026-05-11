import {
	appendDemoOp,
	createIntentPacket,
	defaultAgentProfiles,
	demoBindings,
	demoSession,
	reduceDemoEvent,
	type AgentProfile,
	type BindingDescriptor,
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

	formalizeLocal(
		session: SessionSnapshot,
		raw: string,
		inputMode: IntentInputMode,
		revisionNotes: string[]
	): IntentPacket {
		return createIntentPacket(session, raw, inputMode, revisionNotes);
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
