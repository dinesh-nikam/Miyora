# Development Guide

This document describes the local development workflow, quality gates, workspace layout, and testing standards for **Miyora** (AgentPet).

---

## 1. System Requirements & Toolchains

- **Rust:** Stable toolchain 1.75+ (or 1.88+ as specified in `Cargo.toml`).
- **Node.js:** v18.0.0+ (v20+ or v24+ recommended) and `npm`.
- **Python:** 3.10+ (for catalog validation scripts).
- **Operating System:**
  - **Windows (x86_64):** GNU (`stable-x86_64-pc-windows-gnu` with WinLibs MinGW) or MSVC (`stable-x86_64-pc-windows-msvc` with Visual C++ Build Tools).
  - **macOS (aarch64 / x86_64):** Xcode Command Line Tools.
  - **Linux (x86_64):** `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`.

---

## 2. Workspace Architecture

Miyora is organized as a Cargo workspace with an accompanying Tauri 2 React frontend:

```
miyora (workspace root)
├── apps/desktop/                  # Tauri 2 Desktop Shell (Rust host + React strict TypeScript)
│   ├── src/                       # Command Deck, Tabs, and transparent overlay UI
│   ├── src/avatar/                # 3D Character Studio, Three.js procedural rigs & shaders
│   └── src-tauri/                 # Window lifecycle, capabilities, local IPC listeners
├── crates/agentpet-core/          # SQLite persistence, session lifecycle, immutable XP ledger
├── crates/pet-engine/             # 15-state deterministic behavior engine & transition allow-list
├── crates/agentpet-adapters/      # Claude Code / Codex hooks & universal process wrapper
├── crates/pet-pack/               # Ed25519 signature checks, manifest parser, image normalization
├── crates/avatar/                 # Procedural 3D character catalog (214 specs), silhouette sculpting, GLB export
├── crates/studio/                 # Unified 3D avatar generation (Tripo/TripoSR), decimation & retopology
├── crates/agentpet-ipc/           # Local authenticated IPC transport (pipes/sockets)
└── crates/agentpet-cli/           # Diagnostic CLI (`agentpet`) for headless execution and doctor checks
```

---

## 3. Local Development Workflows

### 3.1 Desktop Application

```bash
cd apps/desktop
npm install
npm run tauri dev
```

The desktop shell serves two windows:
- **Main Window (Command Deck):** Primary dashboard at `http://127.0.0.1:1420`.
- **Overlay Window (`pet-overlay`):** Transparent companion overlay loaded via `index.html?overlay=1`.

### 3.2 CLI Headless Execution

```bash
# Build the CLI
cargo build -p agentpet-cli

# Run system diagnostics
cargo run -p agentpet-cli -- doctor

# Inspect pet runtime state
cargo run -p agentpet-cli -- status

# Verify cryptographic SHA-256 XP ledger
cargo run -p agentpet-cli -- verify-ledger

# Wrap any agent command
cargo run -p agentpet-cli -- run -- claude
```

---

## 4. Verification & Quality Gates

Always run the full suite of verification commands before committing changes:

### 4.1 Rust Workspace Tests & Lints

```bash
# Formatting check
cargo fmt --all -- --check

# Strict linting (warnings are treated as errors)
cargo clippy --workspace --all-targets -- -D warnings

# Strict linting including developer simulator
cargo clippy --workspace --all-features --all-targets -- -D warnings

# Run all workspace unit and integration tests
cargo test --workspace

# Run single test
cargo test -p agentpet-core <test_name_substring>
```

### 4.2 Desktop Frontend Tests & Build

```bash
cd apps/desktop

# Typecheck and production bundle build with bundle budget check
npm run build

# Run Vitest test suites (512+ tests)
npm test
```

### 4.3 Python Validation Scripts

```bash
# Verify 3D avatar catalog matches character specifications (214 characters)
python scripts/generate_avatar_catalog.py --check

# Validate bundled Character Card V2 profiles against the avatar catalog
python scripts/validate_character_cards.py

# Validate all 47 bundled PetPacks under pet-packs/
python scripts/validate_pet_packs.py
```

---

## 5. Developer Simulator Feature Flag

To test event state transitions, reactions, and overlay rendering without launching a live coding session, Miyora provides an isolated developer simulator:

- **Cargo Feature:** `--features dev-simulator`
- **UI Banner:** `[SIMULATED - DEV ONLY]` banner rendered only in development mode (`import.meta.env.DEV`).
- **Isolation:** Operates against an isolated SQLite test database (`agentpet_simulated.sqlite3`).
- **Integrity Guarantee:** Exactly zero XP or achievements are awarded to the live ledger.

---

## 6. Git & Commit Conventions

- Use Conventional Commits (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`).
- Always run GitNexus impact analysis prior to editing symbols.
- Never commit secrets, credentials, or generated temporary build artifacts.
