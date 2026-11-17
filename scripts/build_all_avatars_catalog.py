import os
import json

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUTPUT_PATH = os.path.join(ROOT, "showcase", "public", "all_avatars_data.json")
SCULPT_DIR = os.path.join(ROOT, "showcase", "public", "models", "sculpted")

def main():
    all_avatars = []

    # 1. Load 3D Models (21 candidates + confirmed)
    models_path = os.path.join(ROOT, "showcase", "public", "models", "models.json")
    if os.path.exists(models_path):
        with open(models_path, "r", encoding="utf-8") as f:
            models_data = json.load(f)
            for m in models_data:
                all_avatars.append({
                    "id": m.get("id"),
                    "name": m.get("name"),
                    "title": m.get("title", ""),
                    "avatar_type": "3d_model",
                    "format": m.get("type", "glb").upper(),
                    "category": m.get("category", "3d_avatar"),
                    "batch": m.get("batch", "batch_2"),
                    "description": m.get("description", ""),
                    "author": m.get("author", "Open Source"),
                    "license": m.get("license", "CC0"),
                    "review_status": m.get("review_status", "unreviewed"),
                    "review_badge": m.get("review_badge", ""),
                    "review_note": m.get("review_note", ""),
                    "bones": m.get("bones", 0),
                    "polygons": m.get("polygons", 0),
                    "ik_compatible": m.get("ik_compatible", True),
                    "url": m.get("url", ""),
                    "has_3d_model": True,
                    "model_source": "staged_3d",
                    "preview_type": "3d",
                    "tags": ["3d", m.get("type", "glb"), m.get("category", ""), m.get("batch", "")]
                })
        print(f"Loaded {len(models_data)} 3D models")

    # 2. Load Bundled PetPacks (47 items) -> check if sculpted to 3D
    pet_packs_dir = os.path.join(ROOT, "pet-packs")
    if os.path.exists(pet_packs_dir):
        count_packs = 0
        sculpted_count = 0
        for entry in sorted(os.listdir(pet_packs_dir)):
            p_path = os.path.join(pet_packs_dir, entry)
            if os.path.isdir(p_path):
                pj_path = os.path.join(p_path, "pet.json")
                if os.path.exists(pj_path):
                    try:
                        with open(pj_path, "r", encoding="utf-8") as f:
                            pdata = json.load(f)
                            
                        # find image
                        img_rel = None
                        assets_dir = os.path.join(p_path, "assets")
                        if os.path.exists(os.path.join(assets_dir, "idle.png")):
                            img_rel = f"./public/pet-packs/{entry}/assets/idle.png"
                        elif os.path.exists(os.path.join(assets_dir, "thumbnail.png")):
                            img_rel = f"./public/pet-packs/{entry}/assets/thumbnail.png"
                            
                        # check sculpted 3D GLB
                        sculpted_glb = os.path.join(SCULPT_DIR, f"{entry}.glb")
                        has_3d = os.path.exists(sculpted_glb)
                        glb_url = f"./public/models/sculpted/{entry}.glb" if has_3d else None
                        if has_3d:
                            sculpted_count += 1

                        # detect category
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
                            "avatar_type": "pet_pack_3d" if has_3d else "pet_pack",
                            "format": "3D Sculpted Mesh (GLB)" if has_3d else "2D Sprite",
                            "category": cat,
                            "batch": "bundled_petpack",
                            "description": pdata.get("description", ""),
                            "author": pdata.get("creator", "Miyora / Community"),
                            "license": pdata.get("license", "CC-BY-4.0"),
                            "review_status": "bundled_pack",
                            "review_badge": "✨ 3D Sculpted Pack" if has_3d else "📦 2D Pack",
                            "review_note": "Sculpted into 3D watertight GLB relief via agentpet-avatar." if has_3d else "2D companion sprite.",
                            "image_url": img_rel,
                            "has_3d_model": has_3d,
                            "url": glb_url,
                            "model_source": "sculpted" if has_3d else None,
                            "preview_type": "image",
                            "tags": ["pet-pack", "3d" if has_3d else "2d", cat, entry]
                        })
                        count_packs += 1
                    except Exception as e:
                        print(f"Error parsing {pj_path}: {e}")
        print(f"Loaded {count_packs} pet-packs ({sculpted_count} successfully converted to 3D GLB!)")

    # 3. Load Catalog Characters (214 items)
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
                    "model_source": "procedural" if three_d else None,
                    "preview_type": "procedural",
                    "tags": ["catalog", cat, c.get("body", "")] + c.get("tags", [])
                })
        print(f"Loaded {len(chars)} catalog characters")

    print(f"\nTotal Unified Avatars: {len(all_avatars)}")

    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(all_avatars, f, indent=2)

    print(f"Saved master avatar database to {OUTPUT_PATH}")

if __name__ == "__main__":
    main()
