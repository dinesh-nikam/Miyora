# Export Pipeline

Status: **local export implemented** (offline, no credits). Code:
`crates/studio/src/export.rs`, `StudioService::export_plan` / `export_output`,
Tauri command `studio_export_output`, UI `apps/desktop/src/studio/ExportPanel.tsx`
(Inspector → Export).

## Flow

1. User picks a format and options in the Inspector and clicks **Export…**.
2. Rust opens the native **Save** dialog (`tauri-plugin-dialog`, called from
   Rust), so the webview never supplies a write path. Cancel → nothing written.
3. The extension is forced to the real format (`a.bin` → `a.glb`); the folder
   must exist.
4. Files are written through `<name>.part` + rename.
5. Every written file is re-read and verified (below). Only then is the export
   recorded in the `exports` table (schema v2) and shown in the history.

## Formats

| Format | Source needed | Contents | Verification |
|---|---|---|---|
| Original | any stored model (GLB or FBX) | byte-for-byte copy | SHA-256 equals the library copy |
| OBJ | GLB | positions, `vt` UVs (v flipped), vertex colours as `v x y z r g b`; base-colour texture written as `<name>_texture.png/jpg` with `<name>.mtl` | `v`/`f` line counts equal source vertex/triangle counts; sidecar sizes |
| STL | GLB | binary, face normals computed, geometry only | header triangle count and file size `84 + 50·n` |
| PLY | GLB | binary little-endian, float xyz, uchar RGB when the model has vertex colours | header element counts |

Options for OBJ/STL/PLY: uniform **scale** (0.001–1000; "×1000" converts
metres to millimetres) and **Z-up** (rotates glTF y-up to z-up; on by default
for STL). Original exports ignore options.

GLB reading applies node transforms (matrix or TRS) over the default scene,
merges all triangle primitives, and supports float / normalized integer
accessors. It refuses: `extensionsRequired` (e.g. meshopt or Draco
compression), external buffers, sparse accessors, out-of-range indices,
node hierarchies deeper than 64 or cyclic, and more than 20 M vertices / 60 M
indices. Those models can still be exported as Original.

## Verified on real outputs (2026-10-29)

`cargo run -p agentpet-studio --example export_all -- <data-dir> <out-dir>`
exported the real TripoSR (44,312 triangles) and TRELLIS (7,569 triangles)
models to all four formats. An independent parser (trimesh) loaded every file
with identical face counts and bounds; the TRELLIS OBJ loaded its 1024×1024
texture via the MTL; TripoSR vertex colours survived in OBJ and PLY.

## Not implemented

- FBX / USDZ / GLTF (separate files) / 3MF output. The Tripo provider has a
  tested request mapping for `POST /v3/models/convert` and `media::inspect_export`
  validates such outputs, but there is no UI or service flow for cloud
  conversion yet (and it needs Tripo credits).
- Texture resolution changes, re-UV, decimation on export.
