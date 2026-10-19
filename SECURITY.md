# Security Model

## Status

This document states requirements for implementation and release. It is not a security certification or a claim that controls have already been implemented.

## Trust boundaries

Treat agent hook payloads, CLI arguments, IPC messages, PetPacks, imported images, cloud responses, provider endpoints, and update artifacts as untrusted. Validate at every boundary and keep the UI, local runtime, cloud API, and operating system privilege boundaries explicit.

## Desktop controls

- Tauri uses explicit capabilities per window and narrowly scoped commands.
- No generic renderer-to-shell command, unrestricted filesystem API, or arbitrary plugin execution.
- Local IPC is per-user, authenticated, versioned, size-bounded, and resistant to replay where feasible.
- Hook installation is minimal, reversible, and preserves user-owned configuration.
- The process wrapper uses argument vectors and never constructs a shell command from untrusted strings.
- Provider credentials are stored with Keychain, Windows Credential Manager, or Secret Service APIs; SQLite stores only a reference.

## PetPack and media controls

Enforce archive/file count, compressed and extracted size, compression ratio, path, format, dimensions, frame count, decoded memory, and audio duration limits. Reject path traversal, symlinks where unsafe, executables, scripts, dynamic libraries, and unknown payload types. Decode media in a constrained worker. Never execute assets or interpret arbitrary HTML/JavaScript.

## Cloud controls

Use authentication with short-lived access tokens and rotating refresh tokens, authorization checks on every object, rate limits, schema validation, secure headers, origin policy, and audit events. Apply CSRF protection if cookies authenticate browser requests. Store secrets in a managed secret service. Never return stack traces or internal paths. Minimize collected data and define deletion/retention behavior.

## Logging and incident response

Redact API keys, tokens, prompts, terminal output, and sensitive paths before logging. Crash reports and diagnostic bundles require consent and preview of included data. Provide a vulnerability reporting contact before public releases, triage process, supported-version policy, and incident notification process.

## Release gates

Before release, run dependency and secret scans, static analysis, package signature verification, SBOM generation, PetPack adversarial tests, IPC capability review, and credential-leak checks. Update metadata and installers must be signed and checksum-verified. A release must support rollback or documented recovery.
