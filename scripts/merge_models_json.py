import json
import os

dest_dir = os.path.join(os.path.dirname(__file__), "..", "showcase", "public", "models")

with open(os.path.join(dest_dir, "batch_2_manifest.json"), "r", encoding="utf-8") as f:
    batch_2 = json.load(f)

# Batch 1 models with explicit rejection insights as requested by LO & coordinator
batch_1 = [
    {
        "id": "model.xbot",
        "name": "Xbot Humanoid",
        "title": "Standard Humanoid Base Rig",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "glb",
        "url": "./public/models/Xbot.glb",
        "icon": "🤖",
        "author": "Three.js / Mixamo",
        "license": "CC0 / Open Source",
        "bones": 67,
        "polygons": 49112,
        "ik_compatible": True,
        "rejection_reason": "Grey developer dummy / No aesthetic appeal or anime charm",
        "description": "Standard rigged humanoid base mesh with complete bone hierarchy. (Batch 1: Technical rig reference)."
    },
    {
        "id": "model.soldier",
        "name": "Humanoid Soldier",
        "title": "Rigged Humanoid Soldier",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "glb",
        "url": "./public/models/Soldier.glb",
        "icon": "🪖",
        "author": "Three.js / Mixamo",
        "license": "CC0 / Open Source",
        "bones": 49,
        "polygons": 11376,
        "ik_compatible": True,
        "rejection_reason": "Military grunt in camo / Lacks anime charm or divine elegance",
        "description": "Realistic anatomical humanoid proportions with humanoid limb hierarchy and animations. (Batch 1: Animation reference)."
    },
    {
        "id": "model.robot_expressive",
        "name": "Robot Expressive",
        "title": "Stylized Humanoid Robot",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "glb",
        "url": "./public/models/RobotExpressive.glb",
        "icon": "🦾",
        "author": "Three.js / Tom Whittock",
        "license": "CC0 / Open Source",
        "bones": 43,
        "polygons": 3237,
        "ik_compatible": True,
        "rejection_reason": "Toy robot / Not a human, anime, or deity character",
        "description": "Stylized robot with facial morphs and skeletal joints. (Batch 1: Morph target reference)."
    },
    {
        "id": "model.readyplayer_me",
        "name": "Ready Player Me Avatar",
        "title": "Ready Player Me Full-Body Humanoid",
        "category": "humanoid_base",
        "batch": "batch_1",
        "type": "glb",
        "url": "./public/models/readyplayer.me.glb",
        "icon": "🧍",
        "author": "Ready Player Me",
        "license": "CC-BY 4.0 / Open",
        "bones": 67,
        "polygons": 15817,
        "ik_compatible": True,
        "rejection_reason": "Western semi-realistic uncanny valley / Plain street clothes",
        "description": "Full-body avatar with Mixamo-compliant humanoid skeletal hierarchy. (Batch 1: Retargeting reference)."
    },
    {
        "id": "model.kira",
        "name": "Kira Anime Humanoid",
        "title": "Low-Poly Anime Humanoid (Three.js IK Rig)",
        "category": "anime_character",
        "batch": "batch_1",
        "type": "glb",
        "url": "./public/models/kira.glb",
        "icon": "🥋",
        "author": "Aki / Antoine Bernier",
        "license": "CC0 1.0 Universal",
        "bones": 23,
        "polygons": 51601,
        "ik_compatible": True,
        "rejection_reason": "Low-poly stylized base / Kept for IK reference only",
        "description": "Official Three.js Inverse Kinematics reference asset (webgl_animation_skinning_ik) with native CCDIKSolver armatures."
    },
    {
        "id": "model.seed_san",
        "name": "Seed-san",
        "title": "VRM 1.0 Consortium Reference Humanoid",
        "category": "anime_character",
        "batch": "batch_1",
        "type": "vrm",
        "url": "./public/models/Seed-san.vrm",
        "icon": "🌱",
        "author": "VirtualCast, Inc.",
        "license": "CC0 1.0 Universal",
        "bones": 128,
        "polygons": 45058,
        "ik_compatible": True,
        "rejection_reason": "Dated 2017 developer test mesh / Flat shading and test jumpsuit",
        "description": "Official VRM Consortium standard reference model demonstrating humanoid bones and secondary motion."
    },
    {
        "id": "model.alicia_solid",
        "name": "Alicia Solid",
        "title": "Canonical Anime Avatar (NicoNico-chan)",
        "category": "anime_character",
        "batch": "batch_1",
        "type": "vrm",
        "url": "./public/models/AliciaSolid.vrm",
        "icon": "🎀",
        "author": "DWANGO Co., Ltd.",
        "license": "NicoNico Character License (CC-BY Compatible)",
        "bones": 128,
        "polygons": 31798,
        "ik_compatible": True,
        "rejection_reason": "Dated 2017 anime mesh / Basic shading compared to modern VRoid",
        "description": "The historical benchmark anime avatar for VRM with humanoid bones, blendshapes, and twin-tail physics."
    },
    {
        "id": "model.sendagaya_shino",
        "name": "Sendagaya Shino",
        "title": "Anime Schoolgirl Avatar",
        "category": "anime_character",
        "batch": "batch_1",
        "type": "vrm",
        "url": "./public/models/Sendagaya_Shino.vrm",
        "icon": "🌸",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "bones": 148,
        "polygons": 31211,
        "ik_compatible": True,
        "description": "Official Pixiv VRoid Studio anime avatar released under CC0 public domain with high-fidelity skeleton."
    },
    {
        "id": "model.victoria_rubin",
        "name": "Victoria Rubin",
        "title": "High-Detail Anime Heroine",
        "category": "anime_character",
        "batch": "batch_1",
        "type": "vrm",
        "url": "./public/models/Victoria_Rubin.vrm",
        "icon": "✨",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "bones": 162,
        "polygons": 28928,
        "ik_compatible": True,
        "description": "Anime female avatar with full humanoid skeletal rig, expressive facial blendshapes, and hair physics."
    },
    {
        "id": "model.darkness_shibu",
        "name": "Darkness Shibu",
        "title": "Dark Fantasy / Deity Character",
        "category": "deity",
        "batch": "batch_1",
        "type": "vrm",
        "url": "./public/models/Darkness_Shibu.vrm",
        "icon": "🔮",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "bones": 149,
        "polygons": 33758,
        "ik_compatible": True,
        "description": "Dark gothic / fantasy deity aesthetic character with cape physics and complete humanoid skeleton."
    },
    {
        "id": "model.angel_baby",
        "name": "Angel Baby",
        "title": "Celestial Winged Guardian / Angelic Humanoid",
        "category": "deity",
        "batch": "batch_1",
        "type": "vrm",
        "url": "./public/models/AngelBaby.vrm",
        "icon": "🪽",
        "author": "frangry55",
        "license": "Open Source / Public Domain",
        "bones": 29,
        "polygons": 21048,
        "ik_compatible": True,
        "rejection_reason": "Chibi infant proportion / Lacks majestic celestial elegance",
        "description": "Winged celestial humanoid with halo features. (Batch 1: Chibi proportion reference)."
    }
]

# Combined list: Batch 2 first, followed by Batch 1
combined = batch_2 + batch_1

out_path = os.path.join(dest_dir, "models.json")
with open(out_path, "w", encoding="utf-8") as f:
    json.dump(combined, f, indent=2)

print(f"Successfully wrote {len(combined)} models ({len(batch_2)} Batch 2, {len(batch_1)} Batch 1) to {out_path}")
