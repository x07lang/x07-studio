use loom_types::api::{
    AgentRole, PipelineStage, RoleOverrides, RolePipeline, RolePreferences, StepBudget,
};
use loom_types::artifacts::{AgentProfile, AgentStatus};

const PIPELINE_SCHEMA: &str = "x07.studio.role_pipeline@0.1.0";

pub fn default_routing(preferences: Option<&RolePreferences>) -> RolePipeline {
    let max_review_rounds = preferences
        .map(|prefs| prefs.default_max_review_rounds.clamp(1, 5))
        .unwrap_or(2);
    RolePipeline {
        schema_version: PIPELINE_SCHEMA.to_string(),
        stages: vec![
            PipelineStage {
                role: AgentRole::Architect,
                action: "confirm_spec".to_string(),
                // Sized for Tier-2 architect-agent enrichment (a real
                // claude subscription invocation). When the deterministic
                // floor already filled the spec, the stage just appends a
                // log and returns instantly — the budget is a ceiling,
                // not a target.
                budget: Some(StepBudget {
                    wall_clock_ms: Some(90_000),
                    prover_seconds: None,
                    on_exhaust: "pause".to_string(),
                }),
            },
            PipelineStage {
                role: AgentRole::Coder,
                action: "write_impl".to_string(),
                budget: Some(StepBudget {
                    wall_clock_ms: Some(60_000),
                    prover_seconds: None,
                    on_exhaust: "pause".to_string(),
                }),
            },
            PipelineStage {
                role: AgentRole::Reviewer,
                action: "review_impl".to_string(),
                budget: Some(StepBudget {
                    wall_clock_ms: Some(30_000),
                    prover_seconds: None,
                    on_exhaust: "pause".to_string(),
                }),
            },
        ],
        max_review_rounds,
    }
}

pub fn resolve_actor(
    role: AgentRole,
    agents: &[AgentProfile],
    overrides: &RoleOverrides,
    preferences: Option<&RolePreferences>,
) -> Option<String> {
    overrides
        .get(&role)
        .map(str::to_string)
        .or_else(|| preference_for(role, preferences))
        .filter(|id| agent_can_fill(agents, id, role))
        .or_else(|| {
            agents
                .iter()
                .find(|agent| agent_can_fill(agents, &agent.id, role))
                .map(|agent| agent.id.clone())
        })
}

pub fn select_reviewer(
    writer_id: Option<&str>,
    agents: &[AgentProfile],
    overrides: &RoleOverrides,
    preferences: Option<&RolePreferences>,
) -> Option<String> {
    let allow_self = preferences
        .map(|prefs| prefs.allow_self_review)
        .unwrap_or(true);
    if let Some(reviewer) = resolve_actor(AgentRole::Reviewer, agents, overrides, preferences) {
        if allow_self || Some(reviewer.as_str()) != writer_id {
            return Some(reviewer);
        }
    }
    agents
        .iter()
        .filter(|agent| agent.status != AgentStatus::Disabled)
        .find(|agent| {
            Some(agent.id.as_str()) != writer_id
                && (agent.default_role == AgentRole::Reviewer
                    || agent.eligible_roles.contains(&AgentRole::Reviewer))
        })
        .map(|agent| agent.id.clone())
        .or_else(|| writer_id.filter(|_| allow_self).map(str::to_string))
}

fn preference_for(role: AgentRole, preferences: Option<&RolePreferences>) -> Option<String> {
    let prefs = preferences?;
    match role {
        AgentRole::Architect => prefs.default_architect.clone(),
        AgentRole::Coder => prefs.default_coder.clone(),
        AgentRole::Reviewer => prefs.default_reviewer.clone(),
        AgentRole::Conductor => None,
    }
}

fn agent_can_fill(agents: &[AgentProfile], id: &str, role: AgentRole) -> bool {
    agents.iter().any(|agent| {
        agent.id == id
            && agent.status != AgentStatus::Disabled
            && (agent.default_role == role || agent.eligible_roles.contains(&role))
    })
}

#[cfg(test)]
mod tests {
    use super::{default_routing, resolve_actor, select_reviewer};
    use loom_types::api::{AgentRole, RoleOverrides, RolePreferences};
    use loom_types::artifacts::AgentProfile;

    #[test]
    fn default_routing_uses_review_round_preference() {
        let prefs = RolePreferences {
            default_max_review_rounds: 3,
            ..RolePreferences::default()
        };

        let pipeline = default_routing(Some(&prefs));

        assert_eq!(pipeline.max_review_rounds, 3);
        assert_eq!(pipeline.stages[1].role, AgentRole::Coder);
    }

    #[test]
    fn override_resolution_wins_before_preferences() {
        let agents = vec![AgentProfile::codex(), AgentProfile::claude_code()];
        let overrides = RoleOverrides {
            coder: Some("claude-code".to_string()),
            ..RoleOverrides::empty()
        };

        let actor = resolve_actor(
            AgentRole::Coder,
            &agents,
            &overrides,
            Some(&RolePreferences::default()),
        );

        assert_eq!(actor.as_deref(), Some("claude-code"));
    }

    #[test]
    fn reviewer_prefers_other_agent_but_can_self_review() {
        let agents = vec![AgentProfile::codex()];
        let reviewer = select_reviewer(
            Some("openai-codex"),
            &agents,
            &RoleOverrides::empty(),
            Some(&RolePreferences::default()),
        );

        assert_eq!(reviewer.as_deref(), Some("openai-codex"));
    }
}
