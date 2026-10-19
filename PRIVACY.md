# Privacy and Data Handling

## Default behavior

The product is local-first. Pet state, local session history, progression, project associations, custom packs, and settings stay on the device unless the user explicitly enables a feature that transmits selected information.

## Data categories

| Data | Default location | Network behavior |
|---|---|---|
| Pet configuration and assets | Local disk | None |
| Agent events and session history | Local SQLite | None |
| Project paths | Local SQLite | Never uploaded by default |
| Provider credentials | OS credential store | Sent only to the configured provider when the user invokes it |
| Community PetPack requests | Local until browsing/install | Manifest and assets downloaded from selected library |
| Sync/profile data | Local outbox until opt-in | Only fields disclosed in sync settings are sent |
| Diagnostics | Local logs | Upload only after explicit user review/consent |

Never silently upload source code, prompts, terminal output, API keys, private files, or full project paths. Usage analytics must be opt-in, minimized, and described in plain language. Do not infer that enabling cloud library browsing means enabling progress sync.

## User controls

Settings must show whether sync is enabled, the categories synchronized, connected accounts/providers, local retention duration, diagnostic upload contents, and controls to export/delete local data and disconnect cloud services. Core offline behavior remains available after sign-out.

## Retention

Session/event retention defaults to 90 days and is configurable. Audit and XP ledger retention is separate; progression history must not be silently deleted by session cleanup. Cloud retention and account deletion behavior must be documented before enabling cloud accounts.
