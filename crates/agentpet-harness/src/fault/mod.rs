//! Fault injection utilities for Miyora persistence boundaries, SQLite lock contention,
//! and IPC framing boundaries.
//!
//! Simulates process crashes before write, mid-transaction aborts, post-commit retry storms,
//! database concurrency lockouts (`SQLITE_BUSY`), and malformed/truncated IPC transport frames.

pub mod db_locks;
pub mod ipc_framing;
pub mod persistence;
