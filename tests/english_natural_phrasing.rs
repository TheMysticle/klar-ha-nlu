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

// --- 4. levels people actually name --------------------------------------

/// "brighten" was rejected outright while "brighter" worked, and "half" and
/// "max" fell through to a state query -- so the light was described rather
/// than set, and nothing looked wrong.
#[test]
fn ordinary_words_for_a_light_level_set_the_level() {
    for (text, slot, value) in [
        ("brighten the living room lights", "brightness_step", "up"),
        ("make the living room lights brighter", "brightness_step", "up"),
        ("dim the living room lights", "brightness_step", "down"),
        ("make the living room lights dimmer", "brightness_step", "down"),
        ("living room lights to half", "brightness", "50"),
        ("max brightness in the living room", "brightness", "100"),
        ("set the living room lights to full brightness", "brightness", "100"),
        ("living room lights to 50 percent", "brightness", "50"),
    ] {
        let plan = plan(text);
        let intent = &plan.steps[0].intent;
        assert_eq!(intent.name, "HassLightSet", "{text:?}");
        assert_eq!(intent.slot(slot), Some(value), "{text:?}");
    }
}

// --- 5. colours ----------------------------------------------------------

/// Only CSS colour names are added, because the slot reaches Home Assistant
/// as a colour NAME and a word it does not know fails the service call. So
/// this asserts the canonical value too, not merely that something parsed.
#[test]
fn common_colours_are_understood() {
    for (text, colour) in [
        ("set the living room lights to cyan", "cyan"),
        ("turn the living room lights teal", "teal"),
        ("make the living room lights magenta", "magenta"),
        ("make the living room purple", "purple"),
        ("make the living room warm white", "warmwhite"),
    ] {
        let plan = plan(text);
        let intent = &plan.steps[0].intent;
        assert_eq!(intent.name, "HassLightSet", "{text:?}");
        assert_eq!(intent.slot("color"), Some(colour), "{text:?}");
    }
}

// --- 6. a thermostat is not a light --------------------------------------

/// "set the bedroom thermostat to 21" produced HassLightSet with
/// brightness=21: the word `thermostat` was missing from climate_nouns, so
/// the number landed on the lights. Asking for 21 degrees and getting a dim
/// bedroom is the kind of wrong that is hard to attribute to a missing noun.
///
/// Asserted as "must not be a light change" rather than "must be
/// HassClimateSetTemperature", because a climate change asks for
/// confirmation first and a Confirm deliberately carries no plan and no
/// candidates -- only an id for the answer. The regression is exactly what
/// is checked here: before, this EXECUTED a light change.
#[test]
fn a_thermostat_is_not_a_light() {
    let home = home();
    let mut session = Session::new();
    session.preferred_area = Some("wohnzimmer".to_owned());
    let outcome = nlu::parse("set the bedroom thermostat to 21", &home, &mut session, &[], &Settings::pinned("en"));
    if let Some(plan) = outcome.plan {
        for step in &plan.steps {
            assert_ne!(step.intent.name, "HassLightSet", "a thermostat request became a light change: {plan:#?}");
            assert_eq!(step.intent.slot("brightness"), None, "{plan:#?}");
        }
    }
    for candidate in &outcome.candidates {
        for step in &candidate.plan.steps {
            assert_ne!(step.intent.name, "HassLightSet", "a thermostat candidate is a light change: {candidate:#?}");
        }
    }
}

// --- 7. "one" is usually a pronoun ---------------------------------------

/// English spells the number and the pronoun the same, and in speech to an
/// assistant the pronoun is commoner. Read as a number, "not that one" -- a
/// CORRECTION -- set the light to brightness 1, and "the other one" did too.
/// Neither should set a level.
#[test]
fn that_one_is_not_the_number_one() {
    let home = home();
    for text in ["not that one", "the other one", "no I meant the kitchen"] {
        let mut session = Session::new();
        session.preferred_area = Some("wohnzimmer".to_owned());
        let outcome = nlu::parse(text, &home, &mut session, &[], &Settings::pinned("en"));
        if let Some(plan) = outcome.plan {
            for step in &plan.steps {
                assert_ne!(step.intent.slot("brightness"), Some("1"), "{text:?} set a brightness: {plan:#?}");
            }
        }
    }
}

/// The number still works where it IS a number, so the rule above cannot
/// quietly swallow it.
#[test]
fn one_is_still_a_number_when_it_is_one() {
    let plan = plan("set the living room lights to one percent");
    assert_eq!(plan.steps[0].intent.slot("brightness"), Some("1"), "{plan:#?}");
}

// --- 8. a bare "stop" stops what is making noise -------------------------

/// Tags the living room player "music", which is what a labelled Spotify
/// player looks like to Klar -- `home::roles::is_music_player` reads tags.
/// The shipped fixture has none, so without this no player qualifies and
/// every media sentence is rejected for want of a target, which says nothing
/// about the phrasing.
fn home_with_music_player() -> HomeGraph {
    let mut home = home();
    for entity in home.entities.iter_mut() {
        if entity.entity_id == "media_player.lg_dsn9yg_8909" {
            entity.tags.push("music".to_string());
        }
    }
    home
}

fn media_plan(home: &HomeGraph, text: &str) -> IntentPlan {
    let mut session = Session::new();
    session.preferred_area = Some("wohnzimmer".to_owned());
    let outcome = nlu::parse(text, home, &mut session, &[], &Settings::pinned("en"));
    assert!(matches!(&outcome.decision, ParseDecision::Execute), "{text:?} did not execute: {:?}", outcome.decision);
    outcome.plan.expect("a plan")
}

/// A BARE "stop" MEANS THE THING MAKING NOISE. It is how people halt music,
/// and it is what Home Assistant's own agent does. Read as Off it had no
/// target at all and came back "I did not catch that" -- blaming the speaker
/// for the shortest clear sentence there is.
///
/// Klar cannot check what is playing: the registry comes from
/// /config/.storage, which records what EXISTS and never its state. So the
/// intent is aimed at the room's player and Home Assistant decides, pausing
/// one that plays and ignoring one that does not -- the same check its own
/// agent makes, in process.
#[test]
fn a_bare_stop_pauses_the_music() {
    let home = home_with_music_player();
    for text in ["stop", "stop it", "pause", "stop playing", "stop the music", "pause the music"] {
        let plan = media_plan(&home, text);
        assert_eq!(plan.steps.len(), 1, "{text:?}: {plan:#?}");
        assert_eq!(plan.steps[0].intent.name, "HassMediaPause", "{text:?}");
        assert_eq!(plan.steps[0].intent.slot("entity_id"), Some("media_player.lg_dsn9yg_8909"), "{text:?}");
    }
}

/// Naming something else must still win, so the bare-stop rule cannot
/// swallow every stop in the language.
#[test]
fn a_stop_that_names_its_target_is_not_about_music() {
    let home = home_with_music_player();
    assert_eq!(media_plan(&home, "stop the timer").steps[0].intent.name, "HassCancelTimer");
    assert_eq!(media_plan(&home, "turn off the living room lights").steps[0].intent.name, "HassTurnOff");
}

/// "stop the vacuum" STARTED the robot: the vacuum noun plus a start cue
/// outranked the stop. Docking is the closest action available -- there is
/// none for vacuum.stop -- and unambiguously better than starting something
/// somebody just asked to stop.
#[test]
fn stopping_the_vacuum_does_not_start_it() {
    let plan = plan("stop the vacuum");
    for step in &plan.steps {
        assert_ne!(step.intent.name, "HassVacuumStart", "asking it to stop started it: {plan:#?}");
    }
    assert!(plan.steps.iter().any(|step| step.intent.name == "HassVacuumReturnToBase"), "{plan:#?}");
}

// --- 9. "all the lights" means all of them -------------------------------

/// Reported from a live house: "turn on kitchen lights" then "turn off all
/// the lights" switched off the KITCHEN only, and announced it as success.
/// The second sentence named a scope and the session narrowed it back to the
/// first one's target.
///
/// Only reachable without an all-lights group entity, which is why it does
/// not show on the shipped fixture -- there the group matches by name and the
/// session never gets a say.
#[test]
fn all_the_lights_is_not_narrowed_by_the_last_room() {
    let mut home = home();
    home.entities.retain(|entity| !entity.entity_id.contains("alle_lichter"));

    let mut session = Session::new();
    session.preferred_area = Some("schlafzimmer".to_owned());
    let first = nlu::parse("turn on kitchen lights", &home, &mut session, &[], &Settings::pinned("en"));
    assert!(matches!(first.decision, ParseDecision::Execute), "{:?}", first.decision);

    // Same session, so the kitchen is still remembered.
    let outcome = nlu::parse("turn off all the lights", &home, &mut session, &[], &Settings::pinned("en"));
    let plan = outcome.plan.expect("a plan for all the lights");
    assert!(plan.steps.len() > 1, "all the lights hit one target only: {plan:#?}");
    for step in &plan.steps {
        assert_eq!(step.intent.name, "HassTurnOff", "{plan:#?}");
    }

    // And identical to a session that never heard about the kitchen.
    let mut fresh = Session::new();
    fresh.preferred_area = Some("schlafzimmer".to_owned());
    let clean = nlu::parse("turn off all the lights", &home, &mut fresh, &[], &Settings::pinned("en"));
    let clean_plan = clean.plan.expect("a plan");
    assert_eq!(plan.steps.len(), clean_plan.steps.len(), "the session changed the scope of \"all\"");
}

/// A home that HAS a group still uses it, one intent instead of a fan-out.
#[test]
fn an_all_lights_group_is_still_preferred() {
    let plan = plan("turn off all the lights");
    assert_eq!(plan.steps.len(), 1, "{plan:#?}");
    assert_eq!(plan.steps[0].intent.slot("entity_id"), Some("light.alle_lichter"));
}
