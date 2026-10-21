# 3D Avatars

## Status

Implemented and covered by tests:

- `crates/avatar` (`agentpet-avatar`): the `CharacterSpec` contract and validator, the bundled catalog of 167 original characters, the image sculptor, and a minimal binary glTF writer/reader.
- `crates/agentpet-core/src/avatars.rs`: SQLite schema v10, avatar selection, customization, custom avatar storage, image import, and VRM import.
- Tauri commands listed in [API.md](../../API.md).
- `apps/desktop/src/avatar/`: the procedural rig builder, state motion, stage props, the `AvatarStage` renderer, shared-context thumbnails, and the Character Studio UI.

Not implemented: a learned (neural) image-to-3D model, per-project avatar assignment, and cloud sharing of avatars.

## Three kinds of avatars

| Kind | Source | Geometry | Animation |
| --- | --- | --- | --- |
| `procedural` | Catalog character, a customized copy, or a character styled from an image | Built at runtime from a validated `CharacterSpec` | Full joint rig: spine, chest, neck, head, arms, elbows, legs, knees, tail chain, wings, eyes, brows, mouth shapes, blush |
| `image_sculpt` | A user image | Closed, textured relief mesh generated in Rust and stored as GLB | Whole-body motion: bob, lean, turn, tilt, squash and stretch |
| `vrm` | A user `.vrm` or `.glb` file | The author's model | The same pose retargeted onto VRM normalized humanoid bones, plus VRM expressions |

Every kind consumes the same `PetState` from `agentpet-pet-engine`. The UI never invents agent activity; the Studio's pose preview only changes what the preview stage shows.

## CharacterSpec v1

A spec is data only. Rust rejects unknown fields (`deny_unknown_fields`), so a spec cannot carry scripts or extra payloads.

```json
{
  "schemaVersion": 1,
  "id": "miyora.ember",
  "name": "Ember",
  "title": "Fox Courier",
  "description": "A quick-pawed fox who carries messages between your agents.",
  "body": "beast",
  "ears": "fox",
  "tail": "fox",
  "wings": "none",
  "hair": "short",
  "eyes": "round",
  "outfit": "jacket",
  "accessories": ["scarf", "backpack"],
  "pattern": "none",
  "palette": { "skin": "#e07a2f", "hair": "#fff7ed", "eyes": "#1e3a8a", "primary": "#1f2937", "secondary": "#374151", "accent": "#f59e0b" },
  "proportions": { "height": 1.0, "headScale": 1.0, "build": 1.0, "limbLength": 1.0 },
  "energy": 1.3,
  "tags": ["fox", "speedy"]
}
```

| Field | Rule |
| --- | --- |
| `id` | Dotted lowercase segments (`a-z`, `0-9`, `-`), 2+ segments, at most 64 bytes. Catalog ids use the `miyora.` namespace; user ids use `user.` |
| `name` / `title` / `description` | 1-32 / 1-48 / 0-240 characters, no control characters |
| `body` | `humanoid`, `beast`, `robot`, `slime`, `bird` |
| `ears` | `none`, `human`, `elf`, `cat`, `fox`, `bunny`, `bear`, `floppy`, `horns`, `antenna`, `fin`, `leaf` |
| `tail` | `none`, `cat`, `fox`, `bunny`, `dragon`, `cable`, `feather`, `flame` (glowing flame tip), `leaf` (leaf-blade tip), `star` (glowing star tip) |
| `wings` | `none`, `feather`, `bat`, `fairy`, `jet` |
| `hair` | `none`, `short`, `bob`, `long`, `ponytail`, `twintails`, `spiky`, `bun`, `mohawk`, `curly`, `hime`, `side_tail` |
| `hairTip` | Optional `#RRGGBB`; hair locks fade to this color at the tips. Omitted means a solid color |
| `eyes` | `round`, `sharp`, `sleepy`, `sparkle`, `visor`, `mono` |
| `outfit` | `hoodie`, `kimono`, `armor`, `suit`, `robe`, `jacket`, `dress`, `overalls`, `labcoat`, `tunic`, `fur`, `plating`, `hakama`, `sailor`, `coat` |
| `pattern` | Optional, default `none`: `dots`, `stripes`, `waves`, `petals`, `stars`, `diamonds`. Applied to outfit cloth |
| `accessories` | Up to 5 unique: `glasses`, `headphones`, `scarf`, `beanie`, `witch_hat`, `crown`, `halo`, `cape`, `backpack`, `headband`, `bow`, `flower`, `goggles`, `bell`, `katana`, `wand`, `staff`, `kitsune_mask`, `shell` (domed back shell), `gem` (glowing forehead jewel) |
| `palette.*` | `#RRGGBB` |
| `proportions` | height 0.8-1.2, headScale 0.85-1.35, build 0.8-1.3, limbLength 0.85-1.15 |
| `energy` | 0.5-1.6; scales animation tempo |
| `tags` | Up to 6, each 1-24 lowercase ASCII letters, digits, or hyphens |

Limits are named constants in `crates/avatar/src/spec.rs` and mirrored in `apps/desktop/src/avatar/types.ts`.

## Catalog

`crates/avatar/catalog/characters.json` holds 167 original characters (78 humanoid, 51 beast, 14 robot, 12 slime, 12 bird), including 30 designs in popular anime archetypes (swordswoman, ronin, kunoichi, shrine maiden, magical girl, idol, mecha pilot, fox spirit, vampire noble, onmyoji, and others) and 25 chibi pocket creatures tagged `pocket-monster` (spark mouse, flame-tail lizard, shell turtle, leaf sprite, a fox kit with seven elemental forms, dream kitten, snoozing giant, lullaby puff, wisp, magnet bot, and others), license CC0-1.0. It is generated deterministically by `scripts/generate_avatar_catalog.py` from hand-authored rows and embedded into the binary with `include_str!`. Validation runs once on first use: schema version, metadata, every spec, unique ids, the `miyora.` namespace, and the presence of the default character `miyora.aoi`.

```powershell
python scripts/generate_avatar_catalog.py          # regenerate
python scripts/generate_avatar_catalog.py --check  # fail when the committed JSON is stale
```

No catalog character reproduces a copyrighted design: archetypes are genres, while names, palettes, and signature details are original. The pocket creatures follow the creature-collecting genre only: no franchise names, logos, or signature designs (for example, no yellow mouse with black-tipped ears and a zigzag tail). Licensed franchise characters can only be used as user-provided VRM imports, which stay local and are never bundled. `pattern` and `hairTip` were added after v1 shipped; both are optional, so earlier specs and stored customizations still parse unchanged. `docs/specs/vrm_pet.md` lists attributes that must never be shipped.

## Image to 3D (sculpt)

`agentpet_avatar::sculpt_image` runs on a Tauri blocking worker. It is deterministic geometry, not a learned model:

1. Decode PNG, JPEG, GIF (first frame), or WebP with limits: 20 MiB file, 4096 px per side, 128 MiB decoder allocation.
2. Resize to a grid of at most 128 px on the long side.
3. Segment the subject. When at least 2% of pixels are translucent, alpha decides. Otherwise the background color is the per-channel median of the border; pixels connected to the border within an adaptive color distance are background. Busy borders (90th percentile spread above 60) are rejected with a message asking for a plain or transparent background.
4. Keep the largest 4-connected subject region and fill interior holes. Coverage must be 3%-97%.
5. Compute a 3-4 chamfer distance transform inside the silhouette, map it through a circular profile (thick regions bulge more), and smooth it twice.
6. Emit a front surface, a back surface at 70% depth, and shared boundary vertices so the shell is closed. A unit test asserts every edge is shared by exactly two triangles.
7. Texture it with the source image (at most 1024 px), with colors bled outward from the silhouette so edge sampling never picks up the old background.
8. Write a self-contained GLB (no external URIs). The mesh is normalized to 1 unit tall and stays under about 66,000 triangles.
9. Cluster subject colors with deterministic k-means (k = 5) and derive a `CharacterSpec`: hair from the top band (preferring a non-skin color), outfit from the middle and bottom bands, accent from the most saturated cluster, skin from a skin-like cluster when present.

Import creates two custom avatars: the sculpt and the styled procedural character (`sourceAvatarId` links them). The styled character can be customized like any procedural character.

Limitations: depth comes from the silhouette only, so facial features and folds stay painted on the surface, and the back of the sculpt shows the mirrored front texture.

Developer tool:

```powershell
cargo run -p agentpet-avatar --example sculpt -- input.png out.glb "Name"
```

## VRM import

`import_vrm_avatar` runs `agentpet_pet_pack::vrm::validate_and_extract_vrm` with the default guardrails (50 MB, 100,000 triangles, 16 images, no external buffer or image URIs), stores the file, and records the extracted title, author, license, allowed-user, and commercial-usage metadata. The Studio shows that metadata. The current validator does not decode embedded textures, so the 2048 px texture limit in `vrm_pet.md` is not enforced yet.

Rendering uses `@pixiv/three-vrm`: `VRMUtils.rotateVRM0` for VRM 0.x, normalization to the stage height, pose retargeting from the procedural rig (arm rotations are conjugated by the rest-direction change because VRM rests in a T-pose), and `happy`, `sad`, `aa`, `surprised`, `blink` expressions.

## Storage (SQLite schema v10)

| Table | Purpose |
| --- | --- |
| `avatar_selection` | Singleton: `source` (`catalog` or `custom`), `avatar_id`, `updated_at`. Missing row means the default `miyora.aoi` |
| `avatar_customizations` | One validated full spec per customized catalog id |
| `custom_avatars` | `avatar_id`, `kind` (`procedural`, `image_sculpt`, `vrm`), `name`, `spec_json` (procedural only), `model_file` and `model_sha256` (model kinds only), `triangle_count`, `metadata_json`, `source_avatar_id`, timestamps. CHECK constraints enforce the kind/column pairing |

Model files live in `<app data>/avatars/<avatar_id>/` and are written to a staging directory first, then renamed. Only the relative file name is stored. Reads canonicalize the path, refuse anything outside the avatar directory, enforce a size bound, and verify the recorded SHA-256 before returning bytes. The library holds at most 200 custom avatars. Deleting the selected custom avatar returns the selection to the default character in the same transaction.

## Rendering and data

`AvatarStage` renders with one WebGL context per stage, four-band cel shading with thin inverted-hull outlines, anime faces for humanoids (tapered jaw, small nose, layered eyes with sclera, iris gradient, pupil, two highlights, and upper and lower lashes), tapered hair locks with optional gradient tips, pleated cloth, CPU-generated fabric pattern textures, and hemisphere, key, and rim lights. It caps rendering at 30 FPS for active states and 12 FPS for sleeping, paused, and offline, and stops while the document is hidden. Thumbnails share one offscreen context and render one at a time.

What the stage shows comes from real data:

| Visual | Source |
| --- | --- |
| Pose, expression, and props (laptop, thought bubbles, glyphs, confetti, sleep Zs, food bowl, level-up aura) | Current `PetState` from the Rust pet engine |
| Pedestal ring arc and `LV n - x/y XP` label | `get_avatar_hud`, computed from the immutable XP ledger |
| Dimmed lighting | `offline` state |

Pointer movement turns the head toward the cursor; dragging orbits the stage.
