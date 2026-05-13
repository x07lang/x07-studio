import {
	appendDemoOp,
	buildCertifyCommandPreview,
	buildRepairCommandPreview,
	buildVerifyCommandPreview,
	certifyRunVars,
	createIntentPacket,
	defaultAgentProfiles,
	defaultProviderProfiles,
	demoBindings,
	demoHealth,
	demoSession,
	normalizeCertifyRunOptions,
	normalizeVerifyRunOptions,
	reduceDemoEvent,
	repairRunVars,
	verifyRunVars,
	type AgentApprovalResponse,
	type AgentContract,
	type AgentHandoffResponse,
	type AgentProfile,
	type AgentRunMode,
	type AgentRunResponse,
	type ArchCheckReport,
	type AutopilotPolicy,
	type AutopilotResponse,
	type ApprovalDecision,
	type ArtifactPreviewResponse,
	type AskAnswer,
	type BindingDescriptor,
	type CassetteEntry,
	type CassetteRibbon,
	type CertificateSummary,
	type CertifyRunOptions,
	type DocPreviewResponse,
	type FormalizeIntentResponse,
	type HealthResponse,
	type HealthSnapshot,
	type IntentAnswer,
	type IntentAnswerResponse,
	type IntentClarifyResponse,
	type IntentInputMode,
	type IntentPacket,
	type LadderState,
	type LintReport,
	type MigrateStatus,
	type LiveDiff,
	type PbtRound,
	type PkgProvidesResult,
	type PlainEnglishSummary,
	type ProcessLane,
	type ProofEvidence,
	type ProviderProbeResponse,
	type ProviderProfile,
	type QuickfixRecord,
	type QuorumRound,
	type PickRealizeProposalResponse,
	type RealizeQuorumRound,
	type ReleaseStatus,
	type ReviewRound,
	type ReplayCapsule,
	type ReplayExportResponse,
	type RequestIntentRevisionResponse,
	type RoleOverrides,
	type RolePreferences,
	type SemanticDiff,
	type SemanticDiffRequest,
	type SessionSnapshot,
	type SessionStreamEvent,
	type SessionTurn,
	type StepEvidence,
	type StudioMemory,
	type SyncClaimResponse,
	type SyncCode,
	type TaskType,
	type TrustPosture,
	type TryItRequest,
	type TryItResult,
	type VoiceTranscript,
	type VisualKind,
	type VisualResponse,
	type WhatIfForecast,
	type RepairRunOptions,
	type VerifyRunOptions,
	type WorkspaceRadarResponse
} from './studio';

type BindingRunOptions =
	| Partial<VerifyRunOptions>
	| Partial<RepairRunOptions>
	| Partial<CertifyRunOptions>;

export class StudioApi {
	private demoMode = false;
	private demoSessions: SessionSnapshot[] = [];

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
			return {
				...demoHealth(),
				workspace_root: this.demoSessions[0]?.root ?? '/workspace/x07-project'
			};
		}
	}

	async workspaceRadar(): Promise<WorkspaceRadarResponse | null> {
		if (this.demoMode) return null;
		try {
			return await request<WorkspaceRadarResponse>('/v1/workspace/radar');
		} catch {
			return null;
		}
	}

	async healthSnapshot(): Promise<HealthSnapshot> {
		if (!this.demoMode) {
			try {
				return await request<HealthSnapshot>('/v1/health/snapshot');
			} catch {
				this.demoMode = true;
			}
		}
		return demoHealthSnapshot();
	}

	async applyMigrate(target = '0.5'): Promise<MigrateStatus> {
		if (!this.demoMode) {
			return await request<MigrateStatus>('/v1/health/migrate', {
				method: 'POST',
				body: JSON.stringify({ target })
			});
		}
		return { needs_migration: false, from_schema: 'x07.project@0.5.0', to_schema: target, project_schema_legacy: false };
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
			this.demoSessions[0] ??
			demoSession()
		);
	}

	async getAgentContract(sessionId: string): Promise<AgentContract> {
		if (!this.demoMode) {
			return await request<AgentContract>(`/v1/sessions/${sessionId}/agent-contract`);
		}
		const session = this.demoSessions.find((item) => item.session_id === sessionId) ?? this.demoSessions[0] ?? demoSession();
		return demoAgentContract(session);
	}

	async saveAgentContract(sessionId: string, markdown: string, priorHash?: string | null): Promise<AgentContract> {
		if (!this.demoMode) {
			return await request<AgentContract>(`/v1/sessions/${sessionId}/agent-contract`, {
				method: 'POST',
				body: JSON.stringify({ markdown, prior_hash: priorHash ?? null })
			});
		}
		const session = this.demoSessions.find((item) => item.session_id === sessionId) ?? this.demoSessions[0] ?? demoSession();
		return { ...demoAgentContract(session), markdown, exists: true, hash: String(markdown.length) };
	}

	async getLintReport(session: SessionSnapshot): Promise<LintReport> {
		if (!this.demoMode) {
			return await request<LintReport>(`/v1/sessions/${session.session_id}/lint`);
		}
		return demoLintReport(session);
	}

	async applyLintQuickfix(session: SessionSnapshot, diagnosticId: string): Promise<QuickfixRecord> {
		if (!this.demoMode) {
			return await request<QuickfixRecord>(
				`/v1/sessions/${session.session_id}/lint/${encodeURIComponent(diagnosticId)}/quickfix`,
				{ method: 'POST' }
			);
		}
		return demoQuickfixRecord(diagnosticId);
	}

	async runPbt(session: SessionSnapshot): Promise<PbtRound> {
		if (!this.demoMode) {
			return await request<PbtRound>(`/v1/sessions/${session.session_id}/pbt/run`, { method: 'POST' });
		}
		return demoPbtRound(session);
	}

	async pbtRegressionFrom(session: SessionSnapshot, reproId: string): Promise<QuickfixRecord> {
		if (!this.demoMode) {
			return await request<QuickfixRecord>(
				`/v1/sessions/${session.session_id}/pbt/regression-from/${encodeURIComponent(reproId)}`,
				{ method: 'POST' }
			);
		}
		return demoQuickfixRecord(reproId);
	}

	async archCheck(session: SessionSnapshot): Promise<ArchCheckReport> {
		if (!this.demoMode) {
			return await request<ArchCheckReport>(`/v1/sessions/${session.session_id}/arch-check`);
		}
		return { schema_version: 'x07.studio.arch_check_report@0.1.0', passed: true, violations: [], raw: { demo: true } };
	}

	async pkgProvides(moduleId: string): Promise<PkgProvidesResult> {
		if (!this.demoMode) {
			return await request<PkgProvidesResult>(`/v1/pkg/provides?module=${encodeURIComponent(moduleId)}`);
		}
		return demoPkgProvides(moduleId);
	}

	async listTurns(sessionId: string): Promise<SessionTurn[]> {
		if (!this.demoMode) {
			try {
				return await request<SessionTurn[]>(`/v1/sessions/${sessionId}/turns`);
			} catch {
				this.demoMode = true;
			}
		}
		const session = this.demoSessions.find((session) => session.session_id === sessionId) ?? this.demoSessions[0];
		return session ? projectDemoTurns(session) : [];
	}

	async getProcessLane(session: SessionSnapshot): Promise<ProcessLane> {
		if (!this.demoMode) {
			return await request<ProcessLane>(`/v1/sessions/${session.session_id}/process-lane`);
		}
		return demoProcessLane(session);
	}

	async getStepEvidence(session: SessionSnapshot, opId: string): Promise<StepEvidence | null> {
		if (!this.demoMode) {
			return await request<StepEvidence>(
				`/v1/sessions/${session.session_id}/process-lane/evidence/${encodeURIComponent(opId)}`
			);
		}
		const op = session.op_log.find((item) => item.id === opId) ?? null;
		return {
			schema_version: 'x07.studio.step_evidence@0.1.0',
			session_id: session.session_id,
			step_id: op?.op ?? 'demo',
			op,
			stream_events: [],
			artifacts: op?.artifacts ?? []
		};
	}

	async getWhatIf(session: SessionSnapshot, stepId: string): Promise<WhatIfForecast> {
		if (!this.demoMode) {
			return await request<WhatIfForecast>(`/v1/sessions/${session.session_id}/process-lane/whatif`, {
				method: 'POST',
				body: JSON.stringify({ step_id: stepId })
			});
		}
		return {
			schema_version: 'x07.studio.what_if_forecast@0.1.0',
			step_id: stepId,
			predicted_delta: null,
			estimated_duration_ms: stepId === 'verify' ? 1800 : 10000,
			confidence: 0.8,
			assumptions: ['Demo forecast uses the canonical step order.']
		};
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

	async setAgentRole(
		agentId: string,
		defaultRole: AgentProfile['default_role'],
		eligibleRoles: AgentProfile['eligible_roles']
	): Promise<AgentProfile | null> {
		if (!this.demoMode) {
			return await request<AgentProfile>(`/v1/agents/${encodeURIComponent(agentId)}`, {
				method: 'PATCH',
				body: JSON.stringify({ default_role: defaultRole, eligible_roles: eligibleRoles })
			});
		}
		const agent = defaultAgentProfiles.find((profile) => profile.id === agentId);
		if (!agent) return null;
		agent.default_role = defaultRole;
		agent.eligible_roles = eligibleRoles;
		return agent;
	}

	async listProviders(): Promise<ProviderProfile[]> {
		if (!this.demoMode) {
			try {
				const profiles = await request<ProviderProfile[]>('/v1/providers');
				return profiles.length ? profiles : defaultProviderProfiles;
			} catch {
				this.demoMode = true;
			}
		}
		return defaultProviderProfiles;
	}

	async probeProvider(profile: ProviderProfile): Promise<ProviderProbeResponse> {
		if (!this.demoMode) {
			try {
				return await request<ProviderProbeResponse>('/v1/providers/probe', {
					method: 'POST',
					body: JSON.stringify({ profile })
				});
			} catch {
				this.demoMode = true;
			}
		}
		return demoProviderProbe(profile);
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
		revisionNotes: string[],
		providerProfileId?: string,
		voiceTranscript?: VoiceTranscript | null
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
							revision_notes: revisionNotes,
							provider_profile_id: providerProfileId || null,
							voice_transcript: voiceTranscript ?? null
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
		next.revision_notes = [...revisionNotes];
		next = appendDemoOp(next, 'intent.formalize', 'succeeded', [
			'studio',
			'intent',
			'formalize',
			inputMode
		]);
		this.replaceDemo(next);
		return { intent: next.intent!, op: next.op_log.at(-1)!, session: next };
	}

	async formalizeVoiceIntent(
		session: SessionSnapshot,
		transcript: VoiceTranscript,
		revisionNotes: string[] = [],
		providerProfileId?: string
	): Promise<FormalizeIntentResponse> {
		if (!this.demoMode) {
			try {
				const response = await request<FormalizeIntentResponse>(
					`/v1/sessions/${session.session_id}/intent/voice`,
					{
						method: 'POST',
						body: JSON.stringify({
							raw: transcript.text,
							input_mode: 'voice',
							revision_notes: revisionNotes,
							provider_profile_id: providerProfileId || null,
							voice_transcript: transcript
						})
					}
				);
				this.replaceDemo(response.session);
				return response;
			} catch {
				this.demoMode = true;
			}
		}
		return this.formalizeIntent(session, transcript.text, 'voice', revisionNotes, providerProfileId, transcript);
	}

	async requestIntentRevision(
		session: SessionSnapshot,
		note: string
	): Promise<RequestIntentRevisionResponse> {
		if (!this.demoMode) {
			try {
				const response = await request<RequestIntentRevisionResponse>(
					`/v1/sessions/${session.session_id}/intent/revision`,
					{
						method: 'POST',
						body: JSON.stringify({ note })
					}
				);
				this.replaceDemo(response.session);
				return response;
			} catch {
				this.demoMode = true;
			}
		}

		let next = structuredClone(session) as SessionSnapshot;
		const revisions = [...(next.revision_notes ?? []), note.trim()].filter(Boolean);
		next.revision_notes = revisions;
		next.room = 'intent';
		next = appendDemoOp(
			next,
			'intent.revision.request',
			'succeeded',
			['studio', 'intent', 'request-changes', String(revisions.length)],
			['.x07/studio/sessions/intent.json'],
			{
				schema_version: 'x07.studio.intent_revision_request@0.1.0',
				revision_index: revisions.length,
				note: revisions.at(-1),
				approval_state: 'changes'
			}
		);
		this.replaceDemo(next);
		return { op: next.op_log.at(-1)!, session: next };
	}

	async runBinding(
		session: SessionSnapshot,
		binding_id: string,
		options?: BindingRunOptions
	): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			try {
				return await request<SessionSnapshot>(`/v1/sessions/${session.session_id}/bindings/run`, {
					method: 'POST',
					body: JSON.stringify({ binding_id, vars: bindingVars(session, binding_id, options) })
				});
			} catch {
				this.demoMode = true;
			}
		}
		const failed = binding_id === 'xtal.verify' && session.phase === 'verify_running';
		const next = appendDemoOp(
			session,
			binding_id,
			failed ? 'failed' : 'succeeded',
			bindingCommandPreview(binding_id, options),
			undefined,
			binding_id === 'xtal.verify'
				? demoVerifySummary(session, options as Partial<VerifyRunOptions>)
				: binding_id === 'xtal.certify'
					? demoCertifySummary(session, options as Partial<CertifyRunOptions>)
					: undefined
		);
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

	async previewDoc(session: SessionSnapshot, docRef: string): Promise<DocPreviewResponse> {
		if (!this.demoMode) {
			return await request<DocPreviewResponse>(
				`/v1/sessions/${session.session_id}/docs/preview`,
				{
					method: 'POST',
					body: JSON.stringify({ doc_ref: docRef })
				}
			);
		}
		return demoDocPreview(docRef);
	}

	async clarifyIntent(
		session: SessionSnapshot,
		agentId: string,
		options: { timeoutSeconds?: number } = {}
	): Promise<IntentClarifyResponse | null> {
		if (this.demoMode) return null;
		try {
			const body: Record<string, unknown> = { agent_id: agentId };
			if (options.timeoutSeconds !== undefined) {
				body.timeout_seconds = options.timeoutSeconds;
			}
			const response = await request<IntentClarifyResponse>(
				`/v1/sessions/${session.session_id}/intent/clarify`,
				{ method: 'POST', body: JSON.stringify(body) }
			);
			this.replaceDemo(response.session);
			return response;
		} catch (error) {
			if (error instanceof HttpRequestError) throw error;
			this.demoMode = true;
			return null;
		}
	}

	async answerIntent(
		session: SessionSnapshot,
		answers: IntentAnswer[]
	): Promise<IntentAnswerResponse | null> {
		if (this.demoMode) return null;
		try {
			const response = await request<IntentAnswerResponse>(
				`/v1/sessions/${session.session_id}/intent/answer`,
				{ method: 'POST', body: JSON.stringify({ answers }) }
			);
			this.replaceDemo(response.session);
			return response;
		} catch (error) {
			if (error instanceof HttpRequestError) throw error;
			this.demoMode = true;
			return null;
		}
	}

	async runIntentQuorum(
		session: SessionSnapshot,
		agentIds: string[],
		options: { timeoutSeconds?: number } = {}
	): Promise<QuorumRound | null> {
		if (this.demoMode) return null;
		const response = await request<QuorumRound>(`/v1/sessions/${session.session_id}/intent/quorum`, {
			method: 'POST',
			body: JSON.stringify({ agent_ids: agentIds, timeout_seconds: options.timeoutSeconds ?? null })
		});
		return response;
	}

	async runBuildPipeline(
		session: SessionSnapshot,
		options: { maxRepairRounds?: number; verifyOptions?: Partial<VerifyRunOptions> } = {}
	): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			try {
				const body: Record<string, unknown> = {
					vars: verifyRunVars(options.verifyOptions),
				};
				if (options.maxRepairRounds !== undefined) {
					body.max_repair_rounds = options.maxRepairRounds;
				}
				const next = await request<SessionSnapshot>(
					`/v1/sessions/${session.session_id}/build`,
					{ method: 'POST', body: JSON.stringify(body) }
				);
				this.replaceDemo(next);
				return next;
			} catch (error) {
				if (error instanceof HttpRequestError) throw error;
				this.demoMode = true;
			}
		}
		// Demo-mode fallback: mimic the build pipeline with an explicit "start"
		// stage, the existing XTAL workflow, and a "done" stage. Plain-English
		// summary is omitted because the demo session has no live evidence to
		// summarize.
		let current = appendDemoOp(session, 'build.stage.start', 'succeeded', [
			'studio',
			'build',
			'stage',
			'start'
		]);
		current = await this.runXtalWorkflow(current, options.verifyOptions);
		current = appendDemoOp(current, 'build.stage.done', 'succeeded', [
			'studio',
			'build',
			'stage',
			'done'
		]);
		current = appendDemoOp(
			current,
			'summary.plain_english',
			'succeeded',
			['studio', 'summary', 'plain-english'],
			[`.x07/studio/sessions/${current.session_id}.json`],
			{
				schema_version: 'x07.studio.plain_english_summary@0.1.0',
				headline: 'Built and verified.',
				behavior_promises: [current.intent?.witnesses[0]?.text ?? 'The requested behavior is ready.'],
				boundaries: [],
				evidence: ['Verified correctness (1 pass).'],
				run_invocation:
					'printf "%s" "<your input here>" | x07 run --project x07.json --profile sandbox --stdin',
				followups: ['Do you want a CLI wrapper for this?']
			}
		);
		this.replaceDemo(current);
		return current;
	}

	async invoke(session: SessionSnapshot, req: TryItRequest): Promise<TryItResult> {
		if (!this.demoMode) {
			const result = await request<TryItResult>(`/v1/sessions/${session.session_id}/invoke`, {
				method: 'POST',
				body: JSON.stringify(req)
			});
			return result;
		}
		return {
			output_kind: 'text',
			output_text: req.input_text ? `demo output for ${req.input_text}` : 'demo output',
			output_json: null,
			stats: { demo: true },
			proof_citations: [
				{
					clause_id: session.intent?.targets[0]?.module_id ?? 'demo',
					proof_report: 'target/xtal/verify/summary.json',
					summary: 'Demo verify citation'
				}
			],
			op_id: `op-try-${Date.now()}`
		};
	}

	async ladderState(session: SessionSnapshot): Promise<LadderState> {
		if (!this.demoMode) {
			return await request<LadderState>(`/v1/sessions/${session.session_id}/ladder`);
		}
		return {
			current_rung: session.phase === 'certified' ? 'team' : 'local_preview',
			rungs: ['local_preview', 'shareable', 'team', 'production'].map((id, index) => ({
				id,
				label: ['Local preview', 'Shareable', 'Team', 'Production'][index],
				profile_path: index === 0 ? null : `arch/trust/profiles/${id}.json`,
				satisfied: index === 0,
				missing: index === 0 ? [] : [`arch/trust/profiles/${id}.json`],
				evidence: index === 0 ? ['demo verify evidence'] : [],
				gates: [
					{
						id: `${id}-gate`,
						label: `${['Local preview', 'Shareable', 'Team', 'Production'][index]} gate`,
						description: 'Demo trust gate',
						currently_satisfied: index === 0
					}
				]
			}))
		};
	}

	async trustPosture(session: SessionSnapshot): Promise<TrustPosture> {
		if (!this.demoMode) {
			return await request<TrustPosture>(`/v1/sessions/${session.session_id}/trust/posture`);
		}
		return demoTrustPosture(session);
	}

	async semanticDiff(session: SessionSnapshot, req: SemanticDiffRequest): Promise<SemanticDiff> {
		if (!this.demoMode) {
			return await request<SemanticDiff>(`/v1/sessions/${session.session_id}/diff`, {
				method: 'POST',
				body: JSON.stringify({
					schema_version: 'x07.studio.semantic_diff_request@0.1.0',
					mode: 'project',
					...req
				})
			});
		}
		return {
			schema_version: 'x07.studio.semantic_diff@0.1.0',
			from: req.from,
			to: req.to,
			headline: 'stays solve-pure · no trust delta',
			trust_delta_color: 'green',
			raw: { demo: true },
			world_changes: [],
			capability_changes: [],
			budget_changes: [],
			proof_changes: []
		};
	}

	async proofEvidence(session: SessionSnapshot, behaviorId: string): Promise<ProofEvidence> {
		if (!this.demoMode) {
			return await request<ProofEvidence>(`/v1/sessions/${session.session_id}/proof/${behaviorId}`);
		}
		return {
			schema_version: 'x07.studio.proof_evidence@0.1.0',
			session_id: session.session_id,
			behavior_id: behaviorId,
			status: 'proved',
			citations: [{ kind: 'proof', file: 'target/xtal/verify/summary.json', region: 'summary' }],
			obligations: [{ id: behaviorId, goal: behaviorId.replaceAll('-', ' '), status: 'proved', note: 'Demo proof evidence' }],
			z3_ms: 12,
			assumptions: []
		};
	}

	async realize(
		session: SessionSnapshot,
		options: { agentId?: string; timeoutSeconds?: number } = {}
	): Promise<{ agent_id: string; ok: boolean; wrote_files: string[]; session: SessionSnapshot }> {
		if (!this.demoMode) {
			const body: Record<string, unknown> = {};
			if (options.agentId) body.agent_id = options.agentId;
			if (options.timeoutSeconds) body.timeout_seconds = options.timeoutSeconds;
			const response = await request<{
				agent_id: string;
				ok: boolean;
				wrote_files: string[];
				session: SessionSnapshot;
			}>(`/v1/sessions/${session.session_id}/realize`, {
				method: 'POST',
				body: JSON.stringify(body)
			});
			this.replaceDemo(response.session);
			return response;
		}
		const next = appendDemoOp(session, `agent.realize.${options.agentId ?? 'claude-code'}`, 'succeeded');
		this.replaceDemo(next);
		return {
			agent_id: options.agentId ?? 'claude-code',
			ok: true,
			wrote_files: ['src/main.x07.json'],
			session: next
		};
	}

	async realizeQuorum(
		session: SessionSnapshot,
		agentIds: string[] = ['claude-code', 'openai-codex'],
		options: { timeoutSeconds?: number } = {}
	): Promise<RealizeQuorumRound | null> {
		if (!this.demoMode) {
			return await request<RealizeQuorumRound>(`/v1/sessions/${session.session_id}/realize/quorum`, {
				method: 'POST',
				body: JSON.stringify({
					schema_version: 'x07.studio.realize_quorum_request@0.1.0',
					agent_ids: agentIds,
					timeout_seconds: options.timeoutSeconds ?? null
				})
			});
		}
		return {
			schema_version: 'x07.studio.realize_quorum_round@0.1.0',
			session_id: session.session_id,
			started_at: String(Date.now()),
			finished_at: String(Date.now()),
			agreed: false,
			judge: null,
			proposals: [
				{
					schema_version: 'x07.studio.realize_proposal@0.1.0',
					agent_id: 'claude-code',
					path: 'src/main.x07.json',
					body: { agent: 'claude-code' },
					digest: 'demo-claude',
					stdout_excerpt: 'demo proposal',
					stderr_excerpt: '',
					status: 'ok'
				},
				{
					schema_version: 'x07.studio.realize_proposal@0.1.0',
					agent_id: 'openai-codex',
					path: 'src/main.x07.json',
					body: { agent: 'openai-codex' },
					digest: 'demo-codex',
					stdout_excerpt: 'demo proposal',
					stderr_excerpt: '',
					status: 'ok'
				}
			]
		};
	}

	async pickRealizeProposal(
		session: SessionSnapshot,
		proposalIndex: number
	): Promise<PickRealizeProposalResponse | null> {
		if (!this.demoMode) {
			const response = await request<PickRealizeProposalResponse>(
				`/v1/sessions/${session.session_id}/realize/pick`,
				{
					method: 'POST',
					body: JSON.stringify({ proposal_index: proposalIndex })
				}
			);
			this.replaceDemo(response.session);
			return response;
		}
		return null;
	}

	async startAutopilot(
		session: SessionSnapshot,
		policy?: Partial<AutopilotPolicy>
	): Promise<AutopilotResponse | null> {
		const resolvedPolicy = {
			auto_answer_min_confidence: 0.7,
			max_clarify_rounds: 3,
			auto_climb_to: null,
			allow_repair_iters: 3,
			allow_quorum: false,
			...policy
		};
		if (!this.demoMode) {
			const response = await request<AutopilotResponse>(
				`/v1/sessions/${session.session_id}/autopilot/start`,
				{
					method: 'POST',
					body: JSON.stringify({ policy: resolvedPolicy })
				}
			);
			this.replaceDemo(response.session);
			return response;
		}
		let next = await this.runBuildPipeline(session, { maxRepairRounds: policy?.allow_repair_iters ?? 3 });
		return {
			state: {
				schema_version: 'x07.studio.autopilot_state@0.1.0',
				session_id: next.session_id,
				engaged: false,
				policy: {
					...resolvedPolicy
				},
				last_decision: {
					at: String(Date.now()),
					stage: 'complete',
					action: 'auto',
					reason: 'Demo autopilot completed the build pipeline.'
				}
			},
			session: next
		};
	}

	async pauseAutopilot(session: SessionSnapshot): Promise<AutopilotResponse | null> {
		if (this.demoMode) return null;
		const response = await request<AutopilotResponse>(
			`/v1/sessions/${session.session_id}/autopilot/pause`,
			{ method: 'POST' }
		);
		this.replaceDemo(response.session);
		return response;
	}

	async reviewSession(session: SessionSnapshot, reviewerId?: string | null): Promise<ReviewRound | null> {
		if (!this.demoMode) {
			return await request<ReviewRound>(`/v1/sessions/${session.session_id}/review`, {
				method: 'POST',
				body: JSON.stringify({ reviewer_id: reviewerId ?? null })
			});
		}
		return {
			schema_version: 'x07.studio.review_round@0.1.0',
			session_id: session.session_id,
			round: 1,
			reviewer: reviewerId ?? 'demo-reviewer',
			verdict: 'accept',
			concerns: [],
			created_at: new Date().toISOString()
		};
	}

	async getRoleOverrides(session: SessionSnapshot): Promise<RoleOverrides> {
		if (!this.demoMode) {
			return await request<RoleOverrides>(`/v1/sessions/${session.session_id}/role-overrides`);
		}
		return { schema_version: 'x07.studio.role_overrides@0.1.0' };
	}

	async setRoleOverrides(session: SessionSnapshot, overrides: RoleOverrides): Promise<RoleOverrides> {
		if (!this.demoMode) {
			return await request<RoleOverrides>(`/v1/sessions/${session.session_id}/role-overrides`, {
				method: 'POST',
				body: JSON.stringify(overrides)
			});
		}
		return overrides;
	}

	async climbRung(session: SessionSnapshot, toRung: string): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			const next = await request<SessionSnapshot>(`/v1/sessions/${session.session_id}/ladder/climb`, {
				method: 'POST',
				body: JSON.stringify({ to_rung: toRung })
			});
			this.replaceDemo(next);
			return next;
		}
		const next = appendDemoOp(session, `trust.certify.${toRung}`, 'succeeded');
		this.replaceDemo(next);
		return next;
	}

	async submitRelease(session: SessionSnapshot, rung: string): Promise<ReleaseStatus | null> {
		if (this.demoMode) return null;
		return await request<ReleaseStatus>(`/v1/sessions/${session.session_id}/ladder/release`, {
			method: 'POST',
			body: JSON.stringify({
				schema_version: 'x07.studio.release_request@0.1.0',
				rung,
				environment: rung === 'team' ? 'team-staging' : rung,
				binding_refs: []
			})
		});
	}

	async getReleaseStatus(
		session: SessionSnapshot,
		releaseId: string
	): Promise<ReleaseStatus | null> {
		if (this.demoMode) return null;
		return await request<ReleaseStatus>(
			`/v1/sessions/${session.session_id}/ladder/release/${encodeURIComponent(releaseId)}`
		);
	}

	async certificateSummary(session: SessionSnapshot): Promise<CertificateSummary> {
		if (!this.demoMode) {
			return await request<CertificateSummary>(`/v1/sessions/${session.session_id}/certificate`);
		}
		return {
			schema_version: 'x07.studio.certificate_summary@0.1.0',
			session_id: session.session_id,
			profile: 'verified_core_pure_v1',
			operational_entry: session.intent?.targets[0]?.entry ?? 'main',
			issued_at: new Date().toISOString(),
			expires_at: null,
			proof_summary: { demo: true },
			trust_report: { demo: true },
			html_summary_path: 'target/xtal/cert/summary.html',
			signature: 'demo-signature'
		};
	}

	async refreshCertificate(session: SessionSnapshot): Promise<CertificateSummary> {
		if (!this.demoMode) {
			return await request<CertificateSummary>(`/v1/sessions/${session.session_id}/certificate/refresh`, {
				method: 'POST'
			});
		}
		return this.certificateSummary(session);
	}

	async scanIncidents(session: SessionSnapshot) {
		if (!this.demoMode) {
			return await request(`/v1/sessions/${session.session_id}/incidents/scan`, { method: 'POST' });
		}
		return [];
	}

	async watchIncidents(session: SessionSnapshot) {
		if (!this.demoMode) {
			return await request(`/v1/sessions/${session.session_id}/incidents/watch`, { method: 'POST' });
		}
		return [];
	}

	async repairIncident(session: SessionSnapshot, incidentId: string): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			const next = await request<SessionSnapshot>(
				`/v1/sessions/${session.session_id}/incidents/${incidentId}/repair`,
				{ method: 'POST' }
			);
			this.replaceDemo(next);
			return next;
		}
		const next = appendDemoOp(session, 'xtal.improve', 'succeeded', ['x07', 'xtal', 'improve', incidentId]);
		this.replaceDemo(next);
		return next;
	}

	async incidentQuickfix(session: SessionSnapshot, incidentId: string): Promise<QuickfixRecord> {
		if (!this.demoMode) {
			return await request<QuickfixRecord>(
				`/v1/sessions/${session.session_id}/incidents/${incidentId}/quickfix`
			);
		}
		return {
			...demoQuickfixRecord('X07-DEMO'),
			summary: `Demo quickfix for ${incidentId}`,
			citations: [{ kind: 'incident', file: `.x07-wasm/incidents/${incidentId}`, region: 'run.report.json' }]
		};
	}

	async cassetteEntries(session: SessionSnapshot): Promise<CassetteEntry[]> {
		if (!this.demoMode) {
			return await request<CassetteEntry[]>(`/v1/sessions/${session.session_id}/cassette`);
		}
		return [];
	}

	async cassetteRibbon(session: SessionSnapshot): Promise<CassetteRibbon> {
		if (!this.demoMode) {
			return await request<CassetteRibbon>(`/v1/sessions/${session.session_id}/cassettes/ribbon`);
		}
		return { schema_version: 'x07.studio.cassette_ribbon@0.1.0', boundaries: [] };
	}

	async branchCassette(session: SessionSnapshot, fromEntry: number, newTitle: string): Promise<string | null> {
		if (!this.demoMode) {
			return await request<string>(`/v1/sessions/${session.session_id}/cassette/branch`, {
				method: 'POST',
				body: JSON.stringify({ from_entry: fromEntry, new_title: newTitle })
			});
		}
		return null;
	}

	async askProject(session: SessionSnapshot, question: string): Promise<AskAnswer> {
		if (!this.demoMode) {
			return await request<AskAnswer>(`/v1/sessions/${session.session_id}/ask`, {
				method: 'POST',
				body: JSON.stringify({ question })
			});
		}
		return {
			text: `Demo answer for ${question}`,
			citations: [{ kind: 'spec', path: 'spec/demo.x07spec.json', locator: '/operations/0' }]
		};
	}

	async mintSyncCode(): Promise<SyncCode | null> {
		if (this.demoMode) return null;
		return await request<SyncCode>('/v1/sync/codes');
	}

	async claimSyncCode(code: string): Promise<SyncClaimResponse | null> {
		if (this.demoMode) return null;
		return await request<SyncClaimResponse>(`/v1/sync/${encodeURIComponent(code)}/claim`, {
			method: 'POST'
		});
	}

	async saveSyncState(code: string, stateBlob: unknown): Promise<SyncCode | null> {
		if (this.demoMode) return null;
		return await request<SyncCode>(`/v1/sync/sessions/${encodeURIComponent(code)}/state`, {
			method: 'POST',
			body: JSON.stringify({ state_blob: stateBlob })
		});
	}

	async loadMemory(): Promise<StudioMemory | null> {
		if (this.demoMode) return null;
		return await request<StudioMemory>('/v1/memory');
	}

	async saveMemory(patch: Partial<StudioMemory>): Promise<StudioMemory | null> {
		if (this.demoMode) return null;
		return await request<StudioMemory>('/v1/memory', {
			method: 'POST',
			body: JSON.stringify(patch)
		});
	}

	async loadRolePreferences(): Promise<RolePreferences> {
		if (!this.demoMode) return await request<RolePreferences>('/v1/memory/role-preferences');
		return {
			schema_version: 'x07.studio.role_preferences@0.1.0',
			default_architect: 'claude-code',
			default_coder: 'openai-codex',
			default_reviewer: 'claude-code',
			allow_self_review: true,
			default_max_review_rounds: 2
		};
	}

	async saveRolePreferences(preferences: RolePreferences): Promise<RolePreferences | null> {
		if (this.demoMode) return preferences;
		return await request<RolePreferences>('/v1/memory/role-preferences', {
			method: 'POST',
			body: JSON.stringify(preferences)
		});
	}

	async exportReplay(session: SessionSnapshot): Promise<ReplayExportResponse | null> {
		if (this.demoMode) return null;
		return await request<ReplayExportResponse>(`/v1/sessions/${session.session_id}/replay/export`, {
			method: 'POST'
		});
	}

	async importReplay(capsule: ReplayCapsule): Promise<SessionSnapshot | null> {
		if (this.demoMode) return null;
		const session = await request<SessionSnapshot>('/v1/replay/import', {
			method: 'POST',
			body: JSON.stringify({ capsule })
		});
		this.replaceDemo(session);
		return session;
	}

	async visualParse(
		session: SessionSnapshot,
		kind: VisualKind,
		source: unknown
	): Promise<VisualResponse | null> {
		if (this.demoMode) {
			return {
				schema_version: 'x07.studio.visual@0.1.0',
				kind,
				value: demoVisualParse(kind, source)
			};
		}
		return await request<VisualResponse>(`/v1/sessions/${session.session_id}/visual/${kind}/parse`, {
			method: 'POST',
			body: JSON.stringify({ source })
		});
	}

	async visualEmit(
		session: SessionSnapshot,
		kind: VisualKind,
		graph: unknown
	): Promise<VisualResponse | null> {
		if (this.demoMode) {
			return {
				schema_version: 'x07.studio.visual@0.1.0',
				kind,
				value: kind === 'streampipe' ? labelsFromGraph(graph).join(' | ') : graph
			};
		}
		return await request<VisualResponse>(`/v1/sessions/${session.session_id}/visual/${kind}/emit`, {
			method: 'POST',
			body: JSON.stringify({ graph })
		});
	}

	subscribeSession(
		sessionId: string,
		listener: (event: SessionStreamEvent) => void
	): () => void {
		if (this.demoMode || typeof EventSource === 'undefined') {
			return () => undefined;
		}
		const source = new EventSource(`/v1/sessions/${sessionId}/stream`);
		source.onmessage = (message) => {
			try {
				const event = JSON.parse(message.data) as SessionStreamEvent;
				listener(event);
			} catch {
				// drop malformed frames; the next correct one will catch the client up
			}
		};
		source.onerror = () => {
			// EventSource auto-reconnects with the same URL; nothing to do here.
		};
		return () => source.close();
	}

	subscribeLiveDiffs(sessionId: string, listener: (diff: LiveDiff) => void): () => void {
		if (this.demoMode || typeof EventSource === 'undefined') {
			return () => undefined;
		}
		const source = new EventSource(`/v1/sessions/${sessionId}/diffs/live`);
		source.onmessage = (message) => {
			try {
				listener(JSON.parse(message.data) as LiveDiff);
			} catch {
				// drop malformed frames; the next correct one will catch the client up
			}
		};
		source.onerror = () => {
			// EventSource auto-reconnects with the same URL; nothing to do here.
		};
		return () => source.close();
	}

	async runXtalWorkflow(
		session: SessionSnapshot,
		verifyOptions?: Partial<VerifyRunOptions>
	): Promise<SessionSnapshot> {
		if (!this.demoMode) {
			try {
				return await request<SessionSnapshot>(`/v1/sessions/${session.session_id}/xtal/run`, {
					method: 'POST',
					body: JSON.stringify({ vars: verifyRunVars(verifyOptions) })
				});
			} catch {
				this.demoMode = true;
			}
		}

		let current = session;
		if (current.intent?.source.kind === 'incident') {
			current = appendDemoOp(current, 'project.init.xtal-pure', 'succeeded');
			current = appendDemoOp(current, 'xtal.manifest.ensure', 'succeeded', [
				'studio',
				'xtal',
				'manifest',
				'ensure',
				'arch/xtal/xtal.json'
			]);
			current = reduceDemoEvent(current, 'ingest_incident');
			current = appendDemoOp(current, 'xtal.ingest', 'succeeded');
			current = appendDemoOp(current, 'xtal.improve', 'succeeded');
			this.replaceDemo(current);
			return current;
		}
		for (const bindingId of [
			demoProjectScaffoldBinding(current),
			'spec.scaffold',
			'spec.check',
			'tests.gen.write'
		]) {
			current = appendDemoOp(current, bindingId, 'succeeded');
		}
		if (current.phase === 'spec_approved') {
			current = reduceDemoEvent(current, 'propose_realization');
		}
		const realizationBindings = isAtlasSession(current)
			? atlasDemoWorkflowBindings
			: ['impl.sync.write', 'impl.check', 'xtal.verify'];
		for (const bindingId of realizationBindings) {
			current = appendDemoOp(
				current,
				bindingId,
				'succeeded',
				bindingId === 'xtal.verify' ? buildVerifyCommandPreview(verifyOptions).split(' ') : undefined,
				undefined,
				bindingId === 'xtal.verify' ? demoVerifySummary(current, verifyOptions) : undefined
			);
		}
		if (current.phase === 'realization_proposed') {
			current = reduceDemoEvent(current, 'accept_realization');
		}
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
			} catch (error) {
				if (error instanceof HttpRequestError) throw error;
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
			} catch (error) {
				if (error instanceof HttpRequestError) throw error;
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

function bindingVars(
	session: SessionSnapshot,
	bindingId: string,
	options?: BindingRunOptions
): Record<string, string> {
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
		const incidentInput =
			session.intent?.source.kind === 'incident'
				? session.intent.source.path
				: '.x07/studio/incidents/manual';
		return { ...common, input: incidentInput };
	}
	if (bindingId === 'xtal.verify') return { ...common, ...verifyRunVars(options as Partial<VerifyRunOptions>) };
	if (bindingId === 'xtal.repair') return { ...common, ...repairRunVars(options as Partial<RepairRunOptions>) };
	if (bindingId === 'xtal.certify') return { ...common, ...certifyRunVars(options as Partial<CertifyRunOptions>) };
	return common;
}

function bindingCommandPreview(bindingId: string, options?: BindingRunOptions): string[] | undefined {
	if (bindingId === 'xtal.verify') {
		return buildVerifyCommandPreview(options as Partial<VerifyRunOptions>).split(' ');
	}
	if (bindingId === 'xtal.repair') {
		return buildRepairCommandPreview(options as Partial<RepairRunOptions>).split(' ');
	}
	if (bindingId === 'xtal.certify') {
		return buildCertifyCommandPreview(options as Partial<CertifyRunOptions>).split(' ');
	}
	return undefined;
}

function demoVerifySummary(session: SessionSnapshot, verifyOptions?: Partial<VerifyRunOptions>) {
	const normalized = normalizeVerifyRunOptions(verifyOptions);
	const target = session.intent?.targets[0];
	const moduleId = target?.module_id || 'toy.sorter';
	const entry = target?.entry || 'sort_u8_asc';
	const fullEntry = `${moduleId}.${entry}`;
	const localPath = fullEntry.replaceAll('.', '/');
	const coveragePath = `target/xtal/verify/coverage/${localPath}.report.json`;
	const provePath = `target/xtal/verify/prove/${localPath}.report.json`;
	const testsPath = 'target/xtal/tests.report.json';
	const diagnosticsPath = 'target/xtal/xtal.verify.diag.json';
	const bounds: Record<string, number> = {};
	if (normalized.unwind) bounds.unwind = Number(normalized.unwind);
	if (normalized.maxBytesLen) bounds.max_bytes_len = Number(normalized.maxBytesLen);
	if (normalized.inputLenBytes) bounds.input_len_bytes = Number(normalized.inputLenBytes);
	return {
		schema_version: 'x07.xtal.verify_summary@0.1.0',
		tool: {
			name: 'x07',
			version: 'demo',
			argv: buildVerifyCommandPreview(normalized).split(' ')
		},
		project: {
			root: '.',
			manifest_path: 'x07.json',
			manifest_sha256: demoSha('manifest')
		},
		settings: {
			world: normalized.allowOsWorld ? 'run-os-approved' : 'solve-pure',
			proof_policy: normalized.proofPolicy,
			verify_bounds: bounds
		},
		inputs: {
			digests: {
				spec_tree_sha256: demoSha('spec'),
				impl_tree_sha256: demoSha('impl'),
				arch_tree_sha256: demoSha('arch'),
				gen_tree_sha256: demoSha('gen')
			},
			spec_modules: [{ path: `spec/${moduleId}.x07spec.json`, sha256: demoSha('spec-module') }],
			generated_artifacts: [{ path: 'gen/xtal/tests.json', sha256: demoSha('generated-tests') }],
			impl_modules: [{ path: `src/${moduleId.replaceAll('.', '/')}.x07.json`, sha256: demoSha('impl-module') }]
		},
		results: {
			outcome: 'warn',
			prechecks: { spec: 'pass', generation: 'pass', impl: 'pass' },
			verification: {
				coverage_outcome: 'pass',
				prove_outcome: 'warn',
				counts: {
					entries_total: 1,
					coverage_fail: 0,
					prove_proven: 0,
					prove_counterexample: 0,
					prove_inconclusive: 0,
					prove_unsupported: 1,
					prove_timeout: 0,
					prove_error: 0,
					prove_tool_missing: 0
				}
			},
			tests: {
				outcome: 'pass',
				report: demoReportRef('x07_tests_report', testsPath, 'x07.x07test@0.4.0'),
				passed: 6,
				failed: 0,
				skipped: 0
			},
			diagnostics: {
				outcome: 'warn',
				report: demoReportRef('xtal_diag_report', diagnosticsPath, 'x07.x07diag@0.1.0'),
				error_count: 0,
				warning_count: 1,
				top_codes: [{ code: 'WXTAL_VERIFY_PROVE_UNSUPPORTED', count: 1 }]
			}
		},
		artifacts: {
			verify_dir: 'target/xtal/verify',
			reports: [
				demoReportRef('x07_verify_coverage_report', coveragePath, 'x07.verify.report@0.8.0'),
				demoReportRef('x07_verify_prove_report', provePath, 'x07.verify.report@0.8.0'),
				demoReportRef('x07_tests_report', testsPath, 'x07.x07test@0.4.0')
			]
		},
		entries: [
			{
				entry: fullEntry,
				op_id: `op.${entry}.v1`,
				spec_path: `spec/${moduleId}.x07spec.json`,
				coverage: {
					outcome: 'pass',
					report: demoReportRef('x07_verify_coverage_report', coveragePath, 'x07.verify.report@0.8.0')
				},
				prove: {
					raw: 'unsupported',
					policy_outcome: 'warn',
					report: demoReportRef('x07_verify_prove_report', provePath, 'x07.verify.report@0.8.0'),
					first_diagnostic: {
						code: 'WXTAL_VERIFY_PROVE_UNSUPPORTED',
						message: 'Demo projection marks proof unsupported so warning evidence stays visible.'
					}
				}
			}
		]
	};
}

function demoCertifySummary(session: SessionSnapshot, certifyOptions?: Partial<CertifyRunOptions>) {
	const normalized = normalizeCertifyRunOptions(certifyOptions);
	const target = session.intent?.targets[0];
	const moduleId = target?.module_id || 'toy.sorter';
	const entry = normalized.entry || target?.entry || 'sort_u8_asc';
	const fullEntry = entry.includes('.') ? entry : `${moduleId}.${entry}`;
	const entryPath = fullEntry.replaceAll('.', '/');
	const outDir = 'target/xtal/cert';
	const entryOutDir = `${outDir}/${entryPath}`;
	return {
		schema_version: 'x07.xtal.certify_summary@0.1.0',
		project: {
			root: '.',
			manifest_path: 'x07.json',
			manifest_sha256: demoSha('manifest'),
			xtal_manifest_path: 'arch/xtal/xtal.json',
			xtal_manifest_sha256: demoSha('xtal-manifest'),
			trust_profile_path: 'arch/trust/profiles/verified_core_pure_v1.json',
			trust_profile_sha256: demoSha('trust-profile'),
			baseline_path: 'target/xtal/cert/baseline.json',
			baseline_sha256: demoSha('baseline')
		},
		settings: {
			out_dir: outDir,
			entries: [fullEntry],
			all_entries: normalized.allEntries,
			run_prechecks: !normalized.noPrechecks,
			review_gates: ['proof_coverage', 'trust_profile', 'spec_examples']
		},
		results: [
			{
				entry: fullEntry,
				out_dir: entryOutDir,
				ok: true,
				certificate_path: `${entryOutDir}/certificate.json`,
				certificate_sha256: demoSha('certificate'),
				trust_report_path: `${entryOutDir}/trust.report.json`,
				trust_report_sha256: demoSha('trust-report'),
				review_diff_json_path: `${entryOutDir}/review.diff.json`,
				review_diff_txt_path: `${entryOutDir}/review.diff.txt`
			}
		],
		ok: true,
		generated_at: new Date(0).toISOString()
	};
}

function demoReportRef(kind: string, path: string, schemaVersion: string) {
	return {
		kind,
		path,
		schema_version: schemaVersion,
		sha256: demoSha(path)
	};
}

function demoSha(seed: string) {
	const alphabet = '0123456789abcdef';
	return Array.from({ length: 64 }, (_, index) => alphabet[(seed.length + index) % alphabet.length]).join('');
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
	const isPatchset = artifact.includes('patchset');
	const json = isPatchset
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
		: artifact.includes('/cert/') && artifact.endsWith('bundle.json')
			? demoCertBundleArtifact()
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
		patchset_preview: isPatchset && json
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

function demoCertBundleArtifact() {
	return {
		schema_version: 'x07.xtal.cert_bundle@0.1.0',
		out_dir: 'target/xtal/cert',
		spec_dir: 'spec',
		generated_at: '1970-01-01T00:00:00Z',
		ok: true,
		entries: [{ entry: 'toy.sorter.sort_u8_asc', dir: 'target/xtal/cert/toy/sorter/sort_u8_asc' }],
		files: [
			{
				path: 'target/xtal/cert/toy/sorter/sort_u8_asc/certificate.json',
				sha256: demoSha('certificate'),
				bytes_len: 4096
			},
			{
				path: 'target/xtal/cert/toy/sorter/sort_u8_asc/trust.report.json',
				sha256: demoSha('trust-report'),
				bytes_len: 8192
			},
			{
				path: 'target/xtal/cert/toy/sorter/sort_u8_asc/review.diff.json',
				sha256: demoSha('review-diff'),
				bytes_len: 1536
			}
		],
		external_files: [],
		spec_digests: [
			{
				path: 'spec/toy.sorter.x07spec.json',
				sha256: demoSha('spec-module'),
				bytes_len: 2048
			}
		],
		examples_digests: [
			{
				path: 'spec/examples/toy.sorter.sort_u8_asc.jsonl',
				sha256: demoSha('examples'),
				bytes_len: 1024
			}
		]
	};
}

function demoProviderProbe(profile: ProviderProfile): ProviderProbeResponse {
	const report = {
		schema_version: 'x07.studio.provider_probe_report@0.1.0' as const,
		profile_id: profile.id,
		base_url: profile.base_url,
		observed_at: new Date(0).toISOString(),
		ok: true,
		http_status: 200,
		models: [profile.model ?? 'qwen3-coder'],
		capabilities: {
			models_endpoint: 'supported' as const,
			responses: 'supported' as const,
			chat_completions: 'supported' as const,
			tools: 'supported' as const,
			json_schema: 'supported' as const,
			streaming: 'unknown' as const
		},
		notes: ['demo provider deep probe covers /models, /responses, /chat/completions, tools, and JSON schema'],
		raw: null
	};
	return { profile, report };
}

function demoDocPreview(docRef: string): DocPreviewResponse {
	const isDirectory = !docRef.endsWith('.md') && !docRef.endsWith('.json');
	const entries = isDirectory
		? [
				{
					path: `${docRef}/agent-quickstart.md`,
					title: 'agent quickstart',
					kind: 'file' as const
				},
				{
					path: `${docRef}/workflow-graph`,
					title: 'workflow graph',
					kind: 'directory' as const
				}
			]
		: [];
	return {
		schema_version: 'x07.studio.doc_preview@0.1.0',
		doc_ref: docRef,
		resolved_path: `/workspace/${docRef}`,
		title: docRef.split('/').at(-1)?.replaceAll('-', ' ').replace(/\.(md|json)$/u, '') || docRef,
		media_kind: isDirectory ? 'directory' : docRef.endsWith('.json') ? 'json' : 'markdown',
		bytes_read: isDirectory ? 0 : 768,
		truncated: false,
		snippet:
			'Use x07 run as the canonical execution front door. Keep the edit, format, lint, and run loop visible before handoff.',
		entries
	};
}

function projectDemoTurns(session: SessionSnapshot): SessionTurn[] {
	const turns: SessionTurn[] = [];
	if (session.intent) {
		const source = session.intent.source;
		const raw =
			source.kind === 'text' || source.kind === 'spec'
				? source.raw
				: source.kind === 'voice'
					? source.transcript
					: source.path;
		turns.push({
			kind: 'user_intent',
			id: `${session.session_id}-intent`,
			at: session.op_log[0]?.started_at ?? 'demo',
			raw,
			source_kind: source.kind
		});
	}
	for (const turn of session.intent?.clarification_history ?? []) {
		turns.push({
			kind: 'agent_clarify',
			id: `${session.session_id}-${turn.question_id}`,
			at: turn.question_recorded_at,
			agent_id: turn.agent_id,
			questions: [
				{
					id: turn.question_id,
					text: turn.question_text,
					witness_kind: turn.witness_kind,
					options: turn.options,
					answer: turn.answer_text
				}
			]
		});
	}
	const buildOps = session.op_log.filter((op) => op.op.startsWith('build.stage.'));
	if (buildOps.length) {
		turns.push({
			kind: 'build_stage',
			id: `${session.session_id}-build`,
			at: buildOps[0].started_at,
			stage: buildOps.at(-1)?.op.replace('build.stage.', '') ?? 'start',
			op_ids: buildOps.map((op) => op.id)
		});
	}
	const summaryOp = [...session.op_log].reverse().find((op) => op.op === 'summary.plain_english');
	if (summaryOp?.report_json) {
		turns.push({
			kind: 'verified',
			id: `${session.session_id}-verified`,
			at: summaryOp.started_at,
			summary: summaryOp.report_json as PlainEnglishSummary,
			op_ids: [summaryOp.id],
			refined_from_scaffold: false
		});
	}
	return turns;
}

function demoProcessLane(session: SessionSnapshot): ProcessLane {
	const ids = ['intent', 'agent_md', 'clarify', 'spec', 'tests', 'impl', 'verify', 'review'];
	const labels: Record<string, string> = {
		intent: 'Capture intent',
		agent_md: 'Sync AGENT.md',
		clarify: 'Clarify assumptions',
		spec: 'Draft and check spec',
		tests: 'Generate tests',
		impl: 'Write implementation',
		verify: 'Verify behavior',
		review: 'Review implementation'
	};
	const actors: Record<string, ProcessLane['steps'][number]['actor']> = {
		intent: 'conductor',
		agent_md: 'architect',
		clarify: 'architect',
		spec: 'architect',
		tests: 'conductor',
		impl: 'coder',
		verify: 'conductor',
		review: 'reviewer'
	};
	const done = new Set<string>();
	for (const op of session.op_log) {
		if (op.op.startsWith('intent.')) done.add('intent');
		else if (op.op.includes('clarify')) done.add('clarify');
		else if (op.op.startsWith('spec.')) done.add('spec');
		else if (op.op.startsWith('tests.')) done.add('tests');
		else if (op.op.startsWith('impl.') || op.op.startsWith('agent.realize') || op.op === 'synthesis.template') done.add('impl');
		else if (op.op.startsWith('xtal.verify')) done.add('verify');
		else if (op.op === 'review.round') done.add('review');
	}
	const currentIndex = ids.findIndex((id) => !done.has(id));
	return {
		schema_version: 'x07.studio.process_lane@0.1.0',
		session_id: session.session_id,
		current_index: currentIndex >= 0 ? currentIndex : null,
		next_index: currentIndex >= 0 && currentIndex + 1 < ids.length ? currentIndex + 1 : null,
		steps: ids.map((id, index) => ({
			schema_version: 'x07.studio.canonical_step@0.1.0',
			id,
			label: labels[id],
			actor: actors[id],
			status: done.has(id) ? 'done' : index === currentIndex ? 'running' : 'pending',
			started_at: null,
			finished_at: null,
			elapsed_ms: null,
			op_id: session.op_log[index]?.id ?? null,
			narration: `${actors[id]} handles ${id}.`,
			next_actor: null,
			budget: id === 'impl' ? { wall_clock_ms: 60000, prover_seconds: null, on_exhaust: 'pause' } : null,
			round: null
		}))
	};
}

const atlasDemoWorkflowBindings = [
	'pkg.lock.atlas.frontend',
	'wasm.app.profile.validate.atlas_dev',
	'wasm.web_ui.contracts.validate',
	'wasm.http.contracts.validate',
	'wasm.caps.validate.atlas_release',
	'wasm.ops.validate',
	'wasm.slo.validate.atlas',
	'wasm.app.build.atlas_dev',
	'wasm.app.serve.smoke.atlas_dev',
	'wasm.app.test.happy_path',
	'wasm.app.test.validation_error',
	'wasm.app.test.regress.atlas_incident',
	'wasm.app.build.atlas_release',
	'wasm.app.pack.atlas_release',
	'wasm.app.verify.atlas_release',
	'wasm.provenance.attest.atlas_release',
	'wasm.provenance.verify.atlas_release',
	'wasm.deploy.plan.atlas_release',
	'wasm.slo.eval.atlas_canary_ok',
	'lp.deploy.accept.local',
	'lp.deploy.run.local.metrics',
	'lp.deploy.query.local',
	'lp.deploy.status.local'
];

function isAtlasSession(session: SessionSnapshot): boolean {
	const haystack = demoSessionHaystack(session);
	return haystack.includes('atlas.app') || haystack.includes('x07_atlas') || haystack.includes('x07 atlas');
}

function demoProjectScaffoldBinding(session: SessionSnapshot): string {
	const haystack = demoSessionHaystack(session);
	if (haystack.includes('workflow-graph') || haystack.includes('workflow.graph')) return 'project.seed.workflow-graph';
	if (haystack.includes('x07-sm-arch-contracts-smoke') || haystack.includes('workflow.lifecycle')) return 'project.seed.state-machine-arch';
	if (haystack.includes('x07-api-gateway') || haystack.includes('api.gateway')) return 'project.seed.x07-api-gateway';
	if (haystack.includes('x07crawl') || haystack.includes('crawl.')) return 'project.seed.x07crawl';
	if (haystack.includes('x07dbguard') || haystack.includes('dbguard')) return 'project.seed.x07dbguard';
	if (haystack.includes('x07_atlas') || haystack.includes('atlas.app') || haystack.includes('x07 atlas')) return 'project.seed.x07_atlas';
	return 'project.init.xtal-pure';
}

function demoSessionHaystack(session: SessionSnapshot): string {
	const target = session.intent?.targets[0];
	let raw = '';
	if (session.intent?.source.kind === 'text' || session.intent?.source.kind === 'spec') {
		raw = session.intent.source.raw;
	} else if (session.intent?.source.kind === 'voice') {
		raw = session.intent.source.transcript;
	} else if (session.intent?.source.kind === 'incident' || session.intent?.source.kind === 'sketch' || session.intent?.source.kind === 'image') {
		raw = session.intent.source.path;
	}
	return `${target?.module_id ?? ''} ${target?.entry ?? ''} ${raw}`.toLowerCase();
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

function demoVisualParse(kind: VisualKind, source: unknown) {
	if (kind === 'streampipe') {
		const labels = String(source ?? '')
			.split('|')
			.map((item) => item.trim())
			.filter(Boolean);
		return {
			nodes: labels.map((label, index) => ({ id: String(index + 1), label })),
			edges: labels.slice(1).map((_, index) => ({
				from: String(index + 1),
				to: String(index + 2),
				label: 'pipe'
			}))
		};
	}
	return source;
}

function labelsFromGraph(graph: unknown): string[] {
	if (!graph || typeof graph !== 'object' || !('nodes' in graph)) return [];
	const nodes = (graph as { nodes?: unknown }).nodes;
	if (!Array.isArray(nodes)) return [];
	return nodes
		.map((node) => {
			if (!node || typeof node !== 'object' || !('label' in node)) return '';
			return String((node as { label?: unknown }).label ?? '').trim();
		})
		.filter(Boolean);
}

function demoTrustPosture(session: SessionSnapshot): TrustPosture {
	const verified = session.op_log.some((op) => op.op === 'xtal.verify' && op.status === 'succeeded');
	return {
		schema_version: 'x07.studio.trust_posture@0.1.0',
		session_id: session.session_id,
		captured_at: new Date().toISOString(),
		trust_profile: verified ? 'verified_core_pure_v1' : 'local_preview',
		worlds: ['solve-pure'],
		capabilities: [],
		budgets: {
			local_cap_ms: null,
			arch_profile: null,
			prover_seconds_used: verified ? 1 : 0,
			prover_seconds_cap: 30
		},
		proof_coverage: {
			support_pct: verified ? 100 : 0,
			proved_pct: verified ? 87 : 0,
			proof_count: verified ? 1 : 0,
			assumptions_open: session.intent?.ambiguities.length ?? 0
		},
		deltas: verified
			? [{ at: new Date().toISOString(), kind: 'proof-coverage', summary: 'proof coverage computed' }]
			: [],
		posture_color: verified ? 'green' : 'amber'
	};
}

function demoHealthSnapshot(): HealthSnapshot {
	return {
		schema_version: 'x07.studio.health_snapshot@0.1.0',
		captured_at: new Date().toISOString(),
		doctor: { ok: true, blockers: [], warnings: [] },
		lockfile: { ok: true, stale: false, yanked: [], advisories: [] },
		migrate: {
			needs_migration: false,
			from_schema: 'x07.project@0.5.0',
			to_schema: '0.5',
			project_schema_legacy: false
		},
		overall_color: 'green'
	};
}

function demoAgentContract(session: SessionSnapshot): AgentContract {
	const markdown = `# AGENT.md

## Purpose
${session.title || 'Demo x07 Studio project'}

## Invariants
- Keep deterministic logic in solve-pure unless a reviewed profile allows OS access.

## Forbidden changes
- Do not widen specs, architecture, worlds, capabilities, or budgets without review.
`;
	return {
		schema_version: 'x07.studio.agent_contract@0.1.0',
		session_id: session.session_id,
		path: 'AGENT.md',
		exists: false,
		markdown,
		sections: [
			{ title: 'Purpose', body: session.title || 'Demo x07 Studio project' },
			{ title: 'Invariants', body: '- Keep deterministic logic in solve-pure unless a reviewed profile allows OS access.' },
			{ title: 'Forbidden changes', body: '- Do not widen specs, architecture, worlds, capabilities, or budgets without review.' }
		],
		last_modified: null,
		hash: String(markdown.length)
	};
}

function demoLintReport(session: SessionSnapshot): LintReport {
	return {
		schema_version: 'x07.studio.lint_report@0.1.0',
		session_id: session.session_id,
		generated_at: new Date().toISOString(),
		diagnostics: [
			{
				id: 'X07-LINT-0042',
				severity: 'warning',
				file: 'src/main.x07.json',
				line: 1,
				column: 1,
				summary: 'Demo lint diagnostic with a deterministic quickfix.',
				fixable: true
			}
		],
		raw: { demo: true }
	};
}

function demoPbtRound(session: SessionSnapshot): PbtRound {
	return {
		schema_version: 'x07.studio.pbt_round@0.1.0',
		session_id: session.session_id,
		started_at: new Date().toISOString(),
		finished_at: new Date().toISOString(),
		properties_run: 47,
		counterexamples: [],
		raw: { demo: true }
	};
}

function demoQuickfixRecord(id: string): QuickfixRecord {
	return {
		schema_version: 'x07.studio.quickfix_record@0.1.0',
		diagnostic_code: id,
		severity: 'warning',
		summary: `Deterministic quickfix for ${id}`,
		patch_ast: {
			schema_version: 'x07.patchset@0.1.0',
			patches: [{ path: 'src/main.x07.json', patch: [{ op: 'add', path: '/metadata', value: { fixed: true } }] }]
		},
		citations: [{ kind: 'lint', file: 'src/main.x07.json', region: '1:1' }],
		before_snippet: '{\n  "kind": "module"\n}',
		after_snippet: '{\n  "kind": "module",\n  "metadata": { "fixed": true }\n}'
	};
}

function demoPkgProvides(moduleId: string): PkgProvidesResult {
	return {
		schema_version: 'x07.studio.pkg_provides_result@0.1.0',
		module_id: moduleId,
		candidates: [
			{
				package: 'ext-text',
				version: '0.5.0',
				source: 'registry',
				install_command: 'x07 pkg add ext-text@0.5.0'
			}
		]
	};
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
		throw new HttpRequestError(response.status, await response.text());
	}
	return response.json() as Promise<T>;
}

class HttpRequestError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message || `HTTP ${status}`);
		this.name = 'HttpRequestError';
	}
}
