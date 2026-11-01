//! Animation clip contract. Rust decides which named clip a renderer plays for
//! each pet state; the renderer only plays it. Procedural rigs implement every
//! clip, while a rig with fewer clips falls back along the state's chain.

use serde::{Deserialize, Serialize};

use crate::PetState;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AnimationClip {
    Idle,
    Work,
    Think,
    Wait,
    Success,
    Error,
    Celebrate,
    Sleep,
    Hungry,
    Eat,
    LevelUp,
    Achievement,
    Offline,
    Paused,
    Attention,
    /// Locomotion layered over the state clip while the pet roams.
    Walk,
}

impl PetState {
    /// Primary clip followed by its fallbacks; every chain ends at `Idle`.
    pub fn clip_chain(self) -> &'static [AnimationClip] {
        use AnimationClip as C;
        match self {
            Self::Idle => &[C::Idle],
            Self::Working => &[C::Work, C::Idle],
            Self::Thinking => &[C::Think, C::Work, C::Idle],
            Self::Waiting => &[C::Wait, C::Idle],
            Self::Success => &[C::Success, C::Celebrate, C::Idle],
            Self::Error => &[C::Error, C::Idle],
            Self::Celebrating => &[C::Celebrate, C::Success, C::Idle],
            Self::Sleeping => &[C::Sleep, C::Idle],
            Self::Hungry => &[C::Hungry, C::Idle],
            Self::Eating => &[C::Eat, C::Idle],
            Self::LevelUp => &[C::LevelUp, C::Celebrate, C::Success, C::Idle],
            Self::Achievement => &[C::Achievement, C::Celebrate, C::Success, C::Idle],
            Self::Offline => &[C::Offline, C::Sleep, C::Idle],
            Self::Paused => &[C::Paused, C::Idle],
            Self::AttentionRequired => &[C::Attention, C::Wait, C::Idle],
        }
    }

    /// First clip in the chain the rig supports, or `Idle`.
    pub fn resolve_clip(self, supports: impl Fn(AnimationClip) -> bool) -> AnimationClip {
        self.clip_chain()
            .iter()
            .copied()
            .find(|clip| supports(*clip))
            .unwrap_or(AnimationClip::Idle)
    }
}

/// What the renderer should play right now.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetPresentation {
    pub state_clip: AnimationClip,
    pub locomotion_clip: Option<AnimationClip>,
}

impl PetPresentation {
    /// Presentation for a procedural rig, which supports every clip. `walking`
    /// is set by the desktop host only while it is moving the overlay.
    pub fn procedural(state: PetState, walking: bool) -> Self {
        Self {
            state_clip: state.resolve_clip(|_| true),
            locomotion_clip: walking.then_some(AnimationClip::Walk),
        }
    }
}

impl Default for PetPresentation {
    fn default() -> Self {
        Self::procedural(PetState::Idle, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ALL_STATES;

    #[test]
    fn every_state_has_a_distinct_primary_clip_and_ends_at_idle() {
        let mut primaries = std::collections::HashSet::new();
        for state in ALL_STATES {
            let chain = state.clip_chain();
            assert_eq!(chain.last(), Some(&AnimationClip::Idle), "{state:?}");
            assert!(
                primaries.insert(chain[0]),
                "{state:?} reuses a primary clip"
            );
            assert!(!chain.contains(&AnimationClip::Walk));
        }
    }

    #[test]
    fn resolution_is_deterministic_and_follows_fallbacks() {
        assert_eq!(
            PetState::LevelUp.resolve_clip(|_| true),
            AnimationClip::LevelUp
        );
        let only_success = |clip| matches!(clip, AnimationClip::Success | AnimationClip::Idle);
        assert_eq!(
            PetState::LevelUp.resolve_clip(only_success),
            AnimationClip::Success
        );
        assert_eq!(
            PetState::Offline.resolve_clip(|clip| clip == AnimationClip::Sleep),
            AnimationClip::Sleep
        );
        assert_eq!(
            PetState::Thinking.resolve_clip(|_| false),
            AnimationClip::Idle
        );
    }

    #[test]
    fn walking_adds_locomotion_without_changing_the_state_clip() {
        let still = PetPresentation::procedural(PetState::Idle, false);
        let walking = PetPresentation::procedural(PetState::Idle, true);
        assert_eq!(still.state_clip, walking.state_clip);
        assert_eq!(still.locomotion_clip, None);
        assert_eq!(walking.locomotion_clip, Some(AnimationClip::Walk));
        assert_eq!(
            serde_json::to_value(walking).expect("json"),
            serde_json::json!({ "stateClip": "idle", "locomotionClip": "walk" })
        );
    }
}
