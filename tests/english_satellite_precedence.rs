//! Regression tests for English room names and satellite-area fallback in the v2 NLU.
use klar_nlu::nlu;
use klar_nlu::session::Session;
use klar_nlu::types::{HomeGraph, IntentPlan, ParseDecision, Settings};

fn home() -> HomeGraph {
    serde_json::from_str(include_str!("fixtures/wohnung_live.json")).expect("wohnung_live fixture")
}

fn settings() -> Settings {
    Settings::pinned("en")
}

fn executed_plan(text: &str, preferred_area: Option<&str>) -> IntentPlan {
    let home = home();
    let mut session = Session::new();
    session.preferred_area = preferred_area.map(str::to_owned);
    let outcome = nlu::parse(text, &home, &mut session, &[], &settings());
    assert!(matches!(&outcome.decision, ParseDecision::Execute), "{text:?}: {outcome:#?}");
    outcome.plan.expect("execute decision must have a plan")
}

fn target_area(intent: &klar_nlu::types::Intent) -> Option<String> {
    if let Some(area) = intent.slot("area") {
        return Some(area.to_owned());
    }
    let entity_id = intent.slot("entity_id")?;
    home().entities.into_iter().find(|entity| entity.entity_id == entity_id).and_then(|entity| entity.area)
}

#[test]
fn explicit_living_room_beats_bedroom_satellite() {
    let plan = executed_plan("turn on living room lights", Some("schlafzimmer"));
    assert!(!plan.steps.is_empty(), "{plan:#?}");
    for step in &plan.steps {
        assert_eq!(step.intent.name, "HassTurnOn", "{plan:#?}");
        assert_eq!(target_area(&step.intent).as_deref(), Some("wohnzimmer"), "{plan:#?}");
    }
}

#[test]
fn explicit_living_room_resolves_without_satellite_hint() {
    let plan = executed_plan("turn on living room lights", None);
    assert!(!plan.steps.is_empty(), "{plan:#?}");
    for step in &plan.steps {
        assert_eq!(step.intent.name, "HassTurnOn", "{plan:#?}");
        assert_eq!(target_area(&step.intent).as_deref(), Some("wohnzimmer"), "{plan:#?}");
    }
}

#[test]
fn brightness_command_uses_explicit_kitchen_not_satellite_room() {
    let plan = executed_plan("dim the kitchen to 50", Some("schlafzimmer"));
    assert!(!plan.steps.is_empty(), "{plan:#?}");
    for step in &plan.steps {
        assert_eq!(step.intent.name, "HassLightSet", "{plan:#?}");
        assert_eq!(step.intent.slot("brightness"), Some("50"), "{plan:#?}");
        assert_eq!(target_area(&step.intent).as_deref(), Some("kuche"), "{plan:#?}");
    }
}
