# Miyora full-anime rigged model collection

This local first-pass collection contains 213 separate skinned models and editable
Blender sources: 119 humanoids, 56 beasts, 12 birds, 12 slimes, and 14 robots.
Humanoid heads and hands use the selected full-anime proportions; other body types
retain species-specific forms. These are real meshes, skeletons, weights, morphs,
and animation tracks, not images or image-relief substitutes.

Each character folder contains:

- `source.blend`: editable Blender 4.5 LTS source with packed textures and 15 actions.
- `model.vrm`: self-contained VRM 1.0 export for Miyora.
- `model.glb`: skinned glTF export with the same geometry and animations.
- `model.validation.json`: structural checks and the model's SHA-256.
- `character.json`: the catalog identity used for authoring.
- `production.json`: actual export status and timing.

`collection.json` maps every catalog ID to its separate files. `production-status.json`
records actual completion and runtime check counts. `runtime-validation.json`
contains browser loader/skin/animation results when that pass has completed.

## Review

Open the running local viewer at `http://127.0.0.1:1423/`, or run `View Collection.cmd`
on this machine. Select a character, rotate/zoom it, choose a pose clip, or display
its skeleton. The preview changes presentation only and never fabricates agent
activity or awards XP. Source `.blend` files can be opened directly in Blender.

This is a generated first art pass. Structural and runtime validation do not make
all 213 models individually polished production artwork. Facial likeness, cloth
intersections under extreme poses, secondary hair/cloth motion, and character-specific
art details still warrant individual refinement. Humanoid motion is shared; bodies
such as slimes use the relevant deforming portions of that rig.

## Miyora import

The repository provides a native, hash-deduplicated importer:

```powershell
cargo run -p agentpet-core --example import_model_collection -- --models output/anime-rigged-catalog-v2 --dry-run
cargo run -p agentpet-core --example import_model_collection -- --models output/anime-rigged-catalog-v2 --data-dir "$env:APPDATA/dev.agentpet.desktop" --select miyora.nezuko
```

The importer validates every model through Rust before changing the library. It
preserves existing models, reuses matching hashes, and optionally selects the 3D
avatar plus its matching installed PetPack. The library now remains bounded at
512 custom avatars. `app-import-index.json` records local import IDs after success;
it is omitted from the distributable archive.

Models were authored and exported locally without paid generation services.
No ownership or redistribution license for franchise identities is asserted.
Generated character art has not been committed or published as a licensed release.
The collection's `revisions/` directory, when present locally, contains superseded
diagnostic models and is excluded from the archive and model registry.
