//! Provider secrets live in the OS credential store (Windows Credential
//! Manager, macOS Keychain, Linux kernel keyring). SQLite only keeps the
//! opaque reference returned by `credential_reference`.

pub const KEYRING_SERVICE: &str = "miyora-studio";
const API_KEY_MIN_CHARS: usize = 8;
const API_KEY_MAX_CHARS: usize = 512;

pub trait CredentialStore: Send + Sync {
    fn get(&self, reference: &str) -> Result<Option<String>, String>;
    fn set(&self, reference: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, reference: &str) -> Result<(), String>;
}

pub fn credential_reference(provider: &str) -> String {
    format!("provider:{provider}")
}

/// Shape check only; the provider is the authority on validity.
pub fn normalize_api_key(raw: &str) -> Result<String, String> {
    let key = raw.trim();
    let length = key.chars().count();
    if !(API_KEY_MIN_CHARS..=API_KEY_MAX_CHARS).contains(&length) {
        return Err(format!(
            "API key must be {API_KEY_MIN_CHARS}-{API_KEY_MAX_CHARS} characters."
        ));
    }
    if key.chars().any(|ch| ch.is_whitespace() || ch.is_control()) {
        return Err("API key must not contain spaces or control characters.".into());
    }
    Ok(key.to_owned())
}

pub struct KeyringStore;

impl CredentialStore for KeyringStore {
    fn get(&self, reference: &str) -> Result<Option<String>, String> {
        let entry =
            keyring::Entry::new(KEYRING_SERVICE, reference).map_err(|error| error.to_string())?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn set(&self, reference: &str, secret: &str) -> Result<(), String> {
        keyring::Entry::new(KEYRING_SERVICE, reference)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|error| error.to_string())
    }

    fn delete(&self, reference: &str) -> Result<(), String> {
        let entry =
            keyring::Entry::new(KEYRING_SERVICE, reference).map_err(|error| error.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;

    /// In-memory credential store for tests only.
    #[derive(Default)]
    pub(crate) struct MemoryCredentials(pub Mutex<HashMap<String, String>>);

    impl CredentialStore for MemoryCredentials {
        fn get(&self, reference: &str) -> Result<Option<String>, String> {
            Ok(self.0.lock().unwrap().get(reference).cloned())
        }
        fn set(&self, reference: &str, secret: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .insert(reference.into(), secret.into());
            Ok(())
        }
        fn delete(&self, reference: &str) -> Result<(), String> {
            self.0.lock().unwrap().remove(reference);
            Ok(())
        }
    }

    #[test]
    fn api_key_shape_validation() {
        assert_eq!(
            normalize_api_key("  tsk_abcdefgh  ").unwrap(),
            "tsk_abcdefgh"
        );
        assert!(normalize_api_key("short").is_err());
        assert!(normalize_api_key("tsk_abc defgh").is_err());
        assert!(normalize_api_key(&"k".repeat(513)).is_err());
    }
}
