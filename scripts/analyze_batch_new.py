import json
import os
import subprocess

ANALYZER = os.path.join("scripts", "analyze_model.py")
MODELS = [
    "AvatarSample_C.vrm",
    "Sendagaya_Shibu.vrm",
    "fem_vroid.vrm",
    "masc_vroid.vrm",
    "Avatar_Orion.vrm"
]

results = {}
for m in MODELS:
    p = os.path.join("showcase", "public", "models", m)
    if os.path.exists(p):
        res = subprocess.run(["python", ANALYZER, p], capture_output=True, text=True)
        try:
            results[m] = json.loads(res.stdout)
        except Exception:
            results[m] = {"error": res.stderr}

print(json.dumps(results, indent=2))
