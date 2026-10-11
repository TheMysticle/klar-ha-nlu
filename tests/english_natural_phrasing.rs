//! How English speakers actually phrase things, as opposed to how German does.
//!
//! The English pack began as the German one with English words put in, and it
//! shows in a specific way: where German carries meaning in a separable
//! prefix, English carries it in a PARTICLE or in a verb that German has no
//! equivalent for. `turn on` works because `turn` is a filler and `on` is the
//! particle doing the work -- but `turn out`, `put out` and `lights out` had
//! no particle at all, so the whole sentence read as a question and the lights
//! stayed on.
//!
//! Three groups, each a thing a person said that did not work:
//!
//! 1. Switching off. English has many ways and only some were understood.
//! 2. Asking vs commanding. "did I leave the lights on" TURNED THEM ON --
//!    a question that acts is worse than a question that fails.
//! 3. Sending the vacuum somewhere. An area next to a vacuum names the job,
//!    not where the robot is parked.
//!
//! These are phrasing tests, so they assert the INTENT and the target, never
//! the wording of the reply.

use klar_nlu::nlu;
use klar_nlu::session::Session;
use klar_nlu::types::{HomeGraph, IntentPlan, ParseDecision, Settings};

fn home() -> HomeGraph {
    serde_json::from_str(include_str!("fixtures/wohnung_live.json")).expect("wohnung_live fixture")
}

fn plan(text: &str) -> IntentPlan {
    let home = home();
    let mut session = Session::new();
    session.preferred_area = Some("wohnzimmer".to_owned());
    let outcome = nlu::parse(text, &home, &mut session, &[], &Settings::pinned("en"));
    assert!(matches!(&outcome.decision, ParseDecision::Execute), "{text:?} did not execute: {:?}", outcome.decision);
    outcome.plan.expect("an execute decision must carry a plan")
}

fn only_intent(text: &str) -> (String, Option<String>, Option<String>) {
    let plan = plan(text);
    assert_eq!(plan.steps.len(), 1, "{text:?} produced {} steps: {plan:#?}", plan.steps.len());
    let intent = &plan.steps[0].intent;
    (intent.name.clone(), intent.slot("entity_id").map(str::to_owned), intent.slot("area").map(str::to_owned))
}

// --- 1. the many ways to switch a light off ------------------------------

/// "switch off ..." is deliberately absent: `group_clarify.trigger` is set to
/// `["switch"]`, so that word asks WHICH light instead of acting. That is an
/// existing design choice rather than a gap, and worth a second look for
/// English -- "switch off the lights" is among the most ordinary phrasings
/// there is -- but it is not this test's business to change it.
///
/// Each of these is one person's ordinary way of saying it. "turn out", "put
/// out", "cut" and the trailing "lights out" produced HassGetState before --
/// the lights were REPORTED rather than switched, so nothing visibly
/// happened and the sentence looked understood.
#[test]
fn every_ordinary_way_of_saying_off_switches_off() {
    for text in [
        "turn off the living room lights",
        "shut off the living room lights",
        "turn out the living room lights",
        "put out the living room lights",
        "cut the living room lights",
        "kill the living room lights",
        "shut down the living room lights",
        "power off the living room lights",
        "lights out in the living room",
    ] {
        let (name, _, _) = only_intent(text);
        assert_eq!(name, "HassTurnOff", "{text:?}");
    }
}

/// The counterpart, so a fix to the above cannot quietly invert anything.
#[test]
fn every_ordinary_way_of_saying_on_switches_on() {
    for text in [
        "turn on the living room lights",
        "put the living room lights on",
        "lights on in the living room",
        "fire up the living room lights",
        "light up the living room",
        "flip on the living room lights",
        "power on the living room lights",
    ] {
        let (name, _, _) = only_intent(text);
        assert_eq!(name, "HassTurnOn", "{text:?}");
    }
}

// --- 2. a question must never act ----------------------------------------

/// THE IMPORTANT ONE. "did I leave the living room lights on" resolved to
/// HassTurnOn: asking whether you had left a light on switched it on. The
/// person then believes they left it on, because now it is.
#[test]
fn asking_whether_a_light_is_on_never_turns_it_on() {
    for text in ["did I leave the living room lights on", "any lights on", "are the living room lights on"] {
        let (name, _, _) = only_intent(text);
        assert_eq!(name, "HassGetState", "{text:?} ACTED on a question");
    }
}

/// And the same words as a command still command, so the query words above
/// are not simply swallowing everything.
#[test]
fn a_command_is_still_a_command() {
    let (name, _, _) = only_intent("turn the living room lights on");
    assert_eq!(name, "HassTurnOn");
}

// --- 3. sending the vacuum somewhere -------------------------------------

/// An area next to a vacuum action names the JOB, not where the robot lives.
///
/// Every one of these was refused with "I did not catch that" unless the area
/// named happened to be where the dock is -- "vacuum the living room" worked
/// and "vacuum the kitchen" did not, which is indistinguishable from the
/// sentence being misunderstood.
///
/// The target must be the ROBOT and not the area: a HassVacuumStart carrying
/// an area Home Assistant has no vacuum in matches nothing there and silently
/// does not run, which is how this would look fixed while still doing nothing.
#[test]
fn a_vacuum_is_sent_to_a_room_it_is_not_parked_in() {
    for text in [
        "vacuum the kitchen",
        "hoover the kitchen",
        "clean the kitchen",
        "send the vacuum to the kitchen",
        "start cleaning the kitchen",
        "vacuum the living room",
    ] {
        let (name, entity, area) = only_intent(text);
        assert_eq!(name, "HassVacuumStart", "{text:?}");
        assert_eq!(entity.as_deref(), Some("vacuum.r2d2"), "{text:?} must target the robot");
        assert_eq!(area, None, "{text:?} must not pass an area Home Assistant cannot match");
    }
}

/// British English. The repo had no word for it at all.
#[test]
fn hoover_is_a_word() {
    let (name, entity, _) = only_intent("hoover the living room");
    assert_eq!(name, "HassVacuumStart");
    assert_eq!(entity.as_deref(), Some("vacuum.r2d2"));
}

/// Docking still docks -- the area rule must not swallow the other vacuum
/// action.
#[test]
fn sending_the_vacuum_to_its_dock_still_docks() {
    let plan = plan("send the vacuum back to its dock");
    assert!(plan.steps.iter().any(|step| step.intent.name == "HassVacuumReturnToBase"), "{plan:#?}");
}
