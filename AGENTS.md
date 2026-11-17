# Miyora (AgentPet) — Agent Guide

Offline-first Tauri 2 desktop companion: real AI-agent process/hook events → 15-state behavior engine → 3D/sprite pet over a transparent overlay, with a local SQLite XP ledger. Zero cloud, zero telemetry.

## Naming — do not "fix" it

Public brand is **Miyora**, but crates (`agentpet-*`), the `agentpet` binary, Tauri command prefixes, and DB identifiers keep the old name. Renaming any of them is a breaking change, not a cleanup.

## Source of truth

- `Cargo.toml` workspace members + `README.md` describe the real repo.
- `ARCHITECTURE.md` describes an aspirational layout (`crates/app-core`, `apps/web`, NestJS cloud sync) that **does not exist**. Ignore it when it conflicts with the code.
- `CLAUDE.md` mirrors this file but contains stale notes (it claims AGENTS.md has a persona block; it does not). Trust `AGENTS.md` + the code.

## Layout — directory ≠ package name

10 workspace members. Four directories have a different Cargo package name, so `cargo test -p <dir>` fails:

| Directory | Package (`-p`) | Role |
| --- | --- | --- |
| `crates/agentpet-core` | `agentpet-core` | Composition root: SQLite schema/migrations, sessions, universal wrapper, immutable XP/achievement ledger, `avatars.rs` |
| `crates/pet-engine` | `agentpet-pet-engine` | 15-state machine, explicit transition allow-list (`ALL_STATES`), priorities, timed reactions |
| `crates/pet-pack` | `agentpet-pet-pack` | PetPack manifest v1, Ed25519 signatures, zip/tar.gz staging, image decode, VRM guardrails |
| `crates/avatar` | **`agentpet-avatar`** | Procedural 3D specs, `catalog/characters.json` (embedded via `include_str!`), silhouette sculpting, GLB writer |
| `crates/studio` | **`agentpet-studio`** | Tripo v3 / local TripoSR generation, task tracking, decimation, GLB export; secrets via OS `keyring` |
| `crates/agentpet-adapters` | `agentpet-adapters` | Claude Code / Codex hook parsing + reversible hook install |
| `crates/agentpet-ipc` | `agentpet-ipc` | Authenticated pipes/sockets, length-prefixed frames, HMAC handshake, 256 KiB cap |
| `crates/agentpet-cli` | `agentpet-cli` | `agentpet` binary: `run`, `status`, `pets`, `agents`, `hook`, `doctor` |
| `crates/agentpet-harness` | `agentpet-harness` | Test-only verification harness (`publish = false`); must never be a production dependency — `tests/isolation.rs` enforces this |
| `apps/desktop/src-tauri` | `agentpet-desktop` | Tauri host (lib name `agentpet_desktop_lib`) |

## Commands

### Rust (workspace root)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings                 # what CI runs
cargo clippy --workspace --all-features --all-targets -- -D warnings  # adds dev-simulator + studio features
cargo test --workspace
cargo test -p agentpet-pet-engine <test_name_substring>               # single crate / test
cargo test -p agentpet-harness                                        # property + replay harness
cargo run -p agentpet-cli -- doctor                                   # also: status, run -- <cmd>, pets list, agents install <id>
```

Windows without MSVC Build Tools: prefix with `cargo +stable-x86_64-pc-windows-gnu`.

### Desktop UI (`apps/desktop`, or root proxies)

```bash
npm install          # run once inside apps/desktop (CI uses npm ci)
npm run build        # tsc -b + vite build + bundle-budget gate
npm test             # vitest run (jsdom)
npx vitest run src/avatar/__tests__/rig.test.ts -t "<name>"
npm run tauri dev    # fixed dev server 127.0.0.1:1420, strictPort
```

- Root `package.json` only proxies `dev`, `build`, `tauri`, `tauri:dev`, `tauri:build`. There is **no root `test`** — run `npm test` inside `apps/desktop`.
- `npm run build` is not just a typecheck: `scripts/check-bundle-budget.mjs` fails on any JS chunk > 500 kB raw (`three-*` gets 700 kB). Shrinking the bundle is part of "build passes".
- Build order matters: the Tauri crate needs `apps/desktop/dist` (`frontendDist: "../dist"`), so build the UI before `cargo` on the desktop crate. CI does UI first for this reason.
- Browser-only avatar/studio preview without Tauri: `apps/desktop/dev/studio-preview.html` on port 1430 (see `.claude/launch.json`); it mocks IPC with the bundled catalog.
- One bundle serves two windows: overlay loads `index.html?overlay=1`; `App.tsx` branches on `isOverlayWindow`.

### Python asset scripts (`pip install -r requirements.txt` — Pillow only)

```bash
python scripts/generate_avatar_catalog.py --check   # exit 1 if committed characters.json is stale
python scripts/validate_pet_packs.py                # read-only validation of pet-packs/
python scripts/validate_character_cards.py          # Character Card V2 profiles vs catalog
```

## Architecture notes that change how you edit

- Rust owns all authoritative state; React only renders and collects input. SQLite (`rusqlite`, bundled) is reachable only from Rust.
- `crates/avatar/catalog/characters.json` is **generated** by `scripts/generate_avatar_catalog.py` (deterministic). Edit the script, regenerate — never hand-edit the JSON. `rig.test.ts` builds every catalog entry, so catalog drift fails `npm test`.
- Tauri commands live in `src-tauri/src/lib.rs` (~38), `studio_commands.rs` (~20), `overlay_controls.rs`, `companion_host.rs`, all registered in one `generate_handler!` in `lib.rs`. Keep them narrow/typed and put logic in domain crates; plugin permissions go in `capabilities/default.json` (`main`) and `capabilities/pet-overlay.json` (overlay).
- Every frontend Tauri call goes through `invokeSafe` in `src/utils/ipc.ts`, returning `{ ok, data | error }` instead of throwing.
- `pet-packs/<id>/` = `pet.json` + per-state PNGs, generated by `scripts/build_*.py`. `pet-packs-review/` is a gitignored staging folder for regenerated art.
- `agentpet-harness` fixtures (`fixtures/golden_session_flow.json`, `golden_snapshot.json`) are real replay baselines — regenerate deliberately, never to make a test pass.

## Invariants you must not break

- **No simulated activity** in release builds: no fake tokens, fabricated sessions, or mock completion. Only `dev-simulator` (core + desktop Cargo features) and `#[cfg(debug_assertions)]` demo commands; simulated output must show `[SIMULATED - DEV ONLY]` and award zero live XP.
- **XP only on successful wrapped child exit.** Replays/duplicate session IDs must never duplicate XP or achievements (hash-chained, idempotent ledger).
- **PetPacks are data only.** Never execute pack content. Treat packs, hook payloads, IPC frames, images, and VRM/GLB as untrusted: validate schema, size, paths (ZipSlip), MIME, dimensions.
- **SQLite schema changes** need versioned sequential migrations plus tests; multi-record writes use transactions.
- **TypeScript is strict** with `noUnusedLocals`/`noUnusedParameters`; no `any` without immediate narrowing. Rust: no `.unwrap()`/`.expect()` in production paths.
- Never commit secrets (`TRIPO_API_KEY` lives in env or the OS keyring), `*.zip`, `pet-packs-review/`, or generated test images — all are gitignored on purpose.

## CI & commits

`.github/workflows/ci.yml` on push/PR to `main`: UI `npm ci && npm run build` (Ubuntu), then fmt/clippy/test on Ubuntu, macOS, Windows. Use Conventional Commits (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`).

## Deeper docs

`docs/HARNESS.md` (verification layers, seed reproduction), `PET_PACK_SPEC.md`, `AGENT_PROVIDER_SPEC.md`, `DATABASE.md`, `API.md`, `docs/specs/*.md` (`avatar_3d`, `event_protocol`, `session_state`, `vrm_pet`, `sync`).

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **Miyora** (2708 symbols, 7058 relationships, 237 execution flows).

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact before editing.** Use `impact({target: "symbolName", direction: "upstream"})` or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .`; report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
- MUST warn on HIGH/CRITICAL `risk` pre-edit; never use `riskSharedAxes` to waive a HIGH/CRITICAL `risk` warning. Compare File/symbol: MCP File omits axes; Graph-RAG expands File.
- **MUST treat `risk: UNKNOWN` as unresolved, not as low.** An empty caller set is not evidence the symbol is unused — it can also mean the callers are not resolvable by the index (plain-object property access, dynamic dispatch, cross-language calls). `impact` pairs `UNKNOWN` with a `riskNote` saying so. Confirm with a text search before treating the symbol as safe to change or delete; do not proceed on the strength of a zero.
- **MUST use `query({search_query: "concept"})` for concepts/flows, `context({name: "symbolName"})` for a named symbol, or `impact` for blast radius, on read-only callers, dependencies, imports, or execution flow.** Graph first; text search only for empty/`UNKNOWN`/literals.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method before MCP/CLI impact analysis.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis, and never read `UNKNOWN` as an all-clear — it means the walk could not answer, which is the one verdict that requires confirming by other means.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit before MCP/CLI graph change analysis.

## Resources

| Resource | Use for |
| --- | --- |
| `gitnexus://repo/Miyora/context` | Codebase overview, check index freshness |
| `gitnexus://repo/Miyora/clusters` | All functional areas |
| `gitnexus://repo/Miyora/processes` | All execution flows |
| `gitnexus://repo/Miyora/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
| --- | --- |
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
