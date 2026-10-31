//! Structured prompt builder and provider-neutral presets.
//!
//! The builder never invents content: it only orders, labels, sanitizes and
//! bounds what the user typed plus the selected preset's short hints.

use serde::{Deserialize, Serialize};

use crate::provider::{GeometryQuality, ProviderLimits, TextureQuality};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PromptIntent {
    pub object_identity: String,
    pub structural_components: String,
    pub materials: String,
    pub visual_style: String,
    pub dimensions: String,
    pub pose: String,
    pub details: String,
    pub topology_intent: String,
    pub exclusions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltPrompt {
    pub prompt: String,
    pub negative_prompt: String,
    /// True when content was cut to fit the provider limit.
    pub truncated: bool,
    /// True when unsupported characters (emoji, control) were removed.
    pub sanitized: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDefaults {
    pub face_limit: Option<u32>,
    pub texture: Option<bool>,
    pub pbr: Option<bool>,
    pub smart_low_poly: Option<bool>,
    pub quad: Option<bool>,
    pub auto_size: Option<bool>,
    pub geometry_quality: Option<GeometryQuality>,
    pub texture_quality: Option<TextureQuality>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub style_hint: &'static str,
    pub topology_hint: &'static str,
    pub negative_hint: &'static str,
    pub defaults: PresetDefaults,
}

const NONE: PresetDefaults = PresetDefaults {
    face_limit: None,
    texture: None,
    pbr: None,
    smart_low_poly: None,
    quad: None,
    auto_size: None,
    geometry_quality: None,
    texture_quality: None,
};

pub const PRESETS: [Preset; 12] = [
    Preset {
        id: "game-asset",
        label: "Game Asset",
        style_hint: "game-ready asset, clean silhouette",
        topology_hint: "optimized real-time topology",
        negative_hint: "floating fragments, noisy surface",
        defaults: PresetDefaults {
            face_limit: Some(50_000),
            pbr: Some(true),
            ..NONE
        },
    },
    Preset {
        id: "product-visualization",
        label: "Product Visualization",
        style_hint: "photorealistic product render, accurate proportions",
        topology_hint: "smooth surfaces, crisp edges",
        negative_hint: "text, logos, warped geometry",
        defaults: PresetDefaults {
            pbr: Some(true),
            auto_size: Some(true),
            geometry_quality: Some(GeometryQuality::Detailed),
            texture_quality: Some(TextureQuality::Detailed),
            ..NONE
        },
    },
    Preset {
        id: "character",
        label: "Character",
        style_hint: "full-body character, neutral A-pose",
        topology_hint: "clean limbs separated from body",
        negative_hint: "merged limbs, extra fingers",
        defaults: PresetDefaults {
            pbr: Some(true),
            ..NONE
        },
    },
    Preset {
        id: "hard-surface",
        label: "Hard Surface",
        style_hint: "hard-surface model, precise mechanical forms",
        topology_hint: "sharp bevelled edges, flat planar faces",
        negative_hint: "melted edges, organic blobs",
        defaults: PresetDefaults {
            geometry_quality: Some(GeometryQuality::Detailed),
            ..NONE
        },
    },
    Preset {
        id: "architecture",
        label: "Architecture",
        style_hint: "architectural model, straight walls, correct proportions",
        topology_hint: "planar surfaces",
        negative_hint: "people, vehicles, terrain clutter",
        defaults: PresetDefaults {
            auto_size: Some(true),
            ..NONE
        },
    },
    Preset {
        id: "low-poly",
        label: "Low Poly",
        style_hint: "stylized low-poly",
        topology_hint: "minimal faceted geometry",
        negative_hint: "high-frequency detail",
        defaults: PresetDefaults {
            face_limit: Some(5_000),
            smart_low_poly: Some(true),
            ..NONE
        },
    },
    Preset {
        id: "3d-printing",
        label: "3D Printing",
        style_hint: "solid printable object, single connected piece",
        topology_hint: "watertight manifold mesh, no thin unsupported parts",
        negative_hint: "floating parts, paper-thin walls",
        defaults: PresetDefaults {
            texture: Some(false),
            pbr: Some(false),
            auto_size: Some(true),
            geometry_quality: Some(GeometryQuality::Detailed),
            ..NONE
        },
    },
    Preset {
        id: "collectible",
        label: "Collectible",
        style_hint: "collectible figurine on a simple base",
        topology_hint: "sculpted detail",
        negative_hint: "busy background props",
        defaults: PresetDefaults {
            geometry_quality: Some(GeometryQuality::Detailed),
            texture_quality: Some(TextureQuality::Detailed),
            ..NONE
        },
    },
    Preset {
        id: "fantasy",
        label: "Fantasy",
        style_hint: "fantasy style, ornate hand-crafted details",
        topology_hint: "",
        negative_hint: "modern technology",
        defaults: NONE,
    },
    Preset {
        id: "sci-fi",
        label: "Sci-Fi",
        style_hint: "sci-fi style, futuristic panels and lights",
        topology_hint: "hard-surface paneling",
        negative_hint: "medieval elements",
        defaults: NONE,
    },
    Preset {
        id: "vehicle",
        label: "Vehicle",
        style_hint: "vehicle, symmetric body, visible wheels",
        topology_hint: "hard-surface body panels",
        negative_hint: "asymmetric body, missing wheels",
        defaults: PresetDefaults {
            auto_size: Some(true),
            ..NONE
        },
    },
    Preset {
        id: "furniture",
        label: "Furniture",
        style_hint: "furniture piece, realistic proportions",
        topology_hint: "clean planar surfaces",
        negative_hint: "room interior, floor, walls",
        defaults: PresetDefaults {
            auto_size: Some(true),
            pbr: Some(true),
            ..NONE
        },
    },
];

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.id == id)
}

fn is_unsupported_char(ch: char) -> bool {
    ch.is_control()
        || matches!(ch as u32,
            0x1F000..=0x1FAFF   // emoji and pictographs
            | 0x2600..=0x27BF   // misc symbols, dingbats
            | 0xFE00..=0xFE0F   // variation selectors
            | 0x200D            // zero-width joiner
            | 0xE0000..=0xE007F // tags
        )
}

/// Removes unsupported characters and collapses whitespace.
fn sanitize(text: &str, removed: &mut bool) -> String {
    let filtered: String = text
        .chars()
        .map(|ch| {
            if matches!(ch, '\n' | '\r' | '\t') {
                ' '
            } else {
                ch
            }
        })
        .filter(|ch| {
            let bad = is_unsupported_char(*ch);
            *removed |= bad;
            !bad
        })
        .collect();
    filtered.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cuts at the last word boundary within `max_chars`.
fn bound(text: &str, max_chars: usize, truncated: &mut bool) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    *truncated = true;
    let cut: String = text.chars().take(max_chars).collect();
    match cut.rfind(' ') {
        Some(index) if index > 0 => cut[..index].trim_end_matches([',', '.', ' ']).to_owned(),
        _ => cut,
    }
}

pub fn build_prompt(
    intent: &PromptIntent,
    preset: Option<&Preset>,
    limits: &ProviderLimits,
) -> Result<BuiltPrompt, String> {
    let mut sanitized = false;
    let mut clean = |text: &str| sanitize(text, &mut sanitized);
    let object = clean(&intent.object_identity);
    if object.is_empty() {
        return Err("Describe the object to generate.".into());
    }
    let pick = |user: String, hint: &str| {
        if user.is_empty() {
            hint.to_owned()
        } else {
            user
        }
    };
    let style = pick(
        clean(&intent.visual_style),
        preset.map_or("", |preset| preset.style_hint),
    );
    let topology = pick(
        clean(&intent.topology_intent),
        preset.map_or("", |preset| preset.topology_hint),
    );
    // Most important first, so word-boundary truncation drops the least important text.
    let sections = [
        ("", object),
        ("Structure", clean(&intent.structural_components)),
        ("Materials", clean(&intent.materials)),
        ("Style", style),
        ("Scale", clean(&intent.dimensions)),
        ("Pose", clean(&intent.pose)),
        ("Details", clean(&intent.details)),
        ("Topology", topology),
    ];
    let joined = sections
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(label, value)| {
            let value = value.trim_end_matches('.');
            if label.is_empty() {
                value.to_owned()
            } else {
                format!("{label}: {value}")
            }
        })
        .collect::<Vec<_>>()
        .join(". ");

    let exclusions = clean(&intent.exclusions);
    let mut terms: Vec<&str> = Vec::new();
    for term in exclusions
        .split(',')
        .chain(preset.map_or("", |preset| preset.negative_hint).split(','))
        .map(str::trim)
    {
        if !term.is_empty()
            && !terms
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(term))
        {
            terms.push(term);
        }
    }

    let mut truncated = false;
    let prompt = bound(&joined, limits.prompt_max_chars, &mut truncated);
    let negative_prompt = bound(
        &terms.join(", "),
        limits.negative_prompt_max_chars,
        &mut truncated,
    );
    Ok(BuiltPrompt {
        prompt,
        negative_prompt,
        truncated,
        sanitized,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: ProviderLimits = ProviderLimits {
        prompt_max_chars: 120,
        negative_prompt_max_chars: 30,
        image_max_bytes: 1,
        quad_max_faces: 1,
    };
    const WIDE: ProviderLimits = ProviderLimits {
        prompt_max_chars: 1024,
        negative_prompt_max_chars: 255,
        ..LIMITS
    };

    #[test]
    fn orders_sections_and_applies_preset_hints_only_when_user_left_them_blank() {
        let intent = PromptIntent {
            object_identity: "Wooden treasure chest".into(),
            materials: "oak, iron hinges".into(),
            ..PromptIntent::default()
        };
        let built = build_prompt(&intent, preset("low-poly"), &WIDE).unwrap();
        assert_eq!(
            built.prompt,
            "Wooden treasure chest. Materials: oak, iron hinges. Style: stylized low-poly. Topology: minimal faceted geometry"
        );
        let custom = PromptIntent {
            visual_style: "voxel".into(),
            ..intent
        };
        let built = build_prompt(&custom, preset("low-poly"), &WIDE).unwrap();
        assert!(built.prompt.contains("Style: voxel"));
        assert!(!built.prompt.contains("stylized low-poly"));
    }

    #[test]
    fn requires_object_and_strips_emoji() {
        assert!(build_prompt(&PromptIntent::default(), None, &LIMITS).is_err());
        let intent = PromptIntent {
            object_identity: "cat 🐱 statue\n\twith  hat".into(),
            ..PromptIntent::default()
        };
        let built = build_prompt(&intent, None, &LIMITS).unwrap();
        assert_eq!(built.prompt, "cat statue with hat");
        assert!(built.sanitized);
    }

    #[test]
    fn bounds_prompt_and_negative_at_word_boundaries() {
        let intent = PromptIntent {
            object_identity: "word ".repeat(60),
            exclusions: "blurry, low quality, broken mesh, extra limbs".into(),
            ..PromptIntent::default()
        };
        let built = build_prompt(&intent, preset("fantasy"), &LIMITS).unwrap();
        assert!(built.truncated);
        assert!(built.prompt.chars().count() <= LIMITS.prompt_max_chars);
        assert!(built.prompt.ends_with("word"));
        assert!(built.negative_prompt.chars().count() <= LIMITS.negative_prompt_max_chars);
        assert!(built.negative_prompt.starts_with("blurry, low quality"));
    }

    #[test]
    fn negative_terms_are_deduplicated() {
        let intent = PromptIntent {
            object_identity: "sword".into(),
            exclusions: "Modern technology, rust".into(),
            ..PromptIntent::default()
        };
        let built = build_prompt(&intent, preset("fantasy"), &WIDE).unwrap();
        assert_eq!(built.negative_prompt, "Modern technology, rust");
    }

    #[test]
    fn twelve_unique_presets() {
        let mut ids: Vec<_> = PRESETS.iter().map(|preset| preset.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 12);
    }
}
