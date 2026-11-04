"""Local catalog surface modeling, full anime humanoid proportions, and rig export."""
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
from author_characters import Author, FPS, STATES, studio
from package_vrm import package

LOG = logging.getLogger("miyora.catalog-author")
FULL_ANIME_HEAD_FACTOR = .62
FULL_ANIME_HEAD_LIFT = .06
HAND_FACTOR = .78


class CatalogAuthor(Author):
    def discard(self, objects):
        for obj in list(objects):
            self.meshes.remove(obj)
            bpy.data.objects.remove(obj, do_unlink=True)

    def head_shape(self):
        factor = FULL_ANIME_HEAD_FACTOR if self.body == "humanoid" else (1 if self.body == "slime" else .86)
        lift = -.025 if self.body == "humanoid" else (0 if self.body == "slime" else FULL_ANIME_HEAD_LIFT)
        old_center = self.face_z + .018
        for obj in self.meshes:
            names = {g.name for g in obj.vertex_groups}
            if "head" not in names or any(b in names for b in self.bones if b != "head"):
                continue
            for v in obj.data.vertices:
                v.co.x *= factor
                v.co.y *= factor
                # Preserve long hair length while resizing the skull and facial details.
                z = v.co.z
                if z > 1.29:
                    v.co.z = 1.29 + (z - 1.29) * factor + lift
                elif obj.name.startswith(("HairLock", "TempleLock")):
                    v.co.z += lift * max(0, min(1, (z - 1.10) / .19))
                else:
                    v.co.z = 1.29 + (z - 1.29) * factor + lift
        self.face_z = 1.29 + (old_center - 1.29) * factor + lift - .018
        if self.body == "slime":
            for obj in self.meshes:
                if obj.vertex_groups.get("head") and not any(g.name in self.bones and g.name != "head" for g in obj.vertex_groups):
                    for v in obj.data.vertices:
                        if obj.name.startswith(("Eye", "Iris", "UpperLashes", "LowerLid", "LashWing")):
                            v.co.y -= .27
                        elif not obj.name.startswith("Slime"):
                            v.co.z -= .10
        if self.body != "humanoid":
            return
        fingers = ("Thumb", "Index", "Middle", "Ring", "Little")
        for obj in self.meshes:
            if any(obj.name.startswith(side + part) for side in ("left", "right") for part in ("Palm",) + fingers):
                sign = 1 if obj.name.startswith("left") else -1
                center = Vector((sign * .60, 0, 1.19))
                for v in obj.data.vertices:
                    v.co = center + (v.co - center) * HAND_FACTOR
        bpy.context.view_layer.objects.active = self.rig
        self.rig.select_set(True)
        bpy.ops.object.mode_set(mode="EDIT")
        for side, sign in (("left", 1), ("right", -1)):
            center = Vector((sign * .60, 0, 1.19))
            for b in self.rig.data.edit_bones:
                if b.name == side + "Hand":
                    b.tail = center + (b.tail - center) * HAND_FACTOR
                elif any(b.name.startswith(side + f) for f in fingers):
                    b.head = center + (b.head - center) * HAND_FACTOR
                    b.tail = center + (b.tail - center) * HAND_FACTOR
        bpy.ops.object.mode_set(mode="OBJECT")
        self.rig.select_set(False)

    def hair(self):
        style = self.spec["hair"]
        if style in ("none", "long", "hime", "bob"):
            return super().hair()
        if style in ("ponytail", "twintails", "side_tail", "bun", "curly"):
            self.spec["hair"] = "short"
            super().hair()
            self.spec["hair"] = style
            mat = self.material("hair")
            signs = (-1, 1) if style == "twintails" else (1,)
            for sign in signs:
                x = sign * (.145 if style in ("twintails", "side_tail") else .025)
                if style == "bun":
                    self.ellipsoid("HairBun", (x, .11, 1.57), (.075, .070, .072), mat)
                else:
                    for lock in range(9):
                        points = [(x + .015 * math.sin(lock), .10 + lock*.003, 1.55-t*.62) for t in (0, .2, .4, .6, .8, 1)]
                        if style == "curly":
                            points = [(p[0]+.035*math.sin(i*2+lock),p[1]+.025*math.cos(i*2+lock),p[2]) for i,p in enumerate(points)]
                        self.tube("PonytailLock", points, [.020,.023,.024,.020,.014,.001], [.014,.016,.015,.012,.008,.001], mat)
            return
        self.spec["hair"] = "short"
        super().hair()
        self.spec["hair"] = style
        mat = self.material("hair")
        count = 15 if style == "spiky" else 9
        for i in range(count):
            a = 2*math.pi*i/count
            x = .13*math.cos(a)
            y = .10*math.sin(a)
            if style == "mohawk":
                x, y = 0, -.09 + i*.022
            self.tube("StyledHairSpike", [(x*.5,y*.5,1.55),(x,y,1.61),(x*1.35,y*1.35,1.68+.025*math.sin(i))], [.031,.026,.0005], [.021,.017,.0005], mat)

    def face(self):
        super().face()
        if self.body != "humanoid" or self.spec["ears"] != "human":
            self.discard([o for o in self.meshes if o.name.startswith(("EarConcha", "Ear.")) or o.name == "Ear"])
        if self.body in ("beast", "bird", "robot", "slime"):
            self.discard([o for o in self.meshes if o.name.startswith(("NoseTip", "UpperLip", "LowerLip"))])
        if self.body == "beast":
            self.ellipsoid("AnimalMuzzle", (0,-.112,1.35), (.085,.056,.047), self.material("muzzle", self.palette["secondary"]))
            self.ellipsoid("AnimalNose", (0,-.169,1.373), (.021,.014,.014), self.material("nose", "#251d29"))
        elif self.body == "bird":
            self.tube("ModeledBeak", [(0,-.104,1.36),(0,-.16,1.35),(0,-.22,1.33)], [.038,.029,.001], [.029,.025,.001], self.material("beak",self.palette["accent"]), sides=12)
        if self.spec["eyes"] in ("sharp", "sleepy"):
            for obj in self.meshes:
                if obj.vertex_groups.get("Expression.Eyes"):
                    for v in obj.data.vertices:
                        v.co.z = self.face_z+.018+(v.co.z-self.face_z-.018)*.70
        if self.spec["eyes"] == "mono":
            for obj in self.meshes:
                if obj.name.startswith(("Eye", "Iris", "UpperLashes", "LowerLid", "LashWing")):
                    for v in obj.data.vertices:
                        v.co.x *= .30
        if self.spec["eyes"] == "visor":
            self.discard([o for o in self.meshes if o.vertex_groups.get("Expression.Eyes")])
            self.ellipsoid("VisorGlass",(0,-.11,self.face_z+.018),(.13,.012,.046),self.material("visor","#172335",metallic=.6,roughness=.25))
            for sign in (-1,1):
                self.ellipsoid("VisorEye",(sign*.057,-.124,self.face_z+.020),(.030,.003,.008),self.material("visorLight",self.palette["eyes"]),blink=True,segments=20,rings=10)

    def fabric(self):
        if self.nezuko:
            return super().fabric()
        mat=self.material("kimono",self.palette["primary"])
        pattern=self.spec.get("pattern","none")
        if pattern=="none":
            return mat
        from author_characters import color
        size=256;pixels=[];base=color(self.palette["primary"]);accent=color(self.palette["accent"])
        image=bpy.data.images.new("WovenFabric."+pattern,width=size,height=size)
        for y in range(size):
            for x in range(size):
                u,v=(x%32)/32-.5,(y%32)/32-.5
                radius=math.hypot(u,v);angle=math.atan2(v,u)
                marks={"dots":radius<.15,"stripes":abs(u)<.07,"diamonds":abs(abs(u)+abs(v)-.32)<.025,"waves":abs(v-.20*math.sin(u*math.tau))<.035,"petals":radius<.28*(.6+.4*math.cos(angle*5)),"stars":radius<.26*(.65+.35*math.cos(angle*5))}
                rgb=accent if marks[pattern] else base
                pixels.extend(rgb)
        image.pixels.foreach_set(pixels);image.pack()
        tex=mat.node_tree.nodes.new("ShaderNodeTexImage");tex.image=image
        mat.node_tree.links.new(tex.outputs["Color"],mat.node_tree.nodes.get("Principled BSDF").inputs["Base Color"])
        return mat

    def pants(self, mat):
        for side, sign in (("left",1),("right",-1)):
            def weights(v, s=side):
                t=max(0,min(1,(v[2]-.42)/.09))
                return {s+"UpperLeg":t,s+"LowerLeg":1-t}
            self.loft("cloth.Trousers", [(sign*.09,0,.12,.036,.039),(sign*.09,0,.30,.053,.052),(sign*.09,-.006,.46,.050,.048),(sign*.088,0,.65,.071,.073),(sign*.085,0,.83,.079,.079)], mat, side+"UpperLeg", count=24,weights_for=weights)
            self.line("TrouserSeam", [(sign*.16,.01,.80),(sign*.157,.01,.62),(sign*.140,.01,.46),(sign*.143,.01,.30),(sign*.126,.01,.13)], .0014, self.material("seam", self.palette["accent"]),side+"UpperLeg")

    def clothing(self):
        if self.nezuko or self.spec["outfit"] in ("kimono","robe","hakama"):
            return super().clothing()
        outfit=self.spec["outfit"]
        primary=self.material("primary",metallic=.6 if outfit in ("armor","plating") else 0)
        secondary=self.material("secondary")
        accent=self.material("accent",metallic=.4 if outfit=="armor" else 0)
        if outfit in ("fur","plating"):
            self.loft("BodySurface",[(0,0,.77,.135,.095),(0,0,.85,.145,.105),(0,0,.95,.125,.088),(0,0,1.09,.156,.103),(0,0,1.19,.166,.081)],primary,weights_for=self.torso_weights)
            if outfit=="fur":
                self.ellipsoid("ChestPelt",(0,-.087,1.04),(.09,.025,.132),secondary,"chest")
            else:
                for z in (.87,.95,1.04,1.13):
                    self.loft("RobotTorsoPlate",[(0,-.003,z,.152,.105),(0,-.003,z+.045,.155,.107)],primary,"chest",count=8)
                    self.line("PanelLight",[(-.07,-.111,z+.023),(.07,-.111,z+.023)],.003,accent,"chest")
            return
        long=outfit in ("dress","coat","labcoat")
        if outfit not in ("dress","sailor"):
            self.pants(secondary)
        bottom=.34 if long or outfit=="sailor" else .79
        self.loft("cloth."+outfit,[(0,0,bottom,.195 if long else .150,.105),(0,0,.83,.151,.103),(0,0,.94,.123,.089),(0,0,1.07,.151,.090),(0,0,1.18,.168,.085)],primary,count=36,weights_for=self.torso_weights)
        for side,sign in (("left",1),("right",-1)):
            self.tube("cloth.FittedSleeve",[(sign*.17,0,1.19),(sign*.28,0,1.19),(sign*.39,0,1.19),(sign*.49,0,1.19),(sign*.58,0,1.19)],[.056,.052,.046,.038,.027],[.058,.055,.048,.041,.029],primary,side+"UpperArm",sides=18,weights_for=lambda v,s=side:{s+"UpperArm":max(0,min(1,(.45-abs(v[0]))/.08)),s+"LowerArm":max(0,min(1,(abs(v[0])-.37)/.08))})
        if outfit=="hoodie":
            self.ellipsoid("Hood",(0,.052,1.215),(.094,.065,.068),primary,"neck")
            self.ellipsoid("KangarooPocket",(0,-.095,.945),(.085,.010,.050),secondary,"spine")
            for s in (-1,1):
                self.line("HoodDrawstring",[(s*.035,-.07,1.20),(s*.041,-.10,1.12),(s*.037,-.10,1.06)],.0025,accent,"chest")
        if outfit in ("jacket","suit","coat","labcoat"):
            for s in (-1,1):
                self.tube("JacketLapel",[(s*.045,-.055,1.23),(s*.105,-.092,1.13),(s*.040,-.099,1.015)],[.023,.026,.009],[.002,.002,.002],secondary,"chest",sides=4)
                self.ellipsoid("Pocket",(s*.092,-.087,.94),(.029,.004,.032),secondary,"spine",segments=16,rings=8)
            for z in (.91,1.00,1.08):
                self.ellipsoid("Button",(.018,-.101,z),(.004,.002,.004),accent,"spine",segments=10,rings=6)
        if outfit=="suit":
            self.tube("Tie",[(0,-.077,1.22),(0,-.105,1.17),(0,-.104,1.04)],[.013,.016,.019],[.003,.003,.002],accent,"chest",sides=4)
        if outfit=="overalls":
            self.ellipsoid("OverallBib",(0,-.089,1.02),(.085,.014,.125),secondary,"chest")
            for s in (-1,1):
                self.tube("OverallStrap",[(s*.067,-.10,1.02),(s*.069,-.085,1.19),(s*.069,.065,1.19)],[.014]*3,[.003]*3,secondary,"chest",sides=4)
        if outfit=="sailor":
            for s in (-1,1):
                self.tube("SailorCollar",[(s*.05,-.06,1.23),(s*.12,-.08,1.16),(0,-.103,1.07)],[.025,.023,.005],[.003]*3,secondary,"chest",sides=4)
            self.ellipsoid("SailorKnot",(0,-.11,1.09),(.015,.010,.020),accent,"chest")
        if outfit=="armor":
            for s in (-1,1):
                side="left" if s==1 else "right"
                self.ellipsoid("Pauldron",(s*.20,0,1.20),(.075,.075,.065),accent,side+"UpperArm",segments=16,rings=10)
            for z in (.88,.94,1.0):
                self.loft("ArmoredWaist",[(0,0,z,.15,.10),(0,0,z+.035,.152,.103)],secondary,"spine",count=12)

    def accessory(self, kind):
        accent=self.material("accent",metallic=.3)
        if self.body=="slime" and kind=="headphones":
            self.line("SlimeHeadphoneBand",[(.55*math.cos(i*math.pi/32),0,1.08+.42*math.sin(i*math.pi/32)) for i in range(33)],.012,accent)
            for sign in (-1,1):
                self.ellipsoid("SlimeHeadphoneCup",(sign*.55,0,1.10),(.044,.080,.080),accent)
        elif kind=="bow" and self.slug in ("arjuna","rama"):
            points=[(-.66-.13*math.sin(i*math.pi/24),0, .84+i*.70/24) for i in range(25)]
            self.line("ArchersBow",points,.008,accent,"rightHand")
            self.line("BowString",[points[0],points[-1]],.001,self.material("ivory","#f7eee9"),"rightHand")
        elif kind=="beanie":
            self.ellipsoid("KnitBeanie",(0,.005,1.55),(.17,.14,.11),accent)
            self.loft("BeanieRibbing",[(0,0,1.51,.17,.14),(0,0,1.54,.17,.14)],self.material("secondary"),"head",count=32)
        elif kind=="flower":
            for i in range(5):
                a=i*math.tau/5
                self.ellipsoid("FlowerPetal",(.14+.022*math.cos(a),-.05,1.54+.022*math.sin(a)),(.018,.005,.012),accent,segments=16,rings=8)
            self.ellipsoid("FlowerCenter",(.14,-.057,1.54),(.009,.004,.009),self.material("gold","#d8ac4d"))
        elif kind=="crown":
            self.loft("CrownBand",[(0,0,1.55,.16,.13),(0,0,1.58,.16,.13)],accent,"head",count=24)
            for i in range(7):
                a=math.tau*i/7
                self.tube("CrownPoint",[(.15*math.cos(a),.12*math.sin(a),1.57),(.16*math.cos(a),.13*math.sin(a),1.65)],[.020,.001],[.016,.001],accent)
        elif kind=="halo":
            self.line("Halo",[(.19*math.cos(i*math.tau/48),.15*math.sin(i*math.tau/48),1.68) for i in range(49)],.008,accent)
        elif kind=="bell":
            self.ellipsoid("Bell",(0,-.08,1.20),(.026,.020,.026),self.material("gold","#d8ac4d",metallic=.7),"neck")
            self.line("BellSlot",[(-.015,-.101,1.196),(.015,-.101,1.196)],.002,self.material("ink","#30252f"),"neck")
        elif kind=="cape":
            verts=[];faces=[]
            for j in range(12):
                t=j/11
                for i in range(15):
                    u=i/14*2-1
                    verts.append((u*(.12+.10*t),.085+.09*t+.008*math.cos(u*math.pi*5),1.20-.76*t))
            for j in range(11):
                for i in range(14):
                    a=j*15+i;faces.append((a,a+1,a+16,a+15))
            self.mesh("cloth.Cape",verts,faces,accent,"chest")
        elif kind=="kitsune_mask":
            mask=self.ellipsoid("KitsuneMask",(.16,-.013,1.47),(.06,.025,.085),self.material("ivory","#f7eee9"))
            for s in (-1,1):
                self.line("MaskMark",[(.16+s*.015,-.04,1.49),(.16+s*.039,-.04,1.48)],.003,self.material("red","#a73848"))
        else:
            super().accessory(kind)

    def signatures(self):
        super().signatures()
        ink=self.material("signatureInk","#20202b")
        gold=self.material("signatureGold","#d9ab52",metallic=.65)
        red=self.material("signatureRed","#af3544")
        if self.slug in ("luffy","sun-wukong"):
            if self.slug=="luffy":
                straw=self.material("straw","#d6af62")
                self.loft("StrawHatBrim",[(0,.025,1.57,.24,.20),(0,.025,1.585,.24,.20)],straw,"head",count=48)
                self.ellipsoid("StrawHatCrown",(0,.025,1.60),(.15,.13,.075),straw)
                self.loft("HatRibbon",[(0,.025,1.585,.151,.131),(0,.025,1.615,.151,.131)],red,"head",count=40)
            else:
                self.line("GoldenFillet",[(.164*math.cos(i*math.tau/40),.126*math.sin(i*math.tau/40),1.48) for i in range(41)],.007,gold)
        if self.slug=="gojo":
            self.tube("BlackBlindfold",[(-.15,-.044,1.43),(-.09,-.126,1.43),(0,-.139,1.43),(.09,-.126,1.43),(.15,-.044,1.43)],[.032]*5,[.006]*5,ink,sides=8)
        if self.slug in ("naruto","sasuke"):
            self.ellipsoid("ForeheadPlate",(0,-.11,1.51),(.078,.008,.027),self.material("steel","#a9becb",metallic=.65),segments=24,rings=12)
            if self.slug=="naruto":
                for sign in (-1,1):
                    for i in range(3):
                        self.line("WhiskerMark",[(sign*.080,-.105,1.36+i*.014),(sign*.124,-.084,1.354+i*.014)],.0016,ink)
        if self.slug=="tanjiro":
            for sign in (-1,1):
                self.tube("HanafudaEarring",[(sign*.147,-.01,1.39),(sign*.147,-.01,1.32)],[.014,.014],[.003,.003],self.material("ivory","#f7eee9"),sides=4)
            self.ellipsoid("ForeheadScar",(.055,-.105,1.51),(.035,.002,.026),red)
        if self.slug=="zoro":
            for i in range(3):
                self.tube("WaistKatana",[(.17,.03+i*.018,.93),(.37,.03+i*.018,.51)],[.012,.009],[.012,.009],ink,"hips")
                self.ellipsoid("SwordGuard",(.19,.03+i*.018,.89),(.025,.025,.007),gold,"hips",segments=16,rings=8)
        if self.slug=="pikachu":
            for sign in (-1,1):
                self.ellipsoid("ElectricCheek",(sign*.12,-.09,1.35),(.025,.006,.022),red)
            vertices=[(0,.14,.83),(.17,.15,.76),(.09,.15,.95),(.27,.15,.92),(.19,.15,1.12),(.38,.15,1.10),(.21,.15,1.32),(.13,.15,1.22),(.22,.15,1.10),(.09,.15,1.13),(.15,.15,.97),(0,.14,.94)]
            self.mesh("LightningTail",vertices,[tuple(range(len(vertices)))],self.material("lightning","#e3b833"),"hips")
        if self.slug=="ganesha":
            self.tube("ElephantTrunk",[(0,-.14,1.36),(0,-.20,1.28),(0,-.22,1.16),(.06,-.23,1.12),(.09,-.22,1.17)],[.035,.034,.029,.020,.006],[.035,.034,.029,.020,.006],self.material("skin"),sides=18)
            for s in (-1,1):
                self.ellipsoid("ElephantEar",(s*.17,.01,1.42),(.10,.025,.14),self.material("skin"))
        if self.slug=="krishna":
            self.tube("BambooFlute",[(-.11,-.13,1.34),(.17,-.13,1.34)],[.007,.007],[.007,.007],gold,"head",sides=12)
            self.ellipsoid("PeacockFeather",(.11,.03,1.68),(.028,.004,.09),self.material("peacock","#258b75"))
        if self.slug=="shiva":
            self.tube("TridentShaft",[(-.66,0,.65),(-.66,0,1.64)],[.009,.009],[.009,.009],gold,"rightHand")
            for i in (-1,0,1):
                self.tube("TridentProng",[(-.66,0,1.55),(-.66+i*.05,0,1.63),(-.66+i*.05,0,1.73)],[.007,.007,.001],[.007,.007,.001],gold,"rightHand")
        if self.slug in ("light","l-lawliet"):
            self.ellipsoid("Notebook",(.67,-.02,1.20),(.065,.015,.09),ink,"leftHand",segments=8,rings=6)

    def species(self):
        ears=self.spec["ears"]
        mat=self.material("skin")
        if ears not in ("none","human"):
            for sign in (-1,1):
                length={"bunny":.23,"fox":.16,"cat":.10,"elf":.13,"floppy":.19,"bear":.07,"horns":.16,"antenna":.21,"fin":.11,"leaf":.15}[ears]
                if self.body=="slime":
                    self.tube("SlimeSpeciesEar",[(sign*.27,0,1.31),(sign*.32,0,1.40),(sign*.38,.01,1.40+length)],[.06,.044,.001],[.032,.024,.001],mat)
                elif ears=="elf":
                    self.tube("ElvenEar",[(sign*.14,0,1.405),(sign*.21,0,1.43),(sign*.28,.005,1.45)],[.032,.025,.001],[.015,.012,.001],mat)
                elif ears=="bear":
                    self.ellipsoid("RoundAnimalEar",(sign*.14,0,1.55),(.055,.030,.055),mat)
                elif ears=="floppy":
                    self.tube("FloppyEar",[(sign*.145,0,1.52),(sign*.20,0,1.48),(sign*.23,-.012,1.33),(sign*.21,-.02,1.28)],[.040,.042,.036,.01],[.019,.020,.018,.008],mat)
                else:
                    self.tube("SpeciesEar",[(sign*.12,0,1.51),(sign*.16,.005,1.55),(sign*.19,.015,1.53+length)],[.046,.038,.001],[.024,.019,.001],mat)
                    self.tube("InnerEar",[(sign*.13,-.024,1.53),(sign*.16,-.015,1.57),(sign*.184,-.001,1.52+length)],[.023,.020,.001],[.003,.003,.001],self.material("earAccent",self.palette["accent"]))
        tail=self.spec["tail"]
        if tail!="none":
            widths={"cat":.025,"fox":.070,"bunny":.065,"dragon":.047,"cable":.014,"feather":.040,"flame":.045,"leaf":.042,"star":.035}
            r=widths[tail]
            self.tube("Tail."+tail,[(0,.075,.82),(0,.19,.77),(.08,.30,.74),(.15,.36,.84),(.17,.37,1.0)],[r,r,r*.9,r*.65,.002],[r,r,r*.9,r*.65,.002],self.material("tail",self.palette["secondary"]),"hips",sides=12)
        wings=self.spec["wings"]
        if wings!="none":
            for sign in (-1,1):
                for i in range(8):
                    start=(sign*.08,.085,1.15-i*.014)
                    end=(sign*(.28+.022*i),.20,1.42-i*.070)
                    self.tube("BackWing."+wings,[start,((start[0]+end[0])/2,.16,(start[2]+end[2])/2),end],[.018,.035,.001],[.008,.012,.001],self.material("wing",self.palette["accent"]),"chest",sides=8)

    def nonhuman_anatomy(self):
        mat=self.material("skin")
        if self.body=="slime":
            self.face_z=1.13
            sections=[]
            for i in range(25):
                a=math.pi*(1-i/24)
                sections.append((0,0,.78+.68*math.cos(a),max(.001,.60*math.sin(a)),max(.001,.43*math.sin(a))))
            def weights(v):
                t=max(0,min(1,(v[2]-.60)/.40))
                return {"hips":1-t,"head":t}
            self.loft("SlimeBody",sections,self.material("slime",self.palette["primary"],roughness=.25),count=48,weights_for=weights)
            return
        if self.body=="bird":
            self.loft("AvianBody",[(0,0,.60,.12,.12),(0,0,.72,.22,.17),(0,0,.95,.23,.18),(0,0,1.18,.15,.13),(0,0,1.28,.09,.08)],self.material("primary"),weights_for=self.torso_weights)
            self.ellipsoid("SculptedFace",(0,0,self.face_z),(.155,.116,.17),mat)
            for side,sign in (("left",1),("right",-1)):
                self.tube("BirdLeg",[(sign*.09,0,.65),(sign*.09,0,.35),(sign*.09,-.01,.10)],[.025,.020,.017],[.025,.020,.017],self.material("feet",self.palette["accent"]),side+"LowerLeg")
                for i in range(3):
                    self.tube("AvianToe",[(sign*.09,0,.09),(sign*.09+(i-1)*.035,-.08,.05),(sign*.09+(i-1)*.05,-.14,.035)],[.012,.012,.001],[.012,.01,.001],self.material("feet"),side+"Foot")
                for i in range(10):
                    self.tube("FlightFeather",[(sign*(.19+i*.022),0,1.19),(sign*(.23+i*.028),.01,1.10-i*.012),(sign*(.25+i*.033),.015,.97-i*.015)],[.022,.025,.001],[.01,.013,.001],self.material("feather",self.palette["secondary"]),side+("UpperArm" if i<5 else "LowerArm"),sides=8)
            return
        super().anatomy()
        if self.body=="beast":
            self.discard([o for o in self.meshes if any(o.name.startswith(s+f) for s in ("left","right") for f in ("Thumb","Index","Middle","Ring","Little","Palm"))])
            for side,sign in (("left",1),("right",-1)):
                self.ellipsoid("Paw",(sign*.655,0,1.19),(.061,.033,.048),mat,side+"Hand")
                for i in range(3):
                    self.ellipsoid("PawDigit",(sign*.695,-.01,1.16+i*.025),(.026,.025,.013),mat,side+"Hand",segments=16,rings=8)

    def build(self):
        if self.body=="humanoid":
            self.anatomy()
        else:
            self.nonhuman_anatomy()
        self.face()
        self.hair()
        if self.body not in ("bird","slime"):
            self.clothing()
        self.species()
        self.signatures()
        self.head_shape()
        self.finish_skin()
        self.animations()
        return self


def export_character(spec, root, render=False):
    slug=spec["id"].split(".")[-1]
    if not re.fullmatch(r"[a-z0-9-]+",slug):
        raise ValueError("Unsafe character ID")
    destination=root/slug
    if destination.exists():
        report=destination/"model.validation.json"
        if report.exists() and json.loads(report.read_text())["structuralValidation"]=="passed":
            return {"id":spec["id"],"status":"already-exported","directory":slug}
        raise ValueError(f"Existing incomplete destination: {destination}")
    staging=root/(".staging-"+slug)
    if staging.exists():
        raise ValueError(f"Existing interrupted staging directory: {staging}")
    staging.mkdir(parents=True)
    started=time.monotonic()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    author=CatalogAuthor(copy.deepcopy(spec)).build()
    studio(author)
    bpy.context.scene.render.fps=FPS
    bpy.context.scene.frame_end=49
    bpy.ops.wm.save_as_mainfile(filepath=str(staging/"source.blend"))
    bpy.ops.object.select_all(action="DESELECT")
    author.rig.select_set(True);author.object.select_set(True)
    bpy.context.view_layer.objects.active=author.rig
    bpy.ops.export_scene.gltf(filepath=str(staging/"model.glb"),export_format="GLB",use_selection=True,export_skins=True,export_morph=True,export_animations=True,export_animation_mode="ACTIONS",export_force_sampling=False,export_extras=True,export_yup=True)
    with contextlib.redirect_stdout(sys.stderr):
        package(staging/"model.glb",staging/"model.vrm",spec["name"])
    (staging/"character.json").write_text(json.dumps(spec,indent=2),encoding="utf-8")
    if render:
        action=bpy.data.actions.get("idle")
        author.rig.animation_data.action=action
        bpy.context.scene.frame_set(9)
        bpy.context.scene.render.filepath=str(staging/"preview.png")
        bpy.ops.render.render(write_still=True)
    result={"id":spec["id"],"status":"structurally-validated","directory":slug,"proportions":"full-anime" if spec["body"]=="humanoid" else "species-specific","seconds":round(time.monotonic()-started,2)}
    (staging/"production.json").write_text(json.dumps(result,indent=2),encoding="utf-8")
    staging.rename(destination)
    print("CHARACTER_DONE "+json.dumps(result),flush=True)
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog",required=True,type=Path)
    parser.add_argument("--output",required=True,type=Path)
    parser.add_argument("--character")
    parser.add_argument("--worker-index",type=int,default=0)
    parser.add_argument("--worker-count",type=int,default=1)
    parser.add_argument("--render",action="store_true")
    args=parser.parse_args(sys.argv[sys.argv.index("--")+1:])
    if not 0<=args.worker_index<args.worker_count<=4:
        parser.error("Worker index/count out of bounds")
    logging.basicConfig(level=logging.INFO,format="%(levelname)s %(message)s")
    specs=json.loads(args.catalog.read_text(encoding="utf-8"))["characters"]
    selected=[s for s in specs if s["id"] in set(args.character.split(","))] if args.character else [s for i,s in enumerate(specs) if i%args.worker_count==args.worker_index]
    if not selected:
        parser.error("No characters selected")
    args.output.mkdir(parents=True,exist_ok=True)
    failed=[]
    for spec in selected:
        try:
            export_character(spec,args.output,args.render)
        except Exception as exc:
            LOG.exception("Failed %s",spec["id"])
            failed.append({"id":spec["id"],"error":str(exc)})
    (args.output/f"worker-{args.worker_index}.json").write_text(json.dumps({"selected":len(selected),"failed":failed},indent=2),encoding="utf-8")
    if failed:
        raise RuntimeError(f"{len(failed)} character exports failed; see worker report")


if __name__=="__main__":
    main()
