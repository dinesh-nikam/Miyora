# Cloud Synchronization Specification

## 1. Principles
- Offline-first: The application functions completely without network or authentication.
- Privacy-by-default: Only last directory name (`project_name`) is shared; full filesystem paths are never uploaded.
- Forward-only merge: Progress cannot move backward on any client.

## 2. Merge Rules
When syncing pet state between client and cloud:
- `xp = max(local.xp, remote.xp)`
- `tokens = max(local.tokens, remote.tokens)`
- `sessions = max(local.sessions, remote.sessions)`
- `achievements = union(local.achievements, remote.achievements)`
- `level = calculate_level(merged_xp)`: Level is deterministically derived from merged XP to maintain invariant consistency with the level progression curve.
