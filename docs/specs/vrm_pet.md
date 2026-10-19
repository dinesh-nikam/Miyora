# 3D Anime Character and VRM Avatar Specification

## 1. Overview
AgentPet supports 3D anime-style companions alongside 2D sprite packs. The desktop runtime consumes standard VRM (glTF 2.0 binary) avatars, ships with one clean-room original 3D anime character, and provides a Bring-Your-Own-Model (BYOM) import pipeline with strict security guardrails.

---

## 2. Original Character Design Baseline
To avoid copyright infringement and third-party IP contamination:
- The shipped default 3D model is a fully original anime-style character ("Aoi") designed from the ground up.
- **Explicit Disclaimers & Prohibitions**:
  - Does NOT feature any signature attributes of copyrighted characters (e.g. Kimetsu no Yaiba's Nezuko Kamado).
  - Strictly omits: bamboo gag/mouthpiece, pink hemp-leaf geometric pattern (asanoha) kimono, black outer haori with red lining, checkered obi sash, or pink-strapped zori footwear.
  - Character features original contemporary cyber/streetwear clothing (teal/slate tech hoodie, dark trousers, and sneakers) and unique hair and eye styling.
- License: Distributed under MIT for application scaffolding; model geometry and textures licensed under CC-BY-4.0 with full open attribution.

---

## 3. Bring Your Own Model (BYOM) Pipeline
Users can import standard `.vrm` files created in tools such as VRoid Studio, Blender, or Unity.

### 3.1 Metadata Extraction & Display
The loader parses VRM 0.x (`VRM` extension) and VRM 1.0 (`VRMC_vrm` extension) headers:
- `title`: Avatar display name.
- `author`: Original creator / modeler.
- `version`: Model version string.
- `licenseUrl` / `licenseName`: Explicit redistribution and usage license.
- `allowedUser`: Allowed user classification (`OnlyAuthor`, `ExplicitlyLicensedPerson`, `Everyone`).
- `commercialUsage`: Commercial usage permission flag.

Extracted metadata is persisted in SQLite upon pet import and surfaced read-only in **Settings -> Pet** and the **Stats HUD**.

---

## 4. Behavior & State Parity
3D pets consume the identical `PetState` and `Stimulus` stream emitted by `agentpet-pet-engine`:

| Pet Engine State | VRM Humanoid Pose & Animation | VRM Blendshape Expression |
| :--- | :--- | :--- |
| `Idle` | Gentle breathing cycle on spine/chest; subtle natural blinking every 3–5s | `neutral` / `relaxed` |
| `Working` | Focused upright/forward posture; typing or pad examination gesture | `neutral` (focused eye gaze) |
| `Waiting` | Alert posture; slight head tilt; questioning gesture | `surprised` / `questioning` |
| `Done` | Joyful celebration jump/wave; celebratory arm motion | `joy` / `happy` |
| `LevelUp` | Energetic victory stance; sparkling particle cue | `joy` (wide smile) |
| `Evolving` | Ascension / transformation stance; model scaling/tinting | `joy` |
| `Hungry` | Drooping shoulders, tired idle rhythm | `sorrow` / `pout` |
| `Sleeping` | Sitting or resting posture; slow breathing rhythm | `blink: 1.0` (eyes closed) |

---

## 5. Head Bone Tracking & Speech Bubbles
- The reactive speech bubble is not statically positioned.
- The 3D renderer continuously queries the world coordinates of the humanoid `head` bone.
- The 3D world position is projected through the active camera onto 2D viewport coordinates:
  $$x_{pixel} = \frac{x_{ndc} + 1}{2} \cdot \text{viewport\_width}$$
  $$y_{pixel} = \frac{1 - y_{ndc}}{2} \cdot \text{viewport\_height}$$
- The UI speech bubble dynamically anchors above the projected head point, with viewport boundary padding preventing bubble clipping.

---

## 6. Security Guardrails & Sandbox Limits
All imported `.vrm` files are treated as untrusted input. Validation occurs in an isolated worker before loading into the WebGL scene.

### 6.1 Guardrail Limits
- **Maximum File Size**: 50 MB (`52,428,800` bytes).
- **Maximum Triangle Count**: 100,000 polygons.
- **Maximum Texture Resolution**: $2048 \times 2048$ pixels.
- **Maximum Texture Count**: 16 images.
- **Maximum Humanoid Bones**: 64 standard humanoid joints.

### 6.2 External Dependency Rejection
- All buffers, meshes, and images MUST be internal binary buffers contained within the glTF binary (`.glb`/`.vrm`) container.
- Any model declaring external URI schemes (`http://`, `https://`, `file://`, relative paths like `../`, or `data:` URIs) is immediately rejected with `VrmValidationError::ExternalResourceForbidden`.

---

## 7. Performance Budget
- **Framerate Capping**: Active animation renders at a maximum of 30 FPS.
- **Occlusion & Idle Throttling**: When the pet window is minimized, hidden behind full-screen applications, or in deep `Sleeping` state without movement, rendering drops to 1 FPS or pauses completely.
- **Idle CPU Target**: $< 1\%$ CPU utilization during idle on all supported desktop platforms.
