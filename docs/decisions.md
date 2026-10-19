# Architectural Decisions Log

## ADR-001: Monorepo Structure and Boundary Enforcement
- **Context**: AgentPet requires desktop integration, CLI hooks, local SQLite persistence, and optional cloud sync.
- **Decision**: Keep Rust domain crates (`agentpet-core`, `pet-engine`, `pet-pack`, `agentpet-cli`) independent of Tauri UI bindings. React presentation layer communicates strictly through typed Tauri IPC commands. SQLite persistence lives in Rust; React never accesses the database directly.
- **Consequences**: Business logic is pure and unit-testable without a display server or graphical harness.

## ADR-002: Transport Layer Separation
- **Context**: Hook latency must remain under 50ms without blocking host agent processes.
- **Decision**: Use a per-user Unix domain socket on macOS/Linux (`~/.agentpet/agentpet.sock`, chmod 0600) and a per-user named pipe (`\\.\pipe\agentpet-<user>`) on Windows. Hook forwarders exit 0 immediately on transport timeout or server absence.
- **Consequences**: CLI remains lightweight; daemon restarts or failures do not interrupt developer agent sessions.

## ADR-003: Tamagotchi Progression and XP Replay Safety
- **Context**: Events may be re-delivered during daemon recovery or offline replay.
- **Decision**: XP transactions are strictly immutable and keyed to deterministic event UUIDs. Formula: Cumulative XP for Level L = 50 * L * (L + 1). Token XP = 1 XP per 5,000 tokens burned (cache reads scaled to 10%). Level starts at 1 (0 XP) as Hatchling.
- **Consequences**: Idempotent event writes prevent double-granting XP across re-runs.

## ADR-004: Forward-Only Cloud Synchronization
- **Context**: Offline-first devices must reconcile stats with the optional cloud backend without data loss.
- **Decision**: Synchronize by taking `max(xp)`, `max(tokens)`, `max(sessions)` and the mathematical set union of achievements. Level is strictly recomputed from the merged XP value, preventing decoupled state anomalies.
- **Consequences**: Zero conflict resolution forks; progress cannot regress regardless of synchronization order.

## ADR-005: Unified Cross-Platform Desktop Stack on Tauri 2 Instead of Native Swift 6
- **Context**: Section 2 of the prompt suggested native Swift 6 / SwiftUI / AppKit on macOS with Sparkle, and Tauri on Windows/Linux.
- **Decision**: Standardize on Tauri 2 across macOS, Windows, and Linux with a single React + TypeScript frontend and Rust backend.
- **Consequences**:
  - Drops native Swift 6, AppKit status item, and Sparkle auto-updater in favor of Tauri 2 native tray, window management, and Tauri updater across all three platforms.
  - Eliminates duplicate state machines, UI implementations, and protocol parsing in Swift.
  - Ensures 100% UI feature parity, shared pixel pet rendering, and identical testing harnesses across all targets at substantially lower maintenance overhead.
