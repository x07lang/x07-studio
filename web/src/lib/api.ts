import {
	appendDemoOp,
	createIntentPacket,
	demoBindings,
	demoSession,
	reduceDemoEvent,
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
					body: JSON.stringify({ binding_id, vars: {} })
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
