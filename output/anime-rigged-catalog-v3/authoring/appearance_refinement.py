"""Local, geometry-based costume and species refinements for Blender authoring."""
import math
import bpy
import numpy as np

from author_characters import color


def texture_material(author, key, kind, first, second):
    mat = author.material(key, first, roughness=.65)
    size = 512
    yy, xx = np.indices((size, size), dtype=np.float32)
    u, v = xx / size, yy / size
    base, accent = np.array(color(first)), np.array(color(second))
    if kind == "check":
        mask = ((np.floor(u * 12) + np.floor(v * 3)) % 2).astype(bool)
    else:
        u, v = u * (2 * math.sqrt(3)), v * 3
        distance = np.full((size, size), 10, dtype=np.float32)
        radius = 1
        def segment(a, b):
            dx, dy = b[0] - a[0], b[1] - a[1]
            t = np.clip(((u - a[0]) * dx + (v - a[1]) * dy) / (dx * dx + dy * dy), 0, 1)
            np.minimum(distance, np.sqrt((u - a[0] - t * dx) ** 2 + (v - a[1] - t * dy) ** 2), out=distance)
        for row in range(-1, 4):
            for col in range(-1, 4):
                cx = (col + .5 * (row % 2)) * radius * math.sqrt(3)
                cy = row * radius * 1.5
                points = [(cx + radius * math.sin(i * math.pi / 3), cy + radius * math.cos(i * math.pi / 3)) for i in range(6)]
                for i, point in enumerate(points):
                    segment((cx, cy), point)
                    segment(point, points[(i + 1) % 6])
                    midway = ((point[0] + points[(i + 1) % 6][0]) / 2, (point[1] + points[(i + 1) % 6][1]) / 2)
                    segment((cx, cy), midway)
        mask = distance < .0018
    pixels = np.where(mask[..., None], accent, base).astype(np.float32)
    pixels[..., :3] *= (1 + .012 * np.sin(xx * 2.1) * np.cos(yy * 2.3))[..., None]
    pixels = np.clip(pixels, 0, 1)
    image = bpy.data.images.new(key + ".WovenTexture", width=size, height=size)
    image.pixels.foreach_set(pixels.ravel())
    image.pack()
    node = mat.node_tree.nodes.new("ShaderNodeTexImage")
    node.image = image
    mat.node_tree.links.new(node.outputs["Color"], mat.node_tree.nodes.get("Principled BSDF").inputs["Base Color"])
    return mat


def footwear(author):
    if author.body in ("bird", "slime"):
        return
    if author.slug == "luffy":
        for obj in author.meshes:
            if "SockFoot" in obj.name:
                obj.data.materials[0] = author.material("skin")
            elif "SandalStrap" in obj.name:
                obj.data.materials[0] = author.material("PirateSandalStrap", "#6b4f32")
        return
    traditional = author.spec["outfit"] in ("kimono", "hakama", "robe")
    bare = author.slug in ("momiji", "krishna", "radha", "rama", "sita", "hanuman", "ganesha", "shiva", "parvati", "lakshmi", "saraswati")
    animal = author.body == "beast" and author.spec["outfit"] == "fur"
    if traditional and not bare and not animal:
        return
    author.discard([o for o in author.meshes if "SandalStrap" in o.name])
    if bare or animal:
        author.discard([o for o in author.meshes if "SandalSole" in o.name])
    upper = author.material("FootUpper", author.palette["skin"] if bare else author.palette["primary"] if animal else "#252632" if author.spec["outfit"] in ("suit", "armor", "labcoat") else author.palette["secondary"])
    sole = author.material("FootSole", "#dedbd9" if author.spec["outfit"] in ("hoodie", "jacket", "overalls") else "#28242d")
    for obj in author.meshes:
        if "SockFoot" in obj.name:
            obj.data.materials[0] = upper
        elif "SandalSole" in obj.name:
            obj.data.materials[0] = sole
    for side, sign in (("left", 1), ("right", -1)):
        if bare or animal:
            for toe in range(5 if bare else 3):
                x = sign * .09 + (toe - (2 if bare else 1)) * (.014 if bare else .025)
                author.ellipsoid("ModeledToe", (x, -.149, .060), (.009 if bare else .015, .023, .013), upper, side + "Foot", segments=12, rings=8)
        elif author.spec["outfit"] in ("hoodie", "jacket", "overalls"):
            for row in range(4):
                y = -.092 + row * .014
                author.line("SneakerLace", [(sign * .09 - .026, y, .091), (sign * .09 + .026, y + .009, .091)], .0019, author.material("ShoeLace", "#eee6dd"), side + "Foot")
        else:
            author.loft("AnkleCuff", [(sign * .09, 0, .083, .036, .042), (sign * .09, 0, .17, .034, .038)], upper, side + "LowerLeg", count=24)


def nezuko_details(author):
    for obj in author.meshes:
        if obj.name.startswith("cloth.Tunic") and obj.data.uv_layers.active:
            for loop in obj.data.loops:
                vertex = obj.data.vertices[loop.vertex_index].co
                uv = obj.data.uv_layers.active.data[loop.index].uv
                uv.x *= 6
                uv.y = (vertex.z - .31) / .87 * 5
    checker = texture_material(author, "CheckedObi", "check", "#f8eee8", "#b64255")
    for obj in author.meshes:
        if obj.name.startswith("ObiSash"):
            obj.data.materials[0] = checker
    author.discard([o for o in author.meshes if o.name.startswith(("ObiTrim", "BambooNode"))])
    green = author.material("ObiageSilk", "#8fbb82")
    author.loft("FoldedObiage", [(0, -.001, 1.004, .136, .102), (0, -.001, 1.026, .136, .102)], green, "spine", count=48)
    for x in (-.055, .055):
        author.tube("BambooNode", [(x - .002, -.146, author.face_z - .080), (x + .002, -.146, author.face_z - .080)], [.027] * 2, [.027] * 2, author.material("bambooNode", "#587c3d"), sides=32)
    author.ellipsoid("ObijimeKnot", (0, -.119, .935), (.012, .006, .011), author.material("ObiKnot", "#be7a39"), "spine", segments=24, rings=12)
    for sign in (-1, 1):
        author.line("ObijimeLoop", [(sign * .007, -.122, .938), (sign * .020, -.122, .944), (sign * .021, -.122, .932), (sign * .007, -.122, .936)], .0022, author.material("ObiKnot"), "spine")


def open_coat(author, mat, bottom, top=1.21):
    vertices, faces, weights, uv = [], [], [], []
    count = 48
    for j in range(12):
        t = j / 11
        z = bottom + (top - bottom) * t
        rx = .19 * (1 - t) + .168 * t
        ry = .118 * (1 - t) + .09 * t
        for i in range(count + 1):
            angle = -math.pi / 2 + .58 + (math.tau - 1.16) * i / count
            point = (rx * math.cos(angle), ry * math.sin(angle), z)
            vertices.append(point)
            weights.append(author.torso_weights(point))
            uv.append((i / count * 3, t * 3))
    for j in range(11):
        for i in range(count):
            a = j * (count + 1) + i
            faces.append((a, a + 1, a + count + 2, a + count + 1))
    obj = author.mesh("cloth.OpenTailoredCoat", vertices, faces, mat, "hips", weights, uv=uv)
    modifier = obj.modifiers.new("FinishedClothThickness", "SOLIDIFY")
    modifier.thickness = .003
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=modifier.name)


def costume(author):
    if author.slug not in ("tanjiro", "luffy"):
        return False
    dark = author.material("UniformDark", "#25252d")
    skin = author.material("skin")
    if author.slug == "tanjiro":
        author.pants(dark)
        author.loft("DemonSlayerUniform", [(0, 0, .79, .143, .10), (0, 0, .96, .125, .088), (0, 0, 1.18, .164, .086)], dark, weights_for=author.torso_weights)
        author.loft("UniformCollar", [(0, 0, 1.185, .050, .043), (0, 0, 1.245, .046, .04)], dark, "chest", count=32)
        check = texture_material(author, "CheckeredHaori", "check", "#2b845d", "#172a26")
        open_coat(author, check, .47)
        for side, sign in (("left", 1), ("right", -1)):
            author.hanging_sleeve(side, sign, check, dark)
            for z in (.14, .18, .22, .26, .30):
                author.loft("ShinWrap", [(sign * .09, 0, z, .041, .044), (sign * .09, 0, z + .014, .041, .044)], author.material("WrapIvory", "#e0d8c9"), side + "LowerLeg", count=24)
        author.loft("UniformBelt", [(0, 0, .86, .147, .103), (0, 0, .895, .147, .103)], author.material("WrapIvory"), "hips", count=40)
        for z in (.95, 1.045, 1.14):
            author.ellipsoid("UniformButton", (0, -.096, z), (.005, .002, .005), author.material("UniformButtonGold", "#c5b993", metallic=.55), "chest", segments=12, rings=8)
    else:
        denim = author.material("Denim", "#3787b5")
        for side, sign in (("left", 1), ("right", -1)):
            author.loft("DenimShorts", [(sign * .09, 0, .49, .066, .066), (sign * .09, 0, .65, .076, .076), (sign * .085, 0, .84, .079, .078)], denim, side + "UpperLeg", count=28)
            author.loft("ShortsRolledHem", [(sign * .09, 0, .48, .069, .070), (sign * .09, 0, .52, .069, .070)], author.material("DenimHem", "#e5d9c6"), side + "UpperLeg", count=28)
        author.loft("BareTorso", [(0, 0, .80, .137, .092), (0, 0, .96, .117, .082), (0, 0, 1.10, .145, .084), (0, 0, 1.195, .160, .078)], skin, weights_for=author.torso_weights)
        open_coat(author, author.material("PirateVest", "#b7202f"), .83)
        scar = author.material("ChestScar", "#ac765f")
        for sign in (-1, 1):
            author.line("ChestCrossScar", [(-.055, -.091, 1.08 + sign * .032), (.055, -.091, 1.08 - sign * .032)], .003, scar, "chest")
    return True


def surface_details(author):
    seam = author.material("Stitching", author.palette["accent"])
    if author.body == "humanoid":
        for obj in list(author.meshes):
            if not obj.name.startswith(("cloth.Tunic", "cloth.dress", "cloth.coat", "cloth.hoodie", "cloth.jacket")):
                continue
            low = min(v.co.z for v in obj.data.vertices)
            points = [v.co.copy() for v in obj.data.vertices if abs(v.co.z - low) < .001]
            if len(points) > 8:
                points.sort(key=lambda p: math.atan2(p.y, p.x))
                points = [(p.x * 1.006, p.y * 1.006, p.z + .008) for p in points]
                author.line("SewnGarmentHem", points + points[:1], .0012, seam, "hips")
        if author.spec["outfit"] in ("hoodie", "jacket", "labcoat"):
            for side, sign in (("left", 1), ("right", -1)):
                for x in (.560, .568, .576):
                    points = [(sign * x, .031 * math.sin(i * math.tau / 24), 1.19 + .032 * math.cos(i * math.tau / 24)) for i in range(25)]
                    author.line("RibbedCuff", points, .0012, seam, side + "LowerArm")
        if author.slug == "gojo":
            author.discard([o for o in author.meshes if o.name.startswith(("Tie", "JacketLapel"))])
            author.loft("HighUniformCollar", [(0, 0, 1.18, .058, .047), (0, 0, 1.275, .051, .044)], author.material("HighCollar", "#1c202c"), "neck", count=40)
        if author.slug in ("naruto", "sasuke", "goku", "yuji"):
            author.loft("SignatureWaistBelt", [(0, 0, .85, .151, .104), (0, 0, .895, .151, .104)], author.material("SignatureBelt", "#7958a3" if author.slug == "sasuke" else author.palette["secondary"]), "hips", count=40)
        if author.slug == "denji":
            author.line("ChestPullCord", [(0, -.105, 1.15), (0, -.109, 1.09)], .003, author.material("PullCord", "#33313b"), "chest")
            author.ellipsoid("StarterHandle", (0, -.111, 1.09), (.018, .006, .007), author.material("PullCord"), "chest", segments=16, rings=8)
    elif author.body == "bird":
        for side, sign in (("left", 1), ("right", -1)):
            for i in range(10):
                points = [(sign * (.19 + i * .022), -.012, 1.19), (sign * (.23 + i * .028), -.014, 1.10 - i * .012), (sign * (.25 + i * .033), -.002, .97 - i * .015)]
                author.line("FeatherRachis", points, .0013, seam, side + ("UpperArm" if i < 5 else "LowerArm"))
    elif author.body == "robot":
        metal = author.material("RecessedConsole", "#303b4b", metallic=.65)
        author.loft("ChestConsole", [(0, -.11, .92, .081, .009), (0, -.11, 1.10, .081, .009)], metal, "chest", count=8)
        for x in (-.060, .060):
            for z in (.94, 1.08):
                author.ellipsoid("PanelFastener", (x, -.120, z), (.004, .002, .004), author.material("Fastener", "#b3bdcb", metallic=.8), "chest", segments=12, rings=8)
        for z in (.985, 1.01, 1.035):
            author.line("ConsoleSlot", [(-.043, -.121, z), (.043, -.121, z)], .002, seam, "chest")
    elif author.body == "beast" and author.spec["outfit"] == "fur":
        fur = author.material("SculptedFur", author.palette["secondary"])
        for sign in (-1, 1):
            for i in range(7):
                author.tube("ChestFurTuft", [(sign * (.02 + i * .015), -.084, 1.15), (sign * (.025 + i * .017), -.10, 1.09), (sign * (.031 + i * .017), -.091, 1.04 - .012 * i)], [.014, .016, .0007], [.006, .008, .0007], fur, "chest", sides=8)
