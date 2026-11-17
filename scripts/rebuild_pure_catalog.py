import os
import json

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MODELS_JSON_PATH = os.path.join(ROOT, "showcase", "public", "models", "models.json")
ALL_AVATARS_DATA_PATH = os.path.join(ROOT, "showcase", "public", "all_avatars_data.json")

# The 22 authentic 3D humanoid models (VRM & GLB)
TRUE_3D_MODELS = [
    # --- 13 Confirmed for Miyora ---
    {
        "id": "model.avatar_sample_a",
        "name": "AvatarSample A (Annie)",
        "title": "Flagship Twin-Tail Anime Heroine",
        "filename": "AvatarSample_A.vrm",
        "url": "./public/models/AvatarSample_A.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "VRoid Studio Conditions (Open Distribution)",
        "description": "Pixiv's flagship anime heroine with signature twin-tails, cel-shaded eyes, pleated school uniform, articulated 3D fingers, and full humanoid skeleton.",
        "bones": 206,
        "polygons": 24854,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.avatar_sample_b",
        "name": "AvatarSample B",
        "title": "Pixiv Anime Student (ChatVRM)",
        "filename": "AvatarSample_B.vrm",
        "url": "./public/models/AvatarSample_B.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "VRoid Studio / MIT",
        "description": "Official ChatVRM student avatar with tailored school blazer, facial blendshapes, articulated fingers, and humanoid skeleton.",
        "bones": 250,
        "polygons": 28292,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.vita",
        "name": "Vita",
        "title": "Magical Fantasy Anime Girl",
        "filename": "Vita.vrm",
        "url": "./public/models/Vita.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Vibrant magical anime girl with fantasy accessories, articulated hands, flowing hair, and full humanoid IK hierarchy.",
        "bones": 310,
        "polygons": 26056,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.vivi",
        "name": "Vivi",
        "title": "High-Fashion Couture Anime Heroine",
        "filename": "Vivi.vrm",
        "url": "./public/models/Vivi.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "High-fashion anime heroine with voluminous layered hairstyle, couture coat, full 3D hand articulation, and legs.",
        "bones": 304,
        "polygons": 25832,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.sakurada_fumiriya",
        "name": "Sakurada Fumiriya",
        "title": "Traditional Japanese Shrine Maiden (Miko)",
        "filename": "Sakurada_Fumiriya.vrm",
        "url": "./public/models/Sakurada_Fumiriya.vrm",
        "category": "deity_inspired",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Sacred shrine maiden with waist-length flowing dark hair, traditional Shinto ribbons, full humanoid joints, and fingers.",
        "bones": 214,
        "polygons": 39520,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.monero_chan",
        "name": "Monero-chan",
        "title": "Kawaii Anime Privacy Guardian",
        "filename": "monero-chan.vrm",
        "url": "./public/models/monero-chan.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "M3-org",
        "license": "CC0 1.0 Universal",
        "description": "Kawaii anime privacy guardian with twin-tails, pleated skirt, articulated hands, and full humanoid skeleton.",
        "bones": 218,
        "polygons": 41208,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.darkness_shibu",
        "name": "Darkness Shibu",
        "title": "Mystic Dark Deity / Sorceress",
        "filename": "Darkness_Shibu.vrm",
        "url": "./public/models/Darkness_Shibu.vrm",
        "category": "deity_inspired",
        "batch": "batch_1",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Mystic dark deity archetype with hooded cape, dynamic cloth physics, 3D articulated hands, and full humanoid bone hierarchy.",
        "bones": 149,
        "polygons": 33758,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.victoria_rubin",
        "name": "Victoria Rubin",
        "title": "High-Detail Anime Heroine",
        "filename": "Victoria_Rubin.vrm",
        "url": "./public/models/Victoria_Rubin.vrm",
        "category": "anime_character",
        "batch": "batch_1",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Refined anime heroine with detailed hair strands, stylish outfit, articulated fingers, and complete humanoid IK hierarchy.",
        "bones": 162,
        "polygons": 28928,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.sendagaya_shino",
        "name": "Sendagaya Shino",
        "title": "Classic Anime School Companion",
        "filename": "Sendagaya_Shino.vrm",
        "url": "./public/models/Sendagaya_Shino.vrm",
        "category": "anime_character",
        "batch": "batch_1",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Classic anime schoolgirl companion with full humanoid skeleton, articulated 3D limbs, and expressive blendshapes.",
        "bones": 148,
        "polygons": 31211,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.alicia_solid",
        "name": "Alicia Solid",
        "title": "Canonical Anime VRM Avatar",
        "filename": "AliciaSolid.vrm",
        "url": "./public/models/AliciaSolid.vrm",
        "category": "anime_character",
        "batch": "batch_1",
        "type": "vrm",
        "author": "DWANGO Co., Ltd.",
        "license": "NicoNico License (CC-BY Compatible)",
        "description": "Canonical anime avatar with twin-tail spring bones, complete facial blendshapes, articulated fingers, and standard humanoid rig.",
        "bones": 128,
        "polygons": 31798,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.seed_san",
        "name": "Seed-san",
        "title": "Official VRM 1.0 Humanoid Reference",
        "filename": "Seed-san.vrm",
        "url": "./public/models/Seed-san.vrm",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "vrm",
        "author": "VirtualCast, Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Official VRM 1.0 standard reference humanoid model demonstrating standard humanoid bone constraints and twist joints.",
        "bones": 128,
        "polygons": 45058,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.readyplayer_me",
        "name": "Ready Player Me Base",
        "title": "Full-Body Standard Humanoid Rig",
        "filename": "readyplayer.me.glb",
        "url": "./public/models/readyplayer.me.glb",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "glb",
        "author": "Ready Player Me",
        "license": "CC-BY 4.0 / Open",
        "description": "Full-body standard humanoid rig with complete finger articulation, facial blendshapes, and standard Mixamo bone naming.",
        "bones": 67,
        "polygons": 15817,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },
    {
        "id": "model.soldier",
        "name": "Soldier Humanoid",
        "title": "Rigged Humanoid with Locomotion Clips",
        "filename": "Soldier.glb",
        "url": "./public/models/Soldier.glb",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "glb",
        "author": "Mixamo / Three.js",
        "license": "CC0 / MIT",
        "description": "Rigged humanoid with embedded locomotion keyframe tracks (Idle, Walk, Run) and standard skeletal joints.",
        "bones": 49,
        "polygons": 11376,
        "ik_compatible": True,
        "review_status": "confirmed",
        "review_badge": "💖 Confirmed for Miyora",
        "review_note": "Approved by LO for Miyora companion engine."
    },

    # --- 5 Brand New Authentic Humanoid VRM Models ---
    {
        "id": "model.avatar_sample_c",
        "name": "AvatarSample C (Chic Hero)",
        "title": "Pixiv Anime Student Hero",
        "filename": "AvatarSample_C.vrm",
        "url": "./public/models/AvatarSample_C.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "VRoid Studio Conditions (Open Distribution)",
        "description": "Official Pixiv VRoid anime character with modern hairstyle, tailored blazer, expressive anime eyes, all 10 articulated fingers, and complete humanoid skeleton.",
        "bones": 101,
        "polygons": 26988,
        "ik_compatible": True,
        "review_status": "new_candidate",
        "review_badge": "✨ Authentic VRM",
        "review_note": "True 3D humanoid mesh with articulated 3D hands and dual leg joints."
    },
    {
        "id": "model.sendagaya_shibu",
        "name": "Sendagaya Shibu",
        "title": "Anime Heroine (Streetwear)",
        "filename": "Sendagaya_Shibu.vrm",
        "url": "./public/models/Sendagaya_Shibu.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "High-fidelity anime heroine with casual streetwear, flowing hair physics, all 10 articulated fingers, and complete humanoid IK hierarchy.",
        "bones": 129,
        "polygons": 34395,
        "ik_compatible": True,
        "review_status": "new_candidate",
        "review_badge": "✨ Authentic VRM",
        "review_note": "Sister to Shino. Full 3D humanoid anatomy."
    },
    {
        "id": "model.fem_vroid",
        "name": "Sakura (Fem Vroid Base)",
        "title": "Authentic Anime Female Companion",
        "filename": "fem_vroid.vrm",
        "url": "./public/models/fem_vroid.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc. / Community",
        "license": "CC0 1.0 Universal",
        "description": "Pure anime heroine base with delicate facial proportions, articulated 3D hands, natural leg silhouette, and fluid humanoid IK motion.",
        "bones": 59,
        "polygons": 19352,
        "ik_compatible": True,
        "review_status": "new_candidate",
        "review_badge": "✨ Authentic VRM",
        "review_note": "Articulated hands and feet with natural human proportions."
    },
    {
        "id": "model.masc_vroid",
        "name": "Ren (Masc Vroid Base)",
        "title": "Authentic Anime Male Companion",
        "filename": "masc_vroid.vrm",
        "url": "./public/models/masc_vroid.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Pixiv Inc. / Community",
        "license": "CC0 1.0 Universal",
        "description": "Anime male protagonist base with clean athletic build, articulated fingers, rigged feet, and standard humanoid skeleton.",
        "bones": 59,
        "polygons": 19354,
        "ik_compatible": True,
        "review_status": "new_candidate",
        "review_badge": "✨ Authentic VRM",
        "review_note": "Articulated hands and feet with natural human proportions."
    },
    {
        "id": "model.avatar_orion",
        "name": "Orion (Celestial Guardian)",
        "title": "Mythic Astral Sentinel",
        "filename": "Avatar_Orion.vrm",
        "url": "./public/models/Avatar_Orion.vrm",
        "category": "deity_inspired",
        "batch": "batch_2",
        "type": "vrm",
        "author": "Polygonal Mind",
        "license": "CC0 1.0 Universal",
        "description": "Astral celestial guardian with futuristic mythic attire, full humanoid bone hierarchy, and articulated 3D limbs.",
        "bones": 65,
        "polygons": 6200,
        "ik_compatible": True,
        "review_status": "new_candidate",
        "review_badge": "✨ Authentic VRM",
        "review_note": "Celestial guardian with complete humanoid bone topology."
    },

    # --- 4 Pending Adjustment ---
    {
        "id": "model.celestia",
        "name": "Celestia (Star Goddess)",
        "title": "Cosmic Star Goddess / Librarian",
        "filename": "celestia.glb",
        "url": "./public/models/celestia.glb",
        "category": "deity_inspired",
        "batch": "batch_2",
        "type": "glb",
        "author": "M3-org",
        "license": "CC0 1.0 Universal",
        "description": "Cosmic star goddess with celestial nebula robes and full humanoid rig (pending polygon decimation to <100k tris).",
        "bones": 83,
        "polygons": 108820,
        "ik_compatible": True,
        "review_status": "adjustment_needed",
        "review_badge": "⚠️ Needs Adjustment",
        "review_note": "Polygon decimation pending to meet 100k tri VRM ceiling."
    },
    {
        "id": "model.paladin",
        "name": "Paladin (Holy Sentinel)",
        "title": "Holy Ceremonial Plate Knight",
        "filename": "paladin.glb",
        "url": "./public/models/paladin.glb",
        "category": "deity_inspired",
        "batch": "batch_2",
        "type": "glb",
        "author": "M3-org",
        "license": "CC0 1.0 Universal",
        "description": "Holy ceremonial plate knight sentinel with humanoid skeleton (pending PBR armor maps and weapon parenting).",
        "bones": 36,
        "polygons": 40016,
        "ik_compatible": True,
        "review_status": "adjustment_needed",
        "review_badge": "⚠️ Needs Adjustment",
        "review_note": "Pending PBR armor roughness maps and weapon hand-parenting."
    },
    {
        "id": "model.kira",
        "name": "Kira Anime Humanoid",
        "title": "Anime Humanoid IK Benchmark",
        "filename": "kira.glb",
        "url": "./public/models/kira.glb",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "glb",
        "author": "Aki / Antoine Bernier",
        "license": "CC0 1.0 Universal",
        "description": "Anime humanoid with native IK target effectors (pending cel shader texture refinement).",
        "bones": 23,
        "polygons": 51601,
        "ik_compatible": True,
        "review_status": "adjustment_needed",
        "review_badge": "⚠️ Needs Adjustment",
        "review_note": "Pending cel shader texture refinement."
    },
    {
        "id": "model.robot_expressive",
        "name": "Robot Expressive",
        "title": "Expressive Stylized Humanoid",
        "filename": "RobotExpressive.glb",
        "url": "./public/models/RobotExpressive.glb",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "glb",
        "author": "Three.js / Tom Whittock",
        "license": "CC0 / MIT",
        "description": "Stylized robot with facial morph targets (pending height scale normalization to 1.6m).",
        "bones": 43,
        "polygons": 3237,
        "ik_compatible": True,
        "review_status": "adjustment_needed",
        "review_badge": "⚠️ Needs Adjustment",
        "review_note": "Pending height normalization to 1.6m."
    }
]

def main():
    # 1. Write models.json
    with open(MODELS_JSON_PATH, "w", encoding="utf-8") as f:
        json.dump(TRUE_3D_MODELS, f, indent=2)
    print(f"Saved {len(TRUE_3D_MODELS)} authentic 3D humanoid models to {MODELS_JSON_PATH}")

    # 2. Build Master all_avatars_data.json
    all_avatars = []

    # A) 22 Authentic 3D Models
    for m in TRUE_3D_MODELS:
        all_avatars.append({
            "id": m["id"],
            "name": m["name"],
            "title": m["title"],
            "avatar_type": "3d_humanoid_model",
            "format": m["type"].upper() + " 3D Humanoid",
            "category": m["category"],
            "batch": m["batch"],
            "description": m["description"],
            "author": m["author"],
            "license": m["license"],
            "review_status": m["review_status"],
            "review_badge": m["review_badge"],
            "review_note": m["review_note"],
            "bones": m["bones"],
            "polygons": m["polygons"],
            "ik_compatible": True,
            "url": m["url"],
            "has_3d_model": True,
            "preview_type": "3d",
            "tags": ["3d", m["type"], m["category"], m["batch"], "humanoid"]
        })

    # B) 47 Bundled 2D PetPacks (honest 2D sprite representation)
    pet_packs_dir = os.path.join(ROOT, "pet-packs")
    if os.path.exists(pet_packs_dir):
        for entry in sorted(os.listdir(pet_packs_dir)):
            p_path = os.path.join(pet_packs_dir, entry)
            if os.path.isdir(p_path):
                pj_path = os.path.join(p_path, "pet.json")
                if os.path.exists(pj_path):
                    try:
                        with open(pj_path, "r", encoding="utf-8") as f:
                            pdata = json.load(f)

                        img_rel = None
                        assets_dir = os.path.join(p_path, "assets")
                        if os.path.exists(os.path.join(assets_dir, "idle.png")):
                            img_rel = f"./public/pet-packs/{entry}/assets/idle.png"
                        elif os.path.exists(os.path.join(assets_dir, "thumbnail.png")):
                            img_rel = f"./public/pet-packs/{entry}/assets/thumbnail.png"

                        desc = pdata.get("description", "").lower()
                        name_lower = pdata.get("name", "").lower()
                        cat = "anime"
                        if any(k in desc or k in name_lower for k in ["krishna", "radha", "rama", "sita", "shiva", "parvati", "ganesha", "hanuman", "lakshmi", "saraswati", "arjuna", "wukong", "deity"]):
                            cat = "deity_inspired"
                        elif any(k in desc or k in name_lower for k in ["cat", "dog", "pikachu", "spirit", "guardian", "beast"]):
                            cat = "creature"

                        all_avatars.append({
                            "id": pdata.get("id", f"pack.{entry}"),
                            "slug": entry,
                            "name": pdata.get("name", entry.replace("-", " ").title()),
                            "title": f"PetPack: {entry.replace('-', ' ').title()}",
                            "avatar_type": "bundled_2d_petpack",
                            "format": "2D Sprite Companion",
                            "category": cat,
                            "batch": "bundled_petpack",
                            "description": pdata.get("description", ""),
                            "author": pdata.get("creator", "Miyora / Community"),
                            "license": pdata.get("license", "CC-BY-4.0"),
                            "review_status": "bundled_pack",
                            "review_badge": "📦 2D PetPack",
                            "review_note": "Bundled 2D animated desktop companion sprite sheet.",
                            "image_url": img_rel,
                            "has_3d_model": False,
                            "preview_type": "image",
                            "tags": ["pet-pack", "2d", cat, entry]
                        })
                    except Exception as e:
                        print(f"Error parsing {pj_path}: {e}")

    # C) 214 Catalog Character Specifications
    catalog_path = os.path.join(ROOT, "crates", "avatar", "catalog", "characters.json")
    if os.path.exists(catalog_path):
        with open(catalog_path, "r", encoding="utf-8") as f:
            cat_data = json.load(f)
            chars = cat_data.get("characters", [])
            for c in chars:
                cat = c.get("category", "character")
                three_d = c.get("threeDEligible", False)
                all_avatars.append({
                    "id": c.get("id"),
                    "name": c.get("name"),
                    "title": c.get("title", ""),
                    "avatar_type": "catalog_spec",
                    "format": "3D Procedural Rig" if three_d else "CharacterSpec (Legacy 2D)",
                    "category": cat,
                    "batch": "catalog_spec",
                    "description": c.get("description", ""),
                    "author": c.get("attribution", {}).get("creator", "Miyora Contributors"),
                    "license": "CC0-1.0",
                    "review_status": "catalog_spec",
                    "review_badge": "🪷 3D Procedural Rig" if three_d else "📜 Legacy 2D",
                    "review_note": f"Specialty: {c.get('specialty', 'General')}. Personality: {c.get('personality', 'neutral')}.",
                    "body": c.get("body", "humanoid"),
                    "hair": c.get("hair", ""),
                    "eyes": c.get("eyes", ""),
                    "ears": c.get("ears", ""),
                    "tail": c.get("tail", ""),
                    "wings": c.get("wings", ""),
                    "outfit": c.get("outfit", ""),
                    "accessories": c.get("accessories", []),
                    "palette": c.get("palette", {}),
                    "proportions": c.get("proportions", {}),
                    "personality": c.get("personality", ""),
                    "specialty": c.get("specialty", ""),
                    "catchphrase": c.get("catchphrase", ""),
                    "three_d_eligible": three_d,
                    "has_3d_model": three_d,
                    "preview_type": "procedural",
                    "tags": ["catalog", cat, c.get("body", "")] + c.get("tags", [])
                })

    print(f"Total Unified Avatars: {len(all_avatars)}")
    with open(ALL_AVATARS_DATA_PATH, "w", encoding="utf-8") as f:
        json.dump(all_avatars, f, indent=2)
    print(f"Saved master avatar database to {ALL_AVATARS_DATA_PATH}")

if __name__ == "__main__":
    main()
