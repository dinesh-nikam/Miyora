import urllib.request
import os
import json
from analyze_model import parse_glb_or_vrm

MODELS_TO_DOWNLOAD = [
    {
        "id": "xbot",
        "filename": "Xbot.glb",
        "url": "https://raw.githubusercontent.com/mrdoob/three.js/dev/examples/models/gltf/Xbot.glb",
        "category": "Standard Humanoid Base Mesh",
        "source": "Three.js Examples / Mixamo",
        "license_override": "CC0 / MIT",
        "author_override": "Mixamo / Three.js"
    },
    {
        "id": "kira",
        "filename": "kira.glb",
        "url": "https://raw.githubusercontent.com/mrdoob/three.js/dev/examples/models/gltf/kira.glb",
        "category": "Anime Humanoid (IK Benchmark)",
        "source": "Three.js Examples (webgl_animation_skinning_ik)",
        "license_override": "CC0",
        "author_override": "Aki / Antoine Bernier"
    },
    {
        "id": "soldier",
        "filename": "Soldier.glb",
        "url": "https://raw.githubusercontent.com/mrdoob/three.js/dev/examples/models/gltf/Soldier.glb",
        "category": "Standard Humanoid with Skeletal Animations",
        "source": "Three.js Examples / Mixamo",
        "license_override": "CC0 / MIT",
        "author_override": "Mixamo / Three.js"
    },
    {
        "id": "robot_expressive",
        "filename": "RobotExpressive.glb",
        "url": "https://raw.githubusercontent.com/mrdoob/three.js/dev/examples/models/gltf/RobotExpressive/RobotExpressive.glb",
        "category": "Stylized Humanoid Robot (Morph Targets + IK)",
        "source": "Three.js Examples",
        "license_override": "CC0 / MIT",
        "author_override": "Three.js / Tom Whittock"
    },
    {
        "id": "readyplayer_me",
        "filename": "readyplayer.me.glb",
        "url": "https://raw.githubusercontent.com/mrdoob/three.js/dev/examples/models/gltf/readyplayer.me.glb",
        "category": "Ready Player Me Humanoid Avatar",
        "source": "Three.js Examples / Ready Player Me",
        "license_override": "CC-BY 4.0 / Open",
        "author_override": "Ready Player Me"
    },
    {
        "id": "seed_san",
        "filename": "Seed-san.vrm",
        "url": "https://raw.githubusercontent.com/vrm-c/vrm-specification/master/samples/Seed-san/vrm/Seed-san.vrm",
        "category": "VRM 1.0 Consortium Reference Humanoid Avatar",
        "source": "vrm-c/vrm-specification",
        "license_override": "CC0 (Public Domain)",
        "author_override": "VirtualCast, Inc."
    },
    {
        "id": "alicia_solid",
        "filename": "AliciaSolid.vrm",
        "url": "https://raw.githubusercontent.com/vrm-c/UniVRM/master/Tests/Models/Alicia_vrm-0.51/AliciaSolid_vrm-0.51.vrm",
        "category": "Canonical Anime Avatar (VRM Benchmark)",
        "source": "DWANGO / vrm-c/UniVRM",
        "license_override": "NicoNico Character License (CC-BY Compatible)",
        "author_override": "DWANGO Co., Ltd."
    },
    {
        "id": "sendagaya_shino",
        "filename": "Sendagaya_Shino.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/beta/Sendagaya_Shino.vrm",
        "category": "Anime Schoolgirl Avatar (Full VRM Bones & Physics)",
        "source": "Pixiv VRoid Studio / madjin/vrm-samples",
        "license_override": "CC0 1.0 Universal",
        "author_override": "Pixiv Inc."
    },
    {
        "id": "darkness_shibu",
        "filename": "Darkness_Shibu.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/beta/Darkness_Shibu.vrm",
        "category": "Dark Fantasy / Deity Style Anime Character",
        "source": "Pixiv VRoid Studio / madjin/vrm-samples",
        "license_override": "CC0 1.0 Universal",
        "author_override": "Pixiv Inc."
    },
    {
        "id": "victoria_rubin",
        "filename": "Victoria_Rubin.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/beta/Victoria_Rubin.vrm",
        "category": "High-Detail Anime Heroine (Full VRM Bones & Physics)",
        "source": "Pixiv VRoid Studio / madjin/vrm-samples",
        "license_override": "CC0 1.0 Universal",
        "author_override": "Pixiv Inc."
    },
    {
        "id": "angel_baby",
        "filename": "AngelBaby.vrm",
        "url": "https://raw.githubusercontent.com/frangry55/AngelElfBbyVRM/main/AngelBaby.vrm",
        "category": "Celestial Winged / Angelic Humanoid",
        "source": "frangry55/AngelElfBbyVRM",
        "license_override": "Open Source / Public Domain",
        "author_override": "frangry55"
    }
]

dest_dir = os.path.join(os.path.dirname(__file__), "..", "showcase", "public", "models")
os.makedirs(dest_dir, exist_ok=True)

results = []

for m in MODELS_TO_DOWNLOAD:
    target_path = os.path.join(dest_dir, m["filename"])
    print(f"Downloading {m['filename']} from {m['url']}...")
    try:
        req = urllib.request.Request(m["url"], headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req) as response, open(target_path, 'wb') as out_file:
            data = response.read()
            out_file.write(data)
        file_size = os.path.getsize(target_path)
        print(f" -> Saved {m['filename']} ({file_size} bytes)")
        
        # Analyze
        analysis = parse_glb_or_vrm(target_path)
        item_doc = {
            "id": m["id"],
            "title": analysis.get("title") or m["filename"],
            "filename": m["filename"],
            "staged_path": f"showcase/public/models/{m['filename']}",
            "file_size_bytes": file_size,
            "category": m["category"],
            "source_link": m["url"],
            "repository": m["source"],
            "license": m["license_override"] if m["license_override"] != "Unknown" else analysis.get("license"),
            "author": m["author_override"] if m["author_override"] != "Unknown" else analysis.get("author"),
            "bone_count": analysis.get("bone_count"),
            "polygon_count": analysis.get("polygon_count"),
            "ik_compatible": analysis.get("ik_compatible"),
            "is_vrm": analysis.get("is_vrm"),
            "animations_count": analysis.get("animations_count")
        }
        results.append(item_doc)
        print(f"    Analysis: bones={analysis.get('bone_count')}, polys={analysis.get('polygon_count')}, ik={analysis.get('ik_compatible')}")
    except Exception as e:
        print(f" !! Failed {m['filename']}: {e}")

output_json = os.path.join(dest_dir, "models_manifest.json")
with open(output_json, 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2)

print(f"\nWrote manifest with {len(results)} models to {output_json}")
