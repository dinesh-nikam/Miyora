use agentpet_core::{EventType, HookEvent, PetEvent};
use chrono::Utc;
use proptest::prelude::*;
use uuid::Uuid;

/// Strategy generating any EventType variant.
pub fn any_event_type() -> impl Strategy<Value = EventType> {
    prop_oneof![
        Just(EventType::AgentStarted),
        Just(EventType::AgentWorking),
        Just(EventType::AgentThinking),
        Just(EventType::AgentWaiting),
        Just(EventType::AgentInputRequired),
        Just(EventType::AgentCompleted),
        Just(EventType::AgentFailed),
        Just(EventType::UserPaused),
        Just(EventType::UserResumed),
        Just(EventType::PetInteracted),
        Just(EventType::PetFed),
        Just(EventType::AgentDisconnected),
        Just(EventType::AgentReconnected),
    ]
}

/// Strategy generating a well-formed PetEvent.
pub fn any_pet_event() -> impl Strategy<Value = PetEvent> {
    (
        any_event_type(),
        "[a-z0-9_-]{1,32}",
        proptest::option::of("[a-z0-9_-]{1,32}"),
    )
        .prop_map(|(event_type, source, session_id)| PetEvent {
            id: Uuid::new_v4(),
            schema_version: 1,
            event_type,
            occurred_at: Utc::now(),
            source,
            session_id,
            project_id: None,
        })
}

/// Strategy generating a valid, coherent HookEvent lifecycle for an agent session.
/// A complete lifecycle has: Start -> (Working / Thinking / Waiting)* -> Complete or Fail.
#[derive(Debug, Clone)]
pub struct SessionLifecycleScenario {
    pub session_id: String,
    pub canonical_events: Vec<HookEvent>,
    pub duplicated_and_permuted_events: Vec<HookEvent>,
}

pub fn session_lifecycle_scenario() -> impl Strategy<Value = SessionLifecycleScenario> {
    (
        "[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}",
        prop::collection::vec(
            prop_oneof![
                Just(EventType::AgentWorking),
                Just(EventType::AgentThinking),
                Just(EventType::AgentWaiting),
                Just(EventType::AgentInputRequired),
            ],
            1..10,
        ),
        prop_oneof![
            Just(EventType::AgentCompleted),
            Just(EventType::AgentFailed)
        ],
        prop::collection::vec(0..10usize, 0..8),
    )
        .prop_map(|(session_id, intermediate, terminal, duplicate_indices)| {
            let mut canonical = Vec::new();
            let agent_id = "test_agent".to_string();
            let ns = Uuid::NAMESPACE_OID;

            // 1. Session start
            let start_id = Uuid::new_v5(&ns, format!("{session_id}-start").as_bytes()).to_string();
            canonical.push(HookEvent {
                id: start_id,
                event_type: EventType::AgentStarted,
                agent_id: agent_id.clone(),
                session_id: Some(session_id.clone()),
                project_path: None,
            });

            // 2. Intermediate events
            for (i, kind) in intermediate.into_iter().enumerate() {
                let step_id =
                    Uuid::new_v5(&ns, format!("{session_id}-step-{i}").as_bytes()).to_string();
                canonical.push(HookEvent {
                    id: step_id,
                    event_type: kind,
                    agent_id: agent_id.clone(),
                    session_id: Some(session_id.clone()),
                    project_path: None,
                });
            }

            // 3. Terminal event
            let term_id = Uuid::new_v5(&ns, format!("{session_id}-term").as_bytes()).to_string();
            canonical.push(HookEvent {
                id: term_id,
                event_type: terminal,
                agent_id: agent_id.clone(),
                session_id: Some(session_id.clone()),
                project_path: None,
            });

            // Build duplicated sequence respecting physical causality:
            // An event cannot be duplicated or retried before it was originally emitted.
            let mut permuted = canonical.clone();
            for idx in duplicate_indices {
                if !canonical.is_empty() {
                    let orig_idx = idx % canonical.len();
                    let chosen = &canonical[orig_idx];
                    let min_pos = orig_idx + 1;
                    let span = (permuted.len() + 1).saturating_sub(min_pos).max(1);
                    let insert_pos = min_pos + (idx % span);
                    permuted.insert(insert_pos.min(permuted.len()), chosen.clone());
                }
            }

            SessionLifecycleScenario {
                session_id,
                canonical_events: canonical,
                duplicated_and_permuted_events: permuted,
            }
        })
}
