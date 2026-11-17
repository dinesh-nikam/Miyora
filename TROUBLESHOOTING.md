# Troubleshooting Guide

This guide provides diagnostic procedures and recovery solutions for common issues encountered during development, installation, and operation of **Miyora** (AgentPet).

---

## 1. Fast Diagnostic Checks

Run the built-in diagnostic tool to inspect your environment, database accessibility, and agent CLI discovery:

```bash
cargo run -p agentpet-cli -- doctor
```

Verify XP ledger cryptographic hash-chain integrity:

```bash
cargo run -p agentpet-cli -- verify-ledger
```

Inspect companion runtime state:

```bash
cargo run -p agentpet-cli -- status
```

---

## 2. Common Categories & Resolutions

### 2.1 Toolchain & Linking Issues (Windows)

- **Symptom:** `error: linking with link.exe failed: exit code: 1104` or missing Visual C++ Build Tools.
- **Cause:** Rust's default target on Windows is `x86_64-pc-windows-msvc`, which requires Microsoft Visual C++ Build Tools.
- **Fix:** If MSVC is not installed, switch to the MinGW GNU toolchain:
  ```bash
  rustup default stable-x86_64-pc-windows-gnu
  cargo test --workspace
  ```

### 2.2 Build Lock Contention

- **Symptom:** `Blocking waiting for file lock on build directory`.
- **Cause:** Another `cargo`, `rustc`, or rust-analyzer instance is holding the target directory lock.
- **Fix:** Inspect running processes and terminate dangling cargo processes:
  ```powershell
  Get-Process | Where-Object { $_.ProcessName -match "cargo|rustc" } | Stop-Process
  ```

### 2.3 Transparent Overlay Not Visible or Misplaced

- **Symptom:** The pet overlay does not appear, or spawns off-screen.
- **Diagnostics:**
  - Miyora stores desktop coordinates in SQLite (`companion_preferences` table, schema v11).
  - If a display was disconnected or display scaling changed, the coordinates may be outside the current monitor's viewport.
- **Fix:**
  - Use the system tray icon to select **Reset Overlay Position**, or invoke the reset action from the Command Deck / Settings tab.
  - Alternatively, launch with default coordinates:
    ```bash
    cargo run -p agentpet-cli -- run -- echo "reset"
    ```

### 2.4 Agent Hook Installation & Discovery

- **Symptom:** Coding sessions run in Claude Code or Codex, but Miyora pet state does not change.
- **Diagnostics:**
  - Inspect hook status:
    ```bash
    cargo run -p agentpet-cli -- agents status claude_code
    cargo run -p agentpet-cli -- agents status codex
    ```
- **Fix:**
  - Install the native hook:
    ```bash
    cargo run -p agentpet-cli -- agents install claude_code
    cargo run -p agentpet-cli -- agents install codex
    ```
  - For universal agents (Aider, OpenCode, Gemini CLI), wrap the invocation:
    ```bash
    agentpet run -- aider
    ```

### 2.5 Database Lock or Corruption Prevention

- **Symptom:** `DATABASE_LOCKED` or `sqlite3: database is locked`.
- **Diagnostics:**
  - Miyora runs SQLite with WAL (Write-Ahead Logging) and busy timeouts.
  - In multi-process scenarios (daemon + CLI + UI), concurrent transactions wait up to 5 seconds.
- **Fix:**
  - Ensure background test runners or instances are shut down cleanly before starting a new session.
  - Never delete the SQLite database to bypass locks—Miyora uses append-only hash chains; deleting the database wipes earned XP and pet milestones.

### 2.6 PetPack Validation Failures

- **Symptom:** `PET_PACK_INVALID` or `ASSET_LIMIT_EXCEEDED` on import.
- **Diagnostics:**
  - PetPacks are data-only and strictly validated against directory traversal (ZipSlip), MIME types, and dimensions (max 4096×4096).
  - Run the validator to see detailed line-by-line lint warnings:
    ```bash
    python scripts/validate_pet_packs.py
    ```

### 2.7 3D WebGL Context Loss in UI

- **Symptom:** 3D avatar viewport goes blank or reports WebGL context loss.
- **Diagnostics:**
  - Occurs when GPU drivers restart, laptops switch between integrated and discrete GPUs, or system resumes from deep sleep.
- **Fix:**
  - Miyora's `AvatarStage.tsx` listens for context loss and safely re-initializes Three.js geometries and textures without leaking GPU memory.
  - Switch tabs or toggle 2D/3D mode in the Den tab to force an immediate re-render.

---

## 3. Privacy-Safe Diagnostic Export

When reporting issues or requesting support:
- Never share database files containing proprietary project paths or personal names.
- All logs produced by `agentpet doctor` and the CLI automatically redact prompts, credentials, terminal stdout/stderr, and API keys.
