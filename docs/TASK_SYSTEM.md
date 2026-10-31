# Task System

Code: `crates/studio/src/task.rs` (states, policy), `store.rs` (guarded writes),
`service.rs` (`run_poller`, `poll_once`, `collect_outputs`).

## States

`QUEUED → RUNNING → SUCCEEDED | FAILED | CANCELLED | EXPIRED`

Allowed transitions (`TaskState::can_transition_to`):

| From | To |
|---|---|
| QUEUED | any |
| RUNNING | RUNNING, SUCCEEDED, FAILED, CANCELLED, EXPIRED |
| FAILED | RUNNING — only via **Resume**, only when the failure kind is transient |
| SUCCEEDED, CANCELLED, EXPIRED | none |

`SUCCEEDED` is written only by `StudioStore::complete_task`, after every model
output was downloaded and passed `media::inspect_model`.

| Provider status | Local state | Stage |
|---|---|---|
| queued | QUEUED | `queued` |
| running | RUNNING | `generating` |
| success | RUNNING → (download, validate) → SUCCEEDED | `downloading` |
| failed / banned | FAILED (`task_failed`) | — |
| expired | EXPIRED (`task_timeout`) | — |
| cancelled | CANCELLED | — |
| unknown value | unchanged | `unknown_status` |

`progress` is stored only when the provider reports it; the UI shows a bar only
then.

## Polling (`PollPolicy`, stored in settings, editable in the Studio settings dialog)

| Field | Default | Bounds |
|---|---|---|
| `intervalSecs` | 3 | 2–60 |
| `maxIntervalSecs` | 30 | ≥ interval |
| `timeoutSecs` | 1800 | 60–10800 |
| `maxConsecutiveErrors` | 6 | ≥ 1 |

- One poller thread per task; a set of active task IDs prevents duplicates.
- `rate_limit` / `provider_unavailable`: delay = interval × 2^errors, capped,
  never below the provider's `Retry-After`. The task keeps its state with stage
  `retrying`; after `maxConsecutiveErrors` it becomes FAILED (resumable).
- Any other provider error, missing credentials, or local failure ends the loop
  with FAILED.
- Elapsed tracking time ≥ `timeoutSecs` → FAILED `task_timeout` (resumable;
  the provider may still finish).
- On app start, all QUEUED/RUNNING tasks are tracked again.
- The React UI never polls the provider; it re-reads local task rows every 2 s
  only while a task is active.

## Resume vs. Retry

| Action | When offered | Effect | Credits |
|---|---|---|---|
| Resume | FAILED with kind `rate_limit`, `provider_unavailable`, `task_timeout`, `conversion` | Same provider task polled again (`attempts + 1`) | none |
| Retry / Generate new version | other failures, or any version | New provider task with the stored request and fresh seeds; new version with `parent_version_id` lineage | yes |

Cancel is not offered: the provider documents no cancel endpoint.

## Idempotency

- `tasks UNIQUE(provider, provider_task_id)`: replayed submissions return the existing task.
- `asset_versions UNIQUE(generation_id)`, `generation_outputs UNIQUE(generation_id, label)`,
  `usage_records PRIMARY KEY(task_id)`: replayed completion creates no duplicates.
- Terminal tasks ignore late poll results (`update_task` / `fail_task` return `false`).

## Logs

One JSON line per event on stderr: `component`, `operation`, `taskId`,
`provider`, `status`, `errorKind`, `durationMs`. Prompts, keys, URLs, file
contents and provider bodies are never logged.
