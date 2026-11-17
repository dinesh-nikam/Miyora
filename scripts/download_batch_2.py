import urllib.request
import os
import json
from analyze_model import parse_glb_or_vrm

BATCH_2_CANDIDATES = [
    {
        "id": "model.avatar_sample_a",
        "name": "AvatarSample A (Annie)",
        "title": "Flagship Anime Heroine (Twin-Tails)",
        "filename": "AvatarSample_A.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/stable/AvatarSample_A.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "icon": "🎀",
        "author": "Pixiv Inc.",
        "license": "VRoid Studio Conditions (Open Distribution / Alteration)",
        "description": "Pixiv VRoid Studio's flagship anime heroine featuring signature twin-tails, cel-shaded anime eyes, stylish school uniform, full humanoid skeleton, and dynamic hair spring physics."
    },
    {
        "id": "model.avatar_sample_b",
        "name": "AvatarSample B",
        "title": "Pixiv Anime Student (ChatVRM Official)",
        "filename": "AvatarSample_B.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/stable/AvatarSample_B.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "icon": "🎓",
        "author": "Pixiv Inc.",
        "license": "VRoid Studio / MIT (Pixiv ChatVRM)",
        "description": "Featured in Pixiv's official ChatVRM application. Polished anime student with styled modern hair, blazer uniform, full facial blendshapes, and standard humanoid bones."
    },
    {
        "id": "model.vita",
        "name": "Vita",
        "title": "Magical Fantasy Anime Girl",
        "filename": "Vita.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/beta/Vita.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "icon": "🪄",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Vibrant magical anime girl with fantasy-themed accessories, expressive anime eyes, and full VRM humanoid bones under public domain dedication."
    },
    {
        "id": "model.vivi",
        "name": "Vivi",
        "title": "High-Fashion Anime Heroine",
        "filename": "Vivi.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/beta/Vivi.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "icon": "👗",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Stylish anime heroine with voluminous layered hairstyle, modern streetwear/couture fusion, and high-fidelity skeletal rig under CC0."
    },
    {
        "id": "model.sakurada_fumiriya",
        "name": "Sakurada Fumiriya",
        "title": "Traditional Japanese Anime Maiden",
        "filename": "Sakurada_Fumiriya.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/beta/Sakurada_Fumiriya.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "icon": "🎋",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Graceful Japanese anime maiden with flowing dark hair and traditional aesthetic, echoing shrine maiden/miko poise with full humanoid bone hierarchy."
    },
    {
        "id": "model.celestia",
        "name": "Celestia",
        "title": "The Cosmic Librarian (Star Goddess)",
        "filename": "celestia.glb",
        "url": "https://raw.githubusercontent.com/M3-org/avatars/main/celestia/celestia.glb",
        "category": "deity",
        "batch": "batch_2",
        "type": "glb",
        "icon": "🌌",
        "author": "M3-org (Metaverse Makers)",
        "license": "CC0 / Open Metaverse",
        "description": "Enigmatic cosmic deity with star-field hair and divine cosmic robes. Fully rigged humanoid skeleton with CC_Base bone mapping capable of natural IK."
    },
    {
        "id": "model.venus",
        "name": "Venus",
        "title": "The Glam Celestial Oracle (Goddess)",
        "filename": "venus.glb",
        "url": "https://raw.githubusercontent.com/M3-org/avatars/main/venus/venus.glb",
        "category": "deity",
        "batch": "batch_2",
        "type": "glb",
        "icon": "👑",
        "author": "M3-org (Metaverse Makers)",
        "license": "CC0 / Open Metaverse",
        "description": "Dazzling celestial goddess with solar-golden attire, elegant divine posture, and complete humanoid bone hierarchy for expressive posing."
    },
    {
        "id": "model.paladin",
        "name": "Paladin",
        "title": "The Security Sentinel (Holy Knight)",
        "filename": "paladin.glb",
        "url": "https://raw.githubusercontent.com/M3-org/avatars/main/paladin/paladin.glb",
        "category": "deity",
        "batch": "batch_2",
        "type": "glb",
        "icon": "⚔️",
        "author": "M3-org (Metaverse Makers)",
        "license": "CC0 / Open Metaverse",
        "description": "Majestic holy paladin / divine guardian clad in ceremonial knightly armor with heroic proportions and full combat-ready humanoid skeleton."
    },
    {
        "id": "model.monero_chan",
        "name": "Monero-chan",
        "title": "The Privacy Guardian (Anime Mascot)",
        "filename": "monero-chan.vrm",
        "url": "https://raw.githubusercontent.com/M3-org/avatars/main/monero-chan/monero-chan.vrm",
        "category": "anime_character",
        "batch": "batch_2",
        "type": "vrm",
        "icon": "🦊",
        "author": "M3-org / Community",
        "license": "CC0 / Open Community",
        "description": "Kawaii anime guardian with fox-adjacent charm, twin-tails, authentic anime facial expressions, pleated skirt, and full VRM humanoid bones."
    },
    {
        "id": "model.aura",
        "name": "Aura",
        "title": "Celestial Spirit Entity",
        "filename": "aura.glb",
        "url": "https://raw.githubusercontent.com/M3-org/avatars/main/aura/aura.glb",
        "category": "deity",
        "batch": "batch_2",
        "type": "glb",
        "icon": "✨",
        "author": "M3-org (Metaverse Makers)",
        "license": "CC0 / Open Metaverse",
        "description": "Ethereal spirit entity with radiant celestial gradients, stylized mystical silhouette, and clean humanoid skeletal rig."
    }
]

dest_dir = os.path.join(os.path.dirname(__file__), "..", "showcase", "public", "models")
os.makedirs(dest_dir, exist_ok=True)

results = []

for m in BATCH_2_CANDIDATES:
    target_path = os.path.join(dest_dir, m["filename"])
    print(f"Downloading {m['filename']} from {m['url']}...")
    try:
        req = urllib.request.Request(m["url"], headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req) as resp, open(target_path, 'wb') as out_file:
            data = resp.read()
            out_file.write(data)
        file_size = os.path.getsize(target_path)
        print(f" -> Saved {m['filename']} ({file_size} bytes)")
        
        analysis = parse_glb_or_vrm(target_path)
        m["file_size_bytes"] = file_size
        m["bones"] = analysis.get("bone_count")
        m["polygons"] = analysis.get("polygon_count")
        m["ik_compatible"] = analysis.get("ik_compatible")
        m["url"] = f"./public/models/{m['filename']}"
        results.append(m)
        print(f"    Analysis: bones={analysis.get('bone_count')}, polys={analysis.get('polygon_count')}, ik={analysis.get('ik_compatible')}")
    except Exception as e:
        print(f" !! Failed {m['filename']}: {e}")

output_json = os.path.join(dest_dir, "batch_2_manifest.json")
with open(output_json, 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2)

print(f"\nWrote Batch 2 manifest with {len(results)} models to {output_json}")
