# Miyora (AgentPet) — Autonomous Agent Operational Guidelines

This document provides mandatory operational rules, architectural boundaries, and engineering standards for all AI coding agents (Antigravity, Claude Code, Cursor, Copilot, Codex, Roo Code, Aider, Windsurf) working within this repository.

---

## 1. Project Context & Identity

- **Project Identity:** **Miyora** is an offline-first, native desktop companion that tracks AI coding agent activity in real time. It observes actual process lifecycles and tool calls, routes events through a deterministic 15-state behavior engine, and renders responsive 3D procedural avatars and animated sprite companions over a transparent desktop overlay.
- **Naming Conventions:** Public branding is **Miyora**. Internal crate prefixes (`agentpet-*`), the `agentpet` CLI binary, and Tauri command prefixes retain the canonical `agentpet` naming. Do not rename crates, commands, or database identifiers.
- **Layout Note:** Trust the actual workspace layout documented below and `README.md`. Historical design documents (such as parts of `ARCHITECTURE.md` referencing `crates/app-core` or cloud services) describe aspirational targets; work strictly against the real crates.

---

## 2. Architecture & Domain Boundaries

Miyora strictly enforces inward, unidirectional dependencies. Rust domain crates own authoritative business logic and persistence. React is strictly presentation and input collection.

### 2.1 Domain Crates (`crates/`)
- **`crates/agentpet-core`:** Composition root. SQLite persistence, schema migrations, universal child process wrapper (`agentpet run -- <cmd>`), immutable & tamper-evident XP ledger, and 3D avatar persistence (`avatars.rs`).
- **`crates/pet-engine`:** 15-state deterministic state machine enforcing explicit transition allow-lists, event priorities, reaction timers, and animation fallbacks.
- **`crates/pet-pack`:** Data-only PetPack manifest v1 validation, Ed25519 signature verification, atomic archive staging/extraction, image decoding, and VRM guardrails.
- **`crates/avatar`:** Procedural 3D character specifications, bundled character catalog (`catalog/characters.json`), image-to-GLB silhouette sculpting, and GLB writer.
- **`crates/agentpet-adapters`:** Real agent observation. Parses documented hooks (Claude Code, Codex) and manages reversible native hook installation.
- **`crates/agentpet-ipc`:** Authenticated local IPC (named pipes / Unix sockets, length-prefixed framing, HMAC handshake, payload caps).
- **`crates/agentpet-cli`:** Diagnostic and command-line execution binary (`agentpet`).

### 2.2 Desktop Application (`apps/desktop/`)
- **Tauri 2 Rust Host (`src-tauri/`):**
  - Commands live in `src-tauri/src/lib.rs` and `src-tauri/src/studio_commands.rs`.
  - Commands must be narrow, strongly typed, and capability-scoped (`src-tauri/capabilities/`).
  - **Forbidden:** Never expose arbitrary shell execution or unrestricted filesystem access.
- **React Frontend (`src/`):**
  - Built with React 18, Vite, and strict TypeScript.
  - Serves two windows from a single bundle: the main Command Deck and the transparent `pet-overlay` (`App.tsx` branches on `isOverlayWindow`).
  - 3D rendering uses Three.js procedural rigs (`src/avatar/`) driven by `CharacterSpec` from the catalog.
  - 2D PetPack sprites render via `CompanionRenderer.tsx`.
  - **Forbidden:** React must never implement authoritative business rules or access SQLite directly.
  - All Tauri calls must go through `invokeSafe` in `src/utils/ipc.ts`, returning `{ ok, data | error }` instead of throwing unhandled rejections.

---

## 3. Core Agent Engineering Rules

### 3.1 Complete Implementations (Zero Stubs / Zero Placeholders)
- Implement every function, type, error branch, and feature completely.
- **Forbidden:** No TODOs, no empty stubs, no fake providers, no simulated agent activity, no fabricated token counts, and no mock completion in release builds.
- *Development Exception:* An isolated developer simulator is permitted **only** behind the explicit `dev-simulator` Cargo feature and Vite dev flag (`[SIMULATED - DEV ONLY]` banner, isolated test database, zero live XP).

### 3.2 Tool-Use & Code Changes
- Agents must modify files using workspace editing tools (`edit_file`, `write_file`) directly. Never output raw code dumps in chat when a file modification is requested.
- Inspect files and their call graph before editing.
- Preserve existing comments, docstrings, formatting, and unrelated user changes.
- Keep diffs minimal, focused, and reviewable.

### 3.3 Rust Standards
- Rust code must be formatted with `cargo fmt` and clean under `cargo clippy --workspace --all-targets -- -D warnings`.
- Handle errors explicitly with `Result<T, E>`. Never silently discard errors or use `.unwrap()` / `.expect()` in production paths.
- Use explicit named constants for timeouts, limits, retries, and buffer sizes. Avoid unexplained magic numbers.

### 3.4 TypeScript Standards
- TypeScript operates under strict mode (`strict: true`).
- Do not use `any`. Use strict domain interfaces or narrow unknown types with type guards or Zod schemas immediately.
- Maintain cohesive, small components and avoid unnecessary re-renders.

### 3.5 Security, Privacy & Sandboxing
- **Air-Gapped Privacy:** Zero background telemetry. Prompts, terminal output, raw tool responses, API keys, and sensitive filesystem paths must be scrubbed before persistence or logging.
- **Secrets Management:** Secrets (e.g., `TRIPO_API_KEY`) must reside in the OS credential store or environment variables. Never persist plaintext credentials or commit them to git.
- **Untrusted Input Hardening:** Treat PetPacks, hook payloads, IPC frames, imported images, and 3D models (VRM/GLB) as untrusted input. Validate schema, size, paths (prevent ZipSlip), MIME types, and dimensions before processing.
- **Data-Only PetPacks:** PetPacks contain data only. Never execute scripts, binaries, or native libraries from packs.

### 3.6 Persistence & Ledger Integrity
- SQLite schema changes require versioned, sequential migrations with test coverage. Foreign keys and transactions are mandatory for multi-record operations.
- XP transactions are immutable, hash-chained, and idempotent. Replays of identical session IDs must never award duplicate XP or milestones.
- Return structured, sanitized error messages. Never expose database stack traces, secret values, or internal paths to users.

---

## 4. Verification & Testing Commands

Always run the relevant validation commands before concluding work on any task:

### Rust Workspace
```bash
# Code formatting check
cargo fmt --all -- --check

# Strict linting (CI standard)
cargo clippy --workspace --all-targets -- -D warnings

# Also lint dev-simulator feature
cargo clippy --workspace --all-features --all-targets -- -D warnings

# Run all workspace tests
cargo test --workspace

# Run single test
cargo test -p agentpet-pet-engine <test_name_substring>

# Diagnostic CLI tool
cargo run -p agentpet-cli -- doctor
```
*Note for Windows without MSVC:* Use GNU toolchain if needed: `cargo +stable-x86_64-pc-windows-gnu test --workspace`.

### Desktop UI (`apps/desktop/`)
```bash
# Typecheck & build (runs tsc -b + vite build)
npm run build

# Unit & component tests (Vitest)
npm test

# Single test
npx vitest run src/avatar/__tests__/rig.test.ts -t "<name>"

# Tauri development app
npm run tauri dev
```

### Python Scripts & Validation
```bash
# Verify avatar catalog is up to date (fails if characters.json is stale)
python scripts/generate_avatar_catalog.py --check

# Validate all bundled PetPacks under pet-packs/
python scripts/validate_pet_packs.py
```

---

## 5. Git & Release Workflow

- Inspect `git status` and diffs before and after modifying files.
- Use Conventional Commits (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`).
- Never commit secrets, credentials, generated temporary files, or unlicensed character artwork.
- Preserve unrelated user changes and never rebase or rewrite working branches blindly.

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
