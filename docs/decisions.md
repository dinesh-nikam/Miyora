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

## ADR-006: 3D Anime Character Support via VRM Standard and WebGL/three-vrm
- **Context**: Users want an anime-style desktop companion alongside 2D sprites, with the ability to bring their own custom models (e.g., from VRoid Studio). The app must ship with an original, non-infringing anime character (avoiding Nezuko's signature outfit, colors, and props) while sandboxing untrusted user-supplied VRM files.
- **Decision**: Standardize 3D avatar support on the VRM (glTF 2.0) specification rendered via WebGL (`@pixiv/three-vrm`) in the Tauri web UI.
  - **Original Character**: The repository ships a single clean-room original anime character with unique hair, attire, and palette, completely detached from any copyrighted IP.
  - **Bring Your Own Model (BYOM)**: Users can load local `.vrm` files. License and author metadata embedded in the VRM header (`VRMC_vrm` or `VRM` extension) are extracted and displayed in the UI.
  - **Behavior & State Parity**: 3D models respond to the identical finite state machine (`idle`, `working`, `waiting`, `done`, `level-up`, `evolving`, `hungry`, `sleeping`) via humanoid bone clips and blendshape expressions. Speech bubbles dynamically anchor to the projected 2D coordinates of the model's `head` bone.
  - **Security & Guardrails**: VRM parsing occurs off-thread in a dedicated worker. Models are rejected if they contain external URIs (`http://`, `https://`, or local path references), exceed 50 MB file size, exceed 100,000 triangles, or exceed 2048x2048 texture dimensions.
  - **Resource Throttling**: Rendering caps at 30 FPS when active, dropping to 1 FPS or pausing when idle or occluded, satisfying the <1% CPU budget.
- **Consequences**: Enables rich 3D anime avatars with full state parity while enforcing zero network leakage, robust IP compliance, and strict memory/CPU bounds.

