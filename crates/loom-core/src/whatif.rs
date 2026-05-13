use loom_types::api::WhatIfForecast;
use loom_types::session::SessionSnapshot;

pub fn forecast(_session: &SessionSnapshot, step_id: &str) -> WhatIfForecast {
    let (duration, confidence, assumption) = match step_id {
        "verify" => (
            1_800,
            0.8,
            "Runs x07 xtal verify in the current default world.",
        ),
        "review" => (
            30_000,
            0.72,
            "Checks latest spec, implementation, verify, and trust evidence.",
        ),
        "impl" => (
            60_000,
            0.68,
            "Uses the configured coder role and current write boundaries.",
        ),
        "repair" => (
            45_000,
            0.62,
            "Attempts a spec-preserving repair before any spec-changing lane.",
        ),
        _ => (
            10_000,
            0.7,
            "Forecast is based on the canonical step and recent session state.",
        ),
    };
    WhatIfForecast {
        schema_version: "x07.studio.what_if_forecast@0.1.0".to_string(),
        step_id: step_id.to_string(),
        predicted_delta: None,
        estimated_duration_ms: duration,
        confidence,
        assumptions: vec![assumption.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::forecast;
    use loom_types::artifacts::TaskType;
    use loom_types::session::SessionSnapshot;
    use uuid::Uuid;

    #[test]
    fn verify_forecast_has_duration_and_assumption() {
        let session =
            SessionSnapshot::new(Uuid::new_v4(), "demo", "/tmp/demo", TaskType::NewBehavior);

        let forecast = forecast(&session, "verify");

        assert_eq!(forecast.step_id, "verify");
        assert!(forecast.estimated_duration_ms > 0);
        assert!(!forecast.assumptions.is_empty());
    }
}
