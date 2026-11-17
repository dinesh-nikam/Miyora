# Miyora Verification & Reliability Test Harness

This document specifies the architecture, operation, coverage matrix, and reproduction workflows for the Miyora Verification Test Harness (`agentpet-harness`).

---

## 1. Overview & Architectural Isolation

The verification harness proves Miyora's core product truth: **Real agent activity becomes correct, durable, non-duplicated state; nothing is lost, invented, or leaked.**

### Production Isolation Guarantee (`MYR-HARN-006`)
- `agentpet-harness` is declared with `publish = false` in `crates/agentpet-harness/Cargo.toml`.
- It is strictly an internal workspace test member and is **never** linked or referenced as a dependency in production crates (`agentpet-cli`, `agentpet-core`, `agentpet-desktop`).
- Enforced by automated test `crates/agentpet-harness/tests/isolation.rs` which verifies `cargo tree` graphs.

---

## 2. Implemented Layers & Test Execution

### Layer 1: Property Testing (Proptest)
Tests invariants across arbitrary input domains, state stimulus permutations, and payload fuzzer streams.

| Test File | Requirement | Invariant Verified |
| :--- | :--- | :--- |
| `tests/l1_property_xp.rs` | `MYR-HARN-001` | **XP & Achievement Idempotency**: Arbitrary duplicate event deliveries and interleavings produce identical final XP, level, and ledger entries as the canonical sequence. |
| `tests/l1_property_state.rs` | `MYR-HARN-002` | **State Machine Transition Invariant**: For all 15 states $\times$ stimuli $\times$ contexts, `engine.decide()` strictly enforces the transition allowlist. Paused state strictly resists non-critical interruptions. |
| `tests/l1_property_normalization.rs` | `MYR-HARN-003` | **Adapter Parsing Non-Panic**: Arbitrary byte arrays, malformed JSON, and control noise delivered to Claude Code and Codex hook parsers never panic and reject oversized inputs (>256KB) with typed errors. |

**Command to run Layer 1:**
```bash
cargo test -p agentpet-harness --test l1_property_xp --test l1_property_state --test l1_property_normalization
```

### Layer 2: Replay Engine & Golden Snapshot Comparator
Replays real agent session event flows against SQLite and validates against a canonical, normalized snapshot.

| Test File | Requirement | Invariant Verified |
| :--- | :--- | :--- |
| `tests/l2_replay_snapshots.rs` | `MYR-HARN-004` | **Golden Snapshot Validation**: Replaying `fixtures/golden_session_flow.json` on a clean database produces a database state byte-identical to `fixtures/golden_snapshot.json`. |
| `tests/l2_replay_snapshots.rs` | `MYR-HARN-005` | **Replay Invariance**: Replaying the identical event log twice into the same DB, or replaying full logs into a half-populated DB, results in a state strictly identical to a single run. |

**Command to run Layer 2:**
```bash
cargo test -p agentpet-harness --test l2_replay_snapshots
```

### Layer 3: Fault Injection Boundaries
Simulates system crashes, transaction rollbacks, concurrent database lockouts, and adversarial IPC frame corruptions.

| Test File | Requirement | Invariant Verified |
| :--- | :--- | :--- |
| `tests/l3_fault_boundaries.rs` | `MYR-HARN-007` | **Mid-Transaction Crash Rollback**: Process termination mid-transaction leaves zero partial rows, zero phantom XP, and rolls back cleanly without database corruption. |
| `tests/l3_fault_boundaries.rs` | `MYR-HARN-008` | **Post-Commit Retry Invariance**: Retrying requests after commit (due to dropped ACK) is strictly idempotent; awards zero duplicate XP, zero duplicate achievements, and maintains valid hash chain. |
| `tests/l3_fault_boundaries.rs` | `MYR-HARN-009` | **SQLite Lock Contention & Recovery**: Concurrently locked SQLite database (`SQLITE_BUSY`) recovers cleanly within the busy timeout window; timeouts return typed `CoreError::Database` errors without panics or deadlocks. |
| `tests/l3_fault_boundaries.rs` | `MYR-HARN-010` | **IPC Truncated Header**: Frames with < 4 header bytes are rejected with typed `IpcError::MalformedFrame`. |
| `tests/l3_fault_boundaries.rs` | `MYR-HARN-011` | **IPC Oversized Frame**: Frames exceeding `max_frame_bytes` are rejected before buffer allocation with `IpcError::FrameTooLarge`. |
| `tests/l3_fault_boundaries.rs` | `MYR-HARN-012` | **IPC Zero-Length Payload**: Frames declaring 0 payload bytes are rejected with `IpcError::MalformedFrame`. |
| `tests/l3_fault_boundaries.rs` | `MYR-HARN-013` | **IPC Truncated Payload**: Payload streams ending prematurely before declared length return `IpcError::TransportUnavailable`. |
| `tests/l3_fault_boundaries.rs` | `MYR-HARN-014` | **IPC Handshake Security**: Tampered HMAC tags reject with `IpcError::AuthenticationFailed`; unsupported versions reject with `IpcError::ProtocolVersionUnsupported`. |

**Command to run Layer 3:**
```bash
cargo test -p agentpet-harness --test l3_fault_boundaries
```

### Production Isolation Guard
```bash
cargo test -p agentpet-harness --test isolation
```

**Run All Harness Tests:**
```bash
cargo test -p agentpet-harness
```

---

## 3. Failure Reproduction by Seed

Every property test runs under `proptest`. When a property test fails, `proptest` prints the failing input and a deterministic regression seed:

```text
proptest: Saving this and future failures in crates/agentpet-harness/tests/l1_property_state.proptest-regressions
cc 87d82ca17f7dd304809f7410dfee2366c723a950ef076dbb962e56557e01152e
```

To deterministically reproduce a failure using the printed seed:
```bash
# Set PROPTEST_SEED and rerun the specific test:
PROPTEST_SEED="87d82ca17f7dd304809f7410dfee2366c723a950ef076dbb962e56557e01152e" cargo test -p agentpet-harness --test l1_property_state
```

---

## 4. How to Add a New Fixture

### 1. Adding an Event Stream Recording
1. Create a JSON file in `crates/agentpet-harness/fixtures/<fixture_name>.json`.
2. Format as an array of `HookEvent` objects:
```json
[
  {
    "id": "<UUIDv4>",
    "eventType": "agent_started",
    "agentId": "claude_code",
    "sessionId": "<UUIDv4>",
    "projectPath": "/path/to/project"
  }
]
```
*Note:* `id` must be a valid RFC 4122 UUID string, as mandated by `agentpet-core`.

### 2. Capturing a Golden Snapshot
To record the golden baseline for a new event flow:
```rust
let temp_dir = tempfile::tempdir().unwrap();
let core = Core::open(temp_dir.path()).unwrap();
replay_hook_events(&core, &events).unwrap();
let snapshot = snapshot_from_data_dir(temp_dir.path()).unwrap();
let json = serde_json::to_string_pretty(&snapshot).unwrap();
std::fs::write("crates/agentpet-harness/fixtures/<snapshot_name>.json", json).unwrap();
```

---

## 5. Coverage Matrix

| Invariant / Tenet | Layer | Test Name | Status |
| :--- | :--- | :--- | :--- |
| **XP & Achievement Idempotency** | L1 | `prop_xp_and_achievements_are_strictly_idempotent` | **COVERED (GREEN)** |
| **XP Idempotency Mutation Sanity** | L1 | `mutation_sanity_check_fails_when_idempotency_broken` | **COVERED (GREEN)** |
| **State Machine Allowlist Safety** | L1 | `prop_state_transitions_never_violate_allow_list` | **COVERED (GREEN)** |
| **State Machine Mutation Sanity** | L1 | `mutation_sanity_check_fails_on_illegal_transition` | **COVERED (GREEN)** |
| **Claude Adapter Fuzzing & Cap** | L1 | `prop_claude_parser_never_panics_on_arbitrary_input` | **COVERED (GREEN)** |
| **Codex Adapter Fuzzing & Cap** | L1 | `prop_codex_parser_never_panics_on_arbitrary_input` | **COVERED (GREEN)** |
| **Noisy JSON Normalization** | L1 | `prop_claude_parser_handles_noisy_json`, `prop_codex_parser_handles_noisy_json` | **COVERED (GREEN)** |
| **Parser Crash Mutation Sanity** | L1 | `mutation_sanity_check_fails_if_parser_panics` | **COVERED (GREEN)** |
| **Golden Snapshot Verification** | L2 | `test_replay_matches_golden_snapshot` | **COVERED (GREEN)** |
| **Replay-Twice Idempotence** | L2 | `test_replay_twice_is_strictly_identical` | **COVERED (GREEN)** |
| **Half-Populated DB Replay** | L2 | `test_replay_into_half_populated_db_is_strictly_identical` | **COVERED (GREEN)** |
| **Snapshot Divergence Mutation Sanity**| L2 | `mutation_sanity_check_fails_on_snapshot_divergence` | **COVERED (GREEN)** |
| **Production Dependency Isolation** | Isolation | `test_harness_is_never_linked_into_production_crates` | **COVERED (GREEN)** |
| **Mid-Tx Crash Rollback** | L3 | `test_persistence_mid_transaction_crash_leaves_zero_traces` | **COVERED (GREEN)** |
| **Post-Commit Retry Invariance** | L3 | `test_persistence_post_commit_retry_is_strictly_idempotent` | **COVERED (GREEN)** |
| **SQLite Lock Recovery** | L3 | `test_sqlite_locked_db_recovers_gracefully` | **COVERED (GREEN)** |
| **SQLite Lock Timeout Error** | L3 | `test_sqlite_persisted_lock_timeout_returns_typed_busy_error` | **COVERED (GREEN)** |
| **IPC Truncated Header Rejection** | L3 | `test_ipc_truncated_header_rejects_without_panic` | **COVERED (GREEN)** |
| **IPC Oversized Frame Rejection** | L3 | `test_ipc_oversized_frame_rejects_without_buffering` | **COVERED (GREEN)** |
| **IPC Zero-Length Payload** | L3 | `test_ipc_empty_payload_rejects_with_typed_error` | **COVERED (GREEN)** |
| **IPC Truncated Payload** | L3 | `test_ipc_truncated_payload_rejects_gracefully` | **COVERED (GREEN)** |
| **IPC Tampered HMAC Rejection** | L3 | `test_ipc_handshake_tampered_hmac_rejects_auth` | **COVERED (GREEN)** |
| **IPC Unsupported Version Rejection**| L3 | `test_ipc_handshake_unsupported_version_rejects` | **COVERED (GREEN)** |
| **IPC Framing Mutation Sanity** | L3 | `mutation_sanity_check_fails_if_corrupted_frame_is_accepted` | **COVERED (GREEN)** |

---

## 6. Uncovered Invariants & Roadmap (Layers 4–8)

The following layers were defined in the verification harness architecture and are scheduled for successive implementation slices:

1. **Layer 4: Historical Schema Migration Suite (L4)**
   - *Scope:* Frozen `.sqlite3` binary fixtures for schemas v1 through v11 asserting foreign key integrity, XP preservation, and ledger continuity when migrated to v12.
   - *Status:* Requires capturing binary snapshots of past migrations.
2. **Layer 5: Adversarial PetPack Corpus (L5)**
   - *Scope:* Automated rejection of zip bombs, nested executables/scripts (`.exe`, `.sh`, `.dll`), symlink escapes, and Unicode case collisions.
   - *Status:* Scheduled following L3.
3. **Layer 6: Security & Privacy Canary Tracing (L6)**
   - *Scope:* Injected canary strings into prompts, paths, and environment variables asserted absent from SQLite persistence, diagnostics logs, and IPC frames.
   - *Status:* Scheduled following L5.
4. **Layer 7: Headless Desktop E2E (L7)**
   - *Scope:* Automated desktop profile driver asserting visual companion state against SQLite state.
   - *Status:* Requires headless Tauri test runner setup.
5. **Layer 8: CI Ordered Gates & Reporting (L8)**
   - *Scope:* Integrating `npm test` into `.github/workflows/ci.yml`, ordered test gating, and JUnit test output.
   - *Status:* Scheduled for CI hardening slice.
