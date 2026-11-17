use agentpet_pet_engine::{EngineContext, PetState, SessionReaction, Stimulus, ALL_STATES};
use proptest::prelude::*;

/// Strategy generating any of the 15 PetState variants.
pub fn any_pet_state() -> impl Strategy<Value = PetState> {
    prop_oneof![
        Just(PetState::Idle),
        Just(PetState::Working),
        Just(PetState::Thinking),
        Just(PetState::Waiting),
        Just(PetState::Success),
        Just(PetState::Error),
        Just(PetState::Celebrating),
        Just(PetState::Sleeping),
        Just(PetState::Hungry),
        Just(PetState::Eating),
        Just(PetState::LevelUp),
        Just(PetState::Achievement),
        Just(PetState::Offline),
        Just(PetState::Paused),
        Just(PetState::AttentionRequired),
    ]
}

/// Strategy generating any Stimulus.
pub fn any_stimulus() -> impl Strategy<Value = Stimulus> {
    prop_oneof![
        Just(Stimulus::AgentStarted),
        Just(Stimulus::AgentWorking),
        Just(Stimulus::AgentThinking),
        Just(Stimulus::AgentWaiting),
        Just(Stimulus::AgentInputRequired),
        Just(Stimulus::AgentCompleted),
        Just(Stimulus::AgentFailed),
        (any::<bool>(), any::<bool>()).prop_map(|(leveled_up, achievement_unlocked)| {
            Stimulus::SessionFinished(SessionReaction::Succeeded {
                leveled_up,
                achievement_unlocked,
            })
        }),
        Just(Stimulus::SessionFinished(SessionReaction::Failed)),
        Just(Stimulus::UserPaused),
        Just(Stimulus::UserResumed),
        Just(Stimulus::PetInteracted),
        Just(Stimulus::PetFed),
        Just(Stimulus::AgentDisconnected),
        Just(Stimulus::AgentReconnected),
    ]
}

/// Strategy generating random EngineContext.
pub fn any_context() -> impl Strategy<Value = EngineContext> {
    any::<bool>().prop_map(|has_active_sessions| EngineContext {
        has_active_sessions,
    })
}

/// Helper returning slice of all 15 states.
pub fn all_states() -> &'static [PetState; 15] {
    &ALL_STATES
}
