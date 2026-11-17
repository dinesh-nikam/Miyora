//! Miyora Verification & Reliability Test Harness
//!
//! Provides deterministic property-testing generators, replay execution engine,
//! golden snapshot comparators, fault injectors, and privacy canary checkers.
//!
//! This crate is strictly for testing and verification; it must never be linked
//! into production or release builds.

pub mod generators;
pub mod replay;
