"""Local Blender refinement and guarded, atomic export of the Miyora catalog."""
from __future__ import annotations

import argparse
import contextlib
import copy
import hashlib
import json
import logging
import math
from pathlib import Path
import re
import sys
import time

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from author_characters import FPS, studio, color
from catalog_author import CatalogAuthor
from package_vrm import package
from appearance_refinement import footwear, nezuko_details, texture_material, costume, surface_details

LOG = logging.getLogger("miyora.refine")
REVISION = "refined-catalog-v3.3"


class RefinedAuthor(CatalogAuthor):
    def anatomy(self):
        super().anatomy()
        footwear(self)

    def nonhuman_anatomy(self):
        super().nonhuman_anatomy()
        footwear(self)

    def fabric(self):
        if self.nezuko:
            if "AsanohaSilk" in self.mats:
                return self.mats["AsanohaSilk"]
            return texture_material(self, "AsanohaSilk", "asanoha", "#eab1c3", "#b46b88")
        return super().fabric()

    def signatures(self):
        super().signatures()
        if self.nezuko:
            nezuko_details(self)
        if self.slug == "tanjiro":
            self.discard([o for o in self.meshes if o.name.startswith("ForeheadScar")])
            outline = [(.023, .081), (.031, .104), (.024, .122), (.042, .115), (.052, .136), (.067, .123), (.074, .134), (.092, .112), (.085, .096), (.065, .091), (.052, .074), (.038, .086)]
            vertices = [(x, self.face_y(x, self.face_z + z) - .002, self.face_z + z) for x, z in outline]
            self.mesh("SculptedBurnScar", vertices, [tuple(range(len(vertices)))], self.material("BurnScar", "#863e40"))
            for sign in (-1, 1):
                self.ellipsoid("EarringRisingSun", (sign * .147, -.015, 1.372), (.006, .001, .006), self.material("EarringSun", "#a34348"), segments=16, rings=8)
                for i in (-1, 0, 1):
                    self.line("EarringRay", [(sign * .147, -.015, 1.365), (sign * .147 + i * .008, -.015, 1.331)], .0007, self.material("eyeliner"))
        surface_details(self)

    def hair(self):
        super().hair()
        for obj in self.meshes:
            if obj.name.startswith("SweptFringe"):
                for vertex in obj.data.vertices:
                    vertex.co.y -= .028
        locks = [o for o in self.meshes if o.name.startswith(("HairLock", "TempleLock", "PonytailLock"))]
        base, tip = color(self.palette["hair"]), color(self.spec.get("hairTip", self.palette["hair"]))
        image = bpy.data.images.new("SilkenHairGradient", width=32, height=256)
        pixels = []
        for y in range(256):
            t = max(0, min(1, (y / 255 - .55) / .43))
            t = t * t * (3 - 2 * t)
            for x in range(32):
                sheen = 1 + .055 * math.cos(x * math.tau / 16)
                pixels.extend([min(1, (base[c] * (1 - t) + tip[c] * t) * sheen) for c in range(3)] + [1])
        image.pixels.foreach_set(pixels)
        image.pack()
        gradient = self.material("HairSilkGradient", roughness=.43)
        texture = gradient.node_tree.nodes.new("ShaderNodeTexImage")
        texture.image = image
        gradient.node_tree.links.new(texture.outputs["Color"], gradient.node_tree.nodes.get("Principled BSDF").inputs["Base Color"])
        for obj in locks:
            obj.data.materials.clear()
            obj.data.materials.append(gradient)
            for polygon in obj.data.polygons:
                polygon.material_index = 0
            if obj.name.startswith("PonytailLock"):
                obj.name = "HairLock.Ponytail." + obj.name
        strand_color = tuple(min(1, c * 1.35 + .008) for c in base[:3]) + (1,)
        strands = self.material("HairStrandLight", roughness=.50)
        strands.diffuse_color = strand_color
        strands.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = strand_color
        for obj in locks:
            sides = 10 if obj.name.startswith("TempleLock") else 8
            points = []
            for start in range(0, len(obj.data.vertices), sides):
                ring = obj.data.vertices[start:start + sides]
                if len(ring) != sides:
                    continue
                chosen = min(ring, key=lambda v: v.co.y) if obj.name.startswith("TempleLock") else max(ring, key=lambda v: v.co.y)
                point = chosen.co.copy()
                point.y += -.0005 if obj.name.startswith("TempleLock") else .0005
                points.append(tuple(point))
            if len(points) > 2:
                self.line("TempleLock.FineStrand" if obj.name.startswith("TempleLock") else "HairLock.FineStrand", points, .0007, strands)

    def head_shape(self):
        super().head_shape()
        self.spring_chains = []
        if self.spec["hair"] not in ("long", "hime", "bob", "ponytail", "twintails", "side_tail", "curly"):
            return
        groups = {}
        for obj in self.meshes:
            if not obj.name.startswith(("HairLock", "TempleLock")):
                continue
            for vertex in obj.data.vertices:
                t = max(0, min(1, (1.35 - vertex.co.z) / .20))
                if obj.name.startswith("TempleLock"):
                    vertex.co.y -= .045 * t
                else:
                    vertex.co.y += .09 * t
            center = sum((v.co for v in obj.data.vertices), Vector()) / len(obj.data.vertices)
            label = ("front" if obj.name.startswith("TempleLock") else "back") + ("Left" if center.x < -.022 else "Right" if center.x > .022 else "Center")
            groups.setdefault(label, []).append(obj)
        if not groups:
            return
        bpy.context.view_layer.objects.active = self.rig
        self.rig.select_set(True)
        bpy.ops.object.mode_set(mode="EDIT")
        for label, objects in groups.items():
            points = [v.co for obj in objects for v in obj.data.vertices]
            low, high = min(p.z for p in points), max(p.z for p in points)
            if high - low < .10:
                continue
            x, y = sum(p.x for p in points) / len(points), sum(p.y for p in points) / len(points)
            names = []
            for i in range(4):
                name = f"Hair.{label}.{i}"
                bone = self.rig.data.edit_bones.new(name)
                bone.head = (x, y, high - (high - low) * i / 3)
                bone.tail = (x, y, high - (high - low) * (i + 1) / 3)
                bone.parent = self.rig.data.edit_bones[names[-1] if names else "head"]
                names.append(name)
                self.bones[name] = (tuple(bone.head), tuple(bone.tail))
            self.spring_chains.append(names)
            for obj in objects:
                if obj.vertex_groups.get("head"):
                    obj.vertex_groups.remove(obj.vertex_groups["head"])
                weights = [obj.vertex_groups.new(name=name) for name in names[:3]]
                for vertex in obj.data.vertices:
                    progress = min(2, max(0, (high - vertex.co.z) / (high - low) * 3 - .25))
                    index = min(1, int(progress))
                    blend = progress - index
                    weights[index].add([vertex.index], 1 - blend, "REPLACE")
                    weights[index + 1].add([vertex.index], blend, "REPLACE")
        bpy.ops.object.mode_set(mode="OBJECT")
        self.rig.select_set(False)

    def finish_skin(self):
        super().finish_skin()
        obj = self.object
        basis = obj.data.shape_keys.key_blocks["Basis"]
        def members(name):
            group = obj.vertex_groups.get(name)
            return [] if group is None else [v.index for v in obj.data.vertices if any(g.group == group.index for g in v.groups)]
        mouth, eyes, brows = members("Expression.Mouth"), members("Expression.Eyes"), members("Expression.Brows")
        if not mouth:
            raise ValueError("Refined model is missing its expressive mouth")
        mouth_z = sum(basis.data[i].co.z for i in mouth) / len(mouth)
        half_width = max(abs(basis.data[i].co.x) for i in mouth)
        for name in ("happy", "sad", "aa", "surprised"):
            key = obj.shape_key_add(name=name)
            for i in mouth:
                source, target = basis.data[i].co, key.data[i].co
                u = min(1, abs(source.x) / max(.001, half_width))
                if name in ("happy", "sad"):
                    target.z += (.006 if name == "happy" else -.005) * (u * u - .25)
                else:
                    target.z += (.009 if name == "aa" else .013) * math.tanh((source.z - mouth_z) / .0012)
                    if name == "surprised":
                        target.x *= .72
            for i in eyes:
                source = basis.data[i].co
                center = self.face_z + .018
                if name == "happy":
                    key.data[i].co.z = center + (source.z - center) * .88
                elif name == "surprised":
                    key.data[i].co.z = center + (source.z - center) * 1.12
            for i in brows:
                x = abs(basis.data[i].co.x)
                key.data[i].co.z += .006 if name == "surprised" else (.005 * (1 - min(1, x / .075)) if name == "sad" else 0)

    def face_y(self, x, z):
        vertical = (z - self.face_z) / .187
        taper = .76 + .24 * min(1, vertical + 1)
        bridge = .012 * math.exp(-(x / .027) ** 2 - ((vertical + .25) / .24) ** 2)
        return -.116 * math.sqrt(max(.025, 1 - (x / (self.head_r * taper)) ** 2 - vertical ** 2)) - bridge

    def eye_patch(self, name, x, z, width, height, mat, *, center_x, eye_height, depth=0):
        vertices, faces = [], []
        segments, rings = 40, 5
        for ring in range(rings + 1):
            radius = ring / rings
            for i in range(segments):
                angle = i * math.tau / segments
                px = x + radius * width * math.cos(angle)
                pz = z + radius * height * math.sin(angle) * (.80 + .20 * abs(math.sin(angle)))
                eye_radius = min(1, ((px - center_x) / .045) ** 2 + ((pz - self.face_z - .020) / eye_height) ** 2)
                py = self.face_y(px, pz) - .003 - .009 * (1 - eye_radius) - depth
                vertices.append((px, py, pz))
        for j in range(rings):
            for i in range(segments):
                a, b = j * segments + i, j * segments + (i + 1) % segments
                faces.append((a, b, b + segments, a + segments))
        return self.mesh(name, vertices, faces, mat, blink=True)

    def face(self):
        if self.body != "humanoid":
            super().face()
            if self.body == "bird":
                candidates = [o for o in self.meshes if o.name.startswith("ModeledBeak")]
            else:
                z, y, width = (self.face_z + .04, -.43 * math.sqrt(1 - ((self.face_z - .06 - .78) / .68) ** 2) - .006, .045) if self.body == "slime" else (1.32, -.172 if self.body == "beast" else -.123, .029)
                candidates = [self.line("ExpressiveMouth", [(x, y, z + .004 * (x / width) ** 2) for x in (-width, -width / 2, 0, width / 2, width)], .0017, self.material("mouthShade", "#503141"))]
            for obj in candidates:
                obj.vertex_groups.new(name="Expression.Mouth").add(list(range(len(obj.data.vertices))), 1, "REPLACE")
            return
        ink = self.material("eyeliner", "#291d2a")
        white = self.material("eyeWhite", "#fff8f1", roughness=.25)
        iris = self.material("eyes", roughness=.28)
        glow = self.material("irisGlow")
        eye_color = color(self.palette["eyes"])
        glow_color = tuple(eye_color[i] * .65 + (.75, .72, .74)[i] * .35 for i in range(3)) + (1,)
        glow.diffuse_color = glow_color
        glow.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = glow_color
        style = self.spec["eyes"]
        height = {"sharp": .020, "sleepy": .017, "sparkle": .028}.get(style, .026)
        for sign in (-1, 1):
            x, z = sign * .061, self.face_z + .020
            options = dict(center_x=x, eye_height=height)
            self.eye_patch("AlmondSclera", x, z, .045, height, white, **options)
            self.eye_patch("IrisRim", x, z, .0195, height * .87, ink, depth=.0007, **options)
            self.eye_patch("LayeredIris", x, z, .0178, height * .81, iris, depth=.0012, **options)
            self.eye_patch("IrisLowerLight", x, z - height * .27, .013, height * .42, glow, depth=.0018, **options)
            self.eye_patch("EyePupil", x, z + .002, .005 if self.nezuko else .006, height * .62, ink, depth=.0025, **options)
            for dx, dz, radius in ((-.006, .010, .0047), (.008, -.007, .0022)):
                px, pz = x + dx, z + dz
                py = self.face_y(px, pz) - .014
                self.ellipsoid("EyeCatchlight", (px, py, pz), (radius, .001, radius * 1.2), white, blink=True, segments=12, rings=8)
            for upper in (False, True):
                points = []
                for i in range(25):
                    angle = math.pi * i / 24 + (0 if upper else math.pi)
                    px = x + .045 * math.cos(angle)
                    pz = z + height * math.sin(angle) * (.80 + .20 * abs(math.sin(angle)))
                    points.append((px, self.face_y(px, pz) - .004, pz))
                self.line("UpperLashes" if upper else "LowerLid", points, .0019 if upper else .00065, ink, blink=True)
            for i in range(2):
                px, pz = x + sign * (.040 + i * .003), z + .006 + i * .004
                self.line("LashWing", [(px, self.face_y(px, pz) - .005, pz), (px + sign * .010, self.face_y(px, pz) - .003, pz + .008)], .0011, ink, blink=True)
            brow = self.line("Eyebrow", [(x + u, self.face_y(x + u, z + .051) - .003, z + .046 + .010 * math.cos(u / .04 * math.pi / 2)) for u in (-.039, -.02, 0, .02, .039)], .0023, self.material("hair"))
            brow.vertex_groups.new(name="Expression.Brows").add(list(range(len(brow.data.vertices))), 1, "REPLACE")
            self.ellipsoid("Ear", (sign * .146, 0, self.face_z - .018), (.021, .026, .043), self.material("skin"), segments=24, rings=16)
            self.ellipsoid("EarConcha", (sign * .153, -.017, self.face_z - .019), (.011, .010, .027), self.material("earPink", "#d99396"), segments=20, rings=12)
        self.ellipsoid("NoseTip", (0, -.124, self.face_z - .038), (.006, .005, .008), self.material("skin"), segments=24, rings=16)
        mouth_z = self.face_z - .080
        mouth = self.ellipsoid("MouthInterior", (0, self.face_y(0, mouth_z) - .002, mouth_z), (.017, .001, .0017), self.material("mouthShade", "#784a57"), segments=24, rings=10)
        mouth.vertex_groups.new(name="Expression.Mouth").add(list(range(len(mouth.data.vertices))), 1, "REPLACE")
        for lower in (False, True):
            points = [(x, self.face_y(x, mouth_z) - .003, mouth_z + (-.002 if lower else .001) + .002 * (x / .019) ** 2) for x in (-.019, -.010, 0, .010, .019)]
            lip = self.line("LowerLip" if lower else "UpperLip", points, .0011, self.material("lips", "#c58b92"))
            lip.vertex_groups.new(name="Expression.Mouth").add(list(range(len(lip.data.vertices))), 1, "REPLACE")
        if self.spec["ears"] != "human":
            self.discard([o for o in self.meshes if o.name.startswith(("EarConcha", "Ear.")) or o.name == "Ear"])

    @staticmethod
    def arm_weights(point, side):
        blend = max(0.0, min(1.0, (abs(point[0]) - .33) / .14))
        return {side + "UpperArm": 1 - blend, side + "LowerArm": blend}

    def hanging_sleeve(self, side, sign, outer, lining):
        sections = [(.155, .065, .075), (.22, .155, .082), (.32, .22, .085),
                    (.40, .235, .086), (.49, .205, .082), (.575, .14, .065)]
        vertices, weights, faces, uvs = [], [], [], []
        sides = 32
        for inner in (False, True):
            for x, drop, depth in sections:
                top, bottom = 1.247, 1.19 - drop
                for i in range(sides):
                    angle = i * math.tau / sides
                    inset = .003 if inner else 0
                    point = (sign * x, (depth - inset) * math.sin(angle),
                             (top + bottom) / 2 + ((top - bottom) / 2 - inset) * math.cos(angle))
                    vertices.append(point)
                    weights.append(self.arm_weights(point, side))
                    uvs.append((i / sides, (x - .155) / .42))
        layer = len(sections) * sides
        for inner in (False, True):
            offset = layer if inner else 0
            for j in range(len(sections) - 1):
                for i in range(sides):
                    a = offset + j * sides + i
                    b = offset + j * sides + (i + 1) % sides
                    face = (a, b, b + sides, a + sides)
                    faces.append(tuple(reversed(face)) if inner else face)
        outer_faces = (len(sections) - 1) * sides
        for ring in (0, len(sections) - 1):
            for i in range(sides):
                a, b = ring * sides + i, ring * sides + (i + 1) % sides
                faces.append((a, a + layer, b + layer, b))
        obj = self.mesh("TailoredHangingSleeve", vertices, faces, outer, side + "UpperArm", weights, uv=uvs)
        obj.data.materials.append(lining)
        for polygon in obj.data.polygons[outer_faces * 2 - sides:outer_faces * 2]:
            polygon.material_index = 1

    def clothing(self):
        if costume(self):
            return
        super().clothing()
        if self.spec["outfit"] not in ("kimono", "robe", "hakama"):
            return
        self.discard([o for o in self.meshes if o.name.startswith(("cloth.SilkSleeve", "cloth.HaoriSleeve", "CollarLapel"))])
        outer = self.material("secondary") if self.nezuko else self.material("kimono" if self.spec["outfit"] == "kimono" else "primary")
        lining = self.fabric() if self.nezuko else self.material("secondary")
        for side, sign in (("left", 1), ("right", -1)):
            self.hanging_sleeve(side, sign, outer, lining)
        collar = self.material("collarIvory", "#f7e8db")
        for sign in (-1, 1):
            points = [(sign * .041, -.045, 1.245), (sign * .025, -.084, 1.19), (0, -.100, 1.145)]
            self.tube("LayeredKimonoCollar", points, [.006] * len(points), [.002] * len(points), collar, "chest", sides=4, weights_for=self.torso_weights)
        self.tube("KimonoOverlap", [(-.031, -.086, 1.193), (.013, -.104, 1.12), (.070, -.106, 1.025)], [.014] * 3, [.002] * 3, self.material("kimonoFold", "#e3a0b2" if self.nezuko else self.palette["primary"]), "chest", sides=4, weights_for=self.torso_weights)


def export_character(spec, root):
    slug = spec["id"].split(".")[-1]
    if not re.fullmatch(r"[a-z0-9-]+", slug):
        raise ValueError("Unsafe catalog identity")
    destination, staging = root / slug, root / (".staging-" + slug)
    if destination.exists():
        report = json.loads((destination / "production.json").read_text())
        validation = json.loads((destination / "model.validation.json").read_text())
        if report.get("revision") == REVISION and validation.get("structuralValidation") == "passed" and hashlib.sha256((destination / "model.vrm").read_bytes()).hexdigest() == validation.get("sha256"):
            return
        raise ValueError(f"Existing output does not match this verified revision: {slug}")
    if staging.exists():
        raise ValueError(f"Refusing to overwrite existing output for {slug}")
    staging.mkdir(parents=True)
    started = time.monotonic()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    author = RefinedAuthor(copy.deepcopy(spec)).build()
    studio(author)
    bpy.context.scene.render.fps = FPS
    bpy.context.scene.frame_end = 49
    bpy.ops.wm.save_as_mainfile(filepath=str(staging / "source.blend"))
    bpy.ops.object.select_all(action="DESELECT")
    author.rig.select_set(True)
    author.object.select_set(True)
    bpy.context.view_layer.objects.active = author.rig
    bpy.ops.export_scene.gltf(filepath=str(staging / "model.glb"), export_format="GLB", use_selection=True,
                              export_skins=True, export_morph=True, export_animations=True,
                              export_animation_mode="ACTIONS", export_nla_strips=True,
                              export_force_sampling=False, export_extras=True, export_yup=True)
    with contextlib.redirect_stdout(sys.stderr):
        package(staging / "model.glb", staging / "model.vrm", spec["name"] + " · Refined", expression_names=("blink", "happy", "sad", "aa", "surprised"), spring_chains=author.spring_chains, toon=True)
    (staging / "character.json").write_text(json.dumps(spec, indent=2), encoding="utf-8")
    report = {"id": spec["id"], "status": "structurally-validated", "revision": REVISION,
              "proportions": "full-anime" if spec["body"] == "humanoid" else "species-specific",
              "seconds": round(time.monotonic() - started, 2)}
    (staging / "production.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    staging.rename(destination)
    LOG.info("CHARACTER_DONE %s", json.dumps(report))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument("--character")
    selection.add_argument("--all", action="store_true")
    parser.add_argument("--worker-index", type=int, default=0)
    parser.add_argument("--worker-count", type=int, default=1)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    logging.basicConfig(level=logging.DEBUG if args.verbose else logging.INFO, format="%(levelname)s %(message)s")
    specs = json.loads(args.catalog.read_text(encoding="utf-8"))["characters"]
    if not 0 <= args.worker_index < args.worker_count <= 2:
        parser.error("Worker index/count are outside the local resource budget")
    ids = set((args.character or "miyora.nezuko").split(","))
    if not args.all and ids - {s["id"] for s in specs}:
        parser.error("Unknown requested character identity")
    selected = [s for i, s in enumerate(specs) if i % args.worker_count == args.worker_index] if args.all else [s for s in specs if s["id"] in ids]
    if not selected:
        parser.error("No matching characters")
    args.output.mkdir(parents=True, exist_ok=True)
    failures = []
    for spec in selected:
        try:
            export_character(spec, args.output)
        except Exception as error:
            LOG.exception("Character failed: %s", spec["id"])
            failures.append({"id": spec["id"], "error": str(error)})
    (args.output / f"worker-{args.worker_index}.json").write_text(json.dumps({"revision": REVISION, "selected": len(selected), "failed": failures}, indent=2))
    if failures:
        raise RuntimeError(f"{len(failures)} exports failed; see worker report")


if __name__ == "__main__":
    try:
        main()
    except Exception:
        LOG.exception("Refinement failed")
        raise
