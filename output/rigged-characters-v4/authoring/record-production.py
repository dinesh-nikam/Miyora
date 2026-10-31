"""Record the actual model production state without inventing finished assets."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    output = root / "output/rigged-characters-v4"
    catalog_path = root / "crates/avatar/catalog/characters.json"
    specs = json.loads(catalog_path.read_text(encoding="utf-8"))["characters"]
    runtime_path = output / "nezuko/runtime-validation.json"
    runtime = json.loads(runtime_path.read_text(encoding="utf-8")) if runtime_path.exists() else None
    if runtime and runtime.get("status") == "passed":
        report_path = output / "nezuko/nezuko.validation.json"
        report = json.loads(report_path.read_text(encoding="utf-8"))
        report["runtimeValidation"] = "passed-in-isolated-chrome-three-vrm"
        report["visualReview"] = "prototype-needs-art-direction-and-refinement"
        report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    entries = []
    for spec in specs:
        entries.append({"id": spec["id"], "name": spec["name"], "body": spec["body"], "status": "prototype-needs-refinement" if spec["id"] == "miyora.nezuko" else "not-started", "model": "nezuko/nezuko.vrm" if spec["id"] == "miyora.nezuko" else None})
    manifest = {"schemaVersion": 1, "requestedCharacters": len(specs), "finalCharacters": 0, "prototypeCharacters": 1, "notStartedCharacters": len(specs)-1, "localOnly": True, "paidGenerationUsed": False, "proportionsDecision": "awaiting-user-full-anime-or-detailed-chibi", "catalogSha256": hashlib.sha256(catalog_path.read_bytes()).hexdigest(), "bodyCounts": dict(Counter(s["body"] for s in specs)), "characters": entries}
    (output / "catalog-status.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    tools = output / "authoring"
    tools.mkdir(exist_ok=True)
    for name in ("author_characters.py", "package_vrm.py", "preview-entry.js", "build-preview.cjs", "verify-preview.cjs", "record-production.py"):
        shutil.copy2(root / "target/model-tools" / name, tools / name)
    print(json.dumps({key: manifest[key] for key in ("requestedCharacters", "finalCharacters", "prototypeCharacters", "notStartedCharacters", "bodyCounts")}))


if __name__ == "__main__":
    main()
