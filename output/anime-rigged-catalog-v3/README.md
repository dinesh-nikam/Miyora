# Miyora refined 3D character collection

This collection refines the same 213 catalog characters using local Blender tools.
Humanoids retain full anime proportions; beasts, birds, slimes, and robots retain
their species forms. Each model is a real skinned mesh with editable geometry,
15 animation clips, and five working facial expressions.

## Files and inspection

Each character folder contains `source.blend`, `model.glb`, `model.vrm`, the frozen
catalog identity, a production record, and a structural validation report. After
runtime validation, `review.png` shows every character. Selected characters also
have working, raised-arm, and happy portrait views.

Open the local viewer at `http://127.0.0.1:1425/`. The character selector, orbit
controls, animation selector, Face button, and expression selector inspect the
actual model. `review/index.html` links comparison sheets to their 3D models.
These are visual previews; they do not synthesize agent activity or award XP.

## Refinements

- Curved almond eye surfaces, layered irises that preserve each character's eye
  color, arched brows, modeled lips, and actual blink/happy/sad/open-mouth/surprised
  morphs. Mouth surfaces follow the modeled face, including its nasal bridge.
- Forearm-weighted hanging sleeves, open lined cuffs, layered collars, garment
  hems, ribbed cuffs, and outfit-specific footwear.
- Nezuko's repeating woven kimono motif, checked obi, green obiage, cord knot,
  smaller bamboo joints, and detailed hair gradient.
- Tanjiro's checked haori, layered uniform, shin wraps, irregular scar and earring
  details; Luffy's open vest, denim shorts and sandals; high-collar and accessory
  refinements for other catalog designs.
- Feather shafts, sculpted fur tufts, and recessed robot panels with fasteners.
- Fine hair strands and smooth tip gradients. Longer hairstyles have editable
  secondary bones and live VRM spring motion, with body/head collision volumes.
  Short spiky hairstyles stay controlled rather than flaring under spring forces.
- Toon materials and thin outlines for stylized surfaces, with softer skin,
  eye, gel, and metallic materials where volume and reflections matter.

The VRMs use the standard [MToon material extension](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_materials_mtoon-1.0/README.md)
and [SpringBone extension](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_springBone-1.0/README.md).
The GLBs keep Blender's material fallback. Blender sources include the editable
mesh, skeleton, facial shape keys, packed textures, and 15 actions; live spring
simulation is supplied by the VRM runtime.

## Validation

`production-status.json` records actual completed counts. `runtime-validation.json`
records model hashes, working morph influences, toon materials, secondary-joint
motion, and all 15 sampled deformed poses for every model. Native Rust validation
uses Miyora's existing import guardrails. `app-import-index.json`, when present,
maps this machine's imported avatars and is excluded from the model archive.

Finite geometry checks do not prove that every possible manually edited pose is
free of cloth intersections. This is locally authored stylized character artwork;
the files remain editable for further character-specific sculpting and art review.

## Local reproduction

`authoring/` contains the frozen Python authoring modules and the catalog snapshot
is `catalog-source.json`. Blender 4.5 LTS supplies Python, NumPy, and glTF export.
No paid generation service or network request is used by the authoring pipeline.

```powershell
blender --background --factory-startup --python authoring/refine_catalog.py -- --catalog catalog-source.json --output ../regenerated-models --all
```

The exporter stages each model, validates it, then renames it into place. It refuses
to overwrite mismatched revisions or interrupted staging output. It supports two
bounded workers and reuses an existing model only after checking its revision and
SHA-256. `collect_refined.py` audits the complete set before writing the registry.

The Miyora repository's native importer supports hash-deduplicated imports:

```powershell
cargo run -p agentpet-core --example import_model_collection -- --models output/anime-rigged-catalog-v3 --dry-run
cargo run -p agentpet-core --example import_model_collection -- --models output/anime-rigged-catalog-v3 --data-dir "$env:APPDATA/dev.agentpet.desktop" --select miyora.nezuko
```

The refined variants are labeled `· Refined` in Miyora. Earlier generated variants
can coexist within the bounded custom library. No ownership or redistribution
license for franchise character identities is asserted, and no character artwork
has been committed or published as a licensed release.
