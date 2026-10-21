import { describe, expect, it } from "vitest";
import * as THREE from "three";
import catalogJson from "../../../../../crates/avatar/catalog/characters.json";
import type { PetState } from "../../components/CompanionRenderer";
import { disposeObject } from "../dispose";
import { applyToRig, easePose, motionFor, restPose } from "../motion";
import { StageProps } from "../props";
import { buildCharacter, MOUTH_SHAPES } from "../rig";
import { subjectFromAvatar, type CharacterSpec, type ResolvedAvatar } from "../types";

const catalog = catalogJson as { characters: CharacterSpec[] };

const ALL_STATES: PetState[] = [
  "idle",
  "working",
  "thinking",
  "waiting",
  "success",
  "error",
  "celebrating",
  "sleeping",
  "hungry",
  "eating",
  "level_up",
  "achievement",
  "offline",
  "paused",
  "attention_required",
];

function allFinite(object: THREE.Object3D): boolean {
  let finite = true;
  object.updateMatrixWorld(true);
  object.traverse((node) => {
    if (!node.matrixWorld.elements.every(Number.isFinite)) finite = false;
  });
  return finite;
}

describe("procedural character rig", () => {
  it("uses the bundled catalog of at least 100 characters", () => {
    expect(catalog.characters.length).toBeGreaterThanOrEqual(100);
  });

  it.each(catalog.characters.map((spec) => [spec.id, spec] as const))("builds %s with a complete, finite rig", (_id, spec) => {
    const rig = buildCharacter(spec);
    for (const node of [rig.hips, rig.spine, rig.chest, rig.neck, rig.head, rig.armL, rig.armR, rig.elbowL, rig.elbowR, rig.legL, rig.legR]) {
      expect(node).toBeInstanceOf(THREE.Object3D);
    }
    for (const shape of MOUTH_SHAPES) expect(rig.mouths[shape]).toBeDefined();
    expect(rig.eyes.length).toBeGreaterThan(0);

    const box = new THREE.Box3().setFromObject(rig.root);
    const height = box.max.y - box.min.y;
    expect(Number.isFinite(height)).toBe(true);
    expect(height).toBeGreaterThan(0.4);
    expect(height).toBeLessThan(1.6);
    expect(box.min.y).toBeGreaterThan(-0.08);
    expect(rig.headTopY).toBeGreaterThan(0.3);

    let meshes = 0;
    rig.root.traverse((node) => {
      if ((node as THREE.Mesh).isMesh) meshes += 1;
    });
    expect(meshes).toBeGreaterThan(10);
    disposeObject(rig.root);
  });

  it("produces distinct geometry for distinct designs", () => {
    const count = (spec: CharacterSpec) => {
      let meshes = 0;
      const rig = buildCharacter(spec);
      rig.root.traverse((node) => {
        if ((node as THREE.Mesh).isMesh) meshes += 1;
      });
      disposeObject(rig.root);
      return meshes;
    };
    const signatures = new Set(catalog.characters.map((spec) => `${spec.body}:${count(spec)}:${JSON.stringify(spec.palette)}`));
    expect(signatures.size).toBe(catalog.characters.length);
  });
});

describe("state motion", () => {
  const sample = catalog.characters.filter((spec, index, all) => all.findIndex((other) => other.body === spec.body) === index);

  it.each(ALL_STATES)("drives every body type through %s with finite transforms", (state) => {
    for (const spec of sample) {
      const rig = buildCharacter(spec);
      const pose = restPose();
      for (let frame = 0; frame < 20; frame += 1) {
        const motion = motionFor(state, frame / 30, spec.energy);
        easePose(pose, motion.pose, 0.3);
        applyToRig(rig, pose, motion.expression, { yaw: 0.1, pitch: -0.05 }, 1, frame / 30);
      }
      expect(allFinite(rig.root)).toBe(true);
      const visibleMouths = Object.values(rig.mouths).filter((mouth) => mouth.visible);
      expect(visibleMouths).toHaveLength(1);
      disposeObject(rig.root);
    }
  });

  it("closes eyes while sleeping and shows the laptop while working", () => {
    expect(motionFor("sleeping", 1).expression.eyeOpen).toBe(0);
    expect(motionFor("working", 1).props.laptop).toBe(true);
    expect(motionFor("idle", 1).props.laptop).toBe(false);
    expect(motionFor("error", 1).props.glyph).not.toBeNull();
    expect(motionFor("offline", 1).props.dim).toBe(true);
  });

  it("eases toward the target and turns the short way around", () => {
    const current = restPose();
    const target = restPose();
    target.headX = 1;
    current.turnY = 3.0;
    target.turnY = -3.0;
    for (let i = 0; i < 60; i += 1) easePose(current, target, 0.2);
    expect(current.headX).toBeCloseTo(1, 3);
    // Shortest path from 3.0 to -3.0 rad crosses +pi, not zero.
    expect(Math.abs(Math.sin(current.turnY - target.turnY))).toBeLessThan(1e-3);
    expect(Math.abs(current.turnY)).toBeGreaterThan(2.5);
  });
});

describe("stage props and subjects", () => {
  it("visualizes XP progress without throwing", () => {
    const props = new StageProps("#38bdf8");
    props.setLayout({ headTopY: 0.9, deskY: 0.35 });
    props.setHud({ level: 3, totalXp: 260, xpIntoLevel: 60, xpToNextLevel: 140 });
    props.update(motionFor("success", 0.5).props, 1 / 30, 0.5);
    props.update(motionFor("working", 0.5).props, 1 / 30, 0.6);
    props.setHud(null);
    disposeObject(props.group);
  });

  it("maps resolved avatars to stage subjects", () => {
    const spec = catalog.characters[0];
    const procedural: ResolvedAvatar = {
      source: "catalog",
      avatarId: spec.id,
      kind: "procedural",
      name: spec.name,
      spec,
      customized: false,
      modelSha256: null,
      metadata: null,
    };
    expect(subjectFromAvatar(procedural)).toEqual({ kind: "procedural", spec });
    const sculpt: ResolvedAvatar = { ...procedural, source: "custom", kind: "image_sculpt", spec: null, modelSha256: "ab" };
    expect(subjectFromAvatar(sculpt)).toEqual({ kind: "image_sculpt", avatarId: spec.id, modelSha256: "ab", name: spec.name });
    expect(subjectFromAvatar({ ...sculpt, modelSha256: null })).toBeNull();
    expect(subjectFromAvatar(null)).toBeNull();
  });
});

describe("anime styling", () => {
  const base = catalog.characters.find((spec) => spec.id === "miyora.aoi") as CharacterSpec;

  const materials = (spec: CharacterSpec) => {
    const rig = buildCharacter(spec);
    const found: THREE.Material[] = [];
    rig.root.traverse((node) => {
      const mesh = node as THREE.Mesh;
      if (mesh.isMesh) found.push(...(Array.isArray(mesh.material) ? mesh.material : [mesh.material]));
    });
    return { rig, found };
  };

  it.each(["dots", "stripes", "waves", "petals", "stars", "diamonds"] as const)("applies the %s fabric pattern as a texture", (pattern) => {
    const { rig, found } = materials({ ...base, pattern });
    const textured = found.filter((material) => (material as THREE.MeshToonMaterial).map instanceof THREE.DataTexture);
    expect(textured.length).toBeGreaterThan(0);
    const texture = (textured[0] as THREE.MeshToonMaterial).map as THREE.DataTexture;
    const pixels = texture.image.data as Uint8Array;
    const colors = new Set<number>();
    for (let i = 0; i < pixels.length; i += 4) colors.add((pixels[i] << 16) | (pixels[i + 1] << 8) | pixels[i + 2]);
    expect(colors.size).toBe(2);
    disposeObject(rig.root);
  });

  it("fades hair to the tip color with vertex colors", () => {
    const { rig, found } = materials({ ...base, hair: "long", hairTip: "#f97316" });
    expect(found.some((material) => (material as THREE.MeshToonMaterial).vertexColors)).toBe(true);
    let gradient = false;
    rig.root.traverse((node) => {
      const colors = (node as THREE.Mesh).geometry?.getAttribute?.("color");
      if (colors && colors.count > 0) gradient = true;
    });
    expect(gradient).toBe(true);
    disposeObject(rig.root);
    const plain = materials({ ...base, hairTip: undefined });
    expect(plain.found.some((material) => (material as THREE.MeshToonMaterial).vertexColors)).toBe(false);
    disposeObject(plain.rig.root);
  });

  it.each(["hakama", "sailor", "coat"] as const)("builds the %s outfit", (outfit) => {
    const { rig } = materials({ ...base, outfit });
    const box = new THREE.Box3().setFromObject(rig.root);
    expect(box.min.y).toBeGreaterThan(-0.08);
    expect(allFinite(rig.root)).toBe(true);
    disposeObject(rig.root);
  });

  it.each(["katana", "wand", "staff", "kitsune_mask"] as const)("attaches the %s prop", (accessory) => {
    const { rig } = materials({ ...base, accessories: [accessory] });
    const name = { katana: "katana", wand: "wand", staff: "staff", kitsune_mask: "kitsuneMask" }[accessory];
    expect(rig.root.getObjectByName(name)).toBeDefined();
    disposeObject(rig.root);
  });

  it.each(["shell", "gem"] as const)("attaches the %s creature part", (accessory) => {
    for (const body of ["beast", "bird", "robot"] as const) {
      const { rig } = materials({ ...base, body, accessories: [accessory] });
      expect(rig.root.getObjectByName(accessory)).toBeDefined();
      expect(allFinite(rig.root)).toBe(true);
      disposeObject(rig.root);
    }
  });

  it.each(["flame", "leaf", "star"] as const)("builds the %s tail as a swaying chain", (tail) => {
    const { rig } = materials({ ...base, body: "beast", tail });
    expect(rig.tail.length).toBe(3);
    let meshes = 0;
    rig.tail[2]?.traverse((node) => {
      if ((node as THREE.Mesh).isMesh) meshes += 1;
    });
    expect(meshes).toBeGreaterThan(0);
    expect(allFinite(rig.root)).toBe(true);
    disposeObject(rig.root);
  });

  it("builds leaf ears on both sides", () => {
    const { rig } = materials({ ...base, body: "beast", ears: "leaf" });
    let ears = 0;
    rig.root.traverse((node) => {
      if (node.name === "ear") ears += 1;
    });
    expect(ears).toBe(2);
    disposeObject(rig.root);
  });

  it("includes original pocket-creature designs that use the new parts", () => {
    const pocket = catalog.characters.filter((spec) => spec.tags.includes("pocket-monster"));
    expect(pocket.length).toBeGreaterThanOrEqual(20);
    const used = new Set<string>(pocket.flatMap((spec) => [spec.tail, spec.ears, ...spec.accessories]));
    for (const part of ["flame", "leaf", "star", "shell", "gem"]) expect(used.has(part)).toBe(true);
  });

  it.each(["hime", "side_tail"] as const)("builds the %s hairstyle with swaying locks", (hair) => {
    const { rig } = materials({ ...base, hair });
    expect(rig.hairSway.length).toBeGreaterThan(0);
    disposeObject(rig.root);
  });
});
