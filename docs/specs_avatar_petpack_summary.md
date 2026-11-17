# Miyora (AgentPet) — Comprehensive Specifications & Architecture Summary
**Avatars, 3D CharacterSpecs, Procedural Rigs, Bone Hierarchies, PetPack Manifests, Animation Mappings, and the 15-State Behavior Engine**

*Document Status: Authoritative Technical Reference*  
*Repository Root: `d:/mirai/nezuko`*  
*Crates Analyzed: `crates/avatar`, `crates/pet-engine`, `crates/pet-pack`, `crates/agentpet-core`, `crates/studio`*  
*Frontend Analyzed: `apps/desktop/src/avatar/`, `apps/desktop/src/components/CompanionRenderer.tsx`*

---

## 1. Executive Summary & Architectural Overview

Miyora is an offline-first, native desktop companion that tracks AI coding agent activity in real time. It observes actual process lifecycles and tool invocations, routes events through a deterministic 15-state behavior engine, and renders responsive 3D procedural avatars and animated 2D sprite companions over a transparent desktop overlay.

### Architectural Boundary & Unidirectional Data Flow
```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       Native Rust Domain Crates                             │
├──────────────────────────────┬─────────────────────────────┬────────────────┤
│    crates/agentpet-core      │     crates/pet-engine       │ crates/avatar  │
│  - SQLite Schema v10         │  - 15-state state machine   │ - CharacterSpec│
│  - Tamper-evident XP ledger  │  - Priority queue & timeouts│ - 214 Catalog  │
│  - Avatar selection/storage  │  - Animation fallbacks      │ - Image sculpt │
├──────────────────────────────┼─────────────────────────────┴────────────────┤
│       crates/pet-pack        │          crates/agentpet-adapters            │
│  - PetPack Manifest v1/v2    │  - Real CLI hook parsers (Claude, Codex)     │
│  - Ed25519 pack signatures   │  - Reversible native hook installation       │
│  - ZIP/tar.gz sandboxing     │                                              │
│  - VRM guardrails & metadata │                                              │
└──────────────────────────────┴──────────────────────────────────────────────┘
                                       │
                      Tauri 2 IPC (invokeSafe / Events)
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Presentation Layer (React 18 / Vite)                     │
├─────────────────────────────────────────────────────────────────────────────┤
│  apps/desktop/src/avatar/                                                   │
│   ├── rig.ts             Procedural joint hierarchy, meshes & toon shading   │
│   ├── motion.ts          State-to-pose mapping, VRM retargeting, gaze       │
│   ├── props.ts           Stage props (laptop, thought, confetti, XP ring)   │
│   ├── AvatarStage.tsx    Three.js WebGL rendering loop & framerate governor  │
│   └── CharacterStudio.tsx Avatar catalog browser & customizer               │
│  apps/desktop/src/components/CompanionRenderer.tsx                          │
│   └── 2D Sprite sheet animator, Web Audio chimes, petting/treat interactions │
└─────────────────────────────────────────────────────────────────────────────┘
```

Miyora strictly prohibits business logic or SQLite operations inside React. Rust domain crates hold authoritative state, enforce validation limits, and verify cryptographic hashes. The UI receives validated data models and renders visual representations accordingly.

---

## 2. The 15-State Behavior Engine (`crates/pet-engine`)

The pet engine is a deterministic, priority-driven state machine with explicit transition allow-lists, reaction timeouts, and fallback chains.

### 2.1 The 15 Canonical States
All states are represented by the `PetState` enum:

| State | Priority | Interruptible | Default Timeout | Timeout Target | Description |
| :--- | :--- | :---: | :--- | :--- | :--- |
| `Idle` | `Low` | Yes | 15 min (`idle_before_sleep`) | `Sleeping` | Resting baseline state |
| `Working` | `Normal` | Yes | None | N/A | Agent actively executing tools / writing code |
| `Thinking` | `Normal` | Yes | None | N/A | Agent reasoning or processing LLM response |
| `Waiting` | `Normal` | Yes | None | N/A | Blocked waiting for user input or approval |
| `Success` | `Normal` | Yes | 4 sec (`success_duration`) | `Resting` | Task or tool execution succeeded |
| `Error` | `Critical` | Yes | 6 sec (`error_duration`) | `Resting` | Agent failed or tool crashed |
| `Celebrating` | `Normal` | Yes | 4 sec (`celebrating_duration`) | `Resting` | User interaction / petting celebration |
| `Sleeping` | `Background` | Yes | 16 hours (`hungry_after`) | `Hungry` | Prolonged inactivity sleep |
| `Hungry` | `Low` | Yes | None | N/A | Inactivity hunger cue |
| `Eating` | `Low` | Yes | 4 sec (`eating_duration`) | `Resting` | Consuming user-offered treat |
| `LevelUp` | `High` | **No** | 6 sec (`level_up_duration`) | `Resting` | XP milestone reached |
| `Achievement`| `High` | **No** | 6 sec (`achievement_duration`)| `Resting` | Achievement unlocked |
| `Offline` | `Background` | Yes | None | N/A | Daemon disconnected or hook inactive |
| `Paused` | `High` | **No** | None | N/A | Explicit user freeze/pause command |
| `AttentionRequired` | `Critical` | Yes | None | N/A | Urgent prompt/permission confirmation blocked |

*Resting target rule:* When a timed state completes, it returns to `Working` if active agent sessions exist; otherwise it returns to `Idle`.

### 2.2 Priority Ordering & Reaction Queue
Priorities: `Background` < `Low` < `Normal` < `High` < `Critical`.

- Non-interruptible states (`LevelUp`, `Achievement`, `Paused`) yield only to strictly higher-priority targets or explicit user commands (`UserPaused`, `UserResumed`).
- When a stimulus arrives during a non-interruptible state, valid reactions (`Success`, `Celebrating`) are queued into a bounded `ReactionQueue` (capacity 4).
- The queue is deduplicated and sorted by priority. If full, a new item will evict the lowest-priority existing element if the new priority is strictly higher.

### 2.3 Transition Allow Matrix (`allows(from, to)`)
Transitions must be explicitly allowed; invalid state transitions are rejected with `Transition::Rejected { from, to }`:
- **`Idle`** $\to$ `Working`, `Thinking`, `Waiting`, `AttentionRequired`, `Error`, `Celebrating`, `LevelUp`, `Achievement`, `Sleeping`, `Hungry`, `Eating`, `Paused`, `Offline`.
- **`Sleeping`** $\to$ `Idle`, `Working`, `Thinking`, `Waiting`, `AttentionRequired`, `Error`, `Paused`, `Offline`, `Hungry`, `Eating`, `Celebrating`.
- **`Working` / `Thinking` / `Waiting`** $\to$ `Working`, `Thinking`, `Waiting`, `AttentionRequired`, `Success`, `Error`, `Celebrating`, `LevelUp`, `Achievement`, `Eating`, `Paused`, `Offline`.
- **`AttentionRequired`** $\to$ `Idle`, `Working`, `Thinking`, `Waiting`, `Success`, `Error`, `LevelUp`, `Achievement`, `Eating`, `Paused`, `Offline`.
- **`Success`** $\to$ `Idle`, `Working`, `Thinking`, `Waiting`, `AttentionRequired`, `Error`, `Celebrating`, `LevelUp`, `Achievement`, `Eating`, `Paused`, `Offline`.
- **`Error`** $\to$ `Idle`, `Working`, `Thinking`, `Waiting`, `AttentionRequired`, `Success`, `LevelUp`, `Achievement`, `Eating`, `Paused`, `Offline`.
- **`Celebrating`** $\to$ `Idle`, `Working`, `Thinking`, `Waiting`, `AttentionRequired`, `Error`, `Success`, `LevelUp`, `Achievement`, `Eating`, `Paused`, `Offline`.
- **`LevelUp`** $\to$ `Idle`, `Working`, `Thinking`, `Waiting`, `AttentionRequired`, `Error`, `Celebrating`, `Success`, `Achievement`, `Paused`, `Offline`.
- **`Achievement`** $\to$ `Idle`, `Working`, `Thinking`, `Waiting`, `AttentionRequired`, `Error`, `Celebrating`, `Success`, `LevelUp`, `Paused`, `Offline`.
- **`Hungry`** $\to$ `Eating`, `Idle`, `Working`, `AttentionRequired`, `Error`, `Paused`, `Offline`, `Celebrating`.
- **`Eating`** $\to$ `Idle`, `Working`, `AttentionRequired`, `Error`, `Paused`, `Offline`, `Celebrating`.
- **`Offline`** $\to$ `Idle`, `Working`, `Paused`.
- **`Paused`** $\to$ `Idle`, `Working`, `AttentionRequired`, `Error`, `Offline`.

### 2.4 Animation Fallback Resolution
When a PetPack or rig lacks an animation named after the active state, `select_animation` walks an explicit fallback chain before defaulting:

| State | Fallback Chain |
| :--- | :--- |
| `Idle` | `[]` (Terminal) |
| `Thinking` | `Working` $\to$ `Idle` |
| `AttentionRequired` | `Waiting` $\to$ `Idle` |
| `Success` | `Celebrating` $\to$ `Idle` |
| `Celebrating` | `Success` $\to$ `Idle` |
| `LevelUp` / `Achievement` | `Celebrating` $\to$ `Success` $\to$ `Idle` |
| `Offline` | `Sleeping` $\to$ `Idle` |
| `Eating` | `Hungry` $\to$ `Idle` |
| `Hungry` | `Idle` |
| `Working` / `Waiting` / `Error` / `Sleeping` / `Paused` | `Idle` |

*Resolution Algorithm:*
1. Check `state_mappings` for the exact state key (normalized: case-insensitive, ASCII alphanumeric).
2. Look for an animation with the state's normalized name.
3. Repeat steps 1 & 2 for each candidate in the fallback chain.
4. Default to the first declared animation in the pack.

### 2.5 Clip Contract (`AnimationClip` & `PetPresentation`)
Rust determines which clip to play via `PetPresentation`:
- **State Clips:** `idle`, `work`, `think`, `wait`, `success`, `error`, `celebrate`, `sleep`, `hungry`, `eat`, `level_up`, `achievement`, `offline`, `paused`, `attention`.
- **Locomotion Clip:** `walk` (layered over the state clip by the desktop host only while moving the transparent overlay).

---

## 3. CharacterSpec v1 Specification & Taxonomy (`crates/avatar`)

A `CharacterSpec` is a data-only procedural specification. Unknown fields are rejected (`deny_unknown_fields`) to prevent executable script injections.

### 3.1 Contract & Validation Constraints

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
  "palette": {
    "skin": "#e07a2f",
    "hair": "#fff7ed",
    "eyes": "#1e3a8a",
    "primary": "#1f2937",
    "secondary": "#374151",
    "accent": "#f59e0b"
  },
  "hairTip": "#fbbf24",
  "proportions": {
    "height": 1.0,
    "headScale": 1.0,
    "build": 1.0,
    "limbLength": 1.0
  },
  "energy": 1.3,
  "tags": ["fox", "speedy"],
  "category": "animal",
  "threeDEligible": true
}
```

| Field | Allowed Values / Limits | Validation Rules |
| :--- | :--- | :--- |
| `schemaVersion` | Must equal `1` | Integer |
| `id` | Dotted lowercase segments (`a-z`, `0-9`, `-`) | 2+ segments, max 64 bytes. Catalog: `miyora.*`, User: `user.*` |
| `name` / `title` | 1-32 chars / 1-48 chars | No control characters |
| `description` | 0-240 chars | No control characters |
| `body` | `humanoid`, `beast`, `robot`, `slime`, `bird` | 5 anatomical base meshes |
| `ears` | `none`, `human`, `elf`, `cat`, `fox`, `bunny`, `bear`, `floppy`, `horns`, `antenna`, `fin`, `leaf` | 12 ear styles |
| `tail` | `none`, `cat`, `fox`, `bunny`, `dragon`, `cable`, `feather`, `flame`, `leaf`, `star` | 10 tail types |
| `wings` | `none`, `feather`, `bat`, `fairy`, `jet` | 5 wing types |
| `hair` | `none`, `short`, `bob`, `long`, `ponytail`, `twintails`, `spiky`, `bun`, `mohawk`, `curly`, `hime`, `side_tail` | 12 hair styles |
| `hairTip` | Optional `#RRGGBB` | Color gradient target at hair tips |
| `eyes` | `round`, `sharp`, `sleepy`, `sparkle`, `visor`, `mono` | 6 anime/stylized eye geometries |
| `outfit` | `hoodie`, `kimono`, `armor`, `suit`, `robe`, `jacket`, `dress`, `overalls`, `labcoat`, `tunic`, `fur`, `plating`, `hakama`, `sailor`, `coat` | 15 clothing styles |
| `pattern` | Optional: `none`, `dots`, `stripes`, `waves`, `petals`, `stars`, `diamonds` | CPU-generated canvas texture mapped onto cloth |
| `accessories` | Up to 5 unique items: `glasses`, `headphones`, `scarf`, `beanie`, `witch_hat`, `crown`, `halo`, `cape`, `backpack`, `headband`, `bow`, `flower`, `goggles`, `bell`, `katana`, `wand`, `staff`, `kitsune_mask`, `shell`, `gem` | 20 unique attachment meshes |
| `palette` | Exactly 6 keys: `skin`, `hair`, `eyes`, `primary`, `secondary`, `accent` | Valid 6-character hex (`#RRGGBB`) |
| `proportions` | `height` (0.8-1.2), `headScale` (0.85-1.35), `build` (0.8-1.3), `limbLength` (0.85-1.15) | Numerical ranges |
| `energy` | Range: `0.5` to `1.6` | Scales animation speed/tempo |
| `tags` | Up to 6 items | 1-24 lowercase ASCII alphanumeric/hyphens |
| `threeDEligible`| `true` or `false` | When `false`, Rust never offers the row as an active 3D pet |

### 3.2 Bundled Avatar Catalog Taxonomy (`characters.json`)
The bundled catalog contains **214 character specifications** structured into a three-tier taxonomy:

1. **Original Designs (169 characters, CC0-1.0)**:
   - 79 humanoid (`character`), 64 animal/beast/bird (`animal`), 14 robot (`robot`), 12 creature/slime (`creature`).
   - Fully eligible for active 3D rendering (`threeDEligible: true`).
   - Default character: `miyora.aoi` (Streetwear Coder with teal tech hoodie, headphones, and matcha latte treat).
2. **Cultural Heritage / Sacred Lore (12 characters, Public Domain)**:
   - Category: `deity_inspired`.
   - Characters: Arjuna, Ganesha, Hanuman, Krishna, Lakshmi, Parvati, Radha, Rama, Saraswati, Shiva, Sita, Sun Wukong.
   - Eligible for 3D procedural pet rendering (`threeDEligible: true`). Disclaims proprietary design ownership.
3. **Parody & Fan Tributes (33 characters, Non-Commercial Legacy)**:
   - Category: `legacy`.
   - 2D tribute entries imitating third-party franchise figures (Gojo, Tanjiro, Luffy, Pikachu, etc.).
   - **Strictly ineligible for active 3D pet rendering** (`threeDEligible: false`).

---

## 4. Procedural Three.js Rig Architecture (`apps/desktop/src/avatar/rig.ts`)

### 4.1 Shared Joint Hierarchy
Every procedural character—regardless of whether it is a humanoid, beast, robot, slime, or bird—shares a unified joint hierarchy. This enables a single motion engine to drive all body types:

```
root (avatar:<id>)
 └── body (position.y = rootY, rotation = leanX, turnY, 0)
      ├── hips (joint at hipY)
      │    ├── spine (rotation.x = spineX)
      │    │    └── chest (rotation = chestX, 0, chestZ)
      │    │         ├── neck (rotation = headX, headY, headZ + look)
      │    │         │    └── head
      │    │         │         ├── headCenter (ears, hair, eyes, mouth, brows, blush)
      │    │         │         └── accessories (glasses, hat, halo, etc.)
      │    │         ├── armL (rotation = armL Euler)
      │    │         │    └── elbowL (rotation.x = elbowL)
      │    │         │         └── handL (rotation.z = wristL)
      │    │         ├── armR (rotation = armR Euler)
      │    │         │    └── elbowR (rotation.x = elbowR)
      │    │         │         └── handR (rotation.z = wristR)
      │    │         └── wingL / wingR (rotation.y = ±0.25 ± wingFlap)
      │    ├── legL (rotation.x = legL)
      │    │    └── kneeL (rotation.x = kneeL)
      │    │         └── footL
      │    ├── legR (rotation.x = legR)
      │    │    └── kneeR (rotation.x = kneeR)
      │    │         └── footR
      │    └── tail[0..n] (rotation.y = tailWag wave)
```

- Orientation: Faces $+Z$, stands on $y = 0$, normalized to approximately 1.0 unit tall.
- Prop Placement Heights: `headTopY` (top of head) and `deskY` (hand height when arm held forward) are computed dynamically per rig.

### 4.2 Cel-Shading & Inverted-Hull Outlines
1. **Four-Band Cel Shading:**
   Uses `THREE.MeshToonMaterial` mapped with a custom 1D `DataTexture` gradient map containing 4 discrete steps: `[96, 158, 214, 255]`.
2. **Inverted Hull Outlines:**
   Every mesh generates a back-face hull: geometry cloned, normals flipped, scaled up slightly (`OUTLINE = 0.0035`), and rendered with `THREE.MeshBasicMaterial` using a darkened shade of the parent mesh (`OUTLINE_DARKEN = 0.72`) or solid dark slate.
3. **Hair Tip Gradients:**
   When `hairTip` is defined, the lock geometry receives interpolated vertex colors (root $=$ `palette.hair`, tip $=$ `hairTip`) rendered via a vertex-colored toon material.
4. **Pattern Canvas Generation:**
   Cloth materials generate dynamic $128 \times 128$ canvas textures for patterns (`dots`, `stripes`, `waves`, `petals`, `stars`, `diamonds`) combining the base outfit color with an accent motif color.

### 4.3 Facial Rigging
- **Eyes:** Built from layered sclera, iris gradient, pupil, dual specular highlights, and upper/lower eyelashes. Eye scales deform along $Y$ for blinking (`eyeOpen * eyeBaseOpen * blink`).
- **Eye Shapes (`EyeShape`):** `open`, `happy` (closed `^^` arcs), `closed` (sleeping curves), `star` (sparkling eyes), `teary` (crying with droplet mesh).
- **Mouth Shapes (`MouthShape`):** Pre-built discrete meshes (`smile`, `grin`, `open`, `flat`, `frown`), toggled via `.visible`.
- **Brows:** Positioned on headCenter; dynamically offset in $Y$ by `browLift` and rotated in $Z$ by `browTilt`.
- **Blush:** Quad planes positioned on the cheeks; opacity modulated by `expression.blush * 0.7`.

### 4.4 Body Type Specializations
- **Humanoid:** Full limb articulation, detailed anime face, clothing/skirt/pants meshes.
- **Beast:** Muzzle/snout mesh, paws, four-legged or chibi standing stance, animated tail chain (3-6 segments).
- **Robot:** Chamfered box torso, glowing chest core (`emissive: 0.7`), cylindrical limbs with ball-and-socket joints, visor/mono eye.
- **Slime:** Single gelatinous translucent sphere (`opacity: 0.94`) with interior bubble core, no legs, exaggerated squash and stretch (`pose.squash * 2.2`).
- **Bird:** Spherical torso, tapered wing arms, feathered tail fan, beak cone.

---

## 5. Avatar Motion & Retargeting Pipeline (`apps/desktop/src/avatar/motion.ts`)

### 5.1 Per-State Motion Choreography
`motionFor(state, t, energy, spec)` calculates the real-time target pose and facial expression:

- **`working`**: Typing pose with alternating left/right arm motions (`sin(w * 16)`), flicking wrists (`pose.wristL/R`), focused flat mouth, laptop prop active. Differentiated by archetype:
  - *Warrior:* Deeper lean, hands gripping forward, stoic brow.
  - *Mage:* Floating posture, rhythmic spellcasting hand rolls.
  - *Beast:* Pawing keystrokes with wagging tail.
- **`thinking`**: Right hand tapping chin (`pose.armR = [-1.55, 0.25, -0.35]`, `elbowR = -1.95`), head tilted upward, thought bubble prop active.
- **`waiting`**: Foot tapping (`kneeR` and `legR` oscillating), looking around, glyph `...`.
- **`attention_required`**: Waving one arm high (`armR = -2.9`), bounce bob, mouth open, warning glyph `!`.
- **`success`**: Friendly wave (`armR = -2.8`), happy `^^` eyes, confetti particles active.
- **`celebrating`**: Both arms raised celebrating, full body twist (`turnY = 0.4 * sin(w * 3)`), joyful grin, confetti active.
- **`level_up`**: Full 360° celebratory spin (`turnY = phase * 2PI`), star eyes, confetti active, glyph `LEVEL UP`.
- **`achievement`**: Victory pose with star eyes, glyph `★`.
- **`error`**: Head drooping, teary eyes, frown mouth, glyph `×`, red alert.
- **`sleeping` / `offline`**: Slow deep breathing rhythm, eyes closed (`eyeOpen: 0`), sleep Zs active (`sleeping`) or stage lighting dimmed by 55% (`offline`).
- **`hungry`**: Drooping shoulders, tired idle, food bowl prop active.
- **`eating`**: Chewing cycle (`sin(w * 5)`), hand moving food to mouth, wagging tail.
- **`paused`**: Frozen resting pose, dimmed glyph `II`.
- **`idle`**: Natural 12-second behavioral loop:
  - 0–4.5s: Gentle breathing float (`breathe = sin(w * 2)`) and subtle weight shifting.
  - 4.5–7.5s: Autonomous playful pacing / micro-steps in place.
  - 7.5–10s: Inquisitive head tilt and ear perking.
  - Autonomous happy squint every few seconds.

### 5.2 Locomotion Walk Layering (`applyWalkCycle`)
When the pet overlay moves across the desktop, `applyWalkCycle` is layered over the current state:
- Alternating leg swings: $\text{legL} = 0.45 \cdot \sin(w)$, $\text{legR} = -0.45 \cdot \sin(w)$.
- Inverse knee bending on back-swing: $\text{knee} = 0.4 \cdot \max(0, \pm\text{stride})$.
- Counter-swinging arms: $\text{armL} \mp= 0.35 \cdot \sin(w)$.
- Light torso bob: $\text{rootY} += 0.02 \cdot |\cos(w)|$.

### 5.3 VRM Retargeting (`applyToVrm`)
VRM avatars (`.vrm` glTF 2.0 binary) are retargeted from the procedural rig pose:
- **Rest Orientation Mapping:** Procedural rigs rest with arms down (facing $+Z$); VRM humanoids rest in a T-pose (arms horizontal). The retargeter multiplies arm Euler rotations by the conjugate quaternion transformation (`restToLeft` / `restToRight`).
- **VRM 0.x vs 1.0:** VRM 0.x normalized bones face $-Z$. For VRM 0.x (`metaVersion === "0"`), rotations are conjugated around $Y$: $[-x, y, -z, w]$.
- **Mapped Bones:** `spine`, `chest`, `neck`, `head`, `leftUpperArm`, `leftLowerArm`, `rightUpperArm`, `rightLowerArm`, `leftUpperLeg`, `rightUpperLeg`, `leftLowerLeg`, `rightLowerLeg`.
- **Blendshape Expressions:** Retargeted onto VRM `expressionManager`: `happy`, `sad`, `aa`, `surprised`, `blink`.

### 5.4 Image Sculpt Whole-Body Motion (`applyToSculpt`)
Image sculpts (2.5D relief meshes) cannot deform joints. The entire model moves as a single rigid body with combined lean, turn, bob, and squash/stretch:
$$\text{rotation} = (\text{leanX} + \text{headX} \cdot 0.25, \; \text{turnY} + \text{headY} \cdot 0.4, \; \text{headZ} \cdot 0.6 + \text{chestZ})$$

### 5.5 Gaze Tracking & Framerate Budget
- **Autonomous Gaze & Cursor Follow:** If the pointer hovers over the stage, the head tracks the cursor within yaw ($\pm 0.45\text{ rad}$) and pitch ($\pm 0.22\text{ rad}$) limits. When not hovering, an autonomous living gaze cycle runs.
- **Framerate Governor:**
  - Active states (`working`, `success`, `level_up`, `celebrating`, `error`): Capped at **30 FPS**.
  - Calm states (`sleeping`, `paused`, `offline`): Throttled to **12 FPS**.
  - Document hidden (`document.hidden`): Rendering completely pauses (**0 FPS**).

---

## 6. Stage Props & Progression HUD (`apps/desktop/src/avatar/props.ts`)

Stage props render around the avatar to communicate status and XP progression:

```
                          [ ! / LEVEL UP / ★ ] (Text Sprite Glyph)
                                 (Head)
                             [Thought / Z z]
                                    │
                                 [Laptop]
                                    │
    ─────────────────────────── [Pedestal] ───────────────────────────
      [ XP Progress Arc (R=0.42) ───● (XP Bead) ]
      [ Badge: "LV 7 · 1,420 / 2,000 XP"       ]
```

- **XP Progress Ring:** Torus arc (`XP_RING_RADIUS = 0.42`) drawn dynamically based on `xpIntoLevel / xpToNextLevel` from the immutable XP ledger. Includes background track ticks and a glowing leading bead.
- **Level HUD Sprite:** Displays `LV <n> · <current>/<target> XP` on a pill badge below the pedestal.
- **Laptop Prop:** Brushed dark-slate laptop positioned at `deskY`, with a canvas screen rendering 9 lines of green terminal code that scroll on keystrokes.
- **Confetti Particle System:** 70 colored particles spawned during `success`, `celebrating`, and `level_up`. Features individual gravitational acceleration, air flutter, and automatic ground resetting.
- **Level-Up Aura:** Dual expanding glowing torus rings and rising motes that ascend around the character.

---

## 7. Image-to-3D Relief Sculpting (`crates/avatar/src/sculpt.rs`)

Miyora features an offline, deterministic image-to-3D geometry generator that runs inside a Tauri worker thread (zero external cloud dependencies, non-neural):

```
User Image (PNG/JPEG/GIF/WebP)
   │
   ├─► 1. File & Memory Validation (Max 20 MiB, 4096px, 128 MiB buffer)
   ├─► 2. Downscale to 128x128 grid
   ├─► 3. Background Segmentation (Alpha test or border flood-fill with color distance)
   ├─► 4. Morphological hole filling & silhouette extraction
   ├─► 5. 3-4 Chamfer Distance Transform (Thicker regions bulge forward)
   ├─► 6. Closed Mesh Generation (Front surface + 70% depth back surface + welded rim)
   ├─► 7. Texture Mapping with bled edge colors
   ├─► 8. Self-Contained GLB Export (< 66,000 triangles)
   └─► 9. Deterministic K-Means Color Clustering (k=5) ──► Paired CharacterSpec
```

Importing a user image creates two custom avatars linked by `sourceAvatarId`:
1. The 3D image sculpt (GLB relief model).
2. A styled procedural character matching the image's clustered palette.

---

## 8. VRM Avatar Import & Security Guardrails (`crates/pet-pack/src/vrm.rs`)

Users can import `.vrm` files created in VRoid Studio, Blender, or Unity. Because 3D model files are untrusted binary inputs, strict guardrails are enforced:

### 8.1 Security Limits
- **Max File Size:** 50 MB (`52,428,800` bytes).
- **Max Triangles:** 100,000 polygons.
- **Max Textures:** 16 images.
- **Max Texture Dimensions:** $2048 \times 2048$ pixels.
- **Max Humanoid Bones:** 64 standard joints.
- **Zero External URIs:** All buffers, images, and meshes must reside internally in the binary glTF container. Any reference to `http://`, `https://`, `file://`, relative paths (`../`), or `data:` URIs is immediately rejected with `VrmValidationError::ExternalResourceForbidden`.

### 8.2 Clean-Room Originality Guarantee
The bundled default companion `miyora.aoi` is a clean-room original design distributed under CC0-1.0. To avoid copyright infringement, all trademarked franchise elements are strictly prohibited in bundled assets:
- No bamboo mouthpieces or gags.
- No pink hemp-leaf (asanoha) geometric kimonos.
- No black outer haoris with red linings.
- No checkered obi sashes or pink-strapped zori sandals.

---

## 9. PetPack v1 / v2 Packaging Specification (`crates/pet-pack/`)

PetPacks provide 2D animated sprite companions packaged as directory trees or `.zip` / `.tar.gz` archives.

### 9.1 Package Structure & Manifest (`pet.json`)

```json
{
  "schemaVersion": 2,
  "id": "creator.pixel-cat",
  "name": "Pixel Cat",
  "version": "1.0.0",
  "description": "An energetic pixel cat companion.",
  "creator": "Artist Name",
  "license": "CC-BY-4.0",
  "source": "https://example.org/pixel-cat",
  "attribution": "Artwork by Artist Name (2026)",
  "commercialUse": false,
  "redistribution": true,
  "derivativeWorks": true,
  "assets": {
    "idleSprite": "assets/idle.png",
    "workSprite": "assets/work.png"
  },
  "animations": {
    "idle": {
      "asset": "idleSprite",
      "frames": [{ "x": 0, "y": 0, "width": 64, "height": 64 }],
      "fps": 4,
      "loop": true,
      "anchor": { "x": 0.5, "y": 1.0 }
    },
    "working": {
      "asset": "workSprite",
      "frames": [
        { "x": 0, "y": 0, "width": 64, "height": 64 },
        { "x": 64, "y": 0, "width": 64, "height": 64 }
      ],
      "fps": 8,
      "loop": true,
      "anchor": { "x": 0.5, "y": 1.0 }
    }
  },
  "stateMappings": {
    "Idle": "idle",
    "Working": "working"
  },
  "tags": ["cat", "pixel"],
  "personality": {
    "themeColor": "#3b82f6",
    "icon": "🐱",
    "treatName": "FishCracker",
    "treatEmoji": "🐟",
    "feedReaction": "Crunching happily! 🐟"
  }
}
```

### 9.2 Archive Security & Sandboxing (`archive.rs`)
- **ZipSlip Protection:** Path traversal (`..`), absolute paths, Windows drive letters, backslashes, control characters, and symlinks are rejected.
- **Decompression Limits:**
  - Max compressed archive: 64 MB.
  - Max extracted size: 128 MB.
  - Max entry count: 1,024 entries.
  - Max compression ratio: 200:1 (prevents decompression zip-bombs).
- **Data-Only Enforcement:** Only `pet.json`, `pet.sig`, and approved image formats (`.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`) are permitted. Executables, scripts, and shared libraries are rejected.
- **Atomic Staging:** Archives extract into a temporary `.staging-<uuid>` directory. Validation runs on the staged copy; only after all checks pass is it atomically renamed into the library directory.

### 9.3 Cryptographic Detached Signatures (`signature.rs`)
Packs can carry a detached Ed25519 signature in `pet.sig`:
```json
{
  "algorithm": "ed25519",
  "keyId": "miyora.official",
  "signature": "<base64-encoded-64-byte-signature>"
}
```
The signature covers the exact byte stream of `pet.json` and is verified against locally registered `TrustedSigner` public keys.

---

## 10. Persistence & Database Schemas (`crates/agentpet-core/src/avatars.rs`)

Avatar selections and customizations are managed in SQLite via **Schema Migration v10**:

### 10.1 Table Definitions
```sql
-- Active avatar selection singleton
CREATE TABLE avatar_selection (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    source TEXT NOT NULL CHECK(source IN ('catalog', 'custom')),
    avatar_id TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Customizations applied to catalog characters
CREATE TABLE avatar_customizations (
    catalog_id TEXT PRIMARY KEY NOT NULL,
    spec_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- User-created custom avatars (procedural, sculpt, or vrm)
CREATE TABLE custom_avatars (
    avatar_id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('procedural', 'image_sculpt', 'vrm')),
    name TEXT NOT NULL,
    spec_json TEXT,
    model_file TEXT,
    model_sha256 TEXT,
    triangle_count INTEGER,
    metadata_json TEXT,
    source_avatar_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK((kind = 'procedural') = (spec_json IS NOT NULL)),
    CHECK((kind = 'procedural') = (model_file IS NULL)),
    CHECK((model_file IS NULL) = (model_sha256 IS NULL))
);

CREATE INDEX custom_avatars_created_idx ON custom_avatars(created_at);
```

### 10.2 Storage Layout & Integrity
- Model files are stored at: `<app_data>/avatars/<avatar_id>/model.<glb|vrm>`.
- File paths are stored as relative names; reads canonicalize paths and ensure they remain inside the avatar directory.
- Model bytes are verified against `model_sha256` before returning to WebGL loaders.
- Total custom avatars are capped at **512** to prevent disk exhaustion.
- Deleting the currently selected avatar resets the selection to `miyora.aoi` in the same database transaction.

---

## 11. Cross-Reference Index

| Spec / Component | Primary Source Files | Primary Responsibilities |
| :--- | :--- | :--- |
| **15-State Machine** | `crates/pet-engine/src/lib.rs`<br>`crates/pet-engine/src/clips.rs` | State definitions, allow-matrix, timeouts, reaction queue, clip resolution |
| **CharacterSpec v1** | `crates/avatar/src/spec.rs`<br>`apps/desktop/src/avatar/types.ts` | Data schema, validation rules, color/proportion limits, category tags |
| **Avatar Catalog** | `crates/avatar/catalog/characters.json`<br>`scripts/generate_avatar_catalog.py` | 214 bundled character specifications, taxonomy, attribution, eligibility |
| **Procedural Rig** | `apps/desktop/src/avatar/rig.ts` | Joint tree, cel-shading, anime facial meshes, outfit geometries |
| **State Motion** | `apps/desktop/src/avatar/motion.ts` | Choreography for 15 states, VRM retargeting, walk cycle, gaze tracking |
| **Stage Props & HUD** | `apps/desktop/src/avatar/props.ts` | Pedestal, XP ring, laptop, thought bubbles, confetti, glyphs |
| **WebGL Stage** | `apps/desktop/src/avatar/AvatarStage.tsx` | Rendering loop, framerate capping (30/12/0 FPS), orbit controls |
| **Image Sculptor** | `crates/avatar/src/sculpt.rs` | Deterministic image downsampling, contour chamfering, GLB generation |
| **PetPack Engine** | `crates/pet-pack/src/lib.rs`<br>`crates/pet-pack/src/archive.rs` | Manifest validation, ZIP/tar.gz sandboxing, frame limits, Ed25519 signatures |
| **VRM Guardrails** | `crates/pet-pack/src/vrm.rs`<br>`docs/specs/vrm_pet.md` | 50 MB / 100k triangle limits, metadata extraction, external URI ban |
| **Avatar Persistence**| `crates/agentpet-core/src/avatars.rs` | SQLite schema v10, SHA-256 verification, avatar staging and atomic swap |

---

## 12. LO's Official 3D Model Review Verdict & Integration Roadmap

On 2026-11-16, LO completed a comprehensive review of all 31 candidate models across the Three.js viewport, the SkeletonHelper inspector, and the Humanoid Inverse Kinematics (IK) Studio (documented in `docs/model_review_verdict.md`).

### 12.1 Approved Models (13 Confirmed)
1. **`model.avatar_sample_a` (AvatarSample A / Annie)**: Flagship twin-tail anime heroine, pleated school uniform (VRM, 206 bones, 24.8k tris).
2. **`model.avatar_sample_b` (AvatarSample B)**: Modern blazer student avatar with expressive blendshapes (VRM, 250 bones, 28.3k tris).
3. **`model.vita` (Vita)**: Magical fantasy anime heroine with vibrant hair and fantasy accessories (VRM, 310 bones, 26.0k tris).
4. **`model.vivi` (Vivi)**: High-fashion anime heroine with layered hair and couture coat (VRM, 304 bones, 25.8k tris).
5. **`model.sakurada_fumiriya` (Sakurada Fumiriya)**: Traditional Japanese shrine maiden (Miko) (VRM, 214 bones, 39.5k tris).
6. **`model.monero_chan` (Monero-chan)**: Kawaii privacy guardian with twin-tails and pleated skirt (VRM, 218 bones, 41.2k tris).
7. **`model.darkness_shibu` (Darkness Shibu)**: Mystic dark deity / fantasy sorceress with hooded cloak (VRM, 149 bones, 33.7k tris).
8. **`model.victoria_rubin` (Victoria Rubin)**: High-detail anime heroine with refined hair physics (VRM, 162 bones, 28.9k tris).
9. **`model.sendagaya_shino` (Sendagaya Shino)**: Classic anime high school companion (VRM, 148 bones, 31.2k tris).
10. **`model.alicia_solid` (Alicia Solid)**: Canonical anime avatar with twin-tail spring bones & complete blendshapes (VRM, 128 bones, 31.8k tris).
11. **`model.seed_san` (Seed-san)**: Official VRM 1.0 standard reference humanoid model (VRM, 128 bones, 45.0k tris).
12. **`model.readyplayer_me` (Ready Player Me Base)**: Full-body standard humanoid base for procedural outfits (GLB, 67 bones, 15.8k tris).
13. **`model.soldier` (Soldier Humanoid)**: Rigged humanoid soldier with embedded locomotion clips (GLB, 49 bones, 11.3k tris).

### 12.2 Architectural Integration Plan
- **VRM Primacy:** With 9 out of 13 approved models in standard VRM format, Miyora prioritizes VRM 0.x / 1.0 loading with `@pixiv/three-vrm` spring-bone secondary physics and facial blendshape drives.
- **PetPack Generation:** Pack manifest generators will package the approved models into atomic, signed PetPacks with 15-state behavior mappings, 512x512 preview portraits, and Ed25519 signatures.
- **Adjustments:** The 4 models pending adjustment (`celestia`, `paladin`, `kira`, `robot_expressive`) will undergo Draco mesh decimation, PBR shader re-mapping, and height re-scaling to meet `crates/pet-pack` guardrails before inclusion.

