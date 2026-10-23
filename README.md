<p align="center">
  <img src="assets/branding/miyora-hero.svg" alt="Miyora — Agent behavior, made visible" width="100%" />
</p>

<p align="center">
  <a href="#features"><img src="https://img.shields.io/badge/Tauri-v2.0-24C8D8?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri 2" /></a>
  <a href="#architecture"><img src="https://img.shields.io/badge/Rust-1.75%2B-dea584?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" /></a>
  <a href="#features"><img src="https://img.shields.io/badge/React-18-61DAFB?style=for-the-badge&logo=react&logoColor=black" alt="React 18" /></a>
  <a href="#features"><img src="https://img.shields.io/badge/TypeScript-Strict-3178C6?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript Strict" /></a>
  <a href="#architecture"><img src="https://img.shields.io/badge/SQLite-Local_Core-003B57?style=for-the-badge&logo=sqlite&logoColor=white" alt="SQLite" /></a>
  <a href="#security--privacy"><img src="https://img.shields.io/badge/Telemetry-Zero%20%7C%20Offline_First-10b981?style=for-the-badge" alt="Privacy First" /></a>
  <a href="#quality-gates--test-suite"><img src="https://img.shields.io/badge/Tests-337_Passing-success?style=for-the-badge" alt="Tests" /></a>
</p>

---

### **Agent behavior, made visible.**

**Miyora** is an offline-first, native desktop companion that tracks AI coding agent activity in real time. It observes actual process lifecycle and tool invocations, translates those signals into an authoritative 15-state behavior engine, and renders responsive 3D procedural avatars and animated sprite companions over a transparent desktop overlay.

Session evidence, progression metrics, and XP transactions are committed to an immutable local SQLite ledger with zero cloud dependencies and zero telemetry.

---

## 📑 Table of Contents

- [Key Capabilities](#-key-capabilities)
- [System Architecture](#-system-architecture)
- [Event Pipeline & Hook Subsystem](#-event-pipeline--hook-subsystem)
- [Pet State Machine](#-pet-state-machine)
- [Companion Universe & PetPacks](#-companion-universe--petpacks)
- [CLI Reference](#-cli-reference)
- [Quick Start & Installation](#-quick-start--installation)
- [Quality Gates & Test Suite](#-quality-gates--test-suite)
- [Security, Privacy & Sandboxing](#-security-privacy--sandboxing)
- [Technical Documentation](#-technical-documentation)
- [License & Attribution](#-license--attribution)

---

## ⚡ Key Capabilities

| Pillar | Description |
| :--- | :--- |
| **🖥️ Native Desktop Shell** | Powered by **Tauri 2** and React strict TypeScript. Features an interactive main management window and a transparent, non-intrusive, always-on-top pet overlay with persistent screen coordinates. |
| **👁️ Real Agent Observation** | Real-time observation through zero-overhead native hooks (**Claude Code**, **Codex**) or the universal process wrapper (`agentpet run -- <command>`) for any agent (Aider, OpenCode, Gemini CLI). No synthetic or mock activity. |
| **🧠 Deterministic Finite State Machine** | 15-state priority-based reactive engine (`pet-engine`) enforcing strict transition allow-lists. Distinguishes active execution, deep reasoning, human attention requests, errors, and task victories. |
| **🎭 3D Studio & 2D PetPacks** | Includes **167 procedural 3D characters** across 5 body archetypes with customizable rigs (ears, tails, auras, props, gradient hair), local silhouette-inflation sculpting from 2D images, VRM imports, and **47+ standalone PetPacks**. |
| **🛡️ Tamper-Safe Progression** | Idempotent local XP ledger awarding XP strictly on verified child process completion. Session XP, level scaling, and milestone achievements are committed together in atomic SQLite transactions. |
| **🔒 Air-Gapped Privacy** | No cloud account required. Zero background telemetry. Prompts, raw tool outputs, API keys, and sensitive filesystem paths are scrubbed before persistence. |

---

## 🏗️ System Architecture

Miyora enforces strict unidirectional boundaries. Rust domain crates own all authoritative state, persistence, event normalization, and crypto verification. The React UI accesses business logic exclusively through narrow, capability-scoped Tauri IPC commands.

<p align="center">
  <img src="assets/diagrams/architecture.svg" alt="Miyora System Architecture" width="100%" />
</p>

### Domain & Crate Boundaries

```
miyora (workspace root)
├── apps/desktop/                  # Tauri 2 Desktop Shell (Rust host + React strict TS)
│   ├── src/                       # Main Command Deck & transparent overlay UI
│   ├── src/avatar/                # 3D Character Studio, Three.js procedural rigs & shaders
│   └── src-tauri/                 # Window lifecycle, capabilities, local IPC listeners
├── crates/agentpet-core/          # SQLite persistence, session lifecycle, immutable XP ledger
├── crates/pet-engine/             # 15-state reactive behavior engine & transition allow-list
├── crates/agentpet-adapters/      # Claude Code / Codex hooks & universal process wrapper
├── crates/pet-pack/               # Ed25519 signature checks, manifest parser, image normalization
└── crates/agentpet-cli/           # Diagnostic CLI (`agentpet`) for headless execution and doctor checks
```

---

## 🔄 Event Pipeline & Hook Subsystem

Miyora connects to AI coding agents via low-latency, deterministic event normalization. Events persist to the local write-ahead log before acknowledgement.

<p align="center">
  <img src="assets/diagrams/event-pipeline.svg" alt="Miyora Event Pipeline" width="100%" />
</p>

1. **Native Hooks (`Claude Code`, `Codex`)**: Installed reversibly without mutating unrelated user settings. Submits JSON payloads to the CLI hook runner.
2. **Universal Child Wrapper (`agentpet run`)**: Spawns real child processes, bidirectionally streams stdout/stderr, measures duration, and captures authentic exit codes.
3. **Privacy Scrubbing & Normalization**: Strips project directory paths, tool arguments containing secrets, and raw prompts into normalized events stamped with deterministic UUIDv5 identifiers.
4. **Local IPC (v1)**: High-speed length-prefixed transport via named pipes (Windows) or Unix domain sockets with shared-secret handshake and 256 KiB frame limits.

---

## 🎮 Pet State Machine

The companion’s mood and posture directly mirror real agent workload. Transitions are governed by an explicit state matrix in `crates/pet-engine`:

<p align="center">
  <img src="assets/diagrams/pet-state-machine.svg" alt="Pet State Machine" width="100%" />
</p>

### State Hierarchy & Response Triggers

| Priority | State | Trigger Condition | Visual Behavior |
| :---: | :--- | :--- | :--- |
| **P0** | `Error` | Non-zero exit code or adapter failure notification | Agitated posture, alert icon, error reaction |
| **P1** | `AttentionRequired` | Tool approval prompt or interactive confirmation needed | Waving, beacon emote, waiting for developer input |
| **P2** | `Celebrating` / `Success` | Process completed with exit code 0 | Cheerful celebration, victory particle burst |
| **P3** | `Thinking` | Pre-tool reasoning, model inference phase | Contemplative pose, gear/spark particle pulse |
| **P4** | `Working` | Tool execution, file write, compilation, shell command | Focused typing, weapon/wand wield, energetic stance |
| **P5** | `Idle` / `Sleeping` | Inactivity timeout (> 15 minutes of quiet session) | Relaxed breathing, seated position, gentle Zzz aura |

---

## 🐾 Companion Universe & PetPacks

Miyora ships with rich 2D sprite packs and a comprehensive procedural 3D avatar engine. Packs are completely data-driven: manifests declare author attribution, license metadata, frame dimensions, and state mappings.

### Featured Companions (47+ Built-in Packs)

<table align="center">
  <tr>
    <td align="center" width="16%">
      <img src="pet-packs/nezuko/assets/thumbnail.png" width="96px" alt="Nezuko" /><br />
      <b>Nezuko</b><br />
      <sub>Anime Archetype</sub>
    </td>
    <td align="center" width="16%">
      <img src="pet-packs/arjuna/assets/thumbnail.png" width="96px" alt="Arjuna" /><br />
      <b>Arjuna</b><br />
      <sub>Mythic Archer</sub>
    </td>
    <td align="center" width="16%">
      <img src="pet-packs/gojo/assets/thumbnail.png" width="96px" alt="Gojo" /><br />
      <b>Gojo</b><br />
      <sub>Special Grade</sub>
    </td>
    <td align="center" width="16%">
      <img src="pet-packs/frieren/assets/thumbnail.png" width="96px" alt="Frieren" /><br />
      <b>Frieren</b><br />
      <sub>Mage Companion</sub>
    </td>
    <td align="center" width="16%">
      <img src="pet-packs/denji/assets/thumbnail.png" width="96px" alt="Denji" /><br />
      <b>Denji</b><br />
      <sub>Hybrid Hunter</sub>
    </td>
    <td align="center" width="16%">
      <img src="pet-packs/anya/assets/thumbnail.png" width="96px" alt="Anya" /><br />
      <b>Anya</b><br />
      <sub>Esper Chibi</sub>
    </td>
  </tr>
</table>

### 3D Character Studio Capabilities

- **167 Procedural Characters**: 5 body types, 30 popular archetypes, and 25 pocket chibi creatures.
- **Parametric Customization**: 12 ear types, 10 dynamic tails (flame, star, leaf), 12 gradient hairstyles, 15 outfits, 20 accessories, and energy aura customization.
- **Geometric Image-to-3D Sculpting**: Local silhouette-inflation sculpts flat images into watertight textured GLB models without sending photos to cloud APIs.
- **VRM 1.0 Import**: Safe avatar loading with polygon-budget guardrails, metadata inspector, and humanoid retargeting.

---

## 💻 CLI Reference

The unified command-line tool `agentpet` handles headless operation, automated wrapping, diagnostics, and hook integration:

```bash
# Verify system dependencies, database integrity, and installed agent hooks
cargo run -p agentpet-cli -- doctor

# Wrap any agent command with automatic telemetry capture and XP allocation
cargo run -p agentpet-cli -- run -- claude-code

# Inspect current companion state, active sessions, and XP progression
cargo run -p agentpet-cli -- status

# List and inspect installed PetPacks
cargo run -p agentpet-cli -- pets list

# Install native agent adapter hooks
cargo run -p agentpet-cli -- agents install claude_code
```

### Full Command Specification

| Command | Arguments | Purpose |
| :--- | :--- | :--- |
| `agentpet doctor` | — | Validates local SQLite accessibility, Tauri paths, and discovers installed agent CLIs (`claude`, `codex`, `aider`, `gemini`). |
| `agentpet run` | `-- <cmd> [args...]` | Spawns child process, redirects stdio streams, records execution metrics, and awards XP on success. |
| `agentpet status` | — | Outputs currently selected companion, current level, lifetime XP, and active session status. |
| `agentpet pets` | `list` | Lists all local packs, signature validity, and highlights the active companion. |
| `agentpet agents` | `list` | Displays supported agent adapters and live registration status. |
| `agentpet agents` | `install <id>` | Registers native hook for `claude_code` or `codex` with reversible configuration backups. |
| `agentpet agents` | `remove <id>` | Safely removes hook configuration without altering unrelated user settings. |
| `agentpet hook` | `--adapter <id>` | Headless ingest parsing agent hook payloads over stdin into the normalized core. |

---

## 🚀 Quick Start & Installation

### Prerequisites

- **Rust**: `1.75+` (with `cargo` and `clippy`)
- **Node.js**: `18.0+` (LTS recommended)
- **C++ Build Tools**: Platform-specific C++ compiler (MSVC on Windows, Xcode on macOS, build-essential on Linux)

### Setup & Launch

```powershell
# 1. Clone the repository
git clone https://github.com/dinesh-nikam/Miyora.git
cd Miyora

# 2. Install desktop dependencies
cd apps/desktop
npm install

# 3. Launch desktop companion in development mode
npm run tauri:dev
```

### Headless CLI Setup

```powershell
# Run the diagnostic doctor from workspace root
cargo run -p agentpet-cli -- doctor
```

---

## 🧪 Quality Gates & Test Suite

All code in Miyora adheres to strict static verification and testing thresholds:

```powershell
# Execute Rust test suite across all workspace crates
cargo test --workspace
# Output: 112 passed, 0 failed

# Run frontend unit & avatar rig tests
cd apps/desktop
npx vitest run
# Output: 225 passed, 0 failed

# Validate strict static analysis & formatting
cargo clippy --workspace --all-features --all-targets -D warnings
cargo fmt --check --all
npm run build
```

---

## 🔐 Security, Privacy & Sandboxing

Miyora is designed with zero-trust architectural boundaries:

1. **Air-Gapped Local Storage**: All sessions, events, and progression live in an offline SQLite database accessed exclusively by Rust domain code.
2. **Untrusted Pack Sandboxing**: PetPacks contain only declarative assets (JSON, PNG, GLB, SVG). Pack scripts and native binaries are strictly rejected.
3. **Payload Sanitization**: File contents, terminal buffers, API tokens, and sensitive system credentials are never captured, logged, or uploaded.
4. **Idempotent Ledgers**: XP transactions require cryptographically safe idempotency keys, preventing replay attacks or inflated leveling.

---

## 📚 Technical Documentation

Deep dive into the architecture and specifications:

- 📐 [ARCHITECTURE.md](ARCHITECTURE.md) — Comprehensive technical stack, data flow, and IPC boundary designs.
- 📦 [PET_PACK_SPEC.md](PET_PACK_SPEC.md) — Data-only PetPack schema, animation mapping, and signature protocol.
- 🎨 [docs/specs/avatar_3d.md](docs/specs/avatar_3d.md) — 3D avatar catalog, procedural joint rigging, and silhouette sculpting.
- 🔌 [AGENT_PROVIDER_SPEC.md](AGENT_PROVIDER_SPEC.md) — Agent adapter specifications, hook payloads, and child wrapper contracts.
- 🗄️ [DATABASE.md](DATABASE.md) — Local SQLite schema, migration policies, and indexing rules.
- 📡 [API.md](API.md) — IPC message format, shared secret handshakes, and wire frames.
- 🛡️ [SECURITY.md](SECURITY.md) & [PRIVACY.md](PRIVACY.md) — Threat model, redaction rules, and credential handling.
- 🤝 [CONTRIBUTING.md](CONTRIBUTING.md) — Contribution guidelines and development workflows.

---

## 📄 License & Attribution

Distributed under dual **MIT** / **Apache-2.0** licensing. See [LICENSES.md](LICENSES.md) for details.

Miyora character models, original art packs, and procedural rig assets are distributed under Creative Commons attribution licenses as specified in their respective pack manifests.

<p align="center">
  <img src="assets/branding/miyora-logo.svg" alt="Miyora Mark" width="280px" />
</p>
