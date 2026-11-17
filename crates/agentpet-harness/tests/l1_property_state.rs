//! L1 Property Tests: State Machine Invariant & Terminal State Absorption
//!
//! Requirement: MYR-HARN-002
//! The 15-state deterministic state machine must never enter an illegal state.
//! Every transition must satisfy the allow-list invariants. Non-interruptible
//! states must not yield to lower-priority stimuli, and Paused state must strictly
//! resist normal agent activity until explicitly resumed or critically interrupted.

use agentpet_harness::generators::pet_state::{any_context, any_pet_state, any_stimulus};
use agentpet_pet_engine::{
    allows, EngineContext, PetEngine, PetState, Priority, Stimulus, Transition,
};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_state_transitions_never_violate_allow_list(
        current in any_pet_state(),
        stimulus in any_stimulus(),
        context in any_context(),
    ) {
        verify_transition_invariants(current, stimulus, context, false);
    }
}

fn verify_transition_invariants(
    current: PetState,
    stimulus: Stimulus,
    context: EngineContext,
    mutate_allow_illegal: bool,
) {
    let engine = PetEngine::default();
    let transition = if mutate_allow_illegal {
        // Deliberate mutation: force transition to LevelUp from Offline (strictly forbidden)
        Transition::Enter(PetState::LevelUp)
    } else {
        engine.decide(current, stimulus, context)
    };

    match transition {
        Transition::Enter(target) => {
            // Invariant 1: Target MUST be explicitly allowed by the allowlist from current
            assert!(
                allows(current, target),
                "ILLEGAL TRANSITION: Engine entered {target:?} from {current:?} on stimulus {stimulus:?}"
            );

            // Invariant 2: When paused, normal agent activity (AgentWorking/Thinking/Started) cannot force Enter
            if current == PetState::Paused && !matches!(stimulus, Stimulus::UserResumed) {
                let target_priority = engine.definition(target).priority;
                assert!(
                    target_priority == Priority::Critical,
                    "Paused pet was interrupted by non-critical target {target:?} on stimulus {stimulus:?}"
                );
            }
        }
        Transition::Rejected { from, to } => {
            // Invariant 3: If rejected, allows(from, to) MUST be false
            assert!(
                !allows(from, to),
                "FALSE REJECTION: Engine rejected transition from {from:?} to {to:?} despite allows() == true"
            );
        }
        Transition::Remain => {
            // Valid no-op
        }
        Transition::Queued(target) => {
            // Invariant 4: Queued reactions must only be high-value reactions (Success, Celebrating)
            assert!(
                target == PetState::Success || target == PetState::Celebrating,
                "Unexpected queued state: {target:?}"
            );
        }
    }
}

#[test]
fn mutation_sanity_check_fails_on_illegal_transition() {
    // Offline -> LevelUp is strictly illegal in the allowlist
    assert!(!allows(PetState::Offline, PetState::LevelUp));

    let result = std::panic::catch_unwind(|| {
        verify_transition_invariants(
            PetState::Offline,
            Stimulus::AgentWorking,
            EngineContext {
                has_active_sessions: false,
            },
            true, // mutate_allow_illegal = true
        );
    });

    assert!(
        result.is_err(),
        "Mutation sanity check failed: illegal transition was not caught!"
    );
}
