//! Validated procedural character specification.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SPEC_SCHEMA_VERSION: u32 = 1;
pub const MAX_ID_BYTES: usize = 64;
pub const MAX_NAME_CHARS: usize = 32;
pub const MAX_TITLE_CHARS: usize = 48;
pub const MAX_DESCRIPTION_CHARS: usize = 240;
pub const MAX_ACCESSORIES: usize = 5;
pub const MAX_TAGS: usize = 6;
pub const MAX_TAG_CHARS: usize = 24;
pub const HEIGHT_RANGE: (f32, f32) = (0.8, 1.2);
pub const HEAD_SCALE_RANGE: (f32, f32) = (0.85, 1.35);
pub const BUILD_RANGE: (f32, f32) = (0.8, 1.3);
pub const LIMB_LENGTH_RANGE: (f32, f32) = (0.85, 1.15);
pub const ENERGY_RANGE: (f32, f32) = (0.5, 1.6);

#[derive(Debug, Error, PartialEq)]
pub enum SpecError {
    #[error("unsupported character spec schema version {0}")]
    UnsupportedSchema(u32),
    #[error("invalid character spec field `{field}`: {reason}")]
    Invalid { field: &'static str, reason: String },
}

fn invalid(field: &'static str, reason: impl Into<String>) -> SpecError {
    SpecError::Invalid {
        field,
        reason: reason.into(),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    Humanoid,
    Beast,
    Robot,
    Slime,
    Bird,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Ears {
    None,
    Human,
    Elf,
    Cat,
    Fox,
    Bunny,
    Bear,
    Floppy,
    Horns,
    Antenna,
    Fin,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Tail {
    None,
    Cat,
    Fox,
    Bunny,
    Dragon,
    Cable,
    Feather,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Wings {
    None,
    Feather,
    Bat,
    Fairy,
    Jet,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HairStyle {
    None,
    Short,
    Bob,
    Long,
    Ponytail,
    Twintails,
    Spiky,
    Bun,
    Mohawk,
    Curly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EyeStyle {
    Round,
    Sharp,
    Sleepy,
    Sparkle,
    Visor,
    Mono,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Outfit {
    Hoodie,
    Kimono,
    Armor,
    Suit,
    Robe,
    Jacket,
    Dress,
    Overalls,
    Labcoat,
    Tunic,
    Fur,
    Plating,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Accessory {
    Glasses,
    Headphones,
    Scarf,
    Beanie,
    WitchHat,
    Crown,
    Halo,
    Cape,
    Backpack,
    Headband,
    Bow,
    Flower,
    Goggles,
    Bell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Palette {
    pub skin: String,
    pub hair: String,
    pub eyes: String,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Proportions {
    pub height: f32,
    pub head_scale: f32,
    pub build: f32,
    pub limb_length: f32,
}

impl Default for Proportions {
    fn default() -> Self {
        Self {
            height: 1.0,
            head_scale: 1.0,
            build: 1.0,
            limb_length: 1.0,
        }
    }
}

/// A complete, data-only description of a procedural 3D character.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterSpec {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub body: BodyType,
    pub ears: Ears,
    pub tail: Tail,
    pub wings: Wings,
    pub hair: HairStyle,
    pub eyes: EyeStyle,
    pub outfit: Outfit,
    #[serde(default)]
    pub accessories: Vec<Accessory>,
    pub palette: Palette,
    #[serde(default)]
    pub proportions: Proportions,
    pub energy: f32,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl CharacterSpec {
    pub fn parse(bytes: &[u8]) -> Result<Self, SpecError> {
        let spec: Self =
            serde_json::from_slice(bytes).map_err(|error| invalid("json", error.to_string()))?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), SpecError> {
        if self.schema_version != SPEC_SCHEMA_VERSION {
            return Err(SpecError::UnsupportedSchema(self.schema_version));
        }
        validate_id(&self.id)?;
        validate_text("name", &self.name, 1, MAX_NAME_CHARS)?;
        validate_text("title", &self.title, 1, MAX_TITLE_CHARS)?;
        validate_text("description", &self.description, 0, MAX_DESCRIPTION_CHARS)?;
        let palette = &self.palette;
        for (field, value) in [
            ("palette.skin", &palette.skin),
            ("palette.hair", &palette.hair),
            ("palette.eyes", &palette.eyes),
            ("palette.primary", &palette.primary),
            ("palette.secondary", &palette.secondary),
            ("palette.accent", &palette.accent),
        ] {
            validate_color(field, value)?;
        }
        let p = &self.proportions;
        validate_range("proportions.height", p.height, HEIGHT_RANGE)?;
        validate_range("proportions.headScale", p.head_scale, HEAD_SCALE_RANGE)?;
        validate_range("proportions.build", p.build, BUILD_RANGE)?;
        validate_range("proportions.limbLength", p.limb_length, LIMB_LENGTH_RANGE)?;
        validate_range("energy", self.energy, ENERGY_RANGE)?;
        if self.accessories.len() > MAX_ACCESSORIES {
            return Err(invalid(
                "accessories",
                format!("at most {MAX_ACCESSORIES} accessories are allowed"),
            ));
        }
        let unique: BTreeSet<_> = self.accessories.iter().collect();
        if unique.len() != self.accessories.len() {
            return Err(invalid("accessories", "accessories must be unique"));
        }
        if self.tags.len() > MAX_TAGS {
            return Err(invalid(
                "tags",
                format!("at most {MAX_TAGS} tags are allowed"),
            ));
        }
        for tag in &self.tags {
            let valid = !tag.is_empty()
                && tag.chars().count() <= MAX_TAG_CHARS
                && tag
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !valid {
                return Err(invalid(
                    "tags",
                    format!("tag `{tag}` must be 1-{MAX_TAG_CHARS} lowercase ASCII letters, digits, or hyphens"),
                ));
            }
        }
        Ok(())
    }
}

/// Avatar identifiers are lowercase dotted names such as `miyora.ember`.
pub fn validate_id(id: &str) -> Result<(), SpecError> {
    if id.is_empty() || id.len() > MAX_ID_BYTES {
        return Err(invalid("id", format!("must be 1-{MAX_ID_BYTES} bytes")));
    }
    let valid_segment = |segment: &str| {
        !segment.is_empty()
            && !segment.starts_with('-')
            && !segment.ends_with('-')
            && segment
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    let segments: Vec<&str> = id.split('.').collect();
    if segments.len() < 2 || !segments.iter().all(|s| valid_segment(s)) {
        return Err(invalid(
            "id",
            "must be a dotted lowercase identifier such as `miyora.ember`",
        ));
    }
    Ok(())
}

fn validate_text(
    field: &'static str,
    value: &str,
    min: usize,
    max: usize,
) -> Result<(), SpecError> {
    let count = value.chars().count();
    if count < min || count > max {
        return Err(invalid(field, format!("must be {min}-{max} characters")));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid(field, "must not contain control characters"));
    }
    if min > 0 && value.trim().is_empty() {
        return Err(invalid(field, "must not be blank"));
    }
    Ok(())
}

fn validate_color(field: &'static str, value: &str) -> Result<(), SpecError> {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err(invalid(field, "must be a #RRGGBB hex color"));
    }
    Ok(())
}

fn validate_range(
    field: &'static str,
    value: f32,
    (min, max): (f32, f32),
) -> Result<(), SpecError> {
    if !value.is_finite() || value < min || value > max {
        return Err(invalid(field, format!("must be between {min} and {max}")));
    }
    Ok(())
}

/// Formats an RGB triple as `#rrggbb`.
pub fn hex_color([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
pub(crate) fn sample_spec() -> CharacterSpec {
    CharacterSpec {
        schema_version: SPEC_SCHEMA_VERSION,
        id: "miyora.test-subject".into(),
        name: "Tester".into(),
        title: "Quality Fox".into(),
        description: String::new(),
        body: BodyType::Beast,
        ears: Ears::Fox,
        tail: Tail::Fox,
        wings: Wings::None,
        hair: HairStyle::Short,
        eyes: EyeStyle::Round,
        outfit: Outfit::Hoodie,
        accessories: vec![Accessory::Scarf],
        palette: Palette {
            skin: "#f2c9a8".into(),
            hair: "#e07a2f".into(),
            eyes: "#2b6cb0".into(),
            primary: "#1f2937".into(),
            secondary: "#374151".into(),
            accent: "#f59e0b".into(),
        },
        proportions: Proportions::default(),
        energy: 1.0,
        tags: vec!["fox".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_spec_is_valid_and_round_trips() {
        let spec = sample_spec();
        spec.validate().expect("valid");
        let json = serde_json::to_vec(&spec).expect("serialize");
        assert_eq!(CharacterSpec::parse(&json).expect("parse"), spec);
    }

    #[test]
    fn rejects_bad_colors_ranges_and_duplicates() {
        let mut spec = sample_spec();
        spec.palette.hair = "red".into();
        assert!(matches!(
            spec.validate(),
            Err(SpecError::Invalid {
                field: "palette.hair",
                ..
            })
        ));

        let mut spec = sample_spec();
        spec.proportions.height = 3.0;
        assert!(spec.validate().is_err());

        let mut spec = sample_spec();
        spec.energy = f32::NAN;
        assert!(spec.validate().is_err());

        let mut spec = sample_spec();
        spec.accessories = vec![Accessory::Bow, Accessory::Bow];
        assert!(spec.validate().is_err());

        let mut spec = sample_spec();
        spec.accessories = vec![
            Accessory::Bow,
            Accessory::Bell,
            Accessory::Cape,
            Accessory::Crown,
            Accessory::Halo,
            Accessory::Scarf,
        ];
        assert!(spec.validate().is_err());
    }

    #[test]
    fn rejects_unknown_fields_and_schema_versions() {
        let mut value = serde_json::to_value(sample_spec()).expect("value");
        value["script"] = serde_json::json!("alert(1)");
        assert!(CharacterSpec::parse(value.to_string().as_bytes()).is_err());

        let mut spec = sample_spec();
        spec.schema_version = 2;
        assert_eq!(spec.validate(), Err(SpecError::UnsupportedSchema(2)));
    }

    #[test]
    fn validates_identifiers() {
        assert!(validate_id("miyora.ember").is_ok());
        assert!(validate_id("user.image-0a1b").is_ok());
        for bad in [
            "",
            "noseparator",
            "Upper.case",
            "a..b",
            "a.-b",
            "../etc",
            "a.b/c",
        ] {
            assert!(validate_id(bad).is_err(), "{bad} must be rejected");
        }
    }

    #[test]
    fn rejects_control_characters_in_text() {
        let mut spec = sample_spec();
        spec.name = "Bad\u{0007}".into();
        assert!(spec.validate().is_err());
    }
}
