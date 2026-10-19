# Local and Cloud Database Design

## Status

The schemas below describe the target data model. The Rust core currently implements SQLite migrations for settings, pet runtime state, installed pack records, accepted agent events, local agent sessions with bounded recent-history queries, progression rules, immutable XP and achievement awards, and materialized per-pet XP/level totals; remaining entities are planned and are not claimed as implemented.

## Local SQLite ownership

Only the Rust `database` crate accesses SQLite. Enable foreign keys, use prepared statements, explicit transactions, versioned migrations, and indexes for all production query paths. Store timestamps as UTC integer milliseconds or normalized ISO timestamps consistently. Use stable UUIDs for cross-device entities and opaque IDs for credentials.

Core local entities:

| Table | Key data and constraints |
|---|---|
| `projects` | ID, display label, normalized local path hash, created/updated time; raw path remains local |
| `installed_pet_packs` | Pack ID/version/name/creator/license/source, local install path, serialized validated manifest, install time; unique `(pack_id, version)` |
| `pet_assets` | Pack FK, relative path, media type, decoded dimensions, content hash |
| `pet_instances` | Instance ID, pack FK, nickname, appearance/configuration, active state |
| `project_pet_assignments` | Project FK, pet instance FK, unique project assignment |
| `agents` | Adapter ID, installation state, safe configuration metadata |
| `agent_sessions` | Agent ID, local project path, pet identity captured at session start, start/end, monotonic duration, status, process exit code, nullable actual usage; wrapper records usage as null |
| `agent_events` | Unique event ID/idempotency key, session FK, type, timestamps, bounded metadata |
| `providers` / `provider_models` | Non-secret configuration and opaque credential reference |
| `xp_transactions` | Immutable transaction ID, pet identity (installed pack selection or reserved `local.default`), source/source ID, amount, timestamp, metadata; unique idempotency key |
| `progression_config` | One local rules row: XP for a successful completed session, base XP to reach level 2, and per-level XP increase |
| `pet_progression` | Materialized XP total and current level; recomputable from immutable XP transactions |
| `achievement_definitions` / `pet_achievements` | Seeded completed-session milestones, XP reward, and immutable per-pet unlock with source session |
| `pet_state` | Singleton row: current state name, last accepted event time, and UTC `state_entered_at` used for timed-state expiry (schema v6; backfilled from the last event time or migration time) |
| `settings` / `custom_messages` | Typed key/value or normalized configuration with validation version; current setting includes physical desktop companion coordinates, never credentials |
| `audit_logs` | Safe action metadata, actor/source and timestamp; excludes secrets and prompts |
| `sync_queue` | Event ID, entity, revision, operation, retry state, idempotency key |
| `schema_versions` | Applied migration ID, checksum, timestamp |

Create foreign keys with deliberate delete behavior. Session retention may remove old session detail according to user settings, but must not erase immutable progression ledger records. Use transactions to accept events, update sessions, add XP, and unlock achievements atomically where they share an event source. Unique constraints prevent duplicate awards during retries and crash recovery.

## Migration and recovery

The current local schema version is 6. Opening a database recorded with a newer version fails with `UnsupportedDatabaseSchema` instead of running against an unknown schema. Each migration is ordered, immutable after release, checksummed, and tested from the prior supported schema. Before destructive or complex migrations, create a recoverable backup. On migration failure, preserve the original database and provide an actionable diagnostic. Do not silently recreate a damaged database or lose user progress.

## Cloud PostgreSQL boundary

Cloud stores account/profile data, public PetPack metadata and versions, moderation/report state, device sync cursors, and privacy-minimized aggregates only with user consent. It does not receive source code, prompts, terminal output, provider keys, private files, or full project paths by default. Use tenant-scoped authorization, foreign keys, indexes, audit trails, retention rules, and transactional migrations.

## Sync records

Every synchronized change carries `deviceId`, `eventId`, `revision`, `updatedAt`, and `idempotencyKey`. Append-only progress merges by event identity. Mutable settings use documented revision conflict rules. Sync cursors and retries are durable; a conflict cannot reduce progression or erase an achievement.
