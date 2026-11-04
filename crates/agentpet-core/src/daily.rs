//! Bundled daily companion moments: one gentle, local message per pet per
//! local date, delivered after 09:00. Moments never touch XP, achievements,
//! or the ledger, make no network calls, and play no sound.

use std::{collections::BTreeSet, sync::OnceLock};

use agentpet_avatar::Category;
use chrono::{DateTime, FixedOffset, NaiveTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::{Core, CoreError};

const BUNDLED_PACKS_JSON: &str = include_str!("../daily/messages.json");
const MAX_PACK_BYTES: usize = 24 * 1024;
const MAX_MESSAGES_PER_PACK: usize = 64;
const MAX_MESSAGE_CHARS: usize = 280;
const MAX_ID_CHARS: usize = 64;
const MAX_HISTORY: usize = 100;
/// Moments are not delivered before 09:00 local time.
const DELIVERY_HOUR: u32 = 9;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MessageFile {
    schema_version: u32,
    creator: String,
    license: String,
    packs: Vec<MessagePack>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MessagePack {
    category: Category,
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Message {
    id: String,
    text: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DailyMoment {
    pub pet_id: String,
    pub local_date: String,
    pub message_id: String,
    pub text: String,
    pub created_at: String,
}

fn invalid(reason: impl Into<String>) -> CoreError {
    CoreError::InvalidDailyPack(reason.into())
}

/// Rejects anything that is not plain, short, local text.
fn validate_text(text: &str) -> Result<(), CoreError> {
    let lower = text.to_ascii_lowercase();
    if text.trim().is_empty() || text.chars().count() > MAX_MESSAGE_CHARS {
        return Err(invalid("message must be 1-280 characters"));
    }
    if text.chars().any(char::is_control) {
        return Err(invalid("message contains control characters"));
    }
    if text.contains('<') || text.contains('>') || lower.contains("javascript:") {
        return Err(invalid("message contains markup or script"));
    }
    if lower.contains("://") || lower.contains("www.") {
        return Err(invalid("message contains a URL"));
    }
    Ok(())
}

fn parse_packs(json: &str) -> Result<Vec<MessagePack>, CoreError> {
    if json.len() > MAX_PACK_BYTES {
        return Err(invalid("pack file exceeds 24 KiB"));
    }
    let file: MessageFile =
        serde_json::from_str(json).map_err(|error| invalid(error.to_string()))?;
    if file.schema_version != 1 || file.creator.trim().is_empty() || file.license.trim().is_empty()
    {
        return Err(invalid(
            "pack needs schema version 1, a creator, and a license",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut categories = Vec::new();
    for pack in &file.packs {
        if categories.contains(&pack.category) {
            return Err(invalid("each category may appear once"));
        }
        categories.push(pack.category);
        if pack.messages.is_empty() || pack.messages.len() > MAX_MESSAGES_PER_PACK {
            return Err(invalid("each pack needs 1-64 messages"));
        }
        for message in &pack.messages {
            let id_ok = !message.id.is_empty()
                && message.id.chars().count() <= MAX_ID_CHARS
                && message
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_');
            if !id_ok || !ids.insert(message.id.clone()) {
                return Err(invalid(format!(
                    "message id `{}` is invalid or repeated",
                    message.id
                )));
            }
            validate_text(&message.text)?;
        }
    }
    Ok(file.packs)
}

fn bundled_packs() -> Result<&'static [MessagePack], CoreError> {
    static PACKS: OnceLock<Result<Vec<MessagePack>, String>> = OnceLock::new();
    PACKS
        .get_or_init(|| parse_packs(BUNDLED_PACKS_JSON).map_err(|error| error.to_string()))
        .as_deref()
        .map_err(|error| invalid(error.clone()))
}

fn pack_for(category: Option<Category>) -> Result<&'static MessagePack, CoreError> {
    let packs = bundled_packs()?;
    let wanted = match category {
        // Legacy rows never become the active pet; fall back defensively.
        None | Some(Category::Legacy) => Category::Character,
        Some(category) => category,
    };
    packs
        .iter()
        .find(|pack| pack.category == wanted)
        .or_else(|| packs.first())
        .ok_or_else(|| invalid("no message packs are bundled"))
}

fn message_text(message_id: &str) -> Option<String> {
    bundled_packs()
        .ok()?
        .iter()
        .flat_map(|pack| &pack.messages)
        .find(|message| message.id == message_id)
        .map(|message| message.text.clone())
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

impl Core {
    /// Records today's moment for `pet_id` when it is at least 09:00 local
    /// time and none exists yet for this local date. Returns the moment only
    /// when it was newly recorded, so restarts and retries never repeat it.
    pub fn deliver_daily_moment(
        &self,
        pet_id: &str,
        category: Option<Category>,
        now: DateTime<FixedOffset>,
    ) -> Result<Option<DailyMoment>, CoreError> {
        if now.time() < NaiveTime::from_hms_opt(DELIVERY_HOUR, 0, 0).unwrap_or(NaiveTime::MIN) {
            return Ok(None);
        }
        let local_date = now.date_naive().format("%Y-%m-%d").to_string();
        let pack = pack_for(category)?;
        let index = (fnv1a(format!("{pet_id}|{local_date}").as_bytes())
            % pack.messages.len() as u64) as usize;
        let message = &pack.messages[index];
        let created_at = Utc::now().to_rfc3339();
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let inserted = connection.execute(
            "INSERT OR IGNORE INTO daily_companion_moments(pet_id, local_date, utc_offset_minutes, message_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                pet_id,
                local_date,
                now.offset().local_minus_utc() / 60,
                message.id,
                created_at
            ],
        )?;
        Ok((inserted == 1).then(|| DailyMoment {
            pet_id: pet_id.to_owned(),
            local_date,
            message_id: message.id.clone(),
            text: message.text.clone(),
            created_at,
        }))
    }

    /// Most recent moments first, for the Command Deck history.
    pub fn daily_moments(&self, limit: usize) -> Result<Vec<DailyMoment>, CoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| CoreError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT pet_id, local_date, message_id, created_at FROM daily_companion_moments
             ORDER BY local_date DESC, created_at DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit.min(MAX_HISTORY) as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut moments = Vec::new();
        for row in rows {
            let (pet_id, local_date, message_id, created_at) = row?;
            let text = message_text(&message_id)
                .unwrap_or_else(|| "A quiet moment with your companion.".into());
            moments.push(DailyMoment {
                pet_id,
                local_date,
                message_id,
                text,
                created_at,
            });
        }
        Ok(moments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(rfc3339: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(rfc3339).expect("timestamp")
    }

    #[test]
    fn bundled_packs_are_valid_and_cover_every_live_category() {
        let packs = bundled_packs().expect("bundled packs validate");
        for category in [
            Category::Character,
            Category::Animal,
            Category::Creature,
            Category::Robot,
            Category::DeityInspired,
        ] {
            assert!(
                packs.iter().any(|pack| pack.category == category),
                "{category:?}"
            );
        }
    }

    #[test]
    fn malformed_packs_are_rejected() {
        let pack = |text: &str| {
            format!(
                r#"{{"schemaVersion":1,"creator":"c","license":"CC0-1.0","packs":[{{"category":"animal","messages":[{{"id":"a.one","text":"{text}"}}]}}]}}"#
            )
        };
        assert!(parse_packs(&pack("Hello there")).is_ok());
        for bad in [
            "",
            "Visit https://example.com",
            "see www.example.com",
            "<script>x</script>",
            "javascript:alert(1)",
            "tab\\there",
        ] {
            assert!(
                parse_packs(&pack(bad)).is_err(),
                "{bad:?} should be rejected"
            );
        }
        assert!(parse_packs(&pack(&"x".repeat(281))).is_err());
        let unknown = r#"{"schemaVersion":1,"creator":"c","license":"l","packs":[],"extra":true}"#;
        assert!(parse_packs(unknown).is_err());
        assert!(parse_packs(&" ".repeat(MAX_PACK_BYTES + 1)).is_err());
    }

    #[test]
    fn one_moment_per_pet_per_local_date_after_nine() {
        let core = Core::in_memory().expect("core");
        let ledger_before = core.verify_ledger().expect("ledger");
        let character = Some(Category::Character);
        assert_eq!(
            core.deliver_daily_moment("miyora.aoi", character, at("2026-11-01T08:59:59+05:30"))
                .expect("early"),
            None
        );
        let first = core
            .deliver_daily_moment("miyora.aoi", character, at("2026-11-01T09:00:00+05:30"))
            .expect("deliver")
            .expect("new moment");
        assert_eq!(first.local_date, "2026-11-01");
        // Restart, retry, clock rollback within the day, or a DST/offset change: no repeat.
        for later in [
            "2026-11-01T09:00:01+05:30",
            "2026-11-01T23:59:00+05:30",
            "2026-11-01T10:00:00+04:30",
        ] {
            assert_eq!(
                core.deliver_daily_moment("miyora.aoi", character, at(later))
                    .expect("again"),
                None
            );
        }
        // A different pet and the next local date each get one.
        assert!(core
            .deliver_daily_moment(
                "miyora.ember",
                Some(Category::Animal),
                at("2026-11-01T12:00:00+05:30")
            )
            .expect("other")
            .is_some());
        assert!(core
            .deliver_daily_moment("miyora.aoi", character, at("2026-11-02T09:30:00+05:30"))
            .expect("next")
            .is_some());

        let history = core.daily_moments(10).expect("history");
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].local_date, "2026-11-02");
        assert!(history.iter().all(|moment| !moment.text.is_empty()));
        let ledger_after = core.verify_ledger().expect("ledger");
        assert_eq!(
            (ledger_before.total_entries, ledger_before.head_hash),
            (ledger_after.total_entries, ledger_after.head_hash)
        );
    }

    #[test]
    fn message_choice_is_deterministic_per_pet_and_date() {
        let a = Core::in_memory().expect("core");
        let b = Core::in_memory().expect("core");
        let when = at("2026-11-03T09:15:00+00:00");
        let deity = Some(Category::DeityInspired);
        let first = a
            .deliver_daily_moment("miyora.ganesha", deity, when)
            .expect("a")
            .expect("new");
        let second = b
            .deliver_daily_moment("miyora.ganesha", deity, when)
            .expect("b")
            .expect("new");
        assert_eq!(first.message_id, second.message_id);
        assert!(first.message_id.starts_with("deity."));
    }
}
