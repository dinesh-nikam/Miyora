# AgentPet Provider Investigation & Evidence

**Platform:** Windows 10/11 x86_64  
**Date Inspected:** 2026-10-21  
**Inspector:** Carxen (Senior Software Engineer)  
**Rule:** Ground Rule 4 — Build only on what exists on the real machine. Never invent quota or fabricate status.

---

## 1. Claude Code (`claude_code`)

### Machine Evidence
- **Home Directory:** `C:\Users\DineshNikam\.claude` (Verified present)
- **Configuration File:** `C:\Users\DineshNikam\.claude\settings.json` (Verified present, 5,695 bytes)
  - Existing `hooks` object contains: `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`
  - Existing `statusLine` command:
    ```json
    {
      "type": "command",
      "command": "powershell -ExecutionPolicy Bypass -File \"C:\\Users\\DineshNikam\\.claude\\plugins\\cache\\caveman\\caveman\\63e797cd753b\\hooks\\caveman-statusline.ps1\""
    }
    ```
- **Session Logs:** `C:\Users\DineshNikam\.claude\projects\**\*.jsonl` (Verified present, multiple project directories e.g. `D--b-erp`)
- **Cost & Token Records:** Verified in session JSONL files with line type `cost-state`:
  ```json
  {
    "type": "cost-state",
    "sessionId": "96005500-0d06-4be0-97ea-5b792d21eb98",
    "totalCostUSD": 5.0305358,
    "modelUsage": {
      "claude-opus-5-5": {
        "inputTokens": 1564,
        "outputTokens": 19269,
        "thinkingTokens": 2653,
        "cacheReadInputTokens": 8704419,
        "cacheCreationInputTokens": 362252
      }
    }
  }
  ```

### Activity Detection Strategy
- **Primary:** Native hooks configured in `~/.claude/settings.json` (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Notification`, `Stop`, `SubagentStop`).
- **Hook Transport:** Hooks call `agentpet hook --adapter claude_code` which passes event data over local IPC (Windows Named Pipe `\\.\pipe\agentpet-<hash>` or falls back to direct core append). Hook binary fails open with <50ms timeout; never blocks Claude Code.
- **Quota & Rate Limits:**
  - Claude Code passes JSON to `statusLine` via stdin containing `rate_limits.five_hour` (`used_percentage`, `resets_at`), `rate_limits.seven_day`, and `context_window.used_percentage`.
  - Only available for Pro/Team/Enterprise accounts. If `rate_limits` is absent, show `N/A` (never fabricated).
  - StatusLine shim intercepts stdin, writes `claude_status.json` locally, and chains the existing `caveman-statusline.ps1` command so existing workflows are preserved.
- **Tokens/Cost:** Scraped from local `~/.claude/projects/**/*.jsonl` files and tagged with provenance chip `LOCAL`.
- **Health State:** `CONNECTED` (when installed and active), `DEGRADED` (when hooks are absent or settings unreadable).

---

## 2. OpenAI Codex CLI (`codex`)

### Machine Evidence
- **Home Directory:** `C:\Users\DineshNikam\.codex` (Verified present)
- **Configuration & Hooks:**
  - `C:\Users\DineshNikam\.codex\hooks.json` exists with structure:
    ```json
    {
      "hooks": {
        "PreToolUse": [{ "matcher": "Grep|Glob|Bash", "hooks": [...] }],
        "PostToolUse": [{ "matcher": "Bash", "hooks": [...] }]
      }
    }
    ```
  - `C:\Users\DineshNikam\.codex\history.jsonl` exists with 86 real session prompt lines:
    `{"session_id":"01a153d2-48a1-7f73-ab3c-7c17b90a55a1","ts":1792429566,"text":"continue"}`
  - `C:\Users\DineshNikam\.codex\sessions` directory exists.

### Activity Detection Strategy
- **Primary:** Hooks merged into `~/.codex/hooks.json` matching `PreToolUse` and `PostToolUse` or log-tail of `history.jsonl`.
- **Secondary:** Process tracking for `codex.exe`.
- **Quota:** Not exposed locally by Codex CLI. Quota gauge displays `N/A` with provenance chip `N/A`.
- **Health State:** `CONNECTED` if hooks or active session files detected, otherwise `DEGRADED` / `NOT_INSTALLED`.

---

## 3. Google Antigravity / Gemini CLI (`antigravity` / `gemini_cli`)

### Machine Evidence
- **Installation Directory:** `C:\Users\DineshNikam\.gemini` (Verified present)
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
- **Home Directory:** `C:\Users\DineshNikam\.copilot` (Verified present)
- **Subdirectories:** `config.json`, `ide`, `logs`, `session-state`.
- **VS Code Extension Storage:** `C:\Users\DineshNikam\AppData\Roaming\Code\User\globalStorage` (Verified present).

### Activity Detection Strategy
- **Primary:** Process observation (`copilot-agent`, `code` with copilot worker) and log tail of `~/.copilot/logs`.
- **Quota:** GitHub Copilot does not expose rate limit numbers in local files. Shows `N/A` with status indicator (Working / Idle / Offline).
- **Health State:** `CONNECTED` if logs active, `IDLE` or `NOT_INSTALLED`.

---

## 5. Cursor (`cursor`)

### Machine Evidence
- `C:\Users\DineshNikam\.cursor`: Absent on this machine.
- `C:\Users\DineshNikam\AppData\Roaming\Cursor`: Absent on this machine.

### Activity Detection Strategy
- **Primary:** Process detection for `Cursor.exe` / window title scanning.
- **Quota:** `N/A`.
- **Health State:** `NOT_INSTALLED`. Displayed as offline with clear `NOT INSTALLED` status. No fake data.

---

## 6. Summary Matrix

| Provider | Activity Detection | Quota Exposure | Health on This Machine | Provenance Label |
|---|---|---|---|---|
| **Claude Code** | Native hooks (`settings.json`) + project JSONL | `statusLine` hook (5h/7d) | `CONNECTED` (ready to hook) | `LIVE` (statusline), `LOCAL` (tokens) |
| **Codex CLI** | Hooks (`hooks.json`) + `history.jsonl` | None | `CONNECTED` (config present) | `LOCAL` / `N/A` |
| **Antigravity** | Transcripts & Process | In-app `/usage` only | `CONNECTED` (active IDE) | `N/A` (with open button) |
| **Copilot** | Process & `~/.copilot/logs` | None | `CONNECTED` (dirs present) | `N/A` |
| **Cursor** | Process detection | None | `NOT_INSTALLED` | `N/A` |
| **Gemini CLI** | Process detection | None | `NOT_INSTALLED` | `N/A` |
