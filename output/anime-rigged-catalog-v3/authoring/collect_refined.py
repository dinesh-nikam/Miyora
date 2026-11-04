"""Audit complete Blender exports before publishing the refined collection registry."""
import argparse
from collections import Counter
import hashlib
import json
import logging
from pathlib import Path
import re

LOG = logging.getLogger("miyora.collect-refined")
REVISION = "refined-catalog-v3.3"


def collect(root):
    catalog = json.loads((root / "catalog-source.json").read_text(encoding="utf-8"))["characters"]
    if not catalog or len(catalog) > 512 or len({s["id"] for s in catalog}) != len(catalog):
        raise ValueError("Catalog identities or capacity are invalid")
    entries = []
    for spec in catalog:
        slug = spec["id"].split(".")[-1]
        if not re.fullmatch(r"[a-z0-9-]+", slug):
            raise ValueError("Unsafe catalog identity")
        folder = root / slug
        production = json.loads((folder / "production.json").read_text())
        report = json.loads((folder / "model.validation.json").read_text())
        model, source = folder / "model.vrm", folder / "source.blend"
        if production.get("revision") != REVISION or production.get("status") != "structurally-validated":
            raise ValueError("Incorrect or incomplete model revision: " + spec["id"])
        digest = hashlib.sha256(model.read_bytes()).hexdigest()
        if digest != report.get("sha256") or report.get("structuralValidation") != "passed":
            raise ValueError("Model integrity check failed: " + spec["id"])
        if set(report.get("expressionNames", [])) != {"blink", "happy", "sad", "aa", "surprised"}:
            raise ValueError("Incomplete facial expressions: " + spec["id"])
        with source.open("rb") as stream:
            if stream.read(7) != b"BLENDER":
                raise ValueError("Editable Blender source is missing or invalid: " + spec["id"])
        entries.append({"id": spec["id"], "name": spec["name"], "body": spec["body"], "model": slug + "/model.vrm", "source": slug + "/source.blend", "sha256": digest, "triangles": report["triangleCount"], "animations": report["animationNames"], "revision": REVISION})
    status = {"schemaVersion": 1, "requested": len(catalog), "structurallyValidated": len(entries), "missing": [], "bodyCounts": dict(Counter(e["body"] for e in entries)), "runtimeValidated": 0, "artReview": "pending-runtime-and-review-images", "style": "full-anime", "refinementRevision": REVISION, "paidServicesUsed": False}
    (root / "collection.json").write_text(json.dumps({"schemaVersion": 1, "style": "full-anime", "revision": REVISION, "characters": entries}, indent=2), encoding="utf-8")
    (root / "production-status.json").write_text(json.dumps(status, indent=2), encoding="utf-8")
    LOG.info("COLLECTION_AUDITED %s", json.dumps(status))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(level=logging.DEBUG if args.verbose else logging.INFO, format="%(levelname)s %(message)s")
    collect(args.output.resolve())


if __name__ == "__main__":
    try:
        main()
    except Exception:
        LOG.exception("Collection audit failed")
        raise
