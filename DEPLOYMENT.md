# Deployment and Release

## Status

No build pipeline, installer, cloud deployment, or update service exists yet. This document sets release requirements; it is not evidence of shipped artifacts.

## Desktop release pipeline

GitHub Actions should run formatting, lint, strict TypeScript typecheck, Rust Clippy, unit/integration tests, dependency and secret scans, then build/package on supported OS runners. Produce platform-specific artifacts for macOS Apple Silicon/Intel where supported, Windows x64 (ARM64 when validated), and Linux x64 (ARM64 when validated). Choose DMG/PKG, MSI/EXE, AppImage/DEB/RPM based on tested platform support.

Release outputs include versioned artifacts, checksums, SBOM, signature metadata, release notes, migration notes, and rollback guidance. Store signing credentials in protected CI secret storage. Never put signing keys in repository files or logs.

## Signing and updates

Sign desktop binaries and installers using platform-supported signing/notarization. Update manifests and payloads must be signed; clients verify signature, hash, application identity, and version compatibility before applying. Support staged rollout where available, user notification, manual update, interrupted-download recovery, and rollback to the last working version. Unsigned downloads must not be executed as updates.

## Optional cloud deployment

Deploy NestJS as a modular service with PostgreSQL migrations gated before application rollout, managed secrets, TLS, rate limiting, health/readiness checks, object storage, CDN, backups, and monitoring. Local Docker Compose supports development only. Define environment separation, data retention, incident response, and regional requirements before production use.

## Backup and disaster recovery

Desktop backup/export must include user-selected local data and packs without exporting provider secrets. Verify backups with restore tests. Cloud operations define encrypted backups, retention, recovery-point/recovery-time targets, restore exercises, and account/data deletion procedures before launch.
