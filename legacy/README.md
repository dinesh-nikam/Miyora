# Legacy Python 3D Scripts

These standalone Python scripts were used for early experimental CLI workflows:
- `avatar_workflow.py`: Early end-to-end Python pipeline
- `anime_character_prompt.py`: Early franchise-sanitizing anime prompt expansion
- `tripo_client.py`: Raw HTTP requests to Tripo API
- `blender_postprocess.py`: Blender headless bpy script for mesh normalization
- `test_avatar_tripo.py`: Python unit tests for the scripts above

## Modern Architecture

The production 3D generation, prompt building, provider transport, mesh decimation, GLB export, and companion library integration have been fully unified into native Rust domain crates and desktop commands:
- `crates/studio`: Full native Rust implementation with provider abstractions (Tripo v3, local TripoSR, HuggingFace), prompt presets with anime character expansion and franchise copyright sanitization, mesh decimation (`decimate.rs`), and GLB/glTF/USDZ export (`export.rs`).
- `apps/desktop`: Integrated Command Deck 3D Generation tab in Pet Library (`CharacterStudio.tsx`) and 3D Studio inspection viewport.
- `crates/avatar`: Silhouetted sculpting and procedural avatar rigging.
