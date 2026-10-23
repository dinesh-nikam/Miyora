# AgentPet Provider Investigation & Evidence

**Platform:** Windows 10/11 x86_64  
**Date Inspected:** 2026-10-21  
**Inspector:** Carxen (Senior Software Engineer)  
**Rule:** Ground Rule 4 — Build only on what exists on the real machine. Never invent quota or fabricate status.

---

## 1. Claude Code (`claude_code`)

### Machine Evidence
- **Home Directory:** `%USERPROFILE%\.claude` (Verified present)
- **Configuration File:** `%USERPROFILE%\.claude\settings.json` (Verified present)
  - Existing `hooks` object contains: `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`
  - Existing `statusLine` command:
    ```json
    {
      "type": "command",
      "command": "powershell -ExecutionPolicy Bypass -File \"%USERPROFILE%\\.claude\\plugins\\cache\\caveman\\caveman\\<hash>\\hooks\\caveman-statusline.ps1\""
    }
    ```
- **Session Logs:** `%USERPROFILE%\.claude\projects\**\*.jsonl` (Verified present, multiple project directories)
- **Cost & Token Records:** Verified in session JSONL files with line type `cost-state`:
  ```json
  {
    "type": "cost-state",
    "sessionId": "00000000-0000-0000-0000-000000000000",
    "totalCostUSD": 1.2345,
    "modelUsage": {
      "claude-opus-5-5": {
        "inputTokens": 100,
        "outputTokens": 500,
        "thinkingTokens": 200,
        "cacheReadInputTokens": 1000,
        "cacheCreationInputTokens": 500
      }
    }
  }
  ```

### Activity Detection Strategy
- **Primary:** Native hooks configured in `~/.claude/settings.json` (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Notification`, `Stop`, `SubagentStop`).
- **Hook Transport:** `agentpet hook --adapter claude_code` runs `run_hook_command`, which opens the core database and writes normalized events directly to SQLite in a transaction.
- **Quota & Rate Limits:**
  - Claude Code passes JSON to `statusLine` via stdin containing `rate_limits.five_hour` (`used_percentage`, `resets_at`), `rate_limits.seven_day`, and `context_window.used_percentage`.
  - Only available for Pro/Team/Enterprise accounts. If `rate_limits` is absent, show `N/A` (never fabricated).
  - StatusLine shim intercepts stdin, writes `claude_statusline.json` locally, and chains the existing statusline command so existing workflows are preserved.
- **Tokens/Cost:** Read from `~/.claude/metrics/costs.jsonl` using the `estimated_cost_usd` field and labeled as estimated with provenance chip `LOCAL`.
- **Health State:** `CONNECTED` (when installed and active), `DEGRADED` (when hooks are absent or settings unreadable).

---

## 2. OpenAI Codex CLI (`codex`)

### Machine Evidence
- **Home Directory:** `%USERPROFILE%\.codex` (Verified present)
- **Configuration & Hooks:**
  - `%USERPROFILE%\.codex\hooks.json` exists with structure:
    ```json
    {
      "hooks": {
        "PreToolUse": [{ "matcher": "Grep|Glob|Bash", "hooks": [...] }],
        "PostToolUse": [{ "matcher": "Bash", "hooks": [...] }]
      }
    }
    ```
  - `%USERPROFILE%\.codex\history.jsonl` exists with sample session prompt lines:
    `{"session_id":"00000000-0000-0000-0000-000000000000","ts":1700000000,"text":"continue"}`
  - `%USERPROFILE%\.codex\sessions` directory exists.

### Activity Detection Strategy
- **Primary (implemented):** `agentpet agents install codex` merges `{"type":"command","command":"agentpet hook --adapter codex"}` into `$CODEX_HOME/hooks.json` (default `~/.codex/hooks.json`) for `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, `Stop`, and `Interrupt`. User hooks and the optional top-level `description` are preserved, a timestamped backup is written first, and `agentpet agents remove codex` removes only entries running that exact command. The adapter lives in `crates/agentpet-adapters/src/codex.rs`.
- **Trust:** Codex hashes each hook definition and skips new or changed hooks until the user trusts them with `/hooks`. The command string must never change between releases, or every user has to trust it again. Hooks are on by default; `[features] hooks = false` in `config.toml` disables them.
- **Event mapping:** `SessionStart` → started; `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostCompact`, `SubagentStart`, `SubagentStop` → working; `PreCompact` → thinking; `PermissionRequest` → input required; `Interrupt` → waiting; `Stop` → completed. Event ids include `turn_id`, so each turn's `Stop` is distinct. Only `session_id`, `hook_event_name`, `turn_id`, `tool_name`, `tool_use_id`, and `cwd` are read; prompts, tool input/output, and assistant messages are ignored. The hook always exits 0 with empty stdout, because Codex treats plain-text stdout on `Stop` as a hook failure.
- **Secondary:** Process tracking for `codex.exe`; log-tail of `history.jsonl` is not implemented.
- **Quota:** Not exposed locally by Codex CLI. Quota gauge displays `N/A` with provenance chip `N/A`.
- **Health State:** `CONNECTED` if hooks or active session files detected, otherwise `DEGRADED` / `NOT_INSTALLED`.

---

## 3. Google Antigravity / Gemini CLI (`antigravity` / `gemini_cli`)

### Machine Evidence
- **Installation Directory:** `%USERPROFILE%\.gemini` (Verified present)
- **Subdirectories:**
  - `antigravity`: contains `cli.log`, `antigravity_state.pbtxt`, `conversation_summaries.db`, `conversations/`
  - `antigravity-ide`: active IDE workspace logs, brain, and knowledge
  - `config`: `config.json`, `mcp_config.json`, `plugins/`, `sidecars/`
  - `logs`: `install.log`
- **CLI Log Inspection:** `cli.log` records HTTP requests (`fetchAvailableModels`, `loadCodeAssist`), but local storage does not maintain offline remaining quota counters. Quota is fetched live by the app/CLI command `/usage`.

### Activity Detection Strategy
- **Primary:** Conversation transcript tailing in `.gemini/antigravity-ide/brain/<conv_id>/` and process detection (`antigravity.exe` / `gemini.exe`).
- **Quota:** No offline local quota database exists. Provenance chip is `N/A` with a direct "Open Antigravity /usage" action button. Never simulate or invent percentages.
- **Health State:** `CONNECTED` when Antigravity IDE process or active conversation transcript is updating; otherwise `IDLE`.

---

## 4. GitHub Copilot (`copilot`)

### Machine Evidence
- **Home Directory:** `%USERPROFILE%\.copilot` (Verified present)
- **Subdirectories:** `config.json`, `ide`, `logs`, `session-state`.
- **VS Code Extension Storage:** `%APPDATA%\Code\User\globalStorage` (Verified present).

### Activity Detection Strategy
- **Primary:** Process observation (`copilot-agent`, `code` with copilot worker) and log tail of `~/.copilot/logs`.
- **Quota:** GitHub Copilot does not expose rate limit numbers in local files. Shows `N/A` with status indicator (Working / Idle / Offline).
- **Health State:** `CONNECTED` if logs active, `IDLE` or `NOT_INSTALLED`.

---

## 5. Cursor (`cursor`)

### Machine Evidence
- `%USERPROFILE%\.cursor`: Absent on this machine.
- `%APPDATA%\Cursor`: Absent on this machine.

### Activity Detection Strategy
- **Primary:** Process detection for `Cursor.exe` / window title scanning.
- **Quota:** `N/A`.
- **Health State:** `NOT_INSTALLED`. Displayed as offline with clear `NOT INSTALLED` status. No fake data.

---

## 6. Summary Matrix

| Provider | Activity Detection | Quota Exposure | Health on This Machine | Provenance Label |
|---|---|---|---|---|
| **Claude Code** | Native hooks (`settings.json`) + costs.jsonl | `statusLine` hook (5h/7d) | `CONNECTED` (ready to hook) | `LIVE` (statusline), `LOCAL` (tokens) |
| **Codex CLI** | Native hooks (`hooks.json`) | None | `CONNECTED` (config present) | `LOCAL` / `N/A` |
| **Antigravity** | Transcripts & Process | In-app `/usage` only | `CONNECTED` (active IDE) | `N/A` (with open button) |
| **Copilot** | Process & `~/.copilot/logs` | None | `CONNECTED` (dirs present) | `N/A` |
| **Cursor** | Process detection | None | `NOT_INSTALLED` | `N/A` |
| **Gemini CLI** | Process detection | None | `NOT_INSTALLED` | `N/A` |
