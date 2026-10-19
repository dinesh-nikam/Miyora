# Session State Machine and Aggregate Model

## 1. Per-Session States
A session operates strictly in one of four states:
- `idle`: Not actively processing.
- `working`: Actively executing commands, writing code, or generating responses.
- `waiting`: Blocked waiting for human input, approval, or clarification.
- `done`: Session completed execution.

### Transition Rules
1. `session_start` -> `working` (or `idle` if started without initial task)
2. `working` -> `waiting`: Triggered when agent asks for input or permission.
3. `waiting` -> `working`: Triggered when human submits response or resumes execution.
4. `working` or `waiting` -> `done`: Triggered on session completion, clean exit, or process termination.
5. Inactivity timeout: If a session in `working` receives no events for 15 minutes (900 seconds), it automatically transitions to `idle` (stale).

## 2. Aggregate Tray State
The tray monitor calculates an aggregate state across all active sessions:
- If ANY session is in `waiting`: Aggregate state is `waiting` (tray icon turns orange with count).
- Else if ANY session is in `working`: Aggregate state is `working`.
- Else: Aggregate state is `idle`.
