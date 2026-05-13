use loom_types::api::StudioMemory;
use loom_types::session::SessionSnapshot;

#[derive(Debug, Clone, Default)]
pub struct AppliedPreferences {
    pub default_agent: Option<String>,
    pub default_trust_profile: Option<String>,
    pub naming_style: Option<String>,
    pub verbosity: Option<String>,
}

impl AppliedPreferences {
    pub fn is_empty(&self) -> bool {
        self.default_agent.is_none()
            && self.default_trust_profile.is_none()
            && self.naming_style.is_none()
            && self.verbosity.is_none()
    }
}

pub fn apply_preferences(
    memory: &StudioMemory,
    session: &mut SessionSnapshot,
) -> AppliedPreferences {
    let mut applied = AppliedPreferences::default();
    if let Some(agent) = clean(&memory.preferences.default_agent) {
        applied.default_agent = Some(agent);
    }
    if let Some(profile) = clean(&memory.preferences.default_trust_profile) {
        applied.default_trust_profile = Some(profile);
    }
    if let Some(style) = clean(&memory.preferences.naming_style) {
        applied.naming_style = Some(style.clone());
        if style == "snake_case" {
            session.title = session.title.replace([' ', '-'], "_").to_ascii_lowercase();
        } else if style == "camelCase" {
            session.title = camel_case(&session.title);
        }
    }
    if let Some(verbosity) = clean(&memory.preferences.verbosity) {
        applied.verbosity = Some(verbosity);
    }
    applied
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn camel_case(input: &str) -> String {
    let mut out = String::new();
    for (index, part) in input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .enumerate()
    {
        if index == 0 {
            out.push_str(&part.to_ascii_lowercase());
        } else {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                out.push(first.to_ascii_uppercase());
                out.push_str(&chars.as_str().to_ascii_lowercase());
            }
        }
    }
    if out.is_empty() {
        input.to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::apply_preferences;
    use loom_types::api::{MemoryPreferences, StudioMemory};
    use loom_types::artifacts::TaskType;
    use loom_types::session::SessionSnapshot;

    #[test]
    fn applies_naming_style_to_new_session() {
        let memory = StudioMemory {
            preferences: MemoryPreferences {
                naming_style: Some("snake_case".to_string()),
                default_agent: Some("openai-codex".to_string()),
                default_trust_profile: None,
                verbosity: None,
            },
            role_preferences: None,
            recent_projects: Vec::new(),
            reusable_specs: Vec::new(),
        };
        let mut session = SessionSnapshot::new(
            Uuid::new_v4(),
            "Email Sorter",
            "/tmp/demo",
            TaskType::NewBehavior,
        );

        let applied = apply_preferences(&memory, &mut session);

        assert_eq!(session.title, "email_sorter");
        assert_eq!(applied.default_agent.as_deref(), Some("openai-codex"));
    }
}
