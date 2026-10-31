//! Local task state machine and polling policy.
//!
//! Provider `success` alone never makes a task `Succeeded`: outputs must be
//! downloaded and validated first (stage `downloading`).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::provider::ProviderStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Expired,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "QUEUED",
            Self::Running => "RUNNING",
            Self::Succeeded => "SUCCEEDED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
            Self::Expired => "EXPIRED",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::Queued,
            Self::Running,
            Self::Succeeded,
            Self::Failed,
            Self::Cancelled,
            Self::Expired,
        ]
        .into_iter()
        .find(|state| state.as_str() == value)
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Expired
        )
    }

    /// Explicit allow-list. `Failed -> Running` exists only for resuming a
    /// task whose failure was local/transient (network, timeout, download).
    pub fn can_transition_to(self, next: Self) -> bool {
        use TaskState::*;
        matches!(
            (self, next),
            (
                Queued,
                Queued | Running | Succeeded | Failed | Cancelled | Expired
            ) | (Running, Running | Succeeded | Failed | Cancelled | Expired)
                | (Failed, Running)
        )
    }
}

/// Provider status -> local state. `None` keeps the current state (unknown status).
pub fn state_for_provider_status(status: ProviderStatus) -> Option<TaskState> {
    match status {
        ProviderStatus::Queued => Some(TaskState::Queued),
        // Success still needs download + validation, so it stays Running here.
        ProviderStatus::Running | ProviderStatus::Success => Some(TaskState::Running),
        ProviderStatus::Failed | ProviderStatus::Banned => Some(TaskState::Failed),
        ProviderStatus::Expired => Some(TaskState::Expired),
        ProviderStatus::Cancelled => Some(TaskState::Cancelled),
        ProviderStatus::Unknown => None,
    }
}

/// All polling timing lives here; nothing else hard-codes intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollPolicy {
    pub interval_secs: u64,
    pub max_interval_secs: u64,
    pub timeout_secs: u64,
    pub max_consecutive_errors: u32,
}

pub const MIN_POLL_INTERVAL_SECS: u64 = 2;
pub const MAX_POLL_INTERVAL_SECS: u64 = 60;
pub const MIN_TASK_TIMEOUT_SECS: u64 = 60;
pub const MAX_TASK_TIMEOUT_SECS: u64 = 3 * 60 * 60;

impl Default for PollPolicy {
    fn default() -> Self {
        Self {
            interval_secs: 3,
            max_interval_secs: 30,
            timeout_secs: 30 * 60,
            max_consecutive_errors: 6,
        }
    }
}

impl PollPolicy {
    pub fn validated(self) -> Result<Self, String> {
        if !(MIN_POLL_INTERVAL_SECS..=MAX_POLL_INTERVAL_SECS).contains(&self.interval_secs) {
            return Err(format!(
                "Polling interval must be {MIN_POLL_INTERVAL_SECS}-{MAX_POLL_INTERVAL_SECS} seconds."
            ));
        }
        if !(MIN_TASK_TIMEOUT_SECS..=MAX_TASK_TIMEOUT_SECS).contains(&self.timeout_secs) {
            return Err(format!(
                "Task timeout must be {MIN_TASK_TIMEOUT_SECS}-{MAX_TASK_TIMEOUT_SECS} seconds."
            ));
        }
        if self.max_interval_secs < self.interval_secs || self.max_consecutive_errors == 0 {
            return Err("Backoff ceiling and error budget must be positive.".into());
        }
        Ok(self)
    }

    /// Delay before the next status check: base interval, doubled per
    /// consecutive error, capped, and never shorter than a provider Retry-After.
    pub fn next_delay(&self, consecutive_errors: u32, retry_after_secs: Option<u64>) -> Duration {
        let backoff = self
            .interval_secs
            .saturating_mul(1u64 << consecutive_errors.min(16))
            .min(self.max_interval_secs);
        Duration::from_secs(backoff.max(retry_after_secs.unwrap_or(0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states_never_regress_except_failed_resume() {
        use TaskState::*;
        for terminal in [Succeeded, Cancelled, Expired] {
            for next in [Queued, Running, Succeeded, Failed, Cancelled, Expired] {
                assert!(
                    !terminal.can_transition_to(next),
                    "{terminal:?} -> {next:?}"
                );
            }
        }
        assert!(Failed.can_transition_to(Running));
        assert!(!Failed.can_transition_to(Succeeded));
        assert!(Running.can_transition_to(Succeeded));
        assert!(!Running.can_transition_to(Queued));
    }

    #[test]
    fn provider_success_is_not_local_success() {
        assert_eq!(
            state_for_provider_status(ProviderStatus::Success),
            Some(TaskState::Running)
        );
        assert_eq!(
            state_for_provider_status(ProviderStatus::Banned),
            Some(TaskState::Failed)
        );
        assert_eq!(state_for_provider_status(ProviderStatus::Unknown), None);
    }

    #[test]
    fn states_round_trip() {
        for state in [
            "QUEUED",
            "RUNNING",
            "SUCCEEDED",
            "FAILED",
            "CANCELLED",
            "EXPIRED",
        ] {
            assert_eq!(TaskState::parse(state).unwrap().as_str(), state);
        }
    }

    #[test]
    fn backoff_doubles_caps_and_honours_retry_after() {
        let policy = PollPolicy::default();
        assert_eq!(policy.next_delay(0, None), Duration::from_secs(3));
        assert_eq!(policy.next_delay(1, None), Duration::from_secs(6));
        assert_eq!(policy.next_delay(2, None), Duration::from_secs(12));
        assert_eq!(policy.next_delay(10, None), Duration::from_secs(30));
        assert_eq!(policy.next_delay(64, None), Duration::from_secs(30));
        assert_eq!(policy.next_delay(0, Some(45)), Duration::from_secs(45));
    }

    #[test]
    fn policy_validation_bounds() {
        assert!(PollPolicy::default().validated().is_ok());
        assert!(PollPolicy {
            interval_secs: 1,
            ..PollPolicy::default()
        }
        .validated()
        .is_err());
        assert!(PollPolicy {
            timeout_secs: 10,
            ..PollPolicy::default()
        }
        .validated()
        .is_err());
        assert!(PollPolicy {
            max_consecutive_errors: 0,
            ..PollPolicy::default()
        }
        .validated()
        .is_err());
    }
}
