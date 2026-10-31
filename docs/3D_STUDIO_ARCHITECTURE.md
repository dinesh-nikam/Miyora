# Miyora 3D Studio — Architecture

Status: Phase 0 + Phase 1 core (text/image → 3D, task tracking, viewer,
library). See `TRIPO_COMPATIBILITY_MATRIX.md` for per-feature status.

## Layers

```
React (apps/desktop/src/studio/*)          renders, collects input; no provider knowledge
  │  invokeSafe → studio_* commands (JSON); studio_read_output (raw bytes)
Tauri adapters (src-tauri/src/studio_commands.rs)   thin, typed, run on blocking workers
  │
StudioService (crates/studio/src/service.rs)        orchestration, polling, validation, storage
  │            │                 │
  │      StudioStore (store.rs)  media.rs / prompt.rs / task.rs
  │      studio.sqlite + files
GenerationProvider trait (provider.rs)
  │
  ├─ LocalTriposrProvider (local.rs) ── job queue ── python -I run_triposr.py (engines/triposr)
  ├─ HfTrellisProvider (hf.rs) ──┐
  └─ TripoProvider (tripo.rs) ───┴─ Transport trait (transport.rs) ── ReqwestTransport (HTTPS only)
```

The studio is a separate crate (`agentpet-studio`) with its own SQLite file, so
it cannot affect the XP ledger or pet state in `agentpet-core`. It is wired into
the desktop shell in two places in `src-tauri/src/lib.rs`: `app.manage(open_studio(..))`
in setup and the `studio_*` entries in the single `generate_handler!` list.

## Modules (`crates/studio/src`)

| Module | Responsibility |
|---|---|
| `provider.rs` | Provider-neutral types: `Capability`, `GenerationSettings`, `ProviderTask`, `GenerationProvider` trait, task-ID validation |
| `local.rs` | Local TripoSR engine: manifest validation, single-GPU job queue, runner process, status files |
| `hf.rs` | TRELLIS on Hugging Face Spaces via Gradio queue protocol |
| `tripo.rs` | Tripo v3 request mapping (incl. multiview, convert), response parsing, error mapping |
| `transport.rs` | HTTP seam; `ReqwestTransport` (HTTPS-only, timeouts, size caps) |
| `task.rs` | `TaskState` machine, provider-status mapping, `PollPolicy` (all timing) |
| `store.rs` | Versioned migrations, transactions, idempotent writes, views for the UI |
| `service.rs` | Submit, poll loop, download + validate, regenerate, resume, settings |
| `media.rs` | Image validation/preview/working copy; GLB/FBX inspection; converted-file sniffing |
| `export.rs` | Local export: GLB scene reader, OBJ/STL/PLY writers, write-then-verify |
| `prompt.rs` | Structured prompt builder, 12 presets |
| `credentials.rs` | OS credential store (`keyring`) behind `CredentialStore` |
| `error.rs` | Stable `ErrorKind`s and the UI `ErrorReport` (what / why / what to do) |

## Frontend (`apps/desktop/src/studio`)

`StudioView` (layout, shortcuts, refresh) → `CreatePanel`, `LibraryPanel`,
`ModelViewport` (three.js; reuses `avatar/dispose.ts`), `Inspector`,
`TaskQueue`, `SettingsDialog`. `api.ts` wraps `invokeSafe` and decodes the
JSON `ErrorReport`. The tab is "3D Studio" in the main window.

Keyboard: `F` frame, `R` reset view, `1/2/3` material/solid/wireframe,
`G` grid, `B` bounds, `Ctrl+Enter` generate.

## Storage layout (`<app data>/studio/`)

```
studio.sqlite                              metadata only (schema v2)
inputs/<generation_id>/source.<ext>        untouched copy of the user's image
local-jobs/<job>/                           TripoSR job: input.png, job.json, status.json, engine.log, model.glb
hf-jobs/<job>/status.json                   TRELLIS job status + result URL
assets/<asset_id>/<generation_id>/<label>.<glb|fbx|png>   validated provider outputs
```

Tables: `settings`, `provider_accounts`, `projects`, `assets`, `generations`,
`generation_inputs`, `tasks`, `asset_versions`, `generation_outputs`,
`usage_records`, `exports` (v2). Foreign keys are on; multi-row writes use transactions.

## Offline behaviour

Startup only opens SQLite and reads local data. If unfinished tasks exist,
pollers start; offline they back off and stop after the configured error budget,
leaving the task `FAILED` (`provider_unavailable`, resumable).

## Dev harness

`apps/desktop/studio3d-preview.html` (`npm run dev`, then open the page) mounts
the Studio with mocked read-only IPC and a local GLB from `output/`. It is
labelled `[SIMULATED - DEV ONLY]`, is not a Vite build entry, and never reaches
a provider.

## Smoke tool

`cargo run -p agentpet-studio --example generate -- <provider-id> <image> [data-dir]`
runs one real generation through `StudioService` (no mocks) and prints the task
and the stored model report.
