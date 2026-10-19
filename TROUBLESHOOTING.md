# Troubleshooting

## Current repository status

The first Tauri/React shell, Rust SQLite runtime, universal wrapper, extracted-folder PetPack import, PATH-only CLI discovery, and local database doctor check are present. The wrapper uses the same per-user app data directory as the desktop app. Native agent hooks, archive import, cloud sync, and credential-store checks are not implemented. PATH discovery only reports known executable names and does not observe agent sessions outside the wrapper. On this Windows host, `cargo test --workspace` and warning-free workspace Clippy pass with `stable-x86_64-pc-windows-gnu` and WinLibs MinGW. The default MSVC target cannot link because Visual C++ Build Tools (`link.exe`) is absent. The workspace test run emitted a MinGW manifest-merge warning for the desktop test binary. These checks do not verify launching the desktop app or overlay.

## Planned diagnostics

`agentpet status` reads local pet state and recent session records. `agentpet pets list` reads the local installed-pack index. `agentpet doctor` reports OS/architecture, CLI path, data-directory resolution, local database availability and installed-pack count, plus known optional agent CLIs found on PATH. It does not currently check migrations independently, app permissions, local IPC, hooks, pack integrity, disk space, credential store, or cloud connectivity. Missing agent CLIs are warnings; an unavailable local database is a failing check.

## Common failure categories

| Code/category | Diagnostic direction |
|---|---|
| `DATABASE_OPEN_FAILED` | Preserve database; check file permissions and disk space; offer backup/restore guidance |
| `MIGRATION_FAILED` | Do not overwrite the DB; collect migration ID and safe diagnostics; use known-good backup |
| `IPC_UNAVAILABLE` | Check daemon process, per-user socket/pipe permissions, and protocol compatibility |
| `AGENT_HOOK_INVALID` | Show adapter and config location; provide reversible repair; preserve unrelated hook entries |
| `PET_PACK_INVALID` | Identify manifest field or asset path and validation rule without executing pack content |
| `ASSET_LIMIT_EXCEEDED` | Report which configured size/dimension/frame limit was reached |
| `CREDENTIAL_STORE_UNAVAILABLE` | Name platform credential store and provide a safe reauthentication path; never print secret values |
| `SYNC_CONFLICT` | Preserve local progress and show retry or account/device resolution steps |
| `UPDATE_SIGNATURE_INVALID` | Reject the update and retain the current installation |

## Diagnostic bundle privacy

Diagnostic export is opt-in and previews included files/fields before creation. Redact credentials, prompts, terminal output, and project paths. Never attach a database or raw event payload automatically. Provide a local-only mode for support analysis.
