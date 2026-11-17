import os
import subprocess
import json

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PET_PACKS_DIR = os.path.join(ROOT, "pet-packs")
SCULPT_DIR = os.path.join(ROOT, "showcase", "public", "models", "sculpted")
SCULPT_EXE = os.path.join(ROOT, "target", "debug", "examples", "sculpt.exe")

os.makedirs(SCULPT_DIR, exist_ok=True)

def main():
    if not os.path.exists(SCULPT_EXE):
        print(f"Error: {SCULPT_EXE} not found.")
        return

    packs = sorted(os.listdir(PET_PACKS_DIR))
    print(f"Starting batch 2D->3D sculpting for {len(packs)} pet-packs...")
    
    success = 0
    failed = 0

    for slug in packs:
        p_path = os.path.join(PET_PACKS_DIR, slug)
        if not os.path.isdir(p_path):
            continue
        
        # look for idle.png
        img_path = os.path.join(p_path, "assets", "idle.png")
        if not os.path.exists(img_path):
            img_path = os.path.join(p_path, "assets", "thumbnail.png")
        if not os.path.exists(img_path):
            continue

        out_glb = os.path.join(SCULPT_DIR, f"{slug}.glb")
        name = slug.replace("-", " ").title()

        cmd = [SCULPT_EXE, img_path, out_glb, name]
        try:
            res = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
            if res.returncode == 0:
                print(f"[OK] 3D sculpted: {slug} -> {out_glb} ({os.path.getsize(out_glb)} bytes)")
                success += 1
            else:
                print(f"[ERR] Failed {slug}: {res.stderr.strip()}")
                failed += 1
        except Exception as e:
            print(f"[EXC] {slug}: {e}")
            failed += 1

    print(f"\nBatch sculpting complete! Success: {success}, Failed: {failed}")

if __name__ == "__main__":
    main()
