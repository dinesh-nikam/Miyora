use std::{collections::BTreeMap, time::Duration};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_SUCCESS_DURATION: Duration = Duration::from_secs(4);
const DEFAULT_ERROR_DURATION: Duration = Duration::from_secs(6);
const DEFAULT_CELEBRATING_DURATION: Duration = Duration::from_secs(4);
const DEFAULT_LEVEL_UP_DURATION: Duration = Duration::from_secs(6);
const DEFAULT_ACHIEVEMENT_DURATION: Duration = Duration::from_secs(6);
const DEFAULT_IDLE_BEFORE_SLEEP: Duration = Duration::from_secs(15 * 60);
const MIN_TIMED_STATE_DURATION: Duration = Duration::from_millis(500);
const MAX_TIMED_STATE_DURATION: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PetState {
    Idle,
    Working,
    Thinking,
    Waiting,
    Success,
    Error,
    Celebrating,
    Sleeping,
    Hungry,
    Eating,
    LevelUp,
    Achievement,
    Offline,
    Paused,
    AttentionRequired,
}

pub const ALL_STATES: [PetState; 15] = [
    PetState::Idle,
    PetState::Working,
    PetState::Thinking,
    PetState::Waiting,
    PetState::Success,
    PetState::Error,
    PetState::Celebrating,
    PetState::Sleeping,
    PetState::Hungry,
    PetState::Eating,
    PetState::LevelUp,
    PetState::Achievement,
    PetState::Offline,
    PetState::Paused,
    PetState::AttentionRequired,
];

impl PetState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Thinking => "thinking",
            Self::Waiting => "waiting",
            Self::Success => "success",
            Self::Error => "error",
            Self::Celebrating => "celebrating",
            Self::Sleeping => "sleeping",
            Self::Hungry => "hungry",
            Self::Eating => "eating",
            Self::LevelUp => "level_up",
            Self::Achievement => "achievement",
            Self::Offline => "offline",
            Self::Paused => "paused",
            Self::AttentionRequired => "attention_required",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        ALL_STATES.into_iter().find(|state| state.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Background,
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutTarget {
    State(PetState),
    /// Working while any agent session is running, otherwise Idle.
    Resting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateTimeout {
    pub after: Duration,
    pub target: TimeoutTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateDefinition {
    pub state: PetState,
    pub priority: Priority,
    /// Non-interruptible states yield only to strictly higher-priority targets or explicit user commands.
    pub interruptible: bool,
    pub timeout: Option<StateTimeout>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatePolicy {
    pub success_duration: Duration,
    pub error_duration: Duration,
    pub celebrating_duration: Duration,
    pub level_up_duration: Duration,
    pub achievement_duration: Duration,
    pub idle_before_sleep: Duration,
}

impl Default for StatePolicy {
    fn default() -> Self {
        Self {
            success_duration: DEFAULT_SUCCESS_DURATION,
            error_duration: DEFAULT_ERROR_DURATION,
            celebrating_duration: DEFAULT_CELEBRATING_DURATION,
            level_up_duration: DEFAULT_LEVEL_UP_DURATION,
            achievement_duration: DEFAULT_ACHIEVEMENT_DURATION,
            idle_before_sleep: DEFAULT_IDLE_BEFORE_SLEEP,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PolicyError {
    #[error("{0} must be between 500 ms and 24 hours")]
    DurationOutOfRange(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionReaction {
    Succeeded {
        leveled_up: bool,
        achievement_unlocked: bool,
    },
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stimulus {
    AgentStarted,
    AgentWorking,
    AgentThinking,
    AgentWaiting,
    AgentInputRequired,
    AgentCompleted,
    AgentFailed,
    SessionFinished(SessionReaction),
    UserPaused,
    UserResumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EngineContext {
    pub has_active_sessions: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Enter(PetState),
    /// The stimulus is valid but does not change the presented state (same state or deferred by priority).
    Remain,
    Rejected {
        from: PetState,
        to: PetState,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PetEngine {
    policy: StatePolicy,
}

impl PetEngine {
    pub fn new(policy: StatePolicy) -> Result<Self, PolicyError> {
        for (name, duration) in [
            ("success_duration", policy.success_duration),
            ("error_duration", policy.error_duration),
            ("celebrating_duration", policy.celebrating_duration),
            ("level_up_duration", policy.level_up_duration),
            ("achievement_duration", policy.achievement_duration),
            ("idle_before_sleep", policy.idle_before_sleep),
        ] {
            if !(MIN_TIMED_STATE_DURATION..=MAX_TIMED_STATE_DURATION).contains(&duration) {
                return Err(PolicyError::DurationOutOfRange(name));
            }
        }
        Ok(Self { policy })
    }

    pub fn policy(&self) -> StatePolicy {
        self.policy
    }

    pub fn definition(&self, state: PetState) -> StateDefinition {
        use PetState::*;
        let resting = |after| {
            Some(StateTimeout {
                after,
                target: TimeoutTarget::Resting,
            })
        };
        let (priority, interruptible, timeout) = match state {
            Idle => (
                Priority::Low,
                true,
                Some(StateTimeout {
                    after: self.policy.idle_before_sleep,
                    target: TimeoutTarget::State(Sleeping),
                }),
            ),
            Sleeping | Offline => (Priority::Background, true, None),
            Working | Thinking | Waiting => (Priority::Normal, true, None),
            AttentionRequired => (Priority::Critical, true, None),
            Success => (
                Priority::Normal,
                true,
                resting(self.policy.success_duration),
            ),
            Celebrating => (
                Priority::Normal,
                true,
                resting(self.policy.celebrating_duration),
            ),
            Error => (
                Priority::Critical,
                true,
                resting(self.policy.error_duration),
            ),
            LevelUp => (
                Priority::High,
                false,
                resting(self.policy.level_up_duration),
            ),
            Achievement => (
                Priority::High,
                false,
                resting(self.policy.achievement_duration),
            ),
            Hungry | Eating => (Priority::Low, true, None),
            Paused => (Priority::High, false, None),
        };
        StateDefinition {
            state,
            priority,
            interruptible,
            timeout,
        }
    }

    pub fn decide(
        &self,
        current: PetState,
        stimulus: Stimulus,
        context: EngineContext,
    ) -> Transition {
        let (target, user_command) = match stimulus {
            Stimulus::UserPaused => (PetState::Paused, true),
            Stimulus::UserResumed if current == PetState::Paused => (resting_state(context), true),
            Stimulus::UserResumed => return Transition::Remain,
            other => (stimulus_target(other), false),
        };
        if target == current {
            return Transition::Remain;
        }
        let current_definition = self.definition(current);
        if !user_command
            && !current_definition.interruptible
            && self.definition(target).priority <= current_definition.priority
        {
            return Transition::Remain;
        }
        if !allows(current, target) {
            return Transition::Rejected {
                from: current,
                to: target,
            };
        }
        Transition::Enter(target)
    }

    pub fn expire(
        &self,
        current: PetState,
        elapsed: Duration,
        context: EngineContext,
    ) -> Option<PetState> {
        let timeout = self.definition(current).timeout?;
        if elapsed < timeout.after {
            return None;
        }
        let next = match timeout.target {
            TimeoutTarget::State(state) => state,
            TimeoutTarget::Resting => resting_state(context),
        };
        (next != current && allows(current, next)).then_some(next)
    }
}

fn resting_state(context: EngineContext) -> PetState {
    if context.has_active_sessions {
        PetState::Working
    } else {
        PetState::Idle
    }
}

fn stimulus_target(stimulus: Stimulus) -> PetState {
    match stimulus {
        Stimulus::AgentStarted | Stimulus::AgentWorking => PetState::Working,
        Stimulus::AgentThinking => PetState::Thinking,
        Stimulus::AgentWaiting => PetState::Waiting,
        Stimulus::AgentInputRequired => PetState::AttentionRequired,
        Stimulus::AgentCompleted => PetState::Success,
        Stimulus::AgentFailed | Stimulus::SessionFinished(SessionReaction::Failed) => {
            PetState::Error
        }
        Stimulus::SessionFinished(SessionReaction::Succeeded {
            leveled_up: true, ..
        }) => PetState::LevelUp,
        Stimulus::SessionFinished(SessionReaction::Succeeded {
            achievement_unlocked: true,
            ..
        }) => PetState::Achievement,
        Stimulus::SessionFinished(SessionReaction::Succeeded { .. }) => PetState::Success,
        Stimulus::UserPaused => PetState::Paused,
        Stimulus::UserResumed => PetState::Idle,
    }
}

pub fn allows(from: PetState, to: PetState) -> bool {
    use PetState::*;
    if from == to {
        return true;
    }
    match from {
        Idle => matches!(
            to,
            Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Error
                | Celebrating
                | LevelUp
                | Achievement
                | Sleeping
                | Hungry
                | Paused
                | Offline
        ),
        Sleeping => matches!(
            to,
            Idle | Working | Thinking | Waiting | AttentionRequired | Error | Paused | Offline
        ),
        Working | Thinking | Waiting => matches!(
            to,
            Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Success
                | Error
                | LevelUp
                | Achievement
                | Paused
                | Offline
        ),
        AttentionRequired => matches!(
            to,
            Idle | Working
                | Thinking
                | Waiting
                | Success
                | Error
                | LevelUp
                | Achievement
                | Paused
                | Offline
        ),
        Success => matches!(
            to,
            Idle | Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Error
                | Celebrating
                | LevelUp
                | Achievement
                | Paused
                | Offline
        ),
        Error => matches!(
            to,
            Idle | Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Success
                | LevelUp
                | Achievement
                | Paused
                | Offline
        ),
        Celebrating => matches!(
            to,
            Idle | Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Error
                | LevelUp
                | Achievement
                | Paused
                | Offline
        ),
        LevelUp => matches!(
            to,
            Idle | Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Error
                | Celebrating
                | Achievement
                | Paused
                | Offline
        ),
        Achievement => matches!(
            to,
            Idle | Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Error
                | Celebrating
                | LevelUp
                | Paused
                | Offline
        ),
        Hungry => matches!(
            to,
            Eating | Idle | Working | AttentionRequired | Error | Paused | Offline
        ),
        Eating => matches!(to, Idle | Working | Paused | Offline),
        Offline => matches!(to, Idle | Working | Paused),
        Paused => matches!(to, Idle | Working | AttentionRequired | Error | Offline),
    }
}

/// States whose animations stand in, in order, when a pack has no animation for the given state.
pub fn animation_fallbacks(state: PetState) -> &'static [PetState] {
    use PetState::*;
    match state {
        Idle => &[],
        Thinking => &[Working, Idle],
        AttentionRequired => &[Waiting, Idle],
        Success => &[Celebrating, Idle],
        Celebrating => &[Success, Idle],
        LevelUp | Achievement => &[Celebrating, Success, Idle],
        Offline => &[Sleeping, Idle],
        Working | Waiting | Error | Sleeping | Hungry | Eating | Paused => &[Idle],
    }
}

/// Resolves the animation for `state` from pack data: explicit state mapping, then an animation
/// named after the state, repeated along the fallback chain, then the first animation.
pub fn select_animation<'a, V>(
    state: PetState,
    state_mappings: &'a BTreeMap<String, String>,
    animations: &'a BTreeMap<String, V>,
) -> Option<&'a str> {
    let chain = std::iter::once(state).chain(animation_fallbacks(state).iter().copied());
    for candidate in chain {
        let key = normalize_key(candidate.as_str());
        let mapped = state_mappings
            .iter()
            .filter(|(mapped_state, _)| normalize_key(mapped_state) == key)
            .find_map(|(_, animation_id)| animations.get_key_value(animation_id.as_str()));
        if let Some((animation_id, _)) = mapped {
            return Some(animation_id);
        }
        if let Some(animation_id) = animations
            .keys()
            .find(|animation_id| normalize_key(animation_id) == key)
        {
            return Some(animation_id);
        }
    }
    animations.keys().next().map(String::as_str)
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTIVE: EngineContext = EngineContext {
        has_active_sessions: true,
    };
    const INACTIVE: EngineContext = EngineContext {
        has_active_sessions: false,
    };

    fn engine() -> PetEngine {
        PetEngine::default()
    }

    #[test]
    fn state_names_round_trip() {
        for state in ALL_STATES {
            assert_eq!(PetState::parse(state.as_str()), Some(state));
        }
        assert_eq!(PetState::parse("LevelUp"), None);
    }

    #[test]
    fn every_timeout_reaches_an_allowed_different_state() {
        let engine = engine();
        for state in ALL_STATES {
            let Some(timeout) = engine.definition(state).timeout else {
                continue;
            };
            for context in [ACTIVE, INACTIVE] {
                let next = engine.expire(state, timeout.after, context);
                assert!(next.is_some(), "{state:?} must leave on timeout");
                assert!(allows(state, next.expect("checked above")));
            }
        }
    }

    #[test]
    fn timed_reactions_wait_for_their_duration_then_return_to_rest() {
        let engine = engine();
        let policy = engine.policy();
        assert_eq!(
            engine.expire(
                PetState::Success,
                policy.success_duration - Duration::from_millis(1),
                INACTIVE
            ),
            None
        );
        assert_eq!(
            engine.expire(PetState::Success, policy.success_duration, INACTIVE),
            Some(PetState::Idle)
        );
        assert_eq!(
            engine.expire(PetState::Error, policy.error_duration, ACTIVE),
            Some(PetState::Working)
        );
        assert_eq!(
            engine.expire(PetState::LevelUp, policy.level_up_duration, ACTIVE),
            Some(PetState::Working)
        );
        assert_eq!(
            engine.expire(PetState::Working, Duration::MAX, INACTIVE),
            None
        );
        assert_eq!(
            engine.expire(PetState::AttentionRequired, Duration::MAX, INACTIVE),
            None
        );
    }

    #[test]
    fn idle_pet_falls_asleep_and_wakes_for_work() {
        let engine = engine();
        assert_eq!(
            engine.expire(PetState::Idle, engine.policy().idle_before_sleep, INACTIVE),
            Some(PetState::Sleeping)
        );
        assert_eq!(
            engine.decide(PetState::Sleeping, Stimulus::AgentStarted, ACTIVE),
            Transition::Enter(PetState::Working)
        );
    }

    #[test]
    fn non_interruptible_celebration_defers_lower_priority_activity() {
        let engine = engine();
        assert_eq!(
            engine.decide(PetState::LevelUp, Stimulus::AgentStarted, ACTIVE),
            Transition::Remain
        );
        assert_eq!(
            engine.decide(
                PetState::LevelUp,
                Stimulus::SessionFinished(SessionReaction::Succeeded {
                    leveled_up: false,
                    achievement_unlocked: true
                }),
                ACTIVE,
            ),
            Transition::Remain
        );
        assert_eq!(
            engine.decide(PetState::LevelUp, Stimulus::AgentFailed, ACTIVE),
            Transition::Enter(PetState::Error)
        );
        assert_eq!(
            engine.decide(PetState::Achievement, Stimulus::AgentInputRequired, ACTIVE),
            Transition::Enter(PetState::AttentionRequired)
        );
    }

    #[test]
    fn pause_holds_until_the_user_resumes_except_for_critical_states() {
        let engine = engine();
        assert_eq!(
            engine.decide(PetState::Working, Stimulus::UserPaused, ACTIVE),
            Transition::Enter(PetState::Paused)
        );
        assert_eq!(
            engine.decide(PetState::Paused, Stimulus::AgentWorking, ACTIVE),
            Transition::Remain
        );
        assert_eq!(
            engine.decide(
                PetState::Paused,
                Stimulus::SessionFinished(SessionReaction::Succeeded {
                    leveled_up: true,
                    achievement_unlocked: false
                }),
                INACTIVE,
            ),
            Transition::Remain
        );
        assert_eq!(
            engine.decide(PetState::Paused, Stimulus::AgentFailed, INACTIVE),
            Transition::Enter(PetState::Error)
        );
        assert_eq!(
            engine.decide(PetState::Paused, Stimulus::UserResumed, ACTIVE),
            Transition::Enter(PetState::Working)
        );
        assert_eq!(
            engine.decide(PetState::Paused, Stimulus::UserResumed, INACTIVE),
            Transition::Enter(PetState::Idle)
        );
        assert_eq!(
            engine.decide(PetState::Working, Stimulus::UserResumed, ACTIVE),
            Transition::Remain
        );
    }

    #[test]
    fn session_reactions_prefer_level_up_then_achievement_then_success() {
        let engine = engine();
        let finish = |leveled_up, achievement_unlocked| {
            engine.decide(
                PetState::Working,
                Stimulus::SessionFinished(SessionReaction::Succeeded {
                    leveled_up,
                    achievement_unlocked,
                }),
                INACTIVE,
            )
        };
        assert_eq!(finish(true, true), Transition::Enter(PetState::LevelUp));
        assert_eq!(
            finish(false, true),
            Transition::Enter(PetState::Achievement)
        );
        assert_eq!(finish(false, false), Transition::Enter(PetState::Success));
        assert_eq!(
            engine.decide(
                PetState::Working,
                Stimulus::SessionFinished(SessionReaction::Failed),
                INACTIVE
            ),
            Transition::Enter(PetState::Error)
        );
    }

    #[test]
    fn completion_without_observed_work_is_rejected() {
        assert_eq!(
            engine().decide(PetState::Idle, Stimulus::AgentCompleted, INACTIVE),
            Transition::Rejected {
                from: PetState::Idle,
                to: PetState::Success
            }
        );
    }

    #[test]
    fn policy_rejects_out_of_range_durations() {
        let policy = StatePolicy {
            success_duration: Duration::from_millis(10),
            ..StatePolicy::default()
        };
        assert_eq!(
            PetEngine::new(policy),
            Err(PolicyError::DurationOutOfRange("success_duration"))
        );
        let policy = StatePolicy {
            idle_before_sleep: Duration::from_secs(25 * 60 * 60),
            ..StatePolicy::default()
        };
        assert_eq!(
            PetEngine::new(policy),
            Err(PolicyError::DurationOutOfRange("idle_before_sleep"))
        );
        assert!(PetEngine::new(StatePolicy::default()).is_ok());
    }

    fn animations(ids: &[&str]) -> BTreeMap<String, ()> {
        ids.iter().map(|id| ((*id).to_owned(), ())).collect()
    }

    #[test]
    fn explicit_mapping_wins_and_keys_are_case_insensitive() {
        let mappings = BTreeMap::from([("Working".to_owned(), "typing".to_owned())]);
        let animations = animations(&["idle", "typing", "working"]);
        assert_eq!(
            select_animation(PetState::Working, &mappings, &animations),
            Some("typing")
        );
    }

    #[test]
    fn missing_state_animation_follows_the_fallback_chain() {
        let mappings = BTreeMap::from([("idle".to_owned(), "rest".to_owned())]);
        let available = animations(&["celebrate", "rest", "zz-other"]);
        let mut with_celebration = mappings.clone();
        with_celebration.insert("celebrating".into(), "celebrate".into());
        assert_eq!(
            select_animation(PetState::LevelUp, &with_celebration, &available),
            Some("celebrate")
        );
        assert_eq!(
            select_animation(PetState::LevelUp, &mappings, &available),
            Some("rest")
        );
        assert_eq!(
            select_animation(
                PetState::Thinking,
                &BTreeMap::new(),
                &animations(&["level-up", "Working"])
            ),
            Some("Working")
        );
        assert_eq!(
            select_animation(
                PetState::LevelUp,
                &BTreeMap::new(),
                &animations(&["level-up", "idle"])
            ),
            Some("level-up")
        );
    }

    #[test]
    fn mappings_to_unknown_animations_are_skipped_and_first_animation_is_last_resort() {
        let mappings = BTreeMap::from([("error".to_owned(), "missing".to_owned())]);
        assert_eq!(
            select_animation(
                PetState::Error,
                &mappings,
                &animations(&["b-wave", "c-jump"])
            ),
            Some("b-wave")
        );
        assert_eq!(
            select_animation::<()>(PetState::Error, &mappings, &BTreeMap::new()),
            None
        );
    }
}
