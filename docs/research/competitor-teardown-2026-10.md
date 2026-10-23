# Competitor teardown: Codex Pets, Agent Pet (VS Code), Agent Pet (Electron)

Date: 2026-10-22. Sources were read from public artifacts only: the Marketplace VSIX
`merttalhayener.agent-pet` 0.20.3, the public repos `MoraLin/agent-pet` and
`portons/codex-pet-share`, and the codex-pets.net production bundle. Scratch copies live in
`D:\mirai\re-scratch\` (outside this repo). Nothing was uploaded, signed in to, or executed.

Three different products share confusingly similar names:

| Product | What it is | License |
| --- | --- | --- |
| codex-pets.net (`portons/codex-pet-share`) | Gallery/marketplace for the OpenAI Codex app's pixel-pet format. React + Cloudflare Worker/D1/Pages. | Code MIT. Uploaded pets carry no license field; the uploader only attests rights. |
| Agent Pet, VS Code (`merttalhayener/agent-pet`) | VS Code extension plus a native macOS Swift/AppKit helper (`NSPanel`, `NSStatusItem`). Apple Silicon, macOS 26+. | MIT |
| Agent Pet, desktop (`MoraLin/agent-pet`, agent-pet-site.vercel.app) | Electron app for macOS and Windows. Hook based, GIF skins. | Code MIT. Bundled dinosaur GIFs are "All rights reserved" and must not be reused. |

## 1. codex-pets.net pet format (de facto ecosystem standard)

Package: `<id>.codex-pet.zip`, unzipped to `$HOME/.codex/pets/<id>/`.

`pet.json`:

```json
{ "id": "debug-duck-v2", "displayName": "Debug Duck v2", "description": "...",
  "spritesheetPath": "spritesheet.webp", "spriteVersionNumber": 2, "kind": "animal" }
```

- `kind` is one of `object | animal | person | creature`. `spriteVersionNumber` is absent (v1) or `2`.
- Spritesheet: WebP only, 8 columns of 192x208 px cells. v1 is 1536x1872 (9 rows). v2 is 1536x2288 (11 rows).
- Rows and frame counts (`src/domain/config.ts`):

| Row | State id | Frames |
| --- | --- | --- |
| 0 | idle | 6 (7 in v2; frame 7 is "neutral look") |
| 1 | running-right | 8 |
| 2 | running-left | 8 |
| 3 | waving | 4 |
| 4 | jumping | 5 |
| 5 | failed | 8 |
| 6 | waiting | 6 |
| 7 | running | 6 |
| 8 | review | 6 |
| 9 (v2) | look-right-side | 8 (16 look directions across rows 9 and 10) |
| 10 (v2) | look-left-side | 8 |

- Server upload limits: spritesheet 4 MiB, share image 2 MiB (must be 1200x630 PNG), preview 1 MiB (WebP, 104 px high), poster WebP of one cell. Dimensions are checked by parsing RIFF/VP8X/VP8/VP8L headers.
- Install paths offered: `npx <handle> add <id>`, a `curl` plus `unzip` command, and a `codex://new?prompt=Install this pet: ...` deep link that asks Codex itself to install it.
- API: public `GET /api/pets` and collections. Uploads need a bearer token. Per-user API keys (`cps_...`, stored hashed) exist for agent-driven uploads. No documented third-party read API terms.
- Legal: uploaders "confirm you have the rights or permission". Tags include `anime`, `celeb`, `clippy`, `nsfw`, so the gallery contains plenty of likely-unlicensed characters.

## 2. MoraLin Agent Pet (Electron): hook driven

Claude Code (`src/main/hooks/claude.js`):
- Merges `type: "http"` hooks (`http://localhost:9876/event`, timeout 5 s) into `~/.claude/settings.json` for `SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, Stop, Notification, PreCompact, PostCompact`. It uses matcher `.*` on tool events.
- Detects its own entries by exact `type + url`, writes atomically, and refuses to touch an unparseable file.
- Removes its hooks on quit, because an http hook pointing at a dead port surfaces an error on every tool call.

Codex (`src/main/hooks/codex.js`, `codex-hook-forward.js`):
- Writes `~/.codex/hooks.json` for `SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PermissionRequest, Stop, PreCompact, PostCompact`.
- Codex supports only `command` hooks. A Node forwarder reads stdin, tags `_petSource: "codex"`, POSTs to the same port, and always exits 0.
- Codex hashes the hook command text and forces a re-trust in `/hooks` whenever it changes. They keep the command string stable and show a notice window when it must change.
- Codex ships a private Node that is not on PATH, so `node "..."` often fails. Fallback: `ELECTRON_RUN_AS_NODE=1 <app.exe> --preserve-symlinks <script>`.
- `PermissionRequest` is normalized to `Notification` with `notification_type: "permission_prompt"`.

Local server (`src/main/hooks/server.js`): HTTP on `127.0.0.1:9876`, POST `/event` only, **no authentication**, 64 KiB body cap (returns 413). A bind failure is fatal (dialog, then exit).

Session heuristics (`src/main/session-state.js`):
- Permission prompt: waving, then "impatient" after 15 s, then auto give-up after 60 s (assumes answered and returns to working).
- Bash `PreToolUse` without `PostToolUse` for 45 s is treated as "possibly stuck on its own prompt". Known slow commands are exempt by regex.
- State is keyed per `session_id`. When several sessions alert, the display rotates every 4 s and routine events from other sessions are suppressed.
- Resolution events: `PostToolUse, PostToolUseFailure, Stop, UserPromptSubmit`.
- Only `notification_type === "permission_prompt"` triggers the alert. Other notifications (idle reminder) are ignored.

Event to animation (`src/renderer.js mapHookEvent`): `Edit/Write/NotebookEdit` map to typing, `Read/Grep/Glob` map to reading, other tools map to working, `PostToolUseFailure` maps to sad, `Stop` maps to success, and compaction has its own pose.

Skins (`skin-keywords.js`, `skin-manager.js`):
- A folder of `<prefix>-<keyword>.gif`, matched by suffix.
- Required: `idle, running, running-left, running-right, jumping, review, waiting, waving, failed`. Optional: `look-left-side, look-right-side, success, eating, playing`.
- These are exactly the codex-pets state ids, so a MoraLin skin is a codex-pets pet exported per row to GIF.
- Import validates keywords, stages in a sibling dir, re-validates, then swaps with two renames plus rollback. It does no size or dimension validation.

Window: transparent, frameless, `alwaysOnTop`, `skipTaskbar`, `setIgnoreMouseEvents(true, {forward: true})` toggled on hover for click-through. Right-click menu offers Auto-Wander, Sleep Timer, Import Skin, Restart, Quit.

Other: updater polls the GitHub releases API of `MoraLin/AgentPet-Downloads` and only shows a menu item. The log is local JSONL capped at 2 MiB with one rotation.

## 3. merttalhayener Agent Pet (VS Code): transcript tailing, no hooks

The extension installs **no hooks**. It polls every 1.5 s, re-discovers files every 5 s, tracks at most 128 files, and reads 256 KiB windows. A line longer than 2 MiB is dropped.

Codex (`activity.cjs`):
- Tails `$CODEX_HOME/sessions/YYYY/MM/DD/*.jsonl`. On macOS it also queries `state_<n>.sqlite` read-only through `/usr/bin/sqlite3` for thread titles and rollout paths.
- Status comes only from explicit `event_msg` lifecycle records: `task_started` is running, `task_complete(d)` is ready, `turn_aborted` is idle, `task_failed` is failed. The comment says it directly: "silence is never complete".
- Pending input is a `function_call` named `request_user_input(_async)`, matched to `function_call_output` by `call_id`.
- `turn_id` matching drops late terminal events from earlier turns.

Claude Code (`claude-activity.cjs`):
- Tails `~/.claude/projects/*/<uuid>.jsonl`.
- Running: user message or assistant `tool_use`.
- Ready: assistant `stop_reason` of `end_turn` or `stop_sequence` with no tool calls and no pending input, or `result.subtype === "success"`. Failed: `result.is_error`.
- Idle: `isAbortedMidStream`, or a user text starting `[Request interrupted by user`.
- Waiting: an outstanding `AskUserQuestion` or `ExitPlanMode` tool_use. Only the question `header` is kept.
- Current step shown in the UI: tool name plus Bash `description` or file basename. Commands, paths and message text are never retained.
- Subagents and workflows: `Agent/Task/Workflow` tool_use, `toolUseResult.status === "async_launched"`, `<task-notification>` blocks, and growth of subagent files, which proves liveness.
- Records older than the last start or finish are ignored. A session whose file disappears becomes `unknown/source_unavailable` instead of done.

Workspace scoping:
- A chat belongs to a window if its `cwd` is under a workspace folder (`path.relative` check) and its entrypoint is `claude-vscode`.
- Alternatively, its CLI process is a descendant of an integrated-terminal shell PID. This uses `ps -A -o pid=,ppid=,lstart=,comm=`, `~/.claude/sessions/<pid>.json` with a `procStart` check against PID reuse, and `lsof` on Codex PIDs to find the open rollout.

Aggregate priority: waiting > running > failed > all-ready > idle. A finished state is shown for 90 s. A running state counts only if there was activity within the last 10 min.

Antigravity: reads `port` and `csrfToken` from the official extension's exports. It opens one read-only Connect-JSON stream (`LanguageServerService/JetboxSubscribeToSummaries`) on 127.0.0.1 with a 4 MiB frame cap.

Extension to helper IPC: atomic JSON snapshots (`client-<pid>-<rand>.json`, mode 0600, every 3 s) plus empty "request" marker files (`desktop-show-request`, `desktop-add-pet-request`, and so on). A `desktop.lock` held by the helper arbitrates between VS Code windows, and `lsof` finds the holder.

Custom pets (`custom-pets.cjs`): `custom-pets/library.json` (1 MiB cap). `id` is `custom-<uuid>`, plus `revision` (uuid), `name` (60 graphemes or fewer), `scale` 0.5 to 1.5, `mirrored`, `motion`, and `poses` taken from `normal, running, waiting, ready, sleeping`. Only `normal` is required; missing poses fall back to it.

Network: GitHub releases API only (updater). No telemetry.

## 4. What this means for Miyora / AgentPet

Ranked by value for effort.

1. **Import codex-pets packages as user-local PetPacks.** The format is simple and strictly checkable: WebP, a fixed 8x9 or 8x11 grid of 192x208 cells, fixed row semantics. It maps cleanly onto `stateMappings`. Suggested mapping: `idle` to idle, `running` to working, `review` to thinking/reading, `waiting` to waiting, `waving` to input-required, `failed` to error, `jumping` to success/celebrating, run-left/right to wander. The same mapping covers MoraLin GIF folders (`<prefix>-<keyword>.gif`). Licensing constraint: these packages have no creator or license metadata, and many depict copyrighted characters. Import them only as user-imported, license-unknown, local-only packs. Never bundle, sign, or redistribute them, and never pull from the gallery API automatically.
2. **Hook coverage gaps in `crates/agentpet-adapters/src/claude_code.rs`.**
   - Add `PostToolUseFailure`, mapped to a failed/sad reaction.
   - Add `PreCompact` and `PostCompact`.
   - Today every `Notification` maps to `AgentInputRequired`. Only `notification_type == "permission_prompt"` means the agent is blocked. The idle reminder should not trigger the alert pose.
3. **Codex hooks installer.** Codex is detected today (`apps/desktop/src-tauri/src/lib.rs` near line 740) but no hooks are installed.
   - Write `~/.codex/hooks.json` `command` hooks that call the `agentpet hook` binary. A native binary avoids the missing-Node problem.
   - Normalize `PermissionRequest` to input-required.
   - Keep the command string byte-stable across releases, because any change forces a Codex re-trust. Detect a change and tell the user.
4. **Timed heuristics in pet-engine.** Use impatient after 15 s of an unanswered permission prompt, give-up after 60 s, and Bash-silent after 45 s with a slow-command allow-list. Key everything per `session_id`. Rotate multi-session alerts. These are display-only heuristics, so they must never award XP or claim completion.
5. **Optional transcript-tail fallback.** For users who decline hook install, tailing `~/.claude/projects/**.jsonl` and `~/.codex/sessions/**.jsonl` gives accurate waiting and done states without editing settings. Copy their privacy rule: keep only lifecycle, tool names, and short labels. The JSONL formats are undocumented, so this source is lower-trust and must be clearly labelled. It is real activity, so it does not conflict with the no-simulation rule. It must still not award XP, because XP stays tied to wrapped-child exit.
6. **Things we already do better, worth keeping.** MoraLin's hook endpoint is unauthenticated on a fixed port; our HMAC IPC is stronger. MoraLin does no image size or dimension validation; our pet-pack validator does. Also make sure `agentpet hook` exits 0 quickly when the app is down, so a stale hook never breaks the agent. MoraLin removes its hooks on quit to avoid this failure.
7. **Custom-pet UX idea** from the VS Code extension: one required PNG plus optional `running/waiting/ready/sleeping` poses, and per-pet `scale` (0.5 to 1.5), `mirrored`, and `motion` toggles. This extends our current "user image becomes a static idle" path.

Do not copy: MoraLin `src/assets/skin/*.gif` (all rights reserved) or the Font Awesome brand icons. Code from all three repos is MIT and may be referenced with attribution, but the points above are re-implementations of ideas and contain no copied code.
