"""Local Blender character authoring and skinned GLB export. No network/API calls.

Run with Blender --background --factory-startup --python this.py -- --help.
Meshes are authored locally from section surfaces and curves, not image reliefs.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import logging
import math
from pathlib import Path
import struct
import sys
import time

import bpy
from mathutils import Euler, Vector, Matrix
import numpy as np

LOG = logging.getLogger("miyora.model-author")
MAX_TRIANGLES = 100_000
MAX_FILE_BYTES = 50 * 1024 * 1024
FPS = 24
STATES = ("idle", "working", "thinking", "waiting", "success", "error", "celebrating", "sleeping", "hungry", "eating", "level_up", "achievement", "offline", "paused", "attention_required")


def color(hex_color):
    s = hex_color.lstrip("#")
    rgb = [int(s[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    return tuple(v / 12.92 if v < 0.04045 else ((v + 0.055) / 1.055) ** 2.4 for v in rgb) + (1,)


class Author:
    def __init__(self, spec):
        self.spec = spec
        self.slug = spec["id"].split(".")[-1]
        self.nezuko = self.slug == "nezuko"
        self.body = spec["body"]
        self.meshes = []
        self.mats = {}
        self.bones = {}
        self.hair_bones = []
        self.scale = spec["proportions"]["height"]
        self.face_z = 1.405
        self.head_r = 0.151 * (0.96 + 0.12 * spec["proportions"]["headScale"])
        self.width = spec["proportions"]["build"]
        self.palette = dict(spec["palette"])
        if self.nezuko:
            self.palette.update(hair="#21151d", eyes="#e873a4", primary="#ef9ab3", secondary="#382730", accent="#a4c85b")
        self.create_rig()

    def material(self, key, value=None, metallic=0, roughness=0.65):
        if key in self.mats:
            return self.mats[key]
        m = bpy.data.materials.new(key)
        m.diffuse_color = color(value or self.palette.get(key, "#ffffff"))
        m.use_nodes = True
        p = m.node_tree.nodes.get("Principled BSDF")
        p.inputs["Base Color"].default_value = m.diffuse_color
        p.inputs["Roughness"].default_value = roughness
        p.inputs["Metallic"].default_value = metallic
        p.inputs["Specular IOR Level"].default_value = .18
        self.mats[key] = m
        if key.startswith("hair") or key.startswith("Hair"):
            p.inputs["Specular IOR Level"].default_value = .05
        return m

    def mesh(self, name, vertices, faces, mat, bone="head", weights=None, blink=False, uv=None):
        data = bpy.data.meshes.new(name)
        data.from_pydata(vertices, [], faces)
        data.update()
        import bmesh
        bm = bmesh.new()
        bm.from_mesh(data)
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
        bm.to_mesh(data)
        bm.free()
        obj = bpy.data.objects.new(name, data)
        bpy.context.collection.objects.link(obj)
        data.materials.append(mat)
        for p in data.polygons:
            p.use_smooth = True
        if uv:
            layer = data.uv_layers.new(name="UVMap")
            for p in data.polygons:
                for li in p.loop_indices:
                    layer.data[li].uv = uv[data.loops[li].vertex_index]
        group = obj.vertex_groups.new(name=bone)
        if weights is None:
            group.add(list(range(len(vertices))), 1, "REPLACE")
        else:
            groups = {bone: group}
            for i, entries in enumerate(weights):
                total = sum(entries.values())
                if total <= 0:
                    raise ValueError(f"Unweighted vertex in {name}")
                for bn, value in entries.items():
                    if bn not in groups:
                        groups[bn] = obj.vertex_groups.new(name=bn)
                    groups[bn].add([i], value / total, "REPLACE")
        if blink:
            obj.vertex_groups.new(name="Expression.Eyes").add(list(range(len(vertices))), 1, "REPLACE")
        self.meshes.append(obj)
        return obj

    def ellipsoid(self, name, center, radii, mat, bone="head", segments=28, rings=16, blink=False, sculpt=False):
        vertices, faces, uv = [], [], []
        for j in range(rings + 1):
            theta = math.pi * j / rings
            z = math.cos(theta)
            taper = 1 if not sculpt else 0.76 + 0.24 * min(1, (z + 1) / 1.0)
            for i in range(segments + 1):
                phi = 2 * math.pi * i / segments
                x = math.sin(theta) * math.cos(phi) * radii[0] * taper
                y = math.sin(theta) * math.sin(phi) * radii[1]
                if sculpt and y < 0:
                    # Gentle modeled nasal bridge and chin, separate from the eyes.
                    y -= 0.012 * math.exp(-(x / 0.027) ** 2 - ((z + 0.25) / 0.24) ** 2)
                vertices.append((center[0] + x, center[1] + y, center[2] + z * radii[2]))
                uv.append((i / segments, 1 - j / rings))
        for j in range(rings):
            for i in range(segments):
                a = j * (segments + 1) + i
                faces.append((a, a + 1, a + segments + 2, a + segments + 1))
        return self.mesh(name, vertices, faces, mat, bone, blink=blink, uv=uv)

    def loft(self, name, sections, mat, bone="hips", count=32, weights_for=None, cap=True):
        # Each section is center xyz, radial x/y, and optional ring twist.
        verts, faces, uvs, weights = [], [], [], []
        for j, section in enumerate(sections):
            x, y, z, rx, ry = section[:5]
            twist = section[5] if len(section) > 5 else 0
            for i in range(count):
                a = 2 * math.pi * i / count + twist
                fold = 1 + (0.025 * math.cos(a * 12) if "cloth" in name else 0)
                v = (x + rx * math.cos(a) * fold, y + ry * math.sin(a) * fold, z)
                verts.append(v)
                uvs.append((i / count, j / max(1, len(sections) - 1)))
                if weights_for:
                    weights.append(weights_for(v))
        for j in range(len(sections) - 1):
            for i in range(count):
                a = j * count + i
                b = j * count + (i + 1) % count
                faces.append((a, b, b + count, a + count))
        if cap:
            faces.extend([tuple(reversed(range(count))), tuple(range((len(sections) - 1) * count, len(sections) * count))])
        return self.mesh(name, verts, faces, mat, bone, weights if weights_for else None, uv=uvs)

    def tube(self, name, points, widths, depths, mat, bone="head", sides=10, weights_for=None):
        verts, faces, weights, uvs = [], [], [], []
        ps = [Vector(p) for p in points]
        for j, p in enumerate(ps):
            tangent = (ps[min(j + 1, len(ps) - 1)] - ps[max(0, j - 1)]).normalized()
            reference = Vector((0, -1, 0))
            if abs(tangent.dot(reference)) > 0.95:
                reference = Vector((1, 0, 0))
            u = tangent.cross(reference).normalized()
            v = tangent.cross(u).normalized()
            for i in range(sides):
                a = 2 * math.pi * i / sides
                pt = p + u * (math.cos(a) * widths[j]) + v * (math.sin(a) * depths[j])
                verts.append(tuple(pt))
                uvs.append((i / sides, j / (len(ps) - 1)))
                if weights_for:
                    weights.append(weights_for(tuple(pt)))
        for j in range(len(ps) - 1):
            for i in range(sides):
                a = j * sides + i
                b = j * sides + (i + 1) % sides
                faces.append((a, b, b + sides, a + sides))
        faces.extend([tuple(reversed(range(sides))), tuple(range((len(ps) - 1) * sides, len(ps) * sides))])
        return self.mesh(name, verts, faces, mat, bone, weights if weights_for else None, uv=uvs)

    def line(self, name, points, radius, mat, bone="head", blink=False):
        obj = self.tube(name, points, [radius] * len(points), [radius] * len(points), mat, bone, sides=6)
        if blink:
            obj.vertex_groups.new(name="Expression.Eyes").add(list(range(len(obj.data.vertices))), 1, "REPLACE")
        return obj

    def create_rig(self):
        arm = bpy.data.armatures.new("MiyoraHumanoidSkeleton")
        obj = bpy.data.objects.new("Rig", arm)
        bpy.context.collection.objects.link(obj)
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.mode_set(mode="EDIT")
        def add(name, head, tail, parent=None):
            b = arm.edit_bones.new(name)
            b.head, b.tail = head, tail
            if parent:
                b.parent = arm.edit_bones[parent]
            self.bones[name] = (head, tail)
        add("hips", (0, 0, .79), (0, 0, .92))
        add("spine", (0, 0, .92), (0, 0, 1.06), "hips")
        add("chest", (0, 0, 1.06), (0, 0, 1.22), "spine")
        add("neck", (0, 0, 1.22), (0, 0, 1.29), "chest")
        add("head", (0, 0, 1.29), (0, 0, 1.57), "neck")
        for side, sign in (("left", 1), ("right", -1)):
            add(side + "Shoulder", (sign * .03, 0, 1.19), (sign * .18, 0, 1.19), "chest")
            add(side + "UpperArm", (sign * .18, 0, 1.19), (sign * .40, 0, 1.19), side + "Shoulder")
            add(side + "LowerArm", (sign * .40, 0, 1.19), (sign * .60, 0, 1.19), side + "UpperArm")
            add(side + "Hand", (sign * .60, 0, 1.19), (sign * .68, 0, 1.19), side + "LowerArm")
            for finger, dz, length in (("Thumb", -.04, .046), ("Index", -.018, .065), ("Middle", 0, .072), ("Ring", .018, .063), ("Little", .035, .048)):
                base = .65 if finger == "Thumb" else .675
                for segment, portion in (("Proximal", 0), ("Intermediate", .40), ("Distal", .73)):
                    bn = side + finger + segment
                    parent = side + "Hand" if portion == 0 else side + finger + ("Proximal" if portion == .40 else "Intermediate")
                    add(bn, (sign * (base + length * portion), 0, 1.19 + dz), (sign * (base + length * min(1, portion + .4)), 0, 1.19 + dz), parent)
            add(side + "UpperLeg", (sign * .088, 0, .82), (sign * .09, -.006, .46), "hips")
            add(side + "LowerLeg", (sign * .09, -.006, .46), (sign * .09, 0, .10), side + "UpperLeg")
            add(side + "Foot", (sign * .09, 0, .10), (sign * .09, -.12, .055), side + "LowerLeg")
            add(side + "Toes", (sign * .09, -.12, .055), (sign * .09, -.17, .055), side + "Foot")
        bpy.ops.object.mode_set(mode="OBJECT")
        obj.show_in_front = True
        obj.select_set(False)
        self.rig = obj

    def torso_weights(self, v):
        z = v[2]
        if z < .92:
            return {"hips": 1}
        if z < 1.06:
            t = (z - .92) / .14
            return {"hips": (1 - t) * .3, "spine": 1 - .5 * t, "chest": .5 * t}
        return {"chest": 1}

    def anatomy(self):
        skin = self.material("skin", roughness=.58)
        self.loft("NeckAnatomy", [(0, 0, 1.18, .045, .041), (0, 0, 1.24, .039, .037), (0, 0, 1.30, .044, .042)], skin, "neck")
        self.ellipsoid("SculptedFace", (0, 0, self.face_z), (self.head_r, .116, .187), skin, segments=48, rings=32, sculpt=True)
        for side, sign in (("left", 1), ("right", -1)):
            arm_points = [(sign * x, 0, 1.19) for x in (.165, .20, .28, .37, .40, .44, .52, .59, .61)]
            def arm_weights(v, s=side):
                t = max(0, min(1, (abs(v[0]) - .36) / .08))
                return {s + "UpperArm": 1 - t, s + "LowerArm": t}
            self.tube(side + "ArmAnatomy", arm_points, [.047, .048, .041, .034, .032, .033, .028, .020, .019], [.048, .048, .040, .035, .034, .032, .026, .020, .019], skin, side + "UpperArm", sides=20, weights_for=arm_weights)
            if self.nezuko:
                arm_mesh = self.meshes[-1]
                import bmesh
                bm = bmesh.new(); bm.from_mesh(arm_mesh.data)
                bmesh.ops.delete(bm, geom=[v for v in bm.verts if abs(v.co.x) < .39], context="VERTS")
                bm.to_mesh(arm_mesh.data); bm.free()
            self.ellipsoid(side + "Palm", (sign * .646, 0, 1.19), (.048, .020, .043), skin, side + "Hand", segments=20, rings=12)
            for finger, dz, length in (("Thumb", -.04, .046), ("Index", -.018, .065), ("Middle", 0, .072), ("Ring", .018, .063), ("Little", .035, .048)):
                base = .65 if finger == "Thumb" else .675
                points = [(sign * (base + length * t), -.002, 1.19 + dz - .008 * math.sin(t * math.pi)) for t in (0, .25, .5, .75, 1)]
                def finger_weights(v, s=side, f=finger, b=base, length=length):
                    t = max(0, min(1, (abs(v[0]) - b) / length))
                    if t < .4:
                        blend = max(0, (t - .25) / .15)
                        return {s + f + "Proximal": 1 - blend, s + f + "Intermediate": blend}
                    blend = max(0, min(1, (t - .60) / .20))
                    return {s + f + "Intermediate": 1 - blend, s + f + "Distal": blend}
                self.tube(side + finger, points, [.008, .008, .0075, .006, .003], [.008, .008, .0075, .006, .003], skin, side + finger + "Proximal", sides=8, weights_for=finger_weights)
            def leg_weights(v, s=side):
                t = max(0, min(1, (v[2] - .42) / .08))
                return {s + "UpperLeg": t, s + "LowerLeg": 1 - t}
            self.loft(side + "LegAnatomy", [(sign * .09, 0, .10, .028, .032), (sign * .09, 0, .18, .032, .038), (sign * .09, .008, .29, .048, .046), (sign * .09, -.006, .43, .042, .042), (sign * .09, -.006, .49, .043, .044), (sign * .088, 0, .64, .063, .065), (sign * .088, 0, .80, .072, .072)], skin, side + "UpperLeg", count=24, weights_for=leg_weights)
            if self.nezuko:
                leg = self.meshes[-1]
                import bmesh
                bm = bmesh.new(); bm.from_mesh(leg.data)
                bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.z > .37], context="VERTS")
                bm.to_mesh(leg.data); bm.free()
            self.ellipsoid(side + "SockFoot", (sign * .09, -.058, .062), (.045, .106, .042), self.material("ivory", "#f7eee9"), side + "Foot", segments=24, rings=12)
            self.ellipsoid(side + "SandalSole", (sign * .09, -.06, .027), (.052, .113, .016), self.material("sole", "#61413b"), side + "Foot", segments=24, rings=10)
            self.line(side + "SandalStrap", [(sign * .13, -.02, .075), (sign * .09, -.105, .105), (sign * .05, -.02, .075)], .008, self.material("strap", "#b55055"), side + "Foot")

    def face(self):
        ink = self.material("eyeliner", "#321f30", roughness=.5)
        iris = self.material("eyes", roughness=.25)
        white = self.material("eyeWhite", "#fff7f3", roughness=.22)
        for sign in (-1, 1):
            x = sign * .064
            z = self.face_z + .018
            self.ellipsoid("EyeSclera", (x, -.103, z), (.049, .008, .032), white, blink=True, segments=28, rings=16)
            self.ellipsoid("IrisOuter", (x, -.112, z), (.023, .002, .028), ink, blink=True)
            self.ellipsoid("IrisColor", (x, -.114, z), (.021, .002, .026), iris, blink=True)
            self.ellipsoid("IrisLowerGlow", (x, -.116, z - .008), (.017, .001, .014), self.material("irisGlow", "#ffc4db" if self.nezuko else self.palette["accent"]), blink=True, segments=20, rings=12)
            self.ellipsoid("EyePupil", (x, -.118, z + .002), (.007, .001, .018), ink, blink=True, segments=20, rings=12)
            self.ellipsoid("EyeCatchlight", (x - .008, -.120, z + .013), (.006, .001, .008), white, blink=True, segments=12, rings=8)
            self.ellipsoid("EyeCatchlightSmall", (x + .010, -.120, z - .008), (.003, .001, .0035), white, blink=True, segments=10, rings=6)
            top = [(x + .049 * math.cos(a), -.112, z + .032 * math.sin(a)) for a in [math.pi * i / 16 for i in range(17)]]
            self.line("UpperLashes", top, .0020, ink, blink=True)
            lower = [(x + .048 * math.cos(a), -.109, z + .030 * math.sin(a)) for a in [math.pi + math.pi * i / 14 for i in range(15)]]
            self.line("LowerLid", lower, .0006, self.material("lid", "#ad6d77"), blink=True)
            for j in range(3):
                lx = x + sign * (.041 + .004 * j)
                self.line("LashWing", [(lx, -.113, z + .013 + j * .003), (lx + sign * .010, -.113, z + .022 + j * .003)], .0015, ink, blink=True)
            self.line("Eyebrow", [(x - .044, -.102, z + .069), (x - .013, -.115, z + .076), (x + .018, -.113, z + .075), (x + .043, -.101, z + .067)], .003, self.material("hair"))
            self.ellipsoid("Ear", (sign * .146, 0, self.face_z - .018), (.021, .026, .043), self.material("skin"), segments=20, rings=12)
            self.ellipsoid("EarConcha", (sign * .153, -.017, self.face_z - .019), (.011, .010, .027), self.material("earPink", "#d99396"), segments=16, rings=10)
        self.ellipsoid("NoseTip", (0, -.122, self.face_z - .038), (.007, .005, .009), self.material("skin"), segments=18, rings=10)
        self.line("UpperLip", [(-.023, -.109, self.face_z - .082), (0, -.116, self.face_z - .086), (.023, -.109, self.face_z - .082)], .0022, self.material("lips", "#ba6d81"))
        self.line("LowerLip", [(-.014, -.109, self.face_z - .089), (0, -.114, self.face_z - .092), (.014, -.109, self.face_z - .089)], .002, self.material("lipLight", "#efb2bb"))

    def hair(self):
        style = self.spec["hair"]
        if style == "none":
            return
        dark = self.material("hair", roughness=.38)
        cap = self.ellipsoid("HairCrown", (0, .006, self.face_z + .018), (self.head_r * 1.035, .125, .188), dark, segments=40, rings=22)
        # Cut the front lower half away; unlike a full sphere the cap cannot cover the face.
        remove = []
        for p in cap.data.polygons:
            c = p.center
            if c.y < -.055 and c.z < self.face_z + .105:
                remove.append(p.index)
        import bmesh
        bm = bmesh.new(); bm.from_mesh(cap.data)
        bm.faces.ensure_lookup_table()
        bmesh.ops.delete(bm, geom=[bm.faces[i] for i in remove], context="FACES")
        bm.to_mesh(cap.data); bm.free()
        long = style in ("long", "hime", "ponytail", "twintails", "side_tail")
        end = .64 if long else (1.12 if style == "bob" else 1.31)
        tip = self.spec.get("hairTip", self.palette["hair"])
        # Individual locks are sculpted curved ribbons with rounded cross-sections.
        for i in range(30 if long else 22):
            a = math.pi * i / ((30 if long else 22) - 1)
            x = math.cos(a) * self.head_r
            y = .03 + math.sin(a) * .096
            pts, widths, depths = [], [], []
            for j in range(11):
                t = j / 10
                spread = 1 + .52 * t
                wave = .027 * math.sin(t * 7 + a * 2) * t
                pts.append((x * spread + wave, y + .012 * math.sin(t * 5 + a), self.face_z + .113 - (self.face_z + .113 - end) * t))
                widths.append(.014 * (1 - t ** 2) + .0005)
                depths.append(.007 * (1 - t ** 2) + .0005)
            lock = self.tube(f"HairLock.{i:02}", pts, widths, depths, dark, sides=8)
            # Hair ombre uses material sections, retained in Blender and glTF.
            if tip != self.palette["hair"]:
                for k in range(1, 5):
                    base = color(self.palette["hair"])
                    target = color(tip)
                    m = self.material(f"HairOmbre{k}", tip, roughness=.42)
                    mix = (k / 4) ** 1.8
                    rgba = tuple(base[c] * (1 - mix) + target[c] * mix for c in range(3)) + (1,)
                    m.diffuse_color = rgba
                    m.node_tree.nodes.get("Principled BSDF").inputs["Base Color"].default_value = rgba
                    lock.data.materials.append(m)
                for p in lock.data.polygons:
                    t = sum(lock.data.vertices[v].co.z for v in p.vertices) / len(p.vertices)
                    fall = (self.face_z - t) / max(.01, self.face_z - end)
                    p.material_index = min(4, max(0, int((fall - .42) / .58 * 4)))
        for sign in (-1, 1):
            for i in range(7):
                pts, widths, depths = [], [], []
                for j in range(18):
                    t = j / 17
                    a = t * math.pi / 2
                    pts.append((sign * (.006 + .144 * math.sin(a) + i*.0015), -.008 - .125 * math.sin(a) * (1-.40*t), self.face_z + .188 * math.cos(a) + .008 - i*.002))
                    widths.append(.011 * (1-.6*t ** 3) + .0002)
                    depths.append(.004 * (1-.6*t ** 2) + .0002)
                self.tube("SweptFringe", pts, widths, depths, dark, sides=8)
            if long:
                for i in range(3):
                    xx = sign * (.143 + .008 * i)
                    points = [(xx, -.054, self.face_z + .08), (xx * 1.03, -.077, self.face_z -.08), (xx * 1.10, -.09, 1.19), (xx * 1.28, -.1, 1.05), (xx * 1.40, -.11, .91), (xx * 1.24, -.11, .78), (xx * 1.30, -.10, .70)]
                    self.tube("TempleLock", points, [.013, .014, .013, .013, .011, .008, .001], [.006, .007, .007, .007, .006, .004, .001], dark, sides=10)

    def fabric(self):
        mat = self.material("kimono", self.palette["primary"])
        size = 512
        image = bpy.data.images.new("AsanohaWovenSilk", width=size, height=size)
        base = color(self.palette["primary"])
        pixels = []
        for y in range(size):
            for x in range(size):
                u, v = (x % 64) / 64, (y % 64) / 64
                lines = min(abs(u - .5), abs(v - .5), abs(u - v) * .7, abs(u + v - 1) * .7)
                shade = .63 if lines < .017 else 1 + .035 * math.sin(x * .7) * math.cos(y * .7)
                pixels.extend([min(1, c * shade) for c in base[:3]] + [1])
        image.pixels.foreach_set(pixels)
        image.pack()
        tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
        tex.image = image
        mat.node_tree.links.new(tex.outputs["Color"], mat.node_tree.nodes.get("Principled BSDF").inputs["Base Color"])
        return mat

    def clothing(self):
        outfit = self.spec["outfit"]
        primary = self.fabric() if self.nezuko or outfit == "kimono" else self.material("primary")
        secondary = self.material("secondary")
        trim = self.material("trim", "#f8e5d8" if self.nezuko else self.palette["accent"])
        long = outfit in ("kimono", "robe", "dress", "coat", "labcoat", "hakama")
        bottom = .31 if long else .78
        self.loft("cloth.Tunic", [(0, 0, bottom, .185 if long else .14, .106), (0, 0, .55, .175 if long else .14, .104), (0, 0, .81, .15, .099), (0, 0, .93, .116, .086), (0, 0, 1.07, .148, .088), (0, 0, 1.18, .166, .082)], primary, count=40, weights_for=self.torso_weights)
        for sign, side in ((1, "left"), (-1, "right")):
            # Broad hanging sleeves are modeled as a closed cloth volume.
            if outfit in ("kimono", "robe", "hakama"):
                pts = [(sign * .16, 0, 1.15), (sign * .26, 0, 1.10), (sign * .40, 0, 1.09), (sign * .57, 0, 1.13)]
                self.tube("cloth.SilkSleeve", pts, [.075, .13, .14, .10], [.065, .067, .068, .061], primary, side + "UpperArm", sides=24)
            else:
                pts = [(sign * .165, 0, 1.19), (sign * .26, 0, 1.19), (sign * .38, 0, 1.19), (sign * .48, 0, 1.19), (sign * .57, 0, 1.19)]
                self.tube("cloth.Sleeve", pts, [.052, .052, .046, .041, .028], [.055, .053, .045, .039, .028], primary, side + "UpperArm", sides=18, weights_for=lambda v, s=side: {s + "UpperArm": max(0, min(1, (.46 - abs(v[0])) / .08)), s + "LowerArm": max(0, min(1, (abs(v[0]) - .38) / .08))})
            self.tube("CollarLapel", [(sign * .049, -.044, 1.245), (sign * .074, -.084, 1.17), (-sign * .045, -.095, 1.045)], [.009, .009, .009], [.002, .002, .002], trim, "chest", sides=4)
            if self.nezuko:
                # Open dark outer haori panels: the pink kimono stays visible between them.
                self.tube("cloth.HaoriSleeve", [(sign * .16, 0, 1.15), (sign * .26, 0, 1.10), (sign * .40, 0, 1.09), (sign * .565, 0, 1.13)], [.092, .143, .153, .112], [.082, .084, .085, .075], secondary, side + "UpperArm", sides=24)
        if self.nezuko:
            verts, faces, weights = [], [], []
            rows = [(1.20,.177,.094), (1.10,.172,.106), (.98,.154,.115), (.82,.17,.117), (.60,.191,.123), (.30,.208,.129)]
            count = 48
            for z, rx, ry in rows:
                for i in range(count + 1):
                    a = -math.pi/2 + .47 + (2*math.pi - .94) * i/count
                    v = (rx*math.cos(a), ry*math.sin(a), z)
                    verts.append(v); weights.append(self.torso_weights(v))
            for j in range(len(rows)-1):
                for i in range(count):
                    a=j*(count+1)+i
                    faces.append((a,a+1,a+count+2,a+count+1))
            jacket = self.mesh("cloth.OpenHaori", verts, faces, secondary, "hips", weights)
            solidify = jacket.modifiers.new("SilkThickness", "SOLIDIFY")
            solidify.thickness = .003
            bpy.context.view_layer.objects.active = jacket
            bpy.ops.object.modifier_apply(modifier=solidify.name)
        self.loft("ObiSash", [(0, -.001, .86, .147, .102), (0, -.001, .89, .147, .104), (0, -.001, .98, .132, .099), (0, -.001, 1.01, .133, .098)], self.material("obi", "#f5eee5" if self.nezuko else self.palette["secondary"]), "spine", count=40)
        for z in (.884, .957, .995):
            self.loft("ObiTrim", [(0, -.003, z, .148 - (z - .88) * .12, .105), (0, -.003, z + .010, .148 - (z - .88) * .12, .105)], self.material("obiRed", "#b44255" if self.nezuko else self.palette["accent"]), "spine", count=40)
        self.line("ObiCord", [(-.14, -.042, .935), (-.10, -.091, .935), (0, -.111, .935), (.10, -.091, .935), (.14, -.042, .935)], .006, self.material("obiCord", "#b96742" if self.nezuko else self.palette["accent"]), "spine")

    def signatures(self):
        if self.nezuko:
            bamboo = self.material("bamboo", "#81a84f", roughness=.42)
            points = [(-.066, -.146, self.face_z - .080), (.066, -.146, self.face_z - .080)]
            self.tube("BambooMuzzle", points, [.026, .026], [.026, .026], bamboo, sides=24)
            for x in (-.057, -.027, .027, .057):
                self.tube("BambooNode", [(x - .003, -.146, self.face_z - .080), (x + .003, -.146, self.face_z - .080)], [.029, .029], [.029, .029], self.material("bambooNode", "#4e7136"), sides=24)
            for sign in (-1, 1):
                self.line("BambooTie", [(sign * .066, -.141, self.face_z - .080), (sign * .12, -.084, self.face_z - .072), (sign * .15, -.015, self.face_z - .050), (sign * .13, .08, self.face_z - .035)], .003, self.material("cord", "#853047"))
            self.bow((.127, -.047, self.face_z + .142), .036)
        for accessory in self.spec["accessories"]:
            if accessory == "bow" and self.nezuko:
                continue
            self.accessory(accessory)

    def bow(self, center, size):
        mat = self.material("ribbon", "#df88a9" if self.nezuko else self.palette["accent"])
        x, y, z = center
        for sign in (-1, 1):
            self.tube("SilkRibbonLoop", [(x, y, z), (x + sign * size * .7, y + .005, z + size * .4), (x + sign * size, y, z + size * .9), (x + sign * size * .1, y -.004, z + size * .6), (x, y, z)], [.006, .012, .009, .007, .005], [.004, .005, .004, .004, .003], mat, sides=8)
            self.tube("SilkRibbonTail", [(x, y, z), (x + sign * size * .2, y, z - size), (x + sign * size * .35, y + .005, z - size * 1.8)], [.012, .010, .006], [.003, .003, .002], mat, sides=8)
        self.ellipsoid("RibbonKnot", center, (.009, .006, .010), mat, segments=12, rings=8)

    def accessory(self, kind):
        # Catalog-wide identity accessories use actual modeled geometry.
        mat = self.material("accent", metallic=.3, roughness=.4)
        if kind in ("bow", "flower"):
            self.bow((.135, -.03, 1.53), .027)
        elif kind in ("glasses", "goggles"):
            for sign in (-1, 1):
                pts = [(sign * .064 + .061 * math.cos(t * 2 * math.pi / 28), -.144, self.face_z + .018 + .046 * math.sin(t * 2 * math.pi / 28)) for t in range(29)]
                self.line("SpectacleFrame", pts, .0035 if kind == "glasses" else .008, mat)
            self.line("SpectacleBridge", [(-.010, -.14, 1.43), (.010, -.14, 1.43)], .003, mat)
        elif kind in ("headband", "beanie", "crown", "halo", "headphones", "witch_hat"):
            self.line("Headband", [(.164 * math.cos(t * math.pi / 28), .003, self.face_z + .17 * math.sin(t * math.pi / 28)) for t in range(29)], .009, mat)
            if kind == "headphones":
                for s in (-1, 1):
                    self.ellipsoid("HeadphoneCup", (s * .17, 0, 1.42), (.024, .044, .050), mat)
            if kind in ("witch_hat", "crown"):
                self.loft("Headwear", [(0, .012, 1.56, .20, .17), (0, .012, 1.58, .14, .12), (.02, .014, 1.72, .08, .07), (.05, .014, 1.81, .001, .001)], mat, "head")
        elif kind in ("katana", "wand", "staff"):
            self.tube("WeaponShaft", [(-.65, -.06, .74), (-.65, -.06, 1.65)], [.010, .007], [.010, .007], mat, "rightHand", sides=10)
            if kind == "katana":
                self.tube("SwordBlade", [(-.65, -.06, 1.20), (-.65, -.06, 1.72)], [.018, .001], [.004, .001], self.material("steel", "#c8d9e6", metallic=.75, roughness=.3), "rightHand", sides=4)
        elif kind in ("scarf", "bell"):
            self.loft("NeckAccessory", [(0, 0, 1.21, .073, .062), (0, 0, 1.255, .065, .055)], mat, "neck")
            self.tube("ScarfTail", [(.04, -.073, 1.23), (.10, -.10, 1.09), (.08, -.10, 1.01)], [.027, .027, .025], [.004, .004, .004], mat, "chest", sides=8)
        elif kind in ("backpack", "shell", "cape"):
            self.ellipsoid("BackAccessory", (0, .105, 1.02), (.14, .065, .20 if kind != "cape" else .31), mat, "chest")
        elif kind in ("gem", "kitsune_mask"):
            self.ellipsoid("ForeheadGem", (0, -.11, 1.51), (.022, .010, .031), mat, segments=8, rings=6)

    def finish_skin(self):
        bpy.ops.object.select_all(action="DESELECT")
        for obj in self.meshes:
            obj.select_set(True)
        bpy.context.view_layer.objects.active = self.meshes[0]
        bpy.ops.object.join()
        obj = bpy.context.object
        obj.name = self.spec["name"] + ".SkinnedMesh"
        # Transform data uniformly; skeleton uses identical coordinates.
        obj.parent = self.rig
        mod = obj.modifiers.new("DeformSkeleton", "ARMATURE")
        mod.object = self.rig
        self.rig.scale = (self.scale,) * 3
        basis = obj.shape_key_add(name="Basis")
        eye_group = obj.vertex_groups.get("Expression.Eyes")
        eye_indices = set()
        if eye_group:
            eye_indices = {v.index for v in obj.data.vertices if any(g.group == eye_group.index for g in v.groups)}
        blink = obj.shape_key_add(name="blink")
        for i in eye_indices:
            blink.data[i].co.z = (self.face_z + .018) + (basis.data[i].co.z - self.face_z - .018) * .04
        self.object = obj
        self.check_skin()

    def check_skin(self):
        valid = {g.index for g in self.object.vertex_groups if g.name in self.bones}
        for v in self.object.data.vertices:
            total = sum(g.weight for g in v.groups if g.group in valid)
            if abs(total - 1) > .001:
                raise ValueError(f"Invalid skin weights for vertex {v.index}: {total}")
            if not all(math.isfinite(c) for c in v.co):
                raise ValueError("Non-finite geometry")
        tri_count = sum(len(p.vertices) - 2 for p in self.object.data.polygons)
        if tri_count > MAX_TRIANGLES:
            raise ValueError(f"Triangle budget exceeded: {tri_count}")
        self.triangles = tri_count

    def motion(self, state, w):
        """One looping pose per state at phase w in [0, 2*pi] (integer frequencies only, so frame 49 == frame 1).
        Rotations are (pitch forward, yaw, side tilt) radians; arm goals (elbow, wrist, palm) are rest-space points
        for the left arm that follow the posed chest; arms() mirrors them for the right arm."""
        s = lambda k, phase=0: math.sin(k * w + phase)
        ease = (1 - math.cos(w)) / 2
        p = {"lift": 0, "sway": 0, "hips": (0, .03 * s(1), 0), "spine": (.012 * s(1), 0, 0), "chest": (.015 * s(2), 0, 0),
             "neck": (0, 0, 0), "head": (.02 * s(1), .10 * s(1), .03 * s(1, 1)), "lie": False, "ground": False,
             "arms": {sign: ((sign * .21, -.02, .98), (sign * .24, -.04 + .012 * s(1, sign), .78), (0, -.06, -1)) for sign in (1, -1)}}
        def arms(left):
            p["arms"] = {1: left, -1: tuple((-v[0], v[1], v[2]) for v in left)}
        if state == "idle":
            p.update(lift=-.006 * (1 - math.cos(2 * w)) / 2)
        elif state == "working":
            p.update(spine=(.06, 0, 0), chest=(.03 + .01 * s(2), 0, 0), head=(.16 + .02 * s(4), .04 * s(1), 0), hips=(0, 0, 0))
            p["arms"] = {sign: ((sign * .27, -.08, .98), (sign * .15, -.26, 1.00 + .014 * s(4, 0 if sign > 0 else math.pi)), (0, -1, -.25)) for sign in (1, -1)}
        elif state == "thinking":
            p.update(hips=(0, 0, .03 * s(1)), head=(.06, .14 + .05 * s(1), .14), sway=.012 * s(1))
            p["arms"] = {-1: ((-.30, -.08, 1.00), (-.12, -.16, 1.24 + .008 * s(2)), (.25, -.1, 1)),
                         1: ((.24, -.06, .98), (-.04, -.19, 1.00), (-1, -.1, .2))}
        elif state == "waiting":
            p.update(sway=.022 * s(1), hips=(0, 0, .05 * s(1)), head=(-.02, .28 * s(1), .05 * s(1)), chest=(.01 * s(2), .06 * s(1), 0))
            arms(((.22, .07, .98), (.10, .13, .86), (-1, .2, -.2)))
        elif state == "success":
            pump = (1 - math.cos(2 * w)) / 2
            p.update(lift=.03 * pump, head=(-.10, 0, .05 * s(2)), chest=(-.05, 0, 0))
            p["arms"] = {-1: ((-.30, -.06, 1.30), (-.24 - .04 * pump, -.08, 1.48 + .10 * pump), (0, 0, 1)),
                         1: ((.22, -.02, .98), (.23, -.05, .78), (0, -.06, -1))}
        elif state == "celebrating":
            hop = (1 - math.cos(4 * w)) / 2
            p.update(lift=.05 * hop, hips=(0, 0, .06 * s(2)), head=(-.12, 0, .10 * s(2)), chest=(-.06, 0, .05 * s(2)))
            p["arms"] = {sign: ((sign * .34, 0, 1.36), (sign * (.40 + .07 * s(2)), -.02, 1.58), (sign * .1, -.1, 1)) for sign in (1, -1)}
        elif state == "level_up":
            p.update(lift=.045 * ease, head=(-.22 * ease, 0, 0), chest=(-.10 * ease, 0, 0), spine=(-.04 * ease, 0, 0))
            p["arms"] = {sign: ((sign * (.21 + .08 * ease), -.02, .98 + .40 * ease), (sign * (.24 - .08 * ease), -.04, .78 + .86 * ease), (0, 0, -1 + 2 * ease)) for sign in (1, -1)}
        elif state == "achievement":
            p.update(chest=(-.07, 0, 0), head=(-.10 + .06 * max(0, s(2)), .05 * s(1), 0), hips=(0, .04 * s(1), 0))
            arms(((.32, .02, 1.00), (.13, -.04, .86 + .006 * s(2)), (-1, 0, -.4)))
        elif state == "error":
            p.update(spine=(-.05, 0, 0), head=(-.06, 0, .10 * s(5)), sway=-.01)
            arms(((.30, -.06, 1.16), (.14, -.10, 1.38 + .01 * s(5)), (-1, 0, .3)))
        elif state == "sleeping":
            # ground=True: the lowest vertex is clamped to the floor per key, measured on the evaluated mesh.
            breath = .025 * s(1)
            if self.body in ("slime", "bird"):
                # No foldable legs: settle in place and droop.
                p.update(ground=True, spine=(.10, 0, 0), chest=(.08 + breath, 0, 0), head=(.30, 0, .18))
                arms(((.21, -.06, .96), (.10, -.16, .86), (-1, -.1, -.3)))
            else:
                # Lie on the right side facing the viewer; garments are skinned rigidly to hips, so sitting would push hems through the floor.
                p.update(ground=True, lie=True, lift=-.55, hips=(0, 0, 1.45), spine=(.10, 0, 0), chest=(.06 + breath, 0, 0),
                         neck=(.05, 0, 0), head=(.15 + .02 * s(1), 0, .22))
                arms(((.22, -.14, 1.06), (.05, -.20, 1.24), (-1, 0, .3)))
        elif state == "hungry":
            p.update(spine=(.10, 0, 0), chest=(.04, 0, 0), head=(.14, .06 * s(1), .08), lift=-.015 * (1 - math.cos(2 * w)) / 2)
            arms(((.22, -.10, 1.00), (.06 + .025 * math.cos(w), -.20, .94 + .025 * s(1)), (-1, -.3, 0)))
        elif state == "eating":
            bite = (1 - math.cos(2 * w)) / 2
            p.update(head=(.10 - .06 * bite + .03 * s(6), 0, 0), chest=(.03, 0, 0))
            arms(((.24, -.12, 1.00), (.06, -.24 + .04 * bite, 1.10 + .20 * bite), (-1, -.4, .4)))
        elif state == "offline":
            p.update(spine=(.14, 0, 0), chest=(.06 + .02 * s(1), 0, 0), neck=(.10, 0, 0), head=(.42 + .03 * s(1), 0, .06 + .04 * s(1)), hips=(0, 0, .03 * s(1)), lift=-.02)
            arms(((.20, -.07, .97), (.15, -.12, .78), (0, -.2, -1)))
        elif state == "paused":
            p.update(head=(.04, .06 * s(1), .10 + .04 * s(1)), hips=(0, 0, .04), sway=.015)
            p["arms"] = {1: ((.27, -.12, 1.02), (-.06, -.22, 1.07 + .004 * s(2)), (-1, 0, .1)),
                         -1: ((-.27, -.10, .99), (.08, -.17, 1.03 + .004 * s(2)), (1, 0, -.1))}
        elif state == "attention_required":
            hop = (1 - math.cos(2 * w)) / 2
            p.update(lift=.02 * hop, spine=(-.03, 0, 0), head=(-.04, 0, .08 * s(3)), chest=(-.03, 0, .05 * s(3)))
            p["arms"] = {-1: ((-.30, -.06, 1.36), (-.32 + .10 * s(3), -.06, 1.56), (0, -.3, 1)),
                         1: ((.22, -.05, .99), (.18, -.10, .80), (0, -.06, -1))}
        return p

    def animations(self):
        # Ground the lowest vertex at z=0 so every species stands on the floor instead of hovering.
        self.rig.location.z = -min(v.co.z for v in self.object.data.vertices) * self.scale
        self.rig.animation_data_create()
        rig, bones = self.rig, self.rig.data.bones
        keyed = ["hips", "spine", "chest", "neck", "head"] + [side + part for side in ("left", "right") for part in ("UpperArm", "LowerArm", "Hand", "UpperLeg", "LowerLeg", "Foot")]
        for name in keyed:
            rig.pose.bones[name].rotation_mode = "QUATERNION"
        for state in STATES:
            action = bpy.data.actions.new(state)
            rig.animation_data.action = action
            last = {}
            for f in range(1, 50, 2):
                p = self.motion(state, (f - 1) / 48 * 2 * math.pi)
                self.pose(p, last)
                if p["ground"]:
                    bpy.context.view_layer.update()
                    ev = self.object.evaluated_get(bpy.context.evaluated_depsgraph_get())
                    co = np.empty(len(ev.data.vertices) * 3)
                    ev.data.vertices.foreach_get("co", co)
                    low = (np.array(ev.matrix_world)[2, :3] @ co.reshape(-1, 3).T).min() + ev.matrix_world[2][3]
                    p["lift"] -= low / self.scale
                    self.pose(p, last)
                for name in keyed:
                    rig.pose.bones[name].keyframe_insert(data_path="rotation_quaternion", frame=f, group=name)
                rig.pose.bones["hips"].keyframe_insert(data_path="location", frame=f, group="hips")
            track = self.rig.animation_data.nla_tracks.new()
            track.name = state
            track.strips.new(state, 1, action)
            track.mute = True
        self.rig.animation_data.action = None
        for track in self.rig.animation_data.nla_tracks:
            track.mute = True
        for pb in self.rig.pose.bones:
            pb.matrix_basis = Matrix.Identity(4)
        bpy.context.scene.frame_set(1)

    def pose(self, p, last):
        """Apply one motion() pose to the rig's pose bones (rig-local, rest-space coordinates)."""
        rig, bones = self.rig, self.rig.data.bones
        for pb in rig.pose.bones:
            pb.matrix_basis = Matrix.Identity(4)
        cache = {}
        def world(name):
            if name not in cache:
                bone, basis = bones[name], rig.pose.bones[name].matrix_basis
                cache[name] = bone.matrix_local @ basis if bone.parent is None else world(bone.parent.name) @ bone.parent.matrix_local.inverted() @ bone.matrix_local @ basis
            return cache[name]
        def rest_world(name):
            bone = bones[name]
            return world(bone.parent.name) @ bone.parent.matrix_local.inverted() @ bone.matrix_local
        def put(name, q, loc=Vector()):
            if name in last and last[name].dot(q) < 0:
                q = -q  # keep keys on one hemisphere so interpolation takes the short path
            last[name] = q
            pb = rig.pose.bones[name]
            pb.rotation_quaternion, pb.location = q, loc
            cache.pop(name, None)
        def aim(name, target):
            # Swing relative to the posed parent: no twist about the bone axis, and the head stays on its joint.
            base = rest_world(name)
            head, axis = base.translation, base.to_3x3() @ Vector((0, 1, 0))
            put(name, base.to_quaternion().inverted() @ axis.rotation_difference((target - head).normalized()) @ base.to_quaternion())
            return head
        put("hips", Euler(p["hips"]).to_quaternion(), bones["hips"].matrix_local.to_3x3().inverted() @ Vector((p["sway"], 0, p["lift"])))
        for name in ("spine", "chest", "neck", "head"):
            put(name, Euler(p[name]).to_quaternion())
        chest = world("chest") @ bones["chest"].matrix_local.inverted()
        hips = world("hips") @ bones["hips"].matrix_local.inverted()
        for side, sign in (("left", 1), ("right", -1)):
            length = lambda n: (Vector(self.bones[side + n][1]) - Vector(self.bones[side + n][0])).length
            elbow_goal, wrist_goal, palm = (Vector(v) for v in p["arms"][sign])
            shoulder = aim(side + "UpperArm", chest @ elbow_goal)
            elbow = shoulder + (chest @ elbow_goal - shoulder).normalized() * length("UpperArm")
            aim(side + "LowerArm", chest @ wrist_goal)
            wrist = elbow + (chest @ wrist_goal - elbow).normalized() * length("LowerArm")
            aim(side + "Hand", wrist + (chest.to_3x3() @ palm).normalized() * .08)
            # Two-bone leg IK: ankle stays planted (or rises with a hop); when lying, legs follow the hips with a slight knee bend.
            hip = rest_world(side + "UpperLeg").translation
            l1, l2 = length("UpperLeg"), length("LowerLeg")
            rest_ankle = Vector(self.bones[side + "Foot"][0])
            ankle = hips @ (rest_ankle + Vector((0, -.06, .06))) if p["lie"] else rest_ankle + Vector((0, 0, max(0, p["lift"])))
            pole = hips.to_3x3() @ Vector((0, -1, 0)) if p["lie"] else Vector((0, -1, 0))
            d = ankle - hip
            dist = min(d.length, l1 + l2 - 1e-4)
            along = (l1 * l1 + dist * dist - l2 * l2) / (2 * dist)
            bend = (pole - d.normalized() * pole.dot(d.normalized())).normalized()
            knee = hip + d.normalized() * along + bend * math.sqrt(max(0, l1 * l1 - along * along))
            aim(side + "UpperLeg", knee)
            aim(side + "LowerLeg", knee + (ankle - knee).normalized() * l2)
            toe = Vector(self.bones[side + "Foot"][1]) - rest_ankle
            aim(side + "Foot", rest_world(side + "Foot").translation + (hips.to_3x3() @ toe if p["lie"] else toe))

    def build(self):
        if self.body != "humanoid":
            raise ValueError(f"This pilot author currently supports humanoid anatomy, not {self.body}; refusing to substitute a human body")
        LOG.info("Authoring %s anatomy", self.spec["name"])
        self.anatomy()
        self.face()
        self.hair()
        self.clothing()
        self.signatures()
        self.finish_skin()
        self.animations()
        return self


def studio(author):
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 24
    scene.cycles.use_denoising = True
    scene.render.resolution_x = 960
    scene.render.resolution_y = 1080
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.view_settings.view_transform = "AgX"
    world = bpy.data.worlds.new("StudioWorld")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (.12, .10, .16, 1)
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = .4
    scene.world = world
    for name, location, power, size, tint in (("Key", (2, -3, 4), 500, 3, (1, .86, .82)), ("Fill", (-2, -1, 2), 240, 2, (.78, .85, 1)), ("Rim", (1, 2, 3), 700, 2, (1, .69, .75))):
        data = bpy.data.lights.new(name, "AREA")
        data.energy, data.shape, data.size, data.color = power, "DISK", size, tint
        obj = bpy.data.objects.new(name, data); bpy.context.collection.objects.link(obj)
        obj.location = location
        obj.rotation_euler = (Vector((0, 0, .9)) - obj.location).to_track_quat("-Z", "Y").to_euler()
    camera = bpy.data.objects.new("PreviewCamera", bpy.data.cameras.new("PreviewCamera"))
    bpy.context.collection.objects.link(camera)
    camera.location = (2.1, -4.3, 2.0)
    target = Vector((0, 0, .83 * author.scale))
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 1.90 * author.scale
    scene.camera = camera
    return camera


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--character", default="miyora.nezuko")
    parser.add_argument("--render", action="store_true")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
    logging.basicConfig(level=logging.DEBUG if args.verbose else logging.INFO, format="%(levelname)s %(message)s")
    characters = json.loads(args.catalog.read_text(encoding="utf-8"))["characters"]
    selected = [s for s in characters if s["id"] == args.character]
    if not selected:
        parser.error("Character ID not in catalog")
    spec = selected[0]
    output = args.output.resolve() / spec["id"].split(".")[-1]
    output.mkdir(parents=True, exist_ok=True)
    for name in ("source.blend", "model.glb", "validation.json"):
        if (output / name).exists():
            parser.error(f"Refusing to overwrite {output / name}")
    started = time.monotonic()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    author = Author(spec).build()
    studio(author)
    bpy.context.scene.render.fps = FPS
    bpy.context.scene.frame_end = 49
    bpy.ops.wm.save_as_mainfile(filepath=str(output / "source.blend"))
    bpy.ops.object.select_all(action="DESELECT")
    author.rig.select_set(True); author.object.select_set(True)
    bpy.context.view_layer.objects.active = author.rig
    bpy.ops.export_scene.gltf(filepath=str(output / "model.glb"), export_format="GLB", use_selection=True, export_skins=True, export_morph=True, export_animations=True, export_animation_mode="ACTIONS", export_nla_strips=True, export_force_sampling=False, export_extras=True, export_yup=True)
    model = output / "model.glb"
    if model.stat().st_size > MAX_FILE_BYTES:
        raise ValueError("Export exceeds Miyora file-size limit")
    report = {"schemaVersion": 1, "characterId": spec["id"], "status": "mesh-exported-pending-runtime-validation", "triangleCount": author.triangles, "boneCount": len(author.bones), "skinWeightsChecked": True, "animationNames": list(STATES), "sha256": hashlib.sha256(model.read_bytes()).hexdigest(), "elapsedSeconds": round(time.monotonic() - started, 2), "designRights": "Character identity rights are not asserted by this generated mesh", "generator": "local Blender surface authoring"}
    (output / "validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    (output / "character.json").write_text(json.dumps(spec, indent=2), encoding="utf-8")
    if args.render:
        # Show the relaxed pose, while keeping the exported bind pose as T-pose.
        author.rig.pose.bones["leftUpperArm"].rotation_quaternion = Euler((-1.34, 0, 0)).to_quaternion()
        author.rig.pose.bones["rightUpperArm"].rotation_quaternion = Euler((-1.34, 0, 0)).to_quaternion()
        bpy.context.scene.render.filepath = str(output / "preview.png")
        bpy.ops.render.render(write_still=True)
    LOG.info("Exported %s: %s triangles, %s bones", spec["id"], author.triangles, len(author.bones))


if __name__ == "__main__":
    try:
        main()
    except Exception:
        LOG.exception("Character authoring failed")
        raise
