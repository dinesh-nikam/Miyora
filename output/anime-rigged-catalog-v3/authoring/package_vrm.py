"""Verify a local Blender GLB and add its actual VRM humanoid/expression mapping.

This does not generate or alter model geometry, skeletons, or animation tracks.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

REQUIRED_BONES = ("hips", "spine", "chest", "neck", "head", "leftUpperArm", "leftLowerArm", "leftHand", "rightUpperArm", "rightLowerArm", "rightHand", "leftUpperLeg", "leftLowerLeg", "leftFoot", "rightUpperLeg", "rightLowerLeg", "rightFoot")
COMPONENTS = {5120: ("b", 1), 5121: ("B", 1), 5122: ("h", 2), 5123: ("H", 2), 5125: ("I", 4), 5126: ("f", 4)}
WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def read_glb(path):
    data = path.read_bytes()
    if len(data) < 20 or struct.unpack_from("<III", data) != (0x46546c67, 2, len(data)):
        raise ValueError("Invalid GLB header or length")
    chunks = []
    off = 12
    while off < len(data):
        n, kind = struct.unpack_from("<II", data, off)
        off += 8
        if off + n > len(data):
            raise ValueError("GLB chunk outside file")
        chunks.append((kind, data[off:off+n])); off += n
    if len(chunks) != 2 or chunks[0][0] != 0x4e4f534a or chunks[1][0] != 0x004e4942:
        raise ValueError("Expected one JSON and one embedded BIN chunk")
    return json.loads(chunks[0][1]), chunks[1][1]


def accessor(doc, binary, index):
    a = doc["accessors"][index]
    fmt, size = COMPONENTS[a["componentType"]]
    width = WIDTHS[a["type"]]
    if a.get("normalized"):
        raise ValueError("Unexpected normalized accessor in local export")
    def read(view_index, offset, count, fmt, width, size):
        v = doc["bufferViews"][view_index]
        stride = v.get("byteStride", width * size)
        start = v.get("byteOffset", 0) + offset
        end = start + max(0, count - 1) * stride + width * size
        if end > v.get("byteOffset", 0) + v["byteLength"] or end > len(binary):
            raise ValueError("Accessor outside buffer view")
        return [struct.unpack_from("<" + fmt * width, binary, start + i * stride) for i in range(count)]
    rows = read(a["bufferView"], a.get("byteOffset", 0), a["count"], fmt, width, size) if "bufferView" in a else [(0,) * width for _ in range(a["count"])]
    if "sparse" in a:
        sparse = a["sparse"]
        si, sv = sparse["indices"], sparse["values"]
        ifmt, isize = COMPONENTS[si["componentType"]]
        indices = read(si["bufferView"], si.get("byteOffset", 0), sparse["count"], ifmt, 1, isize)
        values = read(sv["bufferView"], sv.get("byteOffset", 0), sparse["count"], fmt, width, size)
        for (i,), row in zip(indices, values):
            if i >= len(rows):
                raise ValueError("Sparse accessor outside vertex array")
            rows[i] = row
    return rows


def validate(doc, binary):
    if any(b.get("uri") for b in doc.get("buffers", [])) or any(i.get("uri") for i in doc.get("images", [])):
        raise ValueError("External resource URI found")
    nodes = doc["nodes"]
    names = {n.get("name"): i for i, n in enumerate(nodes) if n.get("name")}
    if any(b not in names for b in REQUIRED_BONES):
        raise ValueError("Missing humanoid bones")
    if not doc.get("skins"):
        raise ValueError("No actual skin in GLB")
    triangles = 0
    vertex_count = 0
    mesh_nodes = [i for i, n in enumerate(nodes) if "mesh" in n and "skin" in n]
    if not mesh_nodes:
        raise ValueError("No mesh node bound to skeleton")
    influenced = set()
    for ni in mesh_nodes:
        skin = doc["skins"][nodes[ni]["skin"]]
        joints = skin["joints"]
        for matrix in accessor(doc, binary, skin["inverseBindMatrices"]):
            if not all(math.isfinite(v) for v in matrix):
                raise ValueError("Invalid bind matrix")
        mesh = doc["meshes"][nodes[ni]["mesh"]]
        for primitive in mesh["primitives"]:
            attrs = primitive["attributes"]
            positions = accessor(doc, binary, attrs["POSITION"])
            weights = accessor(doc, binary, attrs["WEIGHTS_0"])
            indices = accessor(doc, binary, attrs["JOINTS_0"])
            if len(positions) != len(weights) or len(weights) != len(indices):
                raise ValueError("Skin attribute counts differ")
            for p, w, js in zip(positions, weights, indices):
                if not all(math.isfinite(v) for v in p) or abs(sum(w) - 1) > .002 or min(w) < 0:
                    raise ValueError("Invalid position or skin weights")
                for j, weight in zip(js, w):
                    if j >= len(joints):
                        raise ValueError("Joint outside skeleton")
                    if weight > .001:
                        influenced.add(nodes[joints[j]].get("name", str(j)))
            vertex_count += len(positions)
            triangles += doc["accessors"][primitive["indices"]]["count"] // 3
            if primitive.get("mode", 4) != 4:
                raise ValueError("Unsupported primitive mode")
            index_values = accessor(doc, binary, primitive["indices"])
            if any(x[0] >= len(positions) for x in index_values):
                raise ValueError("Mesh index outside vertices")
    animations = {a.get("name") for a in doc.get("animations", [])}
    required = {"idle", "working", "thinking", "waiting", "success", "error", "celebrating", "sleeping", "hungry", "eating", "level_up", "achievement", "offline", "paused", "attention_required"}
    if not required.issubset(animations):
        raise ValueError(f"Missing exported actions: {sorted(required - animations)}")
    for animation in doc["animations"]:
        for sampler in animation["samplers"]:
            times = [x[0] for x in accessor(doc, binary, sampler["input"])]
            if len(times) < 2 or times[-1] <= times[0]:
                raise ValueError("Empty or zero-duration animation")
            if not all(math.isfinite(t) for t in times):
                raise ValueError("Non-finite animation time")
            values = accessor(doc, binary, sampler["output"])
            if not all(math.isfinite(v) for row in values for v in row):
                raise ValueError("Non-finite animation track")
    if triangles > 100_000 or len(doc.get("images", [])) > 16:
        raise ValueError("Miyora geometry/texture budget exceeded")
    return {"triangleCount": triangles, "vertexCount": vertex_count, "skeletonJointCount": len(doc["skins"][0]["joints"]), "influencedBoneNames": sorted(influenced), "animationNames": sorted(animations), "embeddedTextureCount": len(doc.get("images", []))}, names, mesh_nodes


def package(source, destination, title, expression_names=("blink",), spring_chains=(), toon=False):
    if destination.exists():
        raise ValueError("Refusing to overwrite packaged model")
    doc, binary = read_glb(source)
    report, names, mesh_nodes = validate(doc, binary)
    human_bones = {b: {"node": names[b]} for b in REQUIRED_BONES}
    for side in ("left", "right"):
        for suffix in ("Shoulder", "Toes"):
            bone = side + suffix
            if bone in names:
                human_bones[bone] = {"node": names[bone]}
        for finger in ("Index", "Middle", "Ring", "Little"):
            for part in ("Proximal", "Intermediate", "Distal"):
                bone = side + finger + part
                if bone in names:
                    human_bones[bone] = {"node": names[bone]}
    presets = {}
    for expression in expression_names:
        binds = []
        for ni in mesh_nodes:
            mesh = doc["meshes"][doc["nodes"][ni]["mesh"]]
            targets = mesh.get("extras", {}).get("targetNames", [])
            if expression in targets:
                index = targets.index(expression)
                changed = False
                for primitive in mesh["primitives"]:
                    deltas = accessor(doc, binary, primitive["targets"][index]["POSITION"])
                    changed |= any(abs(v) > 1e-6 for row in deltas for v in row)
                if not changed:
                    raise ValueError(f"Expression {expression} has no deformation")
                binds.append({"node": ni, "index": index, "weight": 1})
        if not binds:
            raise ValueError(f"Missing actual {expression} morph")
        presets[expression] = {"morphTargetBinds": binds}
    meta = {"name": title, "version": "0.1.0", "authors": ["Miyora local character authoring"], "licenseUrl": "https://vrm.dev/licenses/1.0/", "avatarPermission": "onlyAuthor", "commercialUsage": "personalNonProfit", "creditNotation": "required", "allowRedistribution": False, "modification": "allowModification", "otherLicenseUrl": "", "copyrightInformation": "Locally authored mesh. Character design rights remain with their respective owners; no franchise license is asserted."}
    doc.setdefault("extensionsUsed", []).append("VRMC_vrm")
    doc.setdefault("extensions", {})["VRMC_vrm"] = {"specVersion": "1.0", "meta": meta, "humanoid": {"humanBones": human_bones}, "expressions": {"preset": presets}}
    if spring_chains:
        seen = set()
        springs = []
        for chain in spring_chains:
            if len(chain) < 2 or any(name not in names or name in seen for name in chain):
                raise ValueError("Invalid or overlapping secondary-motion chain")
            seen.update(chain)
            joints = [{"node": names[name], "hitRadius": .012, "stiffness": 1.7, "gravityPower": .08, "gravityDir": [0, -1, 0], "dragForce": .62} for name in chain]
            springs.append({"name": chain[0], "center": names["hips"], "joints": joints, "colliderGroups": [0]})
        doc["extensionsUsed"].append("VRMC_springBone")
        colliders = [{"node": names["chest"], "shape": {"capsule": {"offset": [0, -.28, 0], "tail": [0, .10, 0], "radius": .13}}}, {"node": names["head"], "shape": {"sphere": {"offset": [0, .047, 0], "radius": .092}}}]
        doc["extensions"]["VRMC_springBone"] = {"specVersion": "1.0", "springs": springs, "colliders": colliders, "colliderGroups": [{"name": "BodyAndHead", "colliders": [0, 1]}]}
    toon_count = 0
    if toon:
        doc["extensionsUsed"].append("VRMC_materials_mtoon")
        for material in doc.get("materials", []):
            pbr = material.get("pbrMetallicRoughness", {})
            if pbr.get("metallicFactor", 0) > .5 or material.get("name", "").lower() in ("skin", "slime", "eyewhite", "eyes", "irisglow", "lips", "mouthshade", "earpink"):
                continue
            pbr["baseColorFactor"] = [c * .68 for c in pbr.get("baseColorFactor", [1, 1, 1, 1])[:3]] + [pbr.get("baseColorFactor", [1, 1, 1, 1])[3]]
            rgb = pbr.get("baseColorFactor", [1, 1, 1, 1])[:3]
            fine = any(word in material.get("name", "").lower() for word in ("iris", "eye", "lash", "lip", "strand", "mouth"))
            settings = {"specVersion": "1.0", "shadeColorFactor": [c * .72 for c in rgb], "shadingShiftFactor": -.04, "shadingToonyFactor": .66, "giEqualizationFactor": .82, "outlineWidthMode": "none" if fine else "worldCoordinates", "outlineWidthFactor": .00075, "outlineColorFactor": [.035, .025, .045], "outlineLightingMixFactor": .25}
            if "baseColorTexture" in pbr:
                settings["shadeMultiplyTexture"] = pbr["baseColorTexture"]
            extensions = material.setdefault("extensions", {})
            # GLTFLoader's built-in specular plugin otherwise chooses a PBR material first.
            extensions.pop("KHR_materials_specular", None)
            extensions["VRMC_materials_mtoon"] = settings
            toon_count += 1
    payload = json.dumps(doc, separators=(",", ":")).encode()
    payload += b" " * (-len(payload) % 4)
    binary += b"\x00" * (-len(binary) % 4)
    data = struct.pack("<III", 0x46546c67, 2, 12 + 8 + len(payload) + 8 + len(binary)) + struct.pack("<II", len(payload), 0x4e4f534a) + payload + struct.pack("<II", len(binary), 0x004e4942) + binary
    if len(data) > 50 * 1024 * 1024:
        raise ValueError("Packaged model exceeds file-size budget")
    destination.write_bytes(data)
    report.update(schemaVersion=1, structuralValidation="passed", runtimeValidation="pending", visualReview="pending", byteLength=len(data), sha256=hashlib.sha256(data).hexdigest(), expressionNames=list(presets), springChainCount=len(spring_chains), toonMaterialCount=toon_count)
    destination.with_suffix(".validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--name", required=True)
    args = parser.parse_args()
    try:
        package(args.source, args.destination, args.name)
    except (ValueError, KeyError, OSError, struct.error) as error:
        print(f"Model validation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
