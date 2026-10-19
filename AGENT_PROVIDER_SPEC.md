# Agent and Provider Integration Specification

## Status and terminology

This defines integration contracts. The universal process wrapper is implemented at an initial level. The CLI can discover five known executable names on PATH, but native agent hooks/adapters and provider integrations are not implemented. PATH discovery does not verify the executable's identity or version and does not mean that activity outside `agentpet run` is observed. An **agent** is a coding application or CLI whose activity is observed. A **provider** is a model API or local model endpoint. These are independent: one agent can use multiple providers, and provider credentials do not belong in agent adapters.

Current PATH names: Claude Code (`claude`), OpenAI Codex CLI (`codex`), Gemini CLI (`gemini`), Aider (`aider`), and OpenCode (`opencode`). `agentpet agents list` reports found paths only; it may detect Windows command launchers whose invocation through the wrapper is not yet supported. When a successfully launched wrapper command itself has one of these executable names, its session receives the corresponding stable local ID; other commands are recorded as `universal`.

## Agent adapter contract

An adapter declares a stable ID, display name, supported platforms, detection strategy, event sources, install/uninstall behavior, and parser version. It consumes documented hook payloads, structured output, or process lifecycle data and emits validated normalized events. It must not infer completion merely from silence unless that heuristic is explicitly labeled and user-configurable.

Normalized event types include agent started/working/thinking/waiting/input-required/completed/failed, tool started/completed/failed, token usage when supplied by a trustworthy source, and session lifecycle events. Each event carries a stable source ID or idempotency key where available. Preserve unknown fields only in bounded, privacy-filtered diagnostic metadata.

### Hook lifecycle

Installation first backs up relevant configuration, applies the smallest reversible change, verifies the result, and records ownership of that change. Uninstall removes only AgentPet-owned entries and preserves concurrent user edits. Hooks use a versioned local IPC protocol, bounded payloads, same-user authentication, and timeouts. Failure to reach the daemon must not corrupt the coding agent's own execution.

### Universal wrapper

`agentpet run -- <command> [args...]` launches the target without shell-string interpolation, streams captured stdout and stderr to their corresponding terminal streams, records working directory/project association and monotonic duration, waits for the actual exit, and persists a session plus start/completion/failure events. A successful completion awards configured XP in the same SQLite transaction as session completion; failed or interrupted sessions do not award XP. It returns the child's normal exit code unchanged. Unix signal termination is stored as interrupted with no exit code and returned using the conventional `128 + signal` shell status. Output is not persisted. If local tracking cannot initialize, the command still runs and the CLI reports that tracking is unavailable. No token counts or intermediate agent states are fabricated.

## Provider contract

Provider configuration includes provider ID, endpoint type, base URL, organization/tenant where relevant, selected model, timeout, bounded retry policy, capability flags, enabled state, health state, and an opaque credential reference. Secrets are resolved only inside a Rust service using the operating-system credential store. They never cross into React state or persistent database fields.

Provider types include hosted APIs, OpenAI-compatible endpoints, and local endpoints such as Ollama or LM Studio. Validate URL schemes and prevent unsafe local-network access according to the feature's threat model. Health checks are explicit, user initiated or clearly disclosed, and do not send prompts.

## Usage and cost

Record token usage only when the integration exposes measured values. Unknown values stay null; they are not estimated from output size. Cost is derived from a versioned pricing record with currency and effective dates and is labeled an estimate. Provider invoices remain the source of billed cost.

## Adapter test contract

Each adapter supplies sanitized fixtures from documented formats, parser version tests, malformed-input tests, deduplication tests, hook install/uninstall tests, and an end-to-end process test when process-based. Fixtures must not contain credentials, private project content, or fabricated claims about production support.
