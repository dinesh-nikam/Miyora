# Contributing

## Before changing code

Read `AGENTS.md`, `ARCHITECTURE.md`, and the relevant domain specification. Inspect repository status, history, module boundaries, tests, build scripts, and platform-specific code before editing. Preserve unrelated work and avoid replacing functioning architecture without evidence.

## Change requirements

- Keep changes scoped and aligned with the owning module.
- Add validation and clear errors at trust boundaries.
- Add or update tests for changed behavior, especially event, persistence, progression, IPC, pack, and sync contracts.
- Update migrations and documentation when schemas or public contracts change.
- Keep TypeScript strict and Rust formatted/Clippy-clean; justify exceptional unsafe or dynamic behavior in code review.
- Never add fake agent events, token usage, session completion, unlicensed character art, credentials, or generated private user data.
- Do not suppress errors or leave TODOs/stubs in shipped paths.

## Pull request checklist

1. Describe the user-visible behavior and affected modules.
2. State data/privacy effects and migration compatibility.
3. List validation performed and platform coverage.
4. Confirm no secrets, private data, or unlicensed assets are included.
5. Include screenshots for UI changes and sanitized fixtures for adapter changes.

## Git conventions

Use focused commits with imperative subjects. Do not rewrite shared history. Do not commit build artifacts, credentials, local databases, signing keys, or user PetPacks. Keep lockfiles committed for applications and tools. Version public contracts and document breaking changes.
