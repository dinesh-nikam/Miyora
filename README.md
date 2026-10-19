# AgentPet Platform

An offline-first desktop companion platform for coding-agent activity, configurable virtual pets, and local developer-session progression.

> **Project status:** Foundation and first local runtime slices are implemented: Tauri/React UI, Rust SQLite event/session core, the universal process wrapper, extracted-folder PetPack import, validated static user-image creation, generated previews, sprite playback, a transparent desktop overlay with persisted position, an explicit priority-based pet state machine with timed reactions and data-driven animation fallback, session-completion XP, local session/XP history, and local completed-session achievements. Native agent hooks/adapters, animated image extraction, archive extraction/signature verification, cloud services, and installer/update pipelines remain unimplemented. The repo does not bundle Nezuko artwork; the starter pack must carry clear licensing.

The local progression slice captures the selected pet when a session starts and awards configured XP only after the child process exits successfully. The XP transaction, session completion, and resulting level are committed together. Failed/interrupted commands award no XP; token usage remains null unless a real provider adapter supplies measured counts.

## Product goals

- Render an optional always-on-top desktop companion with configurable behavior.
- Observe coding agents through real hooks/adapters or `agentpet run -- <command>`.
- Keep pet state, sessions, settings, and progression locally available without login.
- Treat characters as replaceable, data-only PetPacks rather than application code.
- Make community library, profile, leaderboard, and sync optional cloud features.

## Architecture

The target stack is Tauri 2 and Rust for the desktop/runtime boundary, React with strict TypeScript for the UI, and SQLite for local persistence. Rust owns authoritative event handling, pet state, session lifecycle, progression, and pack validation. The UI accesses these through explicit Tauri commands and typed events.

Optional cloud components use a NestJS modular monolith and PostgreSQL. Cloud services never gate local startup or core functionality. See [ARCHITECTURE.md](ARCHITECTURE.md) for diagrams, boundaries, security rules, and delivery phases.

## Planned workspace

```text
apps/desktop/       Tauri desktop app and React UI
apps/web/           Optional community/dashboard frontend
crates/             Rust runtime, domain, database, CLI, and platform crates
services/           Agent daemon, asset worker, optional sync worker
cloud/api/          Optional NestJS API
packages/           Shared contracts, UI, and localization
pet-packs/          Licensed data-only starter packs
infra/              CI, local cloud development, and deployment definitions
docs/               Product and engineering specifications
```

The layout is a target, not a list of already-created modules.

## Run the desktop shell

Prerequisites: Rust and Cargo, Node.js with npm, and the platform dependencies required by Tauri 2. From `apps/desktop`, install the frontend packages and start the Tauri development app:

```powershell
npm install
npm run tauri dev
```

The Rust core creates its SQLite database in the operating system's application data directory. The UI can import an extracted PetPack or create a static companion from a selected PNG, JPEG, GIF, or WebP image; imports are decoded, normalized, validated, stored locally, and selected. GIF uploads use the first decoded frame. The local Pet Library supports metadata search and a selected-companion filter across installed packs. The desktop overlay can persistently pause and resume companion reactions through the Rust state machine. The universal wrapper runs a real child process, streams stdout/stderr through, records start/end/duration/status, and preserves normal exit codes. Native agent hooks and archive processing are not implemented. `cargo test --workspace` runs the Rust tests.

Run the wrapper from the workspace root after installing the Rust toolchain:

```powershell
cargo run -p agentpet-cli -- run -- git --version
```

Replace `git --version` with the command to monitor. The wrapper streams child stdout/stderr to the same terminal and stores session metadata locally. It does not persist terminal output or invent token usage.

The CLI also provides `cargo run -p agentpet-cli -- status`, `cargo run -p agentpet-cli -- pets list`, `cargo run -p agentpet-cli -- agents list`, and `cargo run -p agentpet-cli -- doctor`. `status` reports persisted pet state, selected companion, recent running sessions, and the most recent accepted event. `pets list` reports installed PetPacks and marks the selected one. Agent listing checks PATH for Claude Code (`claude`), Codex (`codex`), Gemini CLI (`gemini`), Aider (`aider`), and OpenCode (`opencode`). It detects executable files and Windows command launchers, but does not verify that each launcher can be started by the universal wrapper. It does not install/configure native hooks or observe sessions started outside the wrapper. The doctor checks platform, CLI path, local data directory, database access, installed-pack count, and those optional PATH entries. It does not check cloud, credentials, hooks, or daemon services.

On Windows, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets` and `cargo test --workspace` pass with the `stable-x86_64-pc-windows-gnu` toolchain (WinLibs MinGW on PATH). `npm run build` passes in `apps/desktop`. The desktop binary compiles, but the running Tauri app and overlay have not been exercised end to end, and macOS/Linux builds are unverified. The desktop crate builds as `rlib` only; mobile targets are not supported.

## Product constraints

- No simulated agent activity, token counts, or session completion.
- No code execution from PetPacks.
- No API keys in SQLite, UI state, logs, URLs, or analytics.
- No source code, prompts, terminal output, or full project paths uploaded by default.
- No cloud login requirement for pet rendering, local events, history, or progression.
- Character assets require creator, license, source, and attribution metadata. The application must not bundle unlicensed copyrighted character art.

## Documentation

- [AGENTS.md](AGENTS.md): mandatory coding-agent rules.
- [ARCHITECTURE.md](ARCHITECTURE.md): selected stack and system design.
- [PET_PACK_SPEC.md](PET_PACK_SPEC.md): data-only pack format and validation.
- [AGENT_PROVIDER_SPEC.md](AGENT_PROVIDER_SPEC.md): agent and provider contracts.
- [DATABASE.md](DATABASE.md): local/cloud data model and migration rules.
- [API.md](API.md): planned local IPC and cloud API contracts.
- [SECURITY.md](SECURITY.md) and [PRIVACY.md](PRIVACY.md): trust boundaries and data handling.
- [DEVELOPMENT.md](DEVELOPMENT.md), [CONTRIBUTING.md](CONTRIBUTING.md), and [DEPLOYMENT.md](DEPLOYMENT.md): contributor and release workflow.

See `API.md`, `DATABASE.md`, `SECURITY.md`, `PET_PACK_SPEC.md`, `AGENT_PROVIDER_SPEC.md`, `PRIVACY.md`, `DEPLOYMENT.md`, and `TROUBLESHOOTING.md` for the remaining contracts.
