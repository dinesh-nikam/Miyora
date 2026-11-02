use std::{collections::BTreeMap, time::Duration};

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod clips;

pub use clips::{AnimationClip, PetPresentation};

const DEFAULT_SUCCESS_DURATION: Duration = Duration::from_secs(4);
const DEFAULT_ERROR_DURATION: Duration = Duration::from_secs(6);
const DEFAULT_CELEBRATING_DURATION: Duration = Duration::from_secs(4);
const DEFAULT_LEVEL_UP_DURATION: Duration = Duration::from_secs(6);
const DEFAULT_ACHIEVEMENT_DURATION: Duration = Duration::from_secs(6);
const DEFAULT_IDLE_BEFORE_SLEEP: Duration = Duration::from_secs(15 * 60);
pub const DEFAULT_HUNGRY_AFTER: Duration = Duration::from_secs(4 * 60 * 60);
pub const DEFAULT_EATING_DURATION: Duration = Duration::from_secs(4);
pub const DEFAULT_REACTION_QUEUE_CAPACITY: usize = 4;
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
    pub hungry_after: Duration,
    pub eating_duration: Duration,
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
            hungry_after: DEFAULT_HUNGRY_AFTER,
            eating_duration: DEFAULT_EATING_DURATION,
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
    PetInteracted,
    PetFed,
    AgentDisconnected,
    AgentReconnected,
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
    /// The stimulus resulted in a reaction queued to play after the current non-interruptible state.
    Queued(PetState),
    Rejected {
        from: PetState,
        to: PetState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionQueue {
    items: Vec<PetState>,
    capacity: usize,
}

impl ReactionQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, state: PetState, engine: &PetEngine) -> bool {
        if self.items.contains(&state) {
            return false;
        }
        if self.items.len() >= self.capacity {
            let min_priority = self
                .items
                .iter()
                .map(|s| engine.definition(*s).priority)
                .min();
            let new_priority = engine.definition(state).priority;
            if min_priority.is_some_and(|p| new_priority > p) {
                if let Some(pos) = self
                    .items
                    .iter()
                    .position(|s| engine.definition(*s).priority == min_priority.unwrap())
                {
                    self.items.remove(pos);
                }
            } else {
                return false;
            }
        }
        self.items.push(state);
        self.items.sort_by(|a, b| {
            engine
                .definition(*b)
                .priority
                .cmp(&engine.definition(*a).priority)
        });
        true
    }

    pub fn pop(&mut self) -> Option<PetState> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.items.remove(0))
        }
    }

    pub fn peek(&self) -> Option<PetState> {
        self.items.first().copied()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl Default for ReactionQueue {
    fn default() -> Self {
        Self::new(DEFAULT_REACTION_QUEUE_CAPACITY)
    }
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
            ("hungry_after", policy.hungry_after),
            ("eating_duration", policy.eating_duration),
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
            Sleeping => (
                Priority::Background,
                true,
                Some(StateTimeout {
                    after: self.policy.hungry_after,
                    target: TimeoutTarget::State(Hungry),
                }),
            ),
            Offline => (Priority::Background, true, None),
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
            Hungry => (Priority::Low, true, None),
            Eating => (Priority::Low, true, resting(self.policy.eating_duration)),
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
        let mut dummy_queue = ReactionQueue::default();
        self.decide_with_queue(current, stimulus, context, &mut dummy_queue)
    }

    pub fn decide_with_queue(
        &self,
        current: PetState,
        stimulus: Stimulus,
        context: EngineContext,
        queue: &mut ReactionQueue,
    ) -> Transition {
        let (target, user_command) = match stimulus {
            Stimulus::UserPaused => (PetState::Paused, true),
            Stimulus::UserResumed if current == PetState::Paused => (resting_state(context), true),
            Stimulus::UserResumed => return Transition::Remain,
            Stimulus::AgentReconnected if current == PetState::Offline => {
                (resting_state(context), false)
            }
            Stimulus::AgentReconnected => return Transition::Remain,
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
            if (target == PetState::Success || target == PetState::Celebrating)
                && queue.push(target, self)
            {
                return Transition::Queued(target);
            }
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
        let mut dummy_queue = ReactionQueue::default();
        self.expire_with_queue(current, elapsed, context, &mut dummy_queue)
    }

    pub fn expire_with_queue(
        &self,
        current: PetState,
        elapsed: Duration,
        context: EngineContext,
        queue: &mut ReactionQueue,
    ) -> Option<PetState> {
        let timeout = self.definition(current).timeout?;
        if elapsed < timeout.after {
            return None;
        }
        if let Some(queued) = queue.pop() {
            if queued != current && allows(current, queued) {
                return Some(queued);
            }
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
        Stimulus::PetInteracted => PetState::Celebrating,
        Stimulus::PetFed => PetState::Eating,
        Stimulus::AgentDisconnected => PetState::Offline,
        Stimulus::AgentReconnected => PetState::Idle,
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
                | Eating
                | Paused
                | Offline
        ),
        Sleeping => matches!(
            to,
            Idle | Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Error
                | Paused
                | Offline
                | Hungry
                | Eating
                | Celebrating
        ),
        Working | Thinking | Waiting => matches!(
            to,
            Working
                | Thinking
                | Waiting
                | AttentionRequired
                | Success
                | Error
                | Celebrating
                | LevelUp
                | Achievement
                | Eating
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
                | Eating
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
                | Eating
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
                | Eating
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
                | Success
                | LevelUp
                | Achievement
                | Eating
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
                | Success
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
                | Success
                | LevelUp
                | Paused
                | Offline
        ),
        Hungry => matches!(
            to,
            Eating | Idle | Working | AttentionRequired | Error | Paused | Offline | Celebrating
        ),
        Eating => matches!(
            to,
            Idle | Working | AttentionRequired | Error | Paused | Offline | Celebrating
        ),
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
        Eating => &[Hungry, Idle],
        Hungry => &[Idle],
        Working | Waiting | Error | Sleeping | Paused => &[Idle],
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
    fn all_fifteen_states_are_reachable_from_documented_stimulus_or_timer() {
        let engine = engine();
        let mut reached = std::collections::HashSet::new();

        // 1. Idle is initial / rest
        reached.insert(PetState::Idle);

        // 2. Working from AgentStarted
        if let Transition::Enter(s) = engine.decide(PetState::Idle, Stimulus::AgentStarted, ACTIVE)
        {
            reached.insert(s);
        }
        // 3. Thinking from AgentThinking
        if let Transition::Enter(s) =
            engine.decide(PetState::Working, Stimulus::AgentThinking, ACTIVE)
        {
            reached.insert(s);
        }
        // 4. Waiting from AgentWaiting
        if let Transition::Enter(s) =
            engine.decide(PetState::Working, Stimulus::AgentWaiting, ACTIVE)
        {
            reached.insert(s);
        }
        // 5. Success from AgentCompleted
        if let Transition::Enter(s) =
            engine.decide(PetState::Working, Stimulus::AgentCompleted, ACTIVE)
        {
            reached.insert(s);
        }
        // 6. Error from AgentFailed
        if let Transition::Enter(s) =
            engine.decide(PetState::Working, Stimulus::AgentFailed, ACTIVE)
        {
            reached.insert(s);
        }
        // 7. Celebrating from PetInteracted
        if let Transition::Enter(s) =
            engine.decide(PetState::Idle, Stimulus::PetInteracted, INACTIVE)
        {
            reached.insert(s);
        }
        // 8. Sleeping from Idle timeout
        if let Some(s) = engine.expire(PetState::Idle, engine.policy().idle_before_sleep, INACTIVE)
        {
            reached.insert(s);
        }
        // 9. Hungry from Sleeping timeout
        if let Some(s) = engine.expire(PetState::Sleeping, engine.policy().hungry_after, INACTIVE) {
            reached.insert(s);
        }
        // 10. Eating from PetFed
        if let Transition::Enter(s) = engine.decide(PetState::Hungry, Stimulus::PetFed, INACTIVE) {
            reached.insert(s);
        }
        // 11. LevelUp from SessionFinished with leveled_up
        if let Transition::Enter(s) = engine.decide(
            PetState::Working,
            Stimulus::SessionFinished(SessionReaction::Succeeded {
                leveled_up: true,
                achievement_unlocked: false,
            }),
            INACTIVE,
        ) {
            reached.insert(s);
        }
        // 12. Achievement from SessionFinished with achievement_unlocked
        if let Transition::Enter(s) = engine.decide(
            PetState::Working,
            Stimulus::SessionFinished(SessionReaction::Succeeded {
                leveled_up: false,
                achievement_unlocked: true,
            }),
            INACTIVE,
        ) {
            reached.insert(s);
        }
        // 13. Offline from AgentDisconnected
        if let Transition::Enter(s) =
            engine.decide(PetState::Idle, Stimulus::AgentDisconnected, INACTIVE)
        {
            reached.insert(s);
        }
        // 14. Paused from UserPaused
        if let Transition::Enter(s) = engine.decide(PetState::Idle, Stimulus::UserPaused, INACTIVE)
        {
            reached.insert(s);
        }
        // 15. AttentionRequired from AgentInputRequired
        if let Transition::Enter(s) =
            engine.decide(PetState::Working, Stimulus::AgentInputRequired, ACTIVE)
        {
            reached.insert(s);
        }

        for state in ALL_STATES {
            assert!(
                reached.contains(&state),
                "State {state:?} must be reachable from a documented stimulus or timer"
            );
        }
        assert_eq!(reached.len(), 15);
    }

    #[test]
    fn table_test_all_state_stimulus_pairs() {
        let engine = engine();
        let all_stimuli = [
            Stimulus::AgentStarted,
            Stimulus::AgentWorking,
            Stimulus::AgentThinking,
            Stimulus::AgentWaiting,
            Stimulus::AgentInputRequired,
            Stimulus::AgentCompleted,
            Stimulus::AgentFailed,
            Stimulus::SessionFinished(SessionReaction::Failed),
            Stimulus::SessionFinished(SessionReaction::Succeeded {
                leveled_up: true,
                achievement_unlocked: false,
            }),
            Stimulus::SessionFinished(SessionReaction::Succeeded {
                leveled_up: false,
                achievement_unlocked: true,
            }),
            Stimulus::SessionFinished(SessionReaction::Succeeded {
                leveled_up: false,
                achievement_unlocked: false,
            }),
            Stimulus::UserPaused,
            Stimulus::UserResumed,
            Stimulus::PetInteracted,
            Stimulus::PetFed,
            Stimulus::AgentDisconnected,
            Stimulus::AgentReconnected,
        ];

        for from in ALL_STATES {
            for stimulus in all_stimuli {
                for context in [ACTIVE, INACTIVE] {
                    let mut queue = ReactionQueue::default();
                    let transition = engine.decide_with_queue(from, stimulus, context, &mut queue);
                    match transition {
                        Transition::Enter(to) => {
                            assert!(
                                allows(from, to),
                                "decide transitioned from {from:?} to {to:?} via {stimulus:?} which is disallowed by allows table"
                            );
                        }
                        Transition::Queued(queued) => {
                            assert!(
                                queue.items.contains(&queued),
                                "queued state must be recorded in reaction queue"
                            );
                        }
                        Transition::Remain | Transition::Rejected { .. } => {}
                    }
                }
            }
        }
    }

    #[test]
    fn queue_preserves_success_arriving_during_level_up() {
        let engine = engine();
        let mut queue = ReactionQueue::default();

        // While in LevelUp (non-interruptible, High priority), Success arrives
        let transition = engine.decide_with_queue(
            PetState::LevelUp,
            Stimulus::AgentCompleted,
            INACTIVE,
            &mut queue,
        );
        assert_eq!(transition, Transition::Queued(PetState::Success));
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.peek(), Some(PetState::Success));

        // When LevelUp expires, the queued reaction is popped and played
        let next = engine.expire_with_queue(
            PetState::LevelUp,
            engine.policy().level_up_duration,
            INACTIVE,
            &mut queue,
        );
        assert_eq!(next, Some(PetState::Success));
        assert!(queue.is_empty());

        // And then Success expires back to resting (Idle)
        let rest = engine.expire_with_queue(
            PetState::Success,
            engine.policy().success_duration,
            INACTIVE,
            &mut queue,
        );
        assert_eq!(rest, Some(PetState::Idle));
    }

    #[test]
    fn interactions_feeding_and_offline_stimuli() {
        let engine = engine();

        // PetInteracted celebrates when idle or working
        assert_eq!(
            engine.decide(PetState::Idle, Stimulus::PetInteracted, INACTIVE),
            Transition::Enter(PetState::Celebrating)
        );
        assert_eq!(
            engine.decide(PetState::Working, Stimulus::PetInteracted, ACTIVE),
            Transition::Enter(PetState::Celebrating)
        );

        // PetFed enters Eating
        assert_eq!(
            engine.decide(PetState::Hungry, Stimulus::PetFed, INACTIVE),
            Transition::Enter(PetState::Eating)
        );
        assert_eq!(
            engine.decide(PetState::Idle, Stimulus::PetFed, INACTIVE),
            Transition::Enter(PetState::Eating)
        );

        // Eating expires to resting
        assert_eq!(
            engine.expire(PetState::Eating, engine.policy().eating_duration, INACTIVE),
            Some(PetState::Idle)
        );
        assert_eq!(
            engine.expire(PetState::Eating, engine.policy().eating_duration, ACTIVE),
            Some(PetState::Working)
        );

        // AgentDisconnected enters Offline
        assert_eq!(
            engine.decide(PetState::Working, Stimulus::AgentDisconnected, INACTIVE),
            Transition::Enter(PetState::Offline)
        );

        // AgentReconnected restores resting state
        assert_eq!(
            engine.decide(PetState::Offline, Stimulus::AgentReconnected, INACTIVE),
            Transition::Enter(PetState::Idle)
        );
        assert_eq!(
            engine.decide(PetState::Offline, Stimulus::AgentReconnected, ACTIVE),
            Transition::Enter(PetState::Working)
        );
    }
}
