//! Data-only 3D avatar contracts for AgentPet companions.
//!
//! This crate owns three things:
//! - [`CharacterSpec`]: the validated, data-only description of a procedural
//!   3D character. The desktop renderer builds geometry from it; the spec never
//!   contains code.
//! - [`catalog`]: the bundled library of original characters.
//! - [`sculpt`]: deterministic conversion of a user image into a textured 3D
//!   relief mesh (binary glTF) plus a stylized spec derived from its palette.

pub mod catalog;
pub mod glb;
pub mod sculpt;
pub mod spec;

pub use catalog::{AvatarCatalog, CatalogError};
pub use sculpt::{
    sculpt_image, sculpt_rgba, ImageSculptLimits, SculptError, SculptOutput, SculptStats,
    Segmentation,
};
pub use spec::{
    Accessory, BodyType, CharacterSpec, Ears, EyeStyle, HairStyle, Outfit, Palette, Pattern,
    Proportions, SpecError, Tail, Wings, SPEC_SCHEMA_VERSION,
};
