"""Validate every pack under pet-packs/ against the v1 PetPack contract (PET_PACK_SPEC.md).

Read-only. Exits 1 when any pack has an error. Warnings do not fail the run.
Requires Pillow (already in requirements.txt).
"""

import hashlib
import json
import os
import re
import sys

from PIL import Image

PACKS_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "pet-packs")
REQUIRED_STRINGS = ("id", "name", "version", "description", "creator", "license", "source", "attribution")
REQUIRED_STATES = ("idle", "working", "thinking", "waiting", "success", "error", "level_up", "sleeping", "thumbnail")
FOLDER_RE = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")
SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+$")
ID_RE = re.compile(r"^[a-z0-9][a-z0-9.-]*$")


def check_pack(folder):
    errors, warnings = [], []
    name = os.path.basename(folder)
    if not FOLDER_RE.match(name):
        warnings.append(f"folder name '{name}' is not lowercase kebab-case")

    pj = os.path.join(folder, "pet.json")
    if not os.path.isfile(pj):
        return ["missing pet.json"], warnings
    try:
        with open(pj, encoding="utf-8") as fh:
            m = json.load(fh)
    except (OSError, json.JSONDecodeError) as exc:
        return [f"pet.json invalid JSON: {exc}"], warnings

    if m.get("schemaVersion") != 1:
        errors.append(f"schemaVersion must be 1, got {m.get('schemaVersion')!r}")
    for key in REQUIRED_STRINGS:
        value = m.get(key)
        if not isinstance(value, str) or not value.strip():
            errors.append(f"missing required field '{key}'")
    if isinstance(m.get("id"), str) and not ID_RE.match(m["id"]):
        errors.append(f"id '{m['id']}' has invalid characters")
    if isinstance(m.get("version"), str) and not SEMVER_RE.match(m["version"]):
        errors.append(f"version '{m['version']}' is not semver")
    for flag in ("commercialUse", "redistribution", "derivativeWorks"):
        if not isinstance(m.get(flag), bool):
            errors.append(f"'{flag}' must be a boolean")

    assets = m.get("assets")
    if not isinstance(assets, dict) or not assets:
        errors.append("'assets' missing or empty")
        assets = {}
    for state in REQUIRED_STATES:
        if state not in assets:
            warnings.append(f"no asset for state '{state}'")

    hashes = {}
    for key, rel in assets.items():
        if os.path.isabs(rel) or ".." in rel.replace("\\", "/").split("/"):
            errors.append(f"asset '{key}' path escapes pack: {rel}")
            continue
        path = os.path.join(folder, rel)
        if not os.path.isfile(path):
            errors.append(f"asset '{key}' file missing: {rel}")
            continue
        with open(path, "rb") as fh:
            hashes.setdefault(hashlib.sha256(fh.read()).hexdigest(), []).append(key)
        try:
            with Image.open(path) as im:
                if im.size[0] > 4096 or im.size[1] > 4096:
                    errors.append(f"asset '{key}' exceeds 4096x4096: {im.size}")
                if im.size != (512, 512):
                    warnings.append(f"asset '{key}' is {im.size[0]}x{im.size[1]}, repo standard is 512x512")
                if im.mode not in ("RGBA", "LA"):
                    warnings.append(f"asset '{key}' has no alpha channel (mode {im.mode})")
        except OSError as exc:
            errors.append(f"asset '{key}' not a decodable image: {exc}")

    animations = m.get("animations") or {}
    for anim_name, anim in animations.items():
        ref = anim.get("asset")
        if ref not in assets:
            errors.append(f"animation '{anim_name}' references unknown asset '{ref}'")
        for frame in anim.get("frames", []):
            if frame.get("width", 0) <= 0 or frame.get("height", 0) <= 0:
                errors.append(f"animation '{anim_name}' has empty frame rect")

    for state, target in (m.get("stateMappings") or {}).items():
        if target not in animations:
            errors.append(f"stateMapping '{state}' -> unknown animation '{target}'")

    shared = [keys for keys in hashes.values() if len(keys) > 1]
    if shared:
        warnings.append(f"identical image reused across states: {shared}")

    return errors, warnings


def main():
    if not os.path.isdir(PACKS_DIR):
        print(f"pet-packs directory not found: {PACKS_DIR}")
        return 1
    failed = 0
    for entry in sorted(os.listdir(PACKS_DIR)):
        folder = os.path.join(PACKS_DIR, entry)
        if not os.path.isdir(folder):
            continue
        errors, warnings = check_pack(folder)
        status = "FAIL" if errors else ("WARN" if warnings else "OK")
        print(f"[{status}] {entry}")
        for msg in errors:
            print(f"    error: {msg}")
        for msg in warnings:
            print(f"    warn:  {msg}")
        failed += bool(errors)
    print(f"\n{failed} pack(s) with errors")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
