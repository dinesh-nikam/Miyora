# Miyora (AgentPet) — Official 3D Model Review Verdict & Integration Plan

**Date:** 2026-11-16  
**Reviewer:** LO  
**Verdict Summary:** 13 Confirmed, 4 Pending Adjustments, 14 Rejected

---

## 1. Executive Summary & Review Breakdown

LO completed an end-to-end audit of all 31 candidate models across the Three.js viewport, the SkeletonHelper bone inspector, and the Humanoid Inverse Kinematics (IK) Studio.

```
Total Models Evaluated: 31
├── 💖 Confirmed (Approved for Miyora Engine): 13 (42%)
├── ⚠️ Needs Adjustment: 4 (13%)
└── ❌ Rejected (Unsuitable / Incompatible): 14 (45%)
```

### Strategic Key Finding
**9 out of 13 approved models are in the VRM standard format.**  
LO decisively favored the expressive fidelity, bone topology, facial blendshapes, and spring-bone secondary physics of modern anime VRM models over generic developer primitives or western-styled avatars. The remaining 2 approved GLB models (`readyplayer_me`, `soldier`) provide solid full-body humanoid baselines with proven skeletal keyframe tracks.

---

## 2. 💖 The 13 Confirmed Models (Approved for Miyora Engine)

These 13 models will be packaged into official Miyora PetPacks and catalog specifications:

| # | Model ID | Character Name | Format | Size | Bones | Tris | Archetype / Persona |
|---|---|---|---|---|---|---|---|
| 1 | `model.avatar_sample_a` | **AvatarSample A (Annie)** | VRM | 14.4 MB | 206 | 24,854 | Flagship twin-tail anime heroine, pleated school uniform, dynamic hair physics |
| 2 | `model.avatar_sample_b` | **AvatarSample B** | VRM | 14.7 MB | 250 | 28,292 | Modern anime student with school blazer and full facial blendshapes |
| 3 | `model.vita` | **Vita** | VRM | 13.7 MB | 310 | 26,056 | Magical fantasy anime heroine with vibrant hair and fantasy accessories |
| 4 | `model.vivi` | **Vivi** | VRM | 17.3 MB | 304 | 25,832 | High-fashion anime protagonist with layered hair and couture coat |
| 5 | `model.sakurada_fumiriya` | **Sakurada Fumiriya** | VRM | 18.4 MB | 214 | 39,520 | Traditional Japanese shrine maiden (Miko) with waist-length hair and ribbons |
| 6 | `model.monero_chan` | **Monero-chan** | VRM | 10.4 MB | 218 | 41,208 | Kawaii anime privacy guardian with twin-tails and pleated skirt |
| 7 | `model.darkness_shibu` | **Darkness Shibu** | VRM | 20.4 MB | 149 | 33,758 | Mystic dark deity / fantasy sorceress with hooded cloak |
| 8 | `model.victoria_rubin` | **Victoria Rubin** | VRM | 15.5 MB | 162 | 28,928 | High-detail anime heroine with refined hair physics |
| 9 | `model.sendagaya_shino` | **Sendagaya Shino** | VRM | 15.1 MB | 148 | 31,211 | Classic anime high school companion |
| 10 | `model.alicia_solid` | **Alicia Solid** | VRM | 7.9 MB | 128 | 31,798 | Canonical anime avatar with twin-tail spring bones & complete blendshapes |
| 11 | `model.seed_san` | **Seed-san** | VRM | 10.9 MB | 128 | 45,058 | Official VRM 1.0 standard reference humanoid model |
| 12 | `model.readyplayer_me` | **Ready Player Me Base** | GLB | 1.8 MB | 67 | 15,817 | Full-body standard humanoid base for procedural outfits |
| 13 | `model.soldier` | **Soldier Humanoid** | GLB | 2.2 MB | 49 | 11,376 | Rigged humanoid with embedded locomotion clips (Idle, Walk, Run) |

---

## 3. ⚠️ The 4 Models Needing Adjustment

| Model ID | Current Status | Technical Diagnosis | Prescribed Adjustment |
|---|---|---|---|
| `model.celestia` | Star Goddess (GLB) | **Exceeds VRM / PetPack polygon limit.** Has 108,820 triangles (limit is 100,000 tris per `crates/pet-pack/src/vrm.rs`). Nebula robes need minor optimization. | Run Draco mesh simplification to decimate to ~85k tris; optimize emissive materials for transparent desktop overlay. |
| `model.paladin` | Holy Sentinel (GLB) | Armor materials are flat in unlit environments; shield and sword nodes need separate bone parenting for IK weapon wielding. | Add metallic/roughness PBR maps, parent weapon nodes to right hand bone (`mixamorig:RightHand`), adjust shoulder pad rigging. |
| `model.kira` | Anime IK Benchmark (GLB) | Low-poly cel shading lacks hair texture depth; native IK targets are static rather than driven by dynamic animation clips. | Retarget facial blendshapes; apply custom toon shader matching Miyora desktop palette. |
| `model.robot_expressive` | Expressive Robot (GLB) | Proportions are squat/blocky compared to humanoid anime companions; animations are energetic but unscaled. | Scale rig joints to match standard humanoid companion height (1.6m); map facial morphs to 15-state emotion cues. |

---

## 4. ❌ The 14 Rejected Models & Rationale

1. **`model.xbot`:** Rejected — Faceless grey mannequin rig. Pure developer test dummy with no visual charm.
2. **`model.angel_baby`:** Rejected — Chibi infant model. Unsuited for desktop companion aesthetic.
3. **`model.aura`:** Rejected — Abstract glowing humanoid shape with low bone coverage (37%).
4. **`model.venus`:** Rejected — Glamour aesthetic did not align with Miyora companion tone.
5. **`miyora.*` (10 procedural primitive avatars):** Rejected — Simple primitive geometric rigs (`Parvati`, `Lakshmi`, `Saraswati`, `Sorako`, `Seraph`, `Sun Wukong`, `Aoi`, `Kenji`, `Mira`, `Yorune`). Now that high-fidelity VRM anime models are approved, these early procedural test rigs can be superseded by actual VRM PetPacks.

---

## 5. PetPack Packaging Pipeline (Next Steps)

Following the authoritative specifications in `docs/specs_avatar_petpack_summary.md` and `crates/pet-pack`:

1. **Manifest v1/v2 Generation (`manifest.json`):**
   - Define unique pack identifiers (e.g. `petpack.anime.annie`, `petpack.deity.shibu`, `petpack.anime.fumiriya`).
   - Map 15 canonical states to corresponding animation clips/poses with the fallback hierarchy:
     `Thinking` $\to$ `Working` $\to$ `Idle`; `AttentionRequired` $\to$ `Waiting` $\to$ `Idle`; `Success` $\to$ `Celebrating` $\to$ `Idle`.
2. **VRM Guardrail Verification (`crates/pet-pack/src/vrm.rs`):**
   - Max file size $\le 50\text{ MB}$ (All 13 approved models pass: largest is Darkness Shibu at 20.4 MB).
   - Max triangles $\le 100,000$ (All 13 approved models pass: highest is Seed-san at 45k).
   - Texture dimensions $\le 2048 \times 2048$.
3. **Thumbnail & Preview Generation:**
   - 512x512 rendered portrait preview for Command Deck and Companion catalog.
4. **Ed25519 Integrity Signing:**
   - Sign pack manifests for tamper-evident offline installation.
