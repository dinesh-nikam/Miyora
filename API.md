# API and IPC Contracts

## Status

The cloud REST and daemon contracts below remain planned. The current desktop shell implements the local Tauri commands listed under **Implemented local commands**.

### Implemented local commands

| Command | Purpose | Access |
|---|---|---|
| `get_runtime_snapshot` | Read selected pack, pet state, and last accepted event time | Main and overlay windows |
| `get_selected_pet_animation` | Read the selected pack’s validated mapped sprite animation | Main and overlay windows |
| `get_progression` | Read current local XP and level | Main window |
| `get_xp_history` | Read the ten most recent immutable XP awards for the active pet | Main window |
| `get_achievements` | Read local achievement definitions, progress, and unlock timestamps for the active pet | Main window |
| `get_session_history` | Read persisted agent sessions with bounded `limit` and `offset` pagination | Main window |
| `list_installed_packs` / `list_pet_pack_cards` | Read installed local packs and generated previews | Main window |
| `install_pet_pack` | Validate and install an extracted local PetPack folder | Main window |
| `create_pet_from_image` | Decode, normalize, validate, and install a static local companion image | Main window |
| `select_pet` | Select an installed PetPack | Main window |
| `show_pet_overlay` | Reveal the configured companion window | Main window |

## Local Tauri commands

Commands are typed, explicit, capability-scoped, validated, and asynchronous for work that can take measurable time. Planned command groups:

| Command | Purpose |
|---|---|
| `get_pet_state` | Read selected pet and current normalized state |
| `list_pets` | Page through installed local PetPacks |
| `install_pet_pack` | Start validated import/install job and return job ID |
| `get_asset_job` | Read worker status and structured error |
| `remove_pet_pack` | Remove an unused pack with confirmation and audit record |
| `list_agents` | Report detected/installed adapters and readiness |
| `update_agent_hook` | Install/remove a specific reversible adapter hook |
| `get_sessions` | Query local session history with bounded filters/page size |
| `update_settings` | Validate and persist a typed settings patch |
| `assign_project_pet` | Set or clear a project-specific pet assignment |

Long-running operations return a job ID and emit progress events. Do not block the renderer or expose a general-purpose command runner.

## Local daemon protocol

Use a versioned JSON envelope over a same-user Unix domain socket (macOS/Linux) or named pipe (Windows):

```json
{
  "protocolVersion": 1,
  "messageId": "uuid",
  "sentAt": "RFC3339 timestamp",
  "kind": "event.submit",
  "payload": {}
}
```

The receiver authenticates the local peer, limits frame size and rate, validates timestamps/schema, deduplicates event IDs, and returns a structured acknowledgement. Never accept arbitrary filesystem paths or shell commands in event payloads. Protocol compatibility is explicit; unsupported versions return `PROTOCOL_VERSION_UNSUPPORTED`.

## Cloud REST API

All endpoints are under `/api/v1`. Planned resource groups include:

```text
POST   /auth/login
POST   /auth/refresh
POST   /auth/logout
GET    /pets
GET    /pets/{id}
POST   /pets
PATCH  /pets/{id}
DELETE /pets/{id}
POST   /pets/{id}/versions
POST   /pets/{id}/install-token
POST   /pets/{id}/favorite
GET    /library
GET    /library/search
POST   /library/{id}/reports
GET    /projects
GET    /sessions
GET    /usage
GET    /achievements
GET    /leaderboard
POST   /sync
GET    /sync/state
```

Mutating requests use authentication, authorization, schema validation, rate limiting, and idempotency keys where retries can duplicate effects. List endpoints use cursor pagination with bounded page sizes and documented filtering/sorting. OpenAPI is the source for generated client contracts.

## Error envelope

```json
{
  "error": {
    "code": "PET_PACK_INVALID",
    "message": "The pet pack did not pass validation.",
    "details": {},
    "requestId": "opaque-request-id"
  }
}
```

Error codes are stable machine-readable values. User messages are safe and localized by the client. Internal traces remain in access-controlled server diagnostics and never appear in the response.

## Live updates

WebSockets are reserved for live web dashboard updates after authenticated connection. Event names use namespaced versionable values such as `session.started`, `session.completed`, `pet.updated`, `achievement.unlocked`, and `sync.completed`. Reconnect requires cursor/replay handling; REST remains authoritative for recovery. Do not add WebSockets to local desktop IPC where a typed event channel suffices.
