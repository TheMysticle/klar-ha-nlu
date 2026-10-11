use crate::lang::{catalog, VerbKind};

use super::Action;

pub(super) fn timer_kind(tokens: &[String]) -> Action {
    let cat = catalog();
    if timer_question(tokens) {
        // ASKING ABOUT A TIMER IS NOT SETTING ONE. Everything else here acts,
        // and with no branch for a question "how long is left on my timer"
        // fell through to the else below and STARTED a timer, or resolved to
        // a state query against some unrelated `timer.` entity.
        //
        // HassTimerStatus is the intent Home Assistant answers from its own
        // assist timers -- which is where a timer set through here lives,
        // since HassStartTimer creates one of those rather than a `timer.`
        // entity. Klar cannot read the remaining time (the registry says what
        // EXISTS, never its state), so it names the intent and HA fills in the
        // answer, the same division of labour as a bare "stop".
        Action::TimerStatus
    } else if cat.any(tokens, cat.timer_pause()) {
        Action::TimerPause
    } else if cat.any(tokens, cat.playback_resume()) {
        Action::TimerStart
    } else if cat.any(tokens, cat.timer_add()) {
        Action::TimerAdd
    } else if timer_decrease(tokens) {
        Action::TimerRemove
    } else if cat.any(tokens, cat.timer_cancel()) || tokens.iter().any(|token| matches!(token.as_str(), "remove" | "removed")) {
        Action::TimerCancel
    } else {
        Action::TimerStart
    }
}

/// Whether the sentence ASKS about a timer rather than changing one.
///
/// Deliberately narrow: a question word or a remaining-time cue, and no
/// number. "set a timer for 5 minutes" has a number and is not a question;
/// "add 5 minutes to the timer" is caught by timer_add before this is asked.
fn timer_question(tokens: &[String]) -> bool {
    let cat = catalog();
    if crate::parse::numbers::first_number(tokens).is_some() {
        return false;
    }
    if cat.any(tokens, cat.timer_cancel()) || cat.any(tokens, cat.timer_pause()) || cat.any(tokens, cat.timer_add()) {
        return false;
    }
    cat.any(tokens, cat.question_words())
        || tokens.first().is_some_and(|token| cat.is_question_start(token))
        || cat.any(tokens, cat.timer_query())
        || cat.any(tokens, cat.status_words())
}

pub(super) fn timer_decrease(tokens: &[String]) -> bool {
    let cat = catalog();
    if cat.any(tokens, cat.timer_add()) {
        return false;
    }
    let numbered = crate::parse::numbers::first_number(tokens).is_some();
    tokens.iter().any(|token| match token.as_str() {
        "minus" => true,
        "remove" | "removed" => numbered,
        _ => {
            cat.timer_remove().contains(token.as_str())
                || matches!(cat.verb(token), Some(VerbKind::Lower))
                || (numbered && matches!(cat.verb(token), Some(VerbKind::Down | VerbKind::Dim)))
        }
    })
}
