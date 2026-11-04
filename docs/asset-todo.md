# PetPack Asset Quality & Artwork Todo

This document tracks the asset status and animation roadmap for Miyora's bundled 2D PetPacks under `pet-packs/`.

## 1. Executive Summary

A comprehensive pixel-level audit revealed that out of 47 bundled companion packs:
- **2 packs** (`ninja-cat`, `sun-wukong`) are **genuinely animated** with multi-frame sprite sheets.
- **4 packs** (`arjuna`, `momiji`, `nezuko`, `starter-aoi`) feature distinct artwork between primary active states, but share static images for non-active states (`idle`/`thumbnail` or `level_up`/`achievement`).
- **41 packs** reuse identical image content across distinct active states (`idle` vs `waiting`, `success` vs `celebrating`), functioning effectively as static cards rather than animated sprites.
- **1 pack** (`starter-aoi`) completely lacks an alpha channel (stored as 768x768 opaque RGB), rendering an opaque rectangular box over the transparent desktop overlay.

Under the Phase 5 asset honesty policy, static packs declare `animation: static` and are rendered using live state-driven CSS physics (breathe, sway, pulse, bounce, shake) rather than pretending to animate non-existent sprite frames. A ratchet allowlist (`scripts/pet_pack_allowlist.json`) prevents new unverified packs from being introduced while grandfathering existing packs.

> [!IMPORTANT]
> **Zero Placeholder Policy:** Do not generate synthetic duplicate frames, AI-hallucinated variations, or blank placeholder images. When genuine hand-crafted or properly licensed sprite frames become available, update the pack, verify with `python scripts/validate_pet_packs.py`, and remove the pack from the allowlist.

---

## 2. Genuinely Animated Packs (Reference Standard)

These packs possess genuine multi-frame sprite sheets and serve as the golden standard:

| Pack ID | Character Name | Animations Declared | Frame Counts | Alpha Channel |
|---|---|---|---|---|
| `ninja-cat` | Ninja Cat | idle, working, thinking, waiting, success, error, level_up, sleeping | Multi-frame strip | RGBA (512x512) |
| `sun-wukong` | Sun Wukong | idle, working, thinking, waiting, success, error, level_up, sleeping | Multi-frame strip | RGBA (512x512) |

---

## 3. Alpha Channel Repair Required

| Pack ID | Character Name | Current Mode | Issue | Required Action |
|---|---|---|---|---|
| `starter-aoi` | Aoi (Starter) | RGB (768x768) | No alpha channel; opaque background blocks desktop overlay. | Re-export or mask background to 32-bit RGBA at 512x512 resolution. |

---

## 4. Multi-Frame Artwork Needed (41 Packs)

The following packs are grandfathered in `scripts/pet_pack_allowlist.json`. They require authentic multi-frame sprite sheets (recommended: 4 frames for `idle`, `working`, `success`, `level_up`; 2 frames for `thinking`, `waiting`, `error`, `sleeping`) with transparent alpha:

| Pack ID | Franchise / Concept | Current Asset Coverage | Target States Needed |
|---|---|---|---|
| `anya` | Spy x Family | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `denji` | Chainsaw Man | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `frieren` | Frieren | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `ganesha` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `genos` | One Punch Man | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `gojo` | Jujutsu Kaisen | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `goku` | Dragon Ball | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `gon` | Hunter x Hunter | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `hanuman` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `ichigo` | Bleach | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `killua` | Hunter x Hunter | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `kodama-spirit` | Studio Ghibli inspired | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `krishna` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `l-lawliet` | Death Note | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `lakshmi` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `levi` | Attack on Titan | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `light` | Death Note | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `luffy` | One Piece | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `mikasa` | Attack on Titan | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `nami` | One Piece | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `naruto` | Naruto | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `parvati` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `pikachu` | Pokémon | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `power` | Chainsaw Man | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `radha` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `rama` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `rukia` | Bleach | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `saitama` | One Punch Man | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `sanji` | One Piece | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `saraswati` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `sasuke` | Naruto | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `shiva` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `sita` | Mythological | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `tanjiro` | Demon Slayer | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `valley-flyer` | Original Mascot | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `vegeta` | Dragon Ball | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `witch-apprentice`| Original Fantasy | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `woodland-guardian`| Original Fantasy | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `yor` | Spy x Family | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `yuji` | Jujutsu Kaisen | 2 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |
| `zoro` | One Piece | 3 static poses | idle, working, thinking, waiting, success, error, level_up, sleeping |

---

## 5. Verification Protocol

Run before and after any artwork additions:
```bash
# Must pass with 0 errors
python scripts/validate_pet_packs.py
```
When real animation frames are provided for any pack, remove its entry from `scripts/pet_pack_allowlist.json`. The validator will confirm the ratchet in the summary.
