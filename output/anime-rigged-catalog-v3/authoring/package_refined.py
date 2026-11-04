"""Create and CRC-verify the private, complete refined-model archive."""
import argparse
import hashlib
import json
import logging
from pathlib import Path
import re
from zipfile import ZipFile, ZIP_DEFLATED

LOG = logging.getLogger("miyora.package-refined")
ROOT_FILES = {"README.md", "collection.json", "catalog-source.json", "production-status.json",
              "runtime-validation.json", "native-validation.json", "authoring-hashes.json",
              "index.html", "viewer.js", "serve_collection.py", "View Collection.cmd",
              "worker-0.json", "worker-1.json"}
MODEL_FILES = {"source.blend", "model.glb", "model.vrm", "character.json", "production.json",
               "model.validation.json", "review.png", "runtime-working.png", "runtime-success.png",
               "portrait-happy.png", "back-review.png"}


def build(root, destination):
    status = json.loads((root / "production-status.json").read_text())
    registry = json.loads((root / "collection.json").read_text())
    entries = registry["characters"]
    expected = len(entries)
    if expected != 213 or any(status.get(k) != expected for k in ("requested", "structurallyValidated", "runtimeValidated", "nativeValidated", "nativeImported")):
        raise ValueError("The requested model set is not fully verified and imported")
    slugs = {entry["model"].split("/")[0] for entry in entries}
    staging = destination.with_suffix(".zip.staging")
    if destination.exists() or staging.exists():
        raise ValueError("Refusing to overwrite an existing archive or interrupted archive")
    with ZipFile(staging, "x", compression=ZIP_DEFLATED, compresslevel=6, allowZip64=True) as archive:
        for file in sorted(root.rglob("*")):
            if not file.is_file() or file.is_symlink():
                continue
            relative = file.relative_to(root)
            parts = relative.parts
            allowed = (len(parts) == 1 and parts[0] in ROOT_FILES)
            allowed |= len(parts) == 2 and parts[0] in slugs and parts[1] in MODEL_FILES
            allowed |= len(parts) == 2 and parts[0] == "authoring" and file.suffix in (".py", ".cjs")
            allowed |= len(parts) == 2 and parts[0] == "review" and bool(re.fullmatch(r"index\.html|sheet-[0-9]+\.(html|png)", parts[1]))
            if allowed:
                archive.write(file, relative.as_posix())
    with ZipFile(staging) as archive:
        names = archive.namelist()
        for suffix in ("/source.blend", "/model.vrm", "/model.glb", "/review.png"):
            if sum(name.endswith(suffix) for name in names) != expected:
                raise ValueError("The archive is missing model artifacts: " + suffix)
        if any("app-import-index" in name or "task-state" in name or "__pycache__" in name for name in names):
            raise ValueError("Local-only metadata entered the archive")
        bad_file = archive.testzip()
        if bad_file is not None:
            raise ValueError("Archive CRC check failed: " + bad_file)
    staging.rename(destination)
    digest = hashlib.sha256()
    with destination.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    checksum = digest.hexdigest()
    destination.with_suffix(".zip.sha256").write_text(checksum + "  " + destination.name + "\n")
    receipt = {"schemaVersion": 1, "status": "passed", "archive": destination.name,
               "bytes": destination.stat().st_size, "sha256": checksum, "crcVerified": True,
               "models": expected, "blenderSources": expected, "vrmFiles": expected,
               "glbFiles": expected, "reviewImages": expected, "localImportIdsExcluded": True}
    destination.with_suffix(".verification.json").write_text(json.dumps(receipt, indent=2))
    LOG.info("ARCHIVE_VERIFIED %s", json.dumps(receipt))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collection", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    logging.basicConfig(level=logging.DEBUG if args.verbose else logging.INFO, format="%(levelname)s %(message)s")
    root = args.collection.resolve()
    build(root, (args.output or root.parent / "miyora-213-rigged-models-refined.zip").resolve())


if __name__ == "__main__":
    try:
        main()
    except Exception:
        LOG.exception("Archive creation failed")
        raise
