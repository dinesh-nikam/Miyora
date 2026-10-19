# Event Protocol Specification

## 1. Scope
Defines the normalized event fields emitted by agent hooks, wrapper CLI, and daemon.

## 2. Normalized Event Schema
Every event sent from CLI hooks to the local daemon contains:
- `agent_id`: string (e.g. `claude_code`, `codex`, `gemini_cli`, `cursor`, `windsurf`, `universal`)
- `session_id`: UUIDv4 string
- `project_path`: string (local file path; never transmitted off-device)
- `project_name`: string (basename of the project directory)
- `timestamp`: ISO 8601 UTC timestamp string
- `event_type`: enum string:
  - `session_start`: Agent session spawned
  - `working`: Agent is executing commands, generating code, or running tools
  - `tool_use`: Agent invoked a specific tool (optional payload: `tool_name`)
  - `waiting_input`: Agent is blocked waiting for user prompt or permission (optional payload: `waiting_reason`)
  - `done`: Agent finished session or exited
  - `session_end`: Terminal event acknowledging cleanup
- `model_name`: optional string (e.g. `claude-3-7-sonnet`, `gemini-2.5-pro`)
- `token_usage_delta`: optional object with `input_tokens`, `output_tokens`, `cache_read_tokens`
- `subagent`: boolean flag indicating whether the event originates from a spawned subagent
