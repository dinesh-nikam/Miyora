# Development Guide

## Current state

The foundation includes a Cargo workspace, a Rust SQLite core with explicit pet transitions and session persistence, a Tauri/React shell, a universal child-process wrapper, and extracted-folder PetPack import/selection. The wrapper streams real process output and stores session lifecycle and exit status. Native agent hooks/adapters, archive processing, decoded media validation, pet rendering, progression, cloud API, and release automation are not implemented. Do not describe those features as available.

## Selected stack

- Rust stable toolchain for the core, CLI, Tauri commands, and workers.
- Tauri 2 for cross-platform desktop windows, tray, notifications, and capability-scoped IPC.
- React and TypeScript strict mode for the desktop UI.
- SQLite for local-first storage, accessed only through Rust.
- NestJS and PostgreSQL for optional cloud features.
- GitHub Actions for validation and release automation.

## Foundation implementation order

1. Add workspace manifests and pinned toolchain/dependency policy.
2. Add a minimal Tauri window and accessible React shell.
3. Define shared event/settings contracts and explicit Tauri commands.
4. Add SQLite migration runner, schema version table, and backup/recovery behavior.
5. Add event engine and pet state machine with unit tests.
6. Add validated local PetPack loading and rendering.
7. Add agent daemon/CLI process lifecycle and real adapter fixtures.
8. Add progression and optional cloud boundaries only after local event persistence works.

## Local development

Install Rust/Cargo, Node.js/npm, and the operating-system prerequisites for Tauri 2. From `apps/desktop`, run `npm install` and `npm run tauri dev`. From the workspace root, `cargo test -p agentpet-core` runs core tests. On Windows, use `cargo +stable-x86_64-pc-windows-gnu test --workspace` and the matching toolchain qualifier for Clippy when WinLibs MinGW is installed. The default MSVC target requires Visual C++ Build Tools with `link.exe`. Add and maintain lockfiles before treating dependency installs as reproducible.

As the workspace grows, provide separate verified commands for formatting, Clippy, TypeScript typechecking, tests, desktop development, and release builds. Keep platform-specific prerequisites explicit.

## Configuration

Configuration should use documented environment variables only for development and deployment. Never put secrets in example files or source. Provide `.env.example` with non-secret values if the cloud service is added. Desktop user settings belong in the app config/database; credentials belong in the OS credential store.

## Debugging

Provide privacy-safe structured logs with levels and correlation IDs. Diagnostics must redact credentials, prompts, terminal output, and sensitive paths. Offer a local `doctor` command that checks runtime, database, IPC, installed assets, adapters, disk space, and credential-store availability, and gives actionable recovery instructions.
