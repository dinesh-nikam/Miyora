# PetPack Specification

## Status

This is the v1 data contract. The Rust crate implements manifest parsing, validation, installation from an already-extracted local folder, and local static pet generation from user images. Archive parsing and online signature verification remain unimplemented.

## Package rules

A PetPack is a ZIP-compatible archive containing `pet.json` and only schema-approved data assets. It must not contain scripts, executables, dynamic libraries, arbitrary HTML, or executable plugins. Pack identity is stable across versions; a new release increments the semantic version and may migrate its manifest schema only through a documented migration.

## Manifest

```json
{
  "schemaVersion": 1,
  "id": "creator.example-companion",
  "name": "Example Companion",
  "version": "1.0.0",
  "description": "A short description.",
  "creator": "Creator name",
  "license": "CC-BY-4.0",
  "source": "https://example.invalid/source",
  "attribution": "Attribution text required by the license.",
  "commercialUse": false,
  "redistribution": true,
  "derivativeWorks": true,
  "assets": {
    "thumbnail": "assets/thumbnail.png",
    "idleSprite": "assets/idle.png"
  },
  "animations": {
    "idle": {
      "asset": "idleSprite",
      "frames": [{ "x": 0, "y": 0, "width": 128, "height": 128 }],
      "fps": 1,
      "loop": true,
      "anchor": { "x": 0.5, "y": 1.0 }
    }
  },
  "stateMappings": { "Idle": "idle" },
  "tags": ["original", "cute"]
}
```

The example is a schema illustration only. It does not ship artwork or establish a license for any real character.

## Current validator and installation contract

The Rust `agentpet-pet-pack` crate parses and validates v1 manifests and installs regular asset files from an already-extracted local directory. Installation copies only declared assets and `pet.json` into a staging directory, validates the staged copy, generates an application-owned `generated/preview.png`, then renames the stage into the local library. The Rust core records the installed version in SQLite and removes the installed directory if the database write fails. The validator decodes declared PNG, JPEG, GIF, and WebP assets with a 4096 × 4096 dimension ceiling and a 64 MiB decoder allocation limit, verifies sprite frame bounds against decoded dimensions, and enforces safe relative paths and file-size limits. User images are decoded, resized to a maximum 512 × 512 PNG, packaged as a static idle animation with state mappings, and installed through the same validator; animated GIF frames are not extracted. The desktop shell renders selected state-mapped animations and static library previews. Archive unpacking and online signature verification are not implemented.

The complete installation pipeline must:

1. Enforce compressed archive size and compression-ratio limits before extraction.
2. Normalize every path and reject absolute paths, `..`, symlinks, duplicate normalized paths, and files outside the pack root.
3. Reject files with executable or unsupported content. Validate extension and decoded format; do not trust MIME declarations alone.
4. Validate manifest schema, unique IDs, semantic version, required creator/license/source/attribution, animation references, frame rectangles, and state mappings.
5. Enforce configurable limits for file count, dimensions, decoded memory, frame count, audio duration, and total extracted size.
6. Decode images in a worker, generate local thumbnail/preview derivatives, and report failures as structured errors.
7. Install into a staging directory and atomically move into the versioned local library only after all checks pass. The current implementation supports an extracted local folder only.
8. Record pack hash, source, license metadata, install time, and validation version. Never execute pack content.

Library downloads additionally require TLS, manifest signature verification or trusted registry metadata, and a cryptographic content hash. Updates are installed side-by-side and activated atomically so rollback remains possible.

## Runtime and compatibility

The renderer consumes validated `AnimationDefinition` values and state-to-animation mappings. It never interprets pack code. The runtime resolves an animation for each pet state in this order: the `stateMappings` entry for the state, then an animation with the state's name. If neither exists, it repeats both checks along the state's fallback chain (for example `level_up → celebrating → success → idle`, `thinking → working → idle`, `offline → sleeping → idle`), and finally uses the first declared animation. State keys match case-insensitively and ignore non-alphanumeric characters, so `LevelUp` and `level_up` are equivalent. A missing mapping never fails rendering. Unknown optional fields may be ignored only when the schema explicitly allows forward-compatible extensions. Unknown schema versions are rejected with a compatibility error. Migrations are deterministic, tested, and preserve original pack files for rollback.

## License and moderation metadata

Public submission requires creator, license identifier or explicit custom terms, source, attribution, and declared commercial-use, redistribution, and derivative-work permissions. A gallery must retain version history, report state, moderation decision, and takedown record. “User uploaded” does not imply that the uploader owns the art; the uploader attests to rights and the service provides a report/takedown path.
