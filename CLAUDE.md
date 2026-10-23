# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Miyora (internal name AgentPet; workspace root `nezuko`) is an offline-first desktop companion that visualizes coding-agent activity as a pet/3D character with local XP progression. Public branding is "Miyora"; crate names (`agentpet-*`), the `agentpet` CLI binary, and commands keep the old name — do not rename them.

`AGENTS.md` (section "AgentPet Project Engineering Rules") holds the mandatory engineering rules; read it before non-trivial changes. Ignore the unrelated persona block at the top of that file. `ARCHITECTURE.md` describes a *target* layout (`crates/app-core`, `event-engine`, `apps/web`, NestJS cloud) that mostly does not exist yet — trust the actual workspace below and the `README.md` "Project status" table over it.

## Commands

Rust (workspace root):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings                 # what CI runs
cargo clippy --workspace --all-features --all-targets -- -D warnings  # also covers dev-simulator
cargo test --workspace
cargo test -p agentpet-pet-engine <test_name_substring>               # single crate / single test
cargo run -p agentpet-cli -- <subcommand> [args...]                   # e.g. doctor, status, run -- <cmd>
```

Package names: `agentpet-core`, `agentpet-pet-engine`, `agentpet-pet-pack`, `agentpet-avatar`, `agentpet-adapters`, `agentpet-ipc`, `agentpet-cli` (binary `agentpet`), `agentpet-desktop` (Tauri). On Windows without MSVC Build Tools, use the GNU toolchain: `cargo +stable-x86_64-pc-windows-gnu test --workspace` (same qualifier for clippy).

Desktop UI (`apps/desktop`):

```bash
npm install
npm run build                 # tsc -b + vite build; this is the TS typecheck
npm test                      # vitest run (jsdom)
npx vitest run src/avatar/__tests__/rig.test.ts -t "<name>"   # single test
npm run tauri dev             # full desktop app (Vite on 127.0.0.1:1420)
```

Root `package.json` proxies these (`npm run tauri:dev`, `npm run build`). The `avatar-harness` launch config (`.claude/launch.json`, port 1430) serves `studio-preview.html`, which mocks Tauri IPC with the bundled catalog — use it to preview avatar/studio UI in a plain browser without Tauri.

Python asset scripts (`pip install -r requirements.txt`, Pillow only):

```bash
python scripts/generate_avatar_catalog.py --check   # fails if committed characters.json is out of date
python scripts/validate_pet_packs.py                # validate every pack under pet-packs/ (read-only)
```

CI (`.github/workflows/ci.yml`): UI `npm ci && npm run build`; Rust fmt/clippy/test on Ubuntu, macOS, Windows. The frontend must build before the Tauri crate compiles.

## Architecture

Rust owns all authoritative logic; React only renders and collects input. SQLite (`rusqlite`, bundled) is reachable only from Rust.

- `crates/agentpet-core` — composition root: SQLite schema/migrations, events, sessions, universal wrapper, XP/achievement ledger (immutable, idempotent, tamper-evident hashing; `verify_ledger`), and `avatars.rs` (3D avatar persistence). Most domain logic lives in its large `lib.rs`.
- `crates/pet-engine` — 15-state pet state machine with an explicit transition allow-list, priorities, timed reactions, animation fallback.
- `crates/pet-pack` — data-only PetPack manifest v1 validation, archive install (zip/tar.gz) via staging, Ed25519 signatures, animated image decoding, VRM guardrails.
- `crates/avatar` — procedural 3D character specs, the bundled catalog (`catalog/characters.json`, embedded with `include_str!`), image-to-GLB silhouette sculpting, GLB writer.
- `crates/agentpet-adapters` — Claude Code hook payload parsing/normalization and native hook install/remove (reversible, preserves unrelated settings).
- `crates/agentpet-ipc` — authenticated local IPC (named pipe / Unix socket, length-prefixed framing, HMAC shared-secret handshake, size cap). Standalone; no other crate depends on it yet.
- `crates/agentpet-cli` — `agentpet` binary: `run`, `status`, `pets`, `agents`, `hook`, `doctor`.
- `apps/desktop/src-tauri/src/lib.rs` — all Tauri commands live here and are registered in one `generate_handler!` list. Keep commands narrow and typed; put logic in the domain crates. Plugin permissions go in `src-tauri/capabilities/*.json` (`default` for `main`, `pet-overlay` for the overlay).

Frontend (`apps/desktop/src`):
- One bundle serves two windows: `main` and the transparent `pet-overlay` (loaded as `index.html?overlay=1`; `App.tsx` branches on `isOverlayWindow`).
- All Tauri calls go through `invokeSafe` in `utils/ipc.ts`, which returns `{ ok, data | error }` instead of throwing.
- `avatar/` is the three.js procedural rig/motion/props renderer driven by `CharacterSpec` from the Rust catalog. `rig.test.ts` imports `characters.json` directly and builds every catalog character, so catalog changes are covered by `npm test`.
- `CompanionRenderer.tsx` renders 2D PetPack sprites; `CommandDeck.tsx` is the status/quota panel.

Data and assets:
- `pet-packs/<id>/` — `pet.json` manifest plus per-state PNGs (see `PET_PACK_SPEC.md`). Generated by `scripts/build_*.py`; `pet-packs-review/` is an untracked staging folder for regenerated art.
- `crates/avatar/catalog/characters.json` is generated by `scripts/generate_avatar_catalog.py` (deterministic). Edit the script rows, then regenerate.
- `fixtures/*.json` are spec-level behavior fixtures (state machine, mood, progression, VRM guardrails); no test loads them yet.
- Specs live in `docs/specs/` (`event_protocol.md`, `pet_pack.md`, `avatar_3d.md`, `session_state.md`, `vrm_pet.md`, `sync.md`).

## Constraints that shape code

- No simulated agent activity, fake token counts, or fabricated session completion in release builds. Only exceptions: the `dev-simulator` Cargo feature (core + desktop) and the `#[cfg(debug_assertions)]` commands `toggle_demo_mode` / `simulate_agent_event`. Simulated output must show a `[SIMULATED - DEV ONLY]` banner and never award live XP.
- XP is awarded only when a wrapped child exits successfully. Replays must never duplicate XP or achievements.
- PetPacks are data only — never execute pack content. Treat packs, hook payloads, IPC messages, images, and VRM files as untrusted; validate size, paths, MIME, and dimensions.
- SQLite schema changes need versioned migrations and tests.
- Character art must carry creator, license, source, and attribution metadata. No unlicensed copyrighted characters.
- TypeScript is strict; no `any` without immediate narrowing.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **Miyora** (2708 symbols, 7058 relationships, 237 execution flows).

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` (or `bun .gitnexus/run.cjs analyze --index-only`) from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact before editing.** Use `impact({target: "symbolName", direction: "upstream"})` or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .` (or `bun .gitnexus/run.cjs ...`); report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (or `bun .gitnexus/run.cjs ...`) (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
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
