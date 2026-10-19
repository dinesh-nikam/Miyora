# PetPack Schema and Packaging Specification

## 1. Structure
A PetPack is a directory or `.zip`/`.tar.gz` archive containing:
- `pet.json`: Manifest file declaring id, name, version, author, license, attribution, and animation mappings.
- `pet.sig`: Optional detached Ed25519 signature covering `pet.json`.
- Asset files: PNG, JPEG, GIF, or WebP spritesheets.

## 2. Security Limits
- Maximum manifest size: 64 KB.
- Maximum compressed archive size: 50 MB.
- Maximum extracted archive size: 100 MB.
- Maximum compression ratio: 20:1.
- Maximum image dimension: 4096 x 4096 pixels.
- Path traversal rejection: Entries containing `..`, absolute paths, symlinks, or non-regular files are rejected.
