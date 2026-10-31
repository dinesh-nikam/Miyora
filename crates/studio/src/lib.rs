//! Miyora 3D Studio domain crate: provider abstraction, Tripo v3 provider,
//! task state machine, prompt building, input/output validation and the
//! local asset library. The desktop shell only adapts these to Tauri commands.

pub mod credentials;
pub mod decimate;
pub mod error;
pub mod export;
pub mod hf;
pub mod local;
pub mod media;
pub mod prompt;
pub mod provider;
pub mod service;
pub mod store;
pub mod task;
pub mod transport;
pub mod tripo;
