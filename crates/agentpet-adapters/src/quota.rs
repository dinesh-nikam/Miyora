//! Truthful provider quota status. A value is shown only when a provider
//! reported it; everything else says why it is unavailable. Nothing here
//! estimates remaining allowance from observed activity.

use serde::Serialize;

use crate::claude_code::ClaudeQuotaReport;

/// Reports older than this are shown as stale rather than as current.
pub const QUOTA_FRESHNESS_LIMIT_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MeasuredWindow {
    /// Provider-defined window name, e.g. `five_hour` or `seven_day`.
    pub window: String,
    pub remaining_percent: f64,
    pub reset_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    NotDetected,
    NotConfigured,
    NotExposedLocally,
    Stale,
    ReadFailed,
}

/// Follow-up the UI may offer for an unavailable quota.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuotaAction {
    /// Copy `/status` so the user can run it in an active Codex session.
    CopyCodexStatusCommand,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuotaStatus {
    Measured {
        windows: Vec<MeasuredWindow>,
        observed_at: String,
        provenance: String,
    },
    Unavailable {
        reason: UnavailableReason,
        action: Option<QuotaAction>,
    },
    Unsupported {
        reason: String,
    },
}

/// Claude Code reports quota through the status line AgentPet records.
/// `report` is `None` when nothing was recorded and `Some(Err)` when the
/// recorded payload could not be parsed.
pub fn claude_quota_status<E>(
    detected: bool,
    report: Option<Result<ClaudeQuotaReport, E>>,
    age_secs: Option<u64>,
) -> QuotaStatus {
    let unavailable = |reason| QuotaStatus::Unavailable {
        reason,
        action: None,
    };
    let report = match report {
        None if !detected => return unavailable(UnavailableReason::NotDetected),
        None => return unavailable(UnavailableReason::NotConfigured),
        Some(Err(_)) => return unavailable(UnavailableReason::ReadFailed),
        Some(Ok(report)) => report,
    };
    if age_secs.is_some_and(|age| age > QUOTA_FRESHNESS_LIMIT_SECS) {
        return unavailable(UnavailableReason::Stale);
    }
    let windows: Vec<MeasuredWindow> = [
        ("five_hour", &report.five_hour),
        ("seven_day", &report.seven_day),
    ]
    .into_iter()
    .filter_map(|(name, window)| {
        window.as_ref().map(|window| MeasuredWindow {
            window: name.to_owned(),
            remaining_percent: (100.0 - window.used_percentage).clamp(0.0, 100.0),
            reset_at: window.resets_at.clone(),
        })
    })
    .collect();
    if windows.is_empty() {
        return unavailable(UnavailableReason::NotExposedLocally);
    }
    QuotaStatus::Measured {
        windows,
        observed_at: report.updated_at,
        provenance: report.provenance,
    }
}

/// Codex keeps remaining allowance in its own Usage view and `/status`;
/// AgentPet never infers it from activity.
pub fn codex_quota_status(detected: bool) -> QuotaStatus {
    if detected {
        QuotaStatus::Unavailable {
            reason: UnavailableReason::NotExposedLocally,
            action: Some(QuotaAction::CopyCodexStatusCommand),
        }
    } else {
        QuotaStatus::Unavailable {
            reason: UnavailableReason::NotDetected,
            action: None,
        }
    }
}

/// Providers without any local quota source.
pub fn unsupported_quota_status(detected: bool) -> QuotaStatus {
    if detected {
        QuotaStatus::Unsupported {
            reason: "This provider does not expose usage locally.".into(),
        }
    } else {
        QuotaStatus::Unavailable {
            reason: UnavailableReason::NotDetected,
            action: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_code::ClaudeQuotaWindow;

    fn report() -> ClaudeQuotaReport {
        ClaudeQuotaReport {
            five_hour: Some(ClaudeQuotaWindow {
                used_percentage: 37.5,
                resets_at: Some("2026-11-01T12:00:00+00:00".into()),
            }),
            seven_day: None,
            context_window_used_percentage: None,
            updated_at: "2026-11-01T09:00:00+00:00".into(),
            provenance: "LIVE".into(),
        }
    }

    fn reason(status: QuotaStatus) -> Option<UnavailableReason> {
        match status {
            QuotaStatus::Unavailable { reason, .. } => Some(reason),
            _ => None,
        }
    }

    #[test]
    fn claude_measured_values_come_only_from_a_fresh_report() {
        let status = claude_quota_status::<()>(true, Some(Ok(report())), Some(10));
        assert_eq!(
            status,
            QuotaStatus::Measured {
                windows: vec![MeasuredWindow {
                    window: "five_hour".into(),
                    remaining_percent: 62.5,
                    reset_at: Some("2026-11-01T12:00:00+00:00".into()),
                }],
                observed_at: "2026-11-01T09:00:00+00:00".into(),
                provenance: "LIVE".into(),
            }
        );
        let stale = claude_quota_status::<()>(
            true,
            Some(Ok(report())),
            Some(QUOTA_FRESHNESS_LIMIT_SECS + 1),
        );
        assert_eq!(reason(stale), Some(UnavailableReason::Stale));
    }

    #[test]
    fn claude_missing_or_broken_reports_are_unavailable_with_a_reason() {
        assert_eq!(
            reason(claude_quota_status::<()>(false, None, None)),
            Some(UnavailableReason::NotDetected)
        );
        assert_eq!(
            reason(claude_quota_status::<()>(true, None, None)),
            Some(UnavailableReason::NotConfigured)
        );
        assert_eq!(
            reason(claude_quota_status(true, Some(Err("bad json")), Some(1))),
            Some(UnavailableReason::ReadFailed)
        );
        let mut empty = report();
        empty.five_hour = None;
        assert_eq!(
            reason(claude_quota_status::<()>(true, Some(Ok(empty)), Some(1))),
            Some(UnavailableReason::NotExposedLocally)
        );
    }

    #[test]
    fn codex_never_reports_a_measured_quota() {
        assert_eq!(
            codex_quota_status(true),
            QuotaStatus::Unavailable {
                reason: UnavailableReason::NotExposedLocally,
                action: Some(QuotaAction::CopyCodexStatusCommand),
            }
        );
        assert_eq!(
            reason(codex_quota_status(false)),
            Some(UnavailableReason::NotDetected)
        );
        assert_eq!(
            serde_json::to_value(codex_quota_status(true)).expect("json"),
            serde_json::json!({ "kind": "unavailable", "reason": "not_exposed_locally", "action": "copy_codex_status_command" })
        );
    }

    #[test]
    fn unsupported_providers_say_so() {
        assert!(matches!(
            unsupported_quota_status(true),
            QuotaStatus::Unsupported { .. }
        ));
        assert_eq!(
            reason(unsupported_quota_status(false)),
            Some(UnavailableReason::NotDetected)
        );
    }
}
