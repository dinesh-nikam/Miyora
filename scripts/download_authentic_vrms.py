import urllib.request
import os
import ssl

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

MODELS_DIR = os.path.join("showcase", "public", "models")
os.makedirs(MODELS_DIR, exist_ok=True)

VRM_DOWNLOADS = [
    {
        "id": "model.avatar_sample_c",
        "name": "AvatarSample C",
        "title": "Pixiv Anime Student (Chic Hero)",
        "filename": "AvatarSample_C.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/stable/AvatarSample_C.vrm",
        "author": "Pixiv Inc.",
        "license": "VRoid Studio Conditions (Open Distribution / Alteration)",
        "description": "Official Pixiv VRoid anime character with modern hairstyle, tailored blazer, expressive anime eyes, articulated fingers, and complete humanoid skeleton for natural IK movement."
    },
    {
        "id": "model.sendagaya_shibu",
        "name": "Sendagaya Shibu",
        "title": "Anime Heroine (Street Casual)",
        "filename": "Sendagaya_Shibu.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/beta/Sendagaya_Shibu.vrm",
        "author": "Pixiv Inc.",
        "license": "CC0 1.0 Universal",
        "description": "Sister character to Shino. High-fidelity anime heroine with casual streetwear, flowing hair physics, full skeletal articulation, and complete humanoid IK hierarchy."
    },
    {
        "id": "model.fem_vroid",
        "name": "Sakura (Fem Vroid Base)",
        "title": "Authentic Anime Female Companion",
        "filename": "fem_vroid.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/fem_vroid.vrm",
        "author": "Pixiv Inc. / Community",
        "license": "CC0 1.0 Universal",
        "description": "Pure anime heroine base with delicate facial proportions, articulated 3D hands, natural leg silhouette, and fluid humanoid IK motion."
    },
    {
        "id": "model.masc_vroid",
        "name": "Ren (Masc Vroid Base)",
        "title": "Authentic Anime Male Companion",
        "filename": "masc_vroid.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/vroid/masc_vroid.vrm",
        "author": "Pixiv Inc. / Community",
        "license": "CC0 1.0 Universal",
        "description": "Anime male protagonist base with clean athletic build, articulated fingers, rigged feet, and standard humanoid skeleton."
    },
    {
        "id": "model.avatar_orion",
        "name": "Orion (Celestial Guardian)",
        "title": "Mythic Astral Sentinel",
        "filename": "Avatar_Orion.vrm",
        "url": "https://raw.githubusercontent.com/madjin/vrm-samples/master/Avatar_Orion.vrm",
        "author": "Madjin / Community",
        "license": "CC0 1.0 Universal",
        "description": "Astral celestial guardian with futuristic mythic attire, full humanoid bone hierarchy, and articulated 3D limbs."
    }
]

def main():
    print("Downloading authentic humanoid VRM models matching confirmed benchmark...")
    for item in VRM_DOWNLOADS:
        dest = os.path.join(MODELS_DIR, item["filename"])
        if os.path.exists(dest) and os.path.getsize(dest) > 1000000:
            print(f"[SKIP] Already exists: {dest} ({os.path.getsize(dest)} bytes)")
            continue

        print(f"[DOWNLOADING] {item['name']} from {item['url']}...")
        req = urllib.request.Request(item["url"], headers={"User-Agent": "Miyora-Avatar-Downloader"})
        try:
            with urllib.request.urlopen(req, context=ctx, timeout=60) as resp:
                data = resp.read()
                with open(dest, "wb") as f:
                    f.write(data)
                print(f"[SUCCESS] Saved {dest} ({len(data)} bytes)")
        except Exception as e:
            print(f"[ERROR] Failed {item['name']}: {e}")

if __name__ == "__main__":
    main()
