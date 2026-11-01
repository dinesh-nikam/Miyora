"""Validate every pack under pet-packs/ against the v1 PetPack contract (PET_PACK_SPEC.md).

Read-only. Exits 1 when any pack has an unallowlisted error.
Requires Pillow (already in requirements.txt).
"""

import hashlib
import json
import os
import re
import sys

from PIL import Image

PACKS_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "pet-packs")
ALLOWLIST_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "pet_pack_allowlist.json")
REQUIRED_STRINGS = ("id", "name", "version", "description", "creator", "license", "source", "attribution")
REQUIRED_STATES = ("idle", "working", "thinking", "waiting", "success", "error", "level_up", "sleeping", "thumbnail")
ACTIVE_STATES = {"idle", "success", "error", "waiting"}
VALID_ANIMATION_KINDS = {"animated", "static", "partial"}
FOLDER_RE = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")
SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+$")
ID_RE = re.compile(r"^[a-z0-9][a-z0-9.-]*$")


def load_allowlist():
    if not os.path.isfile(ALLOWLIST_PATH):
        return {"undeclared_static_reuse": set(), "missing_alpha": set()}
    try:
        with open(ALLOWLIST_PATH, encoding="utf-8") as fh:
            data = json.load(fh)
        return {
            "undeclared_static_reuse": set(data.get("undeclared_static_reuse", [])),
            "missing_alpha": set(data.get("missing_alpha", [])),
        }
    except Exception as exc:
        print(f"Warning: could not read allowlist: {exc}", file=sys.stderr)
        return {"undeclared_static_reuse": set(), "missing_alpha": set()}


def check_pack(folder, allowlist):
    errors, warnings = [], []
    name = os.path.basename(folder)
    pack_id = name
    if not FOLDER_RE.match(name):
        warnings.append(f"folder name '{name}' is not lowercase kebab-case")

    pj = os.path.join(folder, "pet.json")
    if not os.path.isfile(pj):
        return ["missing pet.json"], warnings, {"is_animated": False, "is_static": False}
    try:
        with open(pj, encoding="utf-8") as fh:
            m = json.load(fh)
    except (OSError, json.JSONDecodeError) as exc:
        return [f"pet.json invalid JSON: {exc}"], warnings, {"is_animated": False, "is_static": False}

    if m.get("schemaVersion") not in (1, 2):
        errors.append(f"schemaVersion must be 1 or 2, got {m.get('schemaVersion')!r}")
    for key in REQUIRED_STRINGS:
        value = m.get(key)
        if not isinstance(value, str) or not value.strip():
            errors.append(f"missing required field '{key}'")
    if isinstance(m.get("id"), str):
        pack_id = m["id"]
        if not ID_RE.match(m["id"]):
            errors.append(f"id '{m['id']}' has invalid characters")
    if isinstance(m.get("version"), str) and not SEMVER_RE.match(m["version"]):
        errors.append(f"version '{m['version']}' is not semver")
    for flag in ("commercialUse", "redistribution", "derivativeWorks"):
        if not isinstance(m.get(flag), bool):
            errors.append(f"'{flag}' must be a boolean")

    animation_kind = m.get("animation")
    if animation_kind is not None:
        if not isinstance(animation_kind, str) or animation_kind not in VALID_ANIMATION_KINDS:
            errors.append(f"'animation' must be one of {sorted(VALID_ANIMATION_KINDS)}, got {animation_kind!r}")

    personality = m.get("personality")
    if personality is not None:
        if not isinstance(personality, dict):
            errors.append("'personality' must be an object")
        else:
            tc = personality.get("themeColor")
            if tc is not None and (not isinstance(tc, str) or not re.match(r"^#[0-9a-fA-F]{6}$", tc)):
                errors.append(f"'themeColor' must be a #rrggbb hex string, got {tc!r}")
            for text_field in ("feedReaction", "wakeReaction", "petReaction"):
                val = personality.get(text_field)
                if val is not None and not isinstance(val, str):
                    errors.append(f"'{text_field}' must be a string")
            dlg = personality.get("dialogue")
            if dlg is not None and not isinstance(dlg, dict):
                errors.append("'dialogue' must be an object")

    assets = m.get("assets")
    if not isinstance(assets, dict) or not assets:
        errors.append("'assets' missing or empty")
        assets = {}
    for state in REQUIRED_STATES:
        if state not in assets:
            warnings.append(f"no asset for state '{state}'")

    hashes = {}
    has_missing_alpha = False
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
                    has_missing_alpha = True
                    msg = f"asset '{key}' has no alpha channel (mode {im.mode})"
                    if name in allowlist["missing_alpha"] or pack_id in allowlist["missing_alpha"]:
                        warnings.append(f"[ALLOWLISTED] {msg}")
                    else:
                        errors.append(msg)
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

    # Check for image reuse across active states
    reused_active_states = []
    for keys in hashes.values():
        overlap = set(keys).intersection(ACTIVE_STATES)
        if len(overlap) > 1:
            reused_active_states.append(sorted(overlap))

    if reused_active_states:
        if animation_kind != "static":
            msg = f"identical image reused across active states {reused_active_states}: must declare 'animation: static'"
            if name in allowlist["undeclared_static_reuse"] or pack_id in allowlist["undeclared_static_reuse"]:
                warnings.append(f"[ALLOWLISTED] {msg}")
            else:
                errors.append(msg)

    # General sharing warning
    shared = [keys for keys in hashes.values() if len(keys) > 1]
    if shared and not reused_active_states:
        warnings.append(f"identical image reused across non-active states: {shared}")

    stats = {
        "is_animated": len(reused_active_states) == 0 and animation_kind != "static",
        "is_static": animation_kind == "static" or len(reused_active_states) > 0,
        "had_undeclared_reuse": len(reused_active_states) > 0 and animation_kind != "static",
        "had_missing_alpha": has_missing_alpha,
    }

    return errors, warnings, stats


def main():
    if not os.path.isdir(PACKS_DIR):
        print(f"pet-packs directory not found: {PACKS_DIR}")
        return 1

    allowlist = load_allowlist()
    failed = 0
    total_packs = 0
    animated_count = 0
    static_count = 0
    allowlisted_reuse_observed = set()
    allowlisted_alpha_observed = set()

    for entry in sorted(os.listdir(PACKS_DIR)):
        folder = os.path.join(PACKS_DIR, entry)
        if not os.path.isdir(folder):
            continue
        total_packs += 1
        errors, warnings, stats = check_pack(folder, allowlist)
        status = "FAIL" if errors else ("WARN" if warnings else "OK")
        print(f"[{status}] {entry}")
        for msg in errors:
            print(f"    error: {msg}")
        for msg in warnings:
            print(f"    warn:  {msg}")
        failed += bool(errors)

        if stats["is_animated"]:
            animated_count += 1
        if stats["is_static"]:
            static_count += 1
        if stats["had_undeclared_reuse"]:
            allowlisted_reuse_observed.add(entry)
        if stats["had_missing_alpha"]:
            allowlisted_alpha_observed.add(entry)

    # Ratchet check: warn if any allowlist entries can be removed
    stale_reuse = allowlist["undeclared_static_reuse"] - allowlisted_reuse_observed
    stale_alpha = allowlist["missing_alpha"] - allowlisted_alpha_observed
    if stale_reuse:
        print(f"\n[RATCHET] The following packs no longer have undeclared active reuse and can be removed from pet_pack_allowlist.json: {sorted(stale_reuse)}")
    if stale_alpha:
        print(f"\n[RATCHET] The following packs no longer lack alpha and can be removed from pet_pack_allowlist.json: {sorted(stale_alpha)}")

    print(f"\n--- PetPack Verification Summary ---")
    print(f"Total packs scanned:     {total_packs}")
    print(f"Genuinely animated:      {animated_count}")
    print(f"Static / Reused frames:  {static_count}")
    print(f"Grandfathered allowlist: {len(allowlist['undeclared_static_reuse'])} reuse, {len(allowlist['missing_alpha'])} alpha")
    print(f"Failed packs:            {failed}")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
