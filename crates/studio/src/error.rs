use serde::Serialize;

/// Stable, provider-neutral error categories surfaced to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Authentication,
    Validation,
    RateLimit,
    InsufficientCredits,
    ProviderUnavailable,
    TaskFailed,
    TaskTimeout,
    FileUpload,
    Conversion,
    UnknownProvider,
}

const ALL_KINDS: [ErrorKind; 10] = [
    ErrorKind::Authentication,
    ErrorKind::Validation,
    ErrorKind::RateLimit,
    ErrorKind::InsufficientCredits,
    ErrorKind::ProviderUnavailable,
    ErrorKind::TaskFailed,
    ErrorKind::TaskTimeout,
    ErrorKind::FileUpload,
    ErrorKind::Conversion,
    ErrorKind::UnknownProvider,
];

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Authentication => "authentication",
            Self::Validation => "validation",
            Self::RateLimit => "rate_limit",
            Self::InsufficientCredits => "insufficient_credits",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::TaskFailed => "task_failed",
            Self::TaskTimeout => "task_timeout",
            Self::FileUpload => "file_upload",
            Self::Conversion => "conversion",
            Self::UnknownProvider => "unknown_provider",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        ALL_KINDS.into_iter().find(|kind| kind.as_str() == value)
    }

    /// Failures where re-checking the same provider task is safe and costs no credits.
    pub fn is_transient(self) -> bool {
        matches!(
            self,
            Self::RateLimit | Self::ProviderUnavailable | Self::TaskTimeout | Self::Conversion
        )
    }

    /// What the user can do about it, phrased for the error panel.
    pub fn suggestion(self) -> &'static str {
        match self {
            Self::Authentication => "Check the API key in Studio settings.",
            Self::Validation => "Adjust the request settings or input file and try again.",
            Self::RateLimit => "Wait a moment, then retry.",
            Self::InsufficientCredits => "Top up credits with the provider, then retry.",
            Self::ProviderUnavailable => "Check your internet connection, then retry.",
            Self::TaskFailed => "Change the prompt or input and generate again.",
            Self::TaskTimeout => "Resume tracking; the provider may still finish the task.",
            Self::FileUpload => "Check the source file and try again.",
            Self::Conversion => "Resume the task to download and validate the result again.",
            Self::UnknownProvider => "Retry later; report the request ID if it persists.",
        }
    }
}

/// Failure from a provider call, already mapped into a stable category.
/// `message` is safe to show: it never contains credentials or stack traces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct ProviderError {
    pub kind: ErrorKind,
    pub message: String,
    pub provider_code: Option<i64>,
    pub request_id: Option<String>,
    pub retry_after_secs: Option<u64>,
}

impl ProviderError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            provider_code: None,
            request_id: None,
            retry_after_secs: None,
        }
    }
}

/// Error returned by studio services to the desktop shell.
#[derive(Debug, thiserror::Error)]
pub enum StudioError {
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0} was not found")]
    NotFound(String),
    #[error("no API key is configured for provider {0}")]
    MissingCredential(String),
    #[error("credential store error: {0}")]
    Credential(String),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database schema version {0} is newer than this build supports")]
    UnsupportedSchema(i64),
}

/// UI-facing error shape: what failed, why, what to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorReport {
    pub kind: String,
    pub message: String,
    pub suggestion: String,
    pub request_id: Option<String>,
    pub provider_code: Option<i64>,
}

impl From<&StudioError> for ErrorReport {
    fn from(error: &StudioError) -> Self {
        let local = |kind: &str, suggestion: &str| Self {
            kind: kind.into(),
            message: error.to_string(),
            suggestion: suggestion.into(),
            request_id: None,
            provider_code: None,
        };
        match error {
            StudioError::Provider(provider) => Self {
                kind: provider.kind.as_str().into(),
                message: provider.message.clone(),
                suggestion: provider.kind.suggestion().into(),
                request_id: provider.request_id.clone(),
                provider_code: provider.provider_code,
            },
            StudioError::InvalidInput(_) => local("validation", "Fix the input and try again."),
            StudioError::NotFound(_) => local("not_found", "Refresh the library."),
            StudioError::MissingCredential(_) => {
                local("authentication", "Add an API key in Studio settings.")
            }
            StudioError::Credential(_) => local(
                "credential_store",
                "Check that the operating-system credential manager is available.",
            ),
            StudioError::Database(_) | StudioError::UnsupportedSchema(_) | StudioError::Io(_) => {
                local(
                    "local_storage",
                    "Check disk space and file permissions, then retry.",
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip_through_storage_strings() {
        for kind in ALL_KINDS {
            assert_eq!(ErrorKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(ErrorKind::parse("nope"), None);
    }

    #[test]
    fn provider_report_keeps_request_id_and_adds_suggestion() {
        let mut provider = ProviderError::new(ErrorKind::InsufficientCredits, "not enough credit");
        provider.request_id = Some("req_1".into());
        let report = ErrorReport::from(&StudioError::Provider(provider));
        assert_eq!(report.kind, "insufficient_credits");
        assert_eq!(report.request_id.as_deref(), Some("req_1"));
        assert!(report.suggestion.contains("Top up"));
    }
}
