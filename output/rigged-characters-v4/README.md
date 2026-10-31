# Local rigged character production — Nezuko candidate

Requested scope: separate detailed models for all 213 Miyora catalog characters,
using local tools only. No paid generation service or cloud model generation was used.

Current delivery is **one Nezuko prototype**, not the completed collection.
The remaining 212 characters have not been authored. This candidate still needs
art direction and final visual refinement before it can establish the collection's quality bar.

## Candidate files

- `nezuko/source.blend` — editable Blender 4.5 LTS source, skeleton, skin weights,
  packed fabric image, blink shape key, lights/camera, and 15 named actions.
- `nezuko/nezuko.vrm` — self-contained VRM 1.0 humanoid export for Miyora's VRM import path.
- `nezuko/model.glb` — self-contained skinned binary glTF with animations.
- `nezuko/preview.html` — standalone offline interactive model viewer. Open in a
  WebGL-capable browser; drag to orbit, scroll to zoom, select animations, or show the skeleton.
- `nezuko/preview.png` — Blender render for diagnostic visual review; this is not the model deliverable.
- `nezuko/nezuko.validation.json` — structural skin/geometry/animation validation.
- `nezuko/runtime-validation.json` — isolated Chrome/three-vrm playback check, when present.
- `catalog-status.json` — honest per-character production status for the requested collection.

Structural checks cover embedded resources, finite vertices/bind matrices,
normalized skin weights, joint references, actual blink deltas, required humanoid
bones, animation durations/tracks, and Miyora's 100,000-triangle / 50 MiB budgets.

The candidate has 34,140 triangles and 51 joints. Browser checks verify that
three-vrm recognizes the humanoid and that real skeletal clips render. Those
checks do not establish final art quality or verify every pose is free of cloth
intersections. Miyora desktop activation has not been completed for this candidate.

## Editable source

The Blender file stores a neutral bind pose. Animation actions are listed by pet
state. To preview one in Blender, choose an armature action in the Action Editor;
the saved NLA tracks are muted to preserve the bind pose. The mesh uses an Armature
modifier and deform vertex groups, rather than a flat image or image relief.

## Local authoring tools

`authoring/` preserves the scripts used for this candidate. Blender's portable
runtime was downloaded from the official Blender 4.5.15 LTS release and its official
SHA-256 verified before execution. The runtime resides in the ignored
`target/model-tools/blender-4.5.15-windows-x64/` directory and is not bundled here.

The authoring script currently supports the humanoid pilot; it explicitly rejects
other body types instead of silently substituting a human mesh. Bird, beast,
robot, and slime production and catalog-wide model selection remain unfinished.

## Design rights

Geometry was authored locally for this prototype. No ownership or redistribution
license for the Nezuko character identity is asserted. This candidate is not a
licensed franchise release asset and has not been committed or published.
