// Procedural 3D character builder.
//
// Builds a jointed, toon-shaded character from a validated `CharacterSpec`.
// Every character shares one joint hierarchy so a single motion system can
// animate all body types:
//
//   root -> body -> hips -> spine -> chest -> neck -> head
//                               chest -> armL/armR -> elbowL/elbowR -> hand
//                   hips -> legL/legR -> kneeL/kneeR -> foot
//                   hips -> tail[0..n]   chest -> wingL/wingR
//
// The character faces +Z, stands on y = 0, and is roughly one unit tall.

import * as THREE from "three";
import { RoundedBoxGeometry } from "three/examples/jsm/geometries/RoundedBoxGeometry.js";
import type { CharacterSpec } from "./types";

export type MouthShape = "smile" | "grin" | "open" | "flat" | "frown";
export const MOUTH_SHAPES: readonly MouthShape[] = ["smile", "grin", "open", "flat", "frown"];

export interface CharacterRig {
  spec: CharacterSpec;
  root: THREE.Group;
  body: THREE.Group;
  hips: THREE.Object3D;
  spine: THREE.Object3D;
  chest: THREE.Object3D;
  neck: THREE.Object3D;
  head: THREE.Object3D;
  armL: THREE.Object3D;
  armR: THREE.Object3D;
  elbowL: THREE.Object3D;
  elbowR: THREE.Object3D;
  legL: THREE.Object3D;
  legR: THREE.Object3D;
  kneeL: THREE.Object3D;
  kneeR: THREE.Object3D;
  handR: THREE.Object3D;
  tail: THREE.Object3D[];
  wingL: THREE.Object3D | null;
  wingR: THREE.Object3D | null;
  earL: THREE.Object3D | null;
  earR: THREE.Object3D | null;
  hairSway: THREE.Object3D[];
  eyes: THREE.Object3D[];
  /** Resting eye openness (sleepy eyes rest half closed). */
  eyeBaseOpen: number;
  mouths: Record<MouthShape, THREE.Object3D>;
  brows: THREE.Object3D[];
  blush: THREE.MeshBasicMaterial[];
  /** Height of the top of the head above the ground, in rig units. */
  headTopY: number;
  /** Height of the hand when the arm is held forward, for prop placement. */
  deskY: number;
  hasLegs: boolean;
}

// ---------------------------------------------------------------------------
// Materials

const TOON_STEPS = [80, 165, 255];
let sharedGradient: THREE.DataTexture | null = null;

function gradientMap(): THREE.DataTexture {
  if (!sharedGradient) {
    const data = new Uint8Array(TOON_STEPS.length * 4);
    TOON_STEPS.forEach((value, index) => data.set([value, value, value, 255], index * 4));
    sharedGradient = new THREE.DataTexture(data, TOON_STEPS.length, 1, THREE.RGBAFormat);
    sharedGradient.minFilter = THREE.NearestFilter;
    sharedGradient.magFilter = THREE.NearestFilter;
    sharedGradient.generateMipmaps = false;
    sharedGradient.needsUpdate = true;
  }
  return sharedGradient;
}

const WHITE = new THREE.Color(0xffffff);
const BLACK = new THREE.Color(0x000000);
const OUTLINE_DARKEN = 0.72;
const BONE_COLOR = "#efe2c4";
const INNER_EAR_COLOR = "#f5a3b5";
const NOSE_COLOR = "#2a1d1d";
const MOUTH_COLOR = "#5b1f2a";
const BLUSH_COLOR = "#ff7a96";
const GOLD_COLOR = "#f5c542";

function shade(hex: string, toward: THREE.Color, amount: number): string {
  return `#${new THREE.Color(hex).lerp(toward, amount).getHexString()}`;
}

interface ToonOptions {
  emissive?: number;
  opacity?: number;
  side?: THREE.Side;
}

class Kit {
  private readonly materials = new Map<string, THREE.Material>();

  toon(hex: string, options: ToonOptions = {}): THREE.MeshToonMaterial {
    const key = `toon:${hex}:${options.emissive ?? 0}:${options.opacity ?? 1}:${options.side ?? THREE.FrontSide}`;
    const cached = this.materials.get(key);
    if (cached) return cached as THREE.MeshToonMaterial;
    const material = new THREE.MeshToonMaterial({
      color: new THREE.Color(hex),
      gradientMap: gradientMap(),
      emissive: options.emissive ? new THREE.Color(hex) : BLACK.clone(),
      emissiveIntensity: options.emissive ?? 0,
      transparent: (options.opacity ?? 1) < 1,
      opacity: options.opacity ?? 1,
      side: options.side ?? THREE.FrontSide,
    });
    this.materials.set(key, material);
    return material;
  }

  flat(hex: string, opacity = 1): THREE.MeshBasicMaterial {
    const key = `flat:${hex}:${opacity}`;
    const cached = this.materials.get(key);
    if (cached) return cached as THREE.MeshBasicMaterial;
    const material = new THREE.MeshBasicMaterial({
      color: new THREE.Color(hex),
      transparent: opacity < 1,
      opacity,
      depthWrite: opacity >= 1,
    });
    this.materials.set(key, material);
    return material;
  }

  outline(hex: string): THREE.MeshBasicMaterial {
    const key = `outline:${hex}`;
    const cached = this.materials.get(key);
    if (cached) return cached as THREE.MeshBasicMaterial;
    const material = new THREE.MeshBasicMaterial({
      color: new THREE.Color(hex).lerp(BLACK, OUTLINE_DARKEN),
      side: THREE.BackSide,
    });
    this.materials.set(key, material);
    return material;
  }

  /** Adds a mesh to `parent`; `outline` adds an inverted-hull ink line. */
  mesh(
    geometry: THREE.BufferGeometry,
    material: THREE.Material,
    parent: THREE.Object3D,
    position: [number, number, number] = [0, 0, 0],
    options: { rotation?: [number, number, number]; scale?: [number, number, number]; outline?: number } = {},
  ): THREE.Mesh {
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(...position);
    if (options.rotation) mesh.rotation.set(...options.rotation);
    if (options.scale) mesh.scale.set(...options.scale);
    if (options.outline && "color" in material) {
      const hex = `#${(material as THREE.MeshToonMaterial).color.getHexString()}`;
      const ink = new THREE.Mesh(geometry, this.outline(hex));
      ink.scale.setScalar(1 + options.outline);
      ink.renderOrder = -1;
      mesh.add(ink);
    }
    parent.add(mesh);
    return mesh;
  }
}

function joint(parent: THREE.Object3D, name: string, x = 0, y = 0, z = 0): THREE.Object3D {
  const node = new THREE.Object3D();
  node.name = name;
  node.position.set(x, y, z);
  parent.add(node);
  return node;
}

const OUTLINE = 0.045;
const SEGMENTS = 24;

const sphere = (radius: number) => new THREE.SphereGeometry(radius, SEGMENTS, Math.round(SEGMENTS * 0.75));
const capsule = (radius: number, length: number) => new THREE.CapsuleGeometry(radius, Math.max(length, 0.001), 6, 16);
const cone = (radius: number, height: number, segments = 16) => new THREE.ConeGeometry(radius, height, segments);
const cylinder = (top: number, bottom: number, height: number, segments = 20) =>
  new THREE.CylinderGeometry(top, bottom, height, segments);
const torus = (radius: number, tube: number, arc = Math.PI * 2) => new THREE.TorusGeometry(radius, tube, 10, 32, arc);
const roundedBox = (width: number, height: number, depth: number, radius: number) =>
  new RoundedBoxGeometry(width, height, depth, 3, Math.min(radius, width / 2, height / 2, depth / 2));
const disc = (radius: number, start = 0, length = Math.PI * 2) => new THREE.CircleGeometry(radius, 28, start, length);

// ---------------------------------------------------------------------------
// Dimensions

interface Dimensions {
  height: number;
  headR: number;
  headCenterY: number;
  torsoR: number;
  torsoH: number;
  legUpper: number;
  legLower: number;
  legR: number;
  armUpper: number;
  armLower: number;
  armR: number;
  hipY: number;
  shoulderX: number;
}

function dimensions(spec: CharacterSpec): Dimensions {
  const { height, headScale, build, limbLength } = spec.proportions;
  const headR = 0.17 * headScale * (0.75 + 0.25 * height);
  if (spec.body === "slime") {
    const blobR = 0.27 * height * Math.sqrt(build);
    return {
      height,
      headR: blobR,
      headCenterY: blobR * 0.8,
      torsoR: blobR,
      torsoH: 0,
      legUpper: 0,
      legLower: 0,
      legR: 0,
      armUpper: 0.001,
      armLower: 0.001,
      armR: 0.05,
      hipY: 0,
      shoulderX: blobR * 0.95,
    };
  }
  const legUpper = 0.13 * limbLength * height;
  const legLower = 0.12 * limbLength * height;
  const torsoR = 0.105 * build * (spec.body === "bird" ? 1.35 : 1);
  return {
    height,
    headR,
    headCenterY: headR * 0.92,
    torsoR,
    torsoH: 0.24 * height,
    legUpper,
    legLower,
    legR: 0.04 * build,
    armUpper: 0.12 * limbLength * height,
    armLower: 0.11 * limbLength * height,
    armR: 0.034 * build,
    hipY: legUpper + legLower + 0.045,
    shoulderX: torsoR + 0.025,
  };
}

// ---------------------------------------------------------------------------
// Builder

export function buildCharacter(spec: CharacterSpec): CharacterRig {
  const kit = new Kit();
  const d = dimensions(spec);
  const p = spec.palette;
  const isBeast = spec.body === "beast";
  const isRobot = spec.body === "robot";
  const isSlime = spec.body === "slime";
  const isBird = spec.body === "bird";
  const furred = isBeast || isBird;
  const bareOutfit = spec.outfit === "fur" || spec.outfit === "plating";

  const root = new THREE.Group();
  root.name = `avatar:${spec.id}`;
  const body = new THREE.Group();
  root.add(body);
  const hips = joint(body, "hips", 0, d.hipY, 0);
  const spine = joint(hips, "spine");
  const chest = joint(spine, "chest", 0, isSlime ? 0 : d.torsoH * 0.45, 0);
  const neck = joint(chest, "neck", 0, isSlime ? 0 : d.torsoH * 0.55, 0);
  const head = joint(neck, "head", 0, isSlime ? 0 : 0.02, 0);
  const headCenter = joint(head, "headCenter", 0, d.headCenterY, 0);

  // ---- Torso ---------------------------------------------------------------
  const skinMat = kit.toon(p.skin);
  const torsoColor = bareOutfit || (furred && spec.outfit === "fur") ? p.skin : outfitTorsoColor(spec);
  if (isSlime) {
    kit.mesh(sphere(d.headR), kit.toon(p.skin, { opacity: 0.94 }), headCenter, [0, 0, 0], {
      scale: [1, 0.8, 0.95],
      outline: OUTLINE * 0.6,
    });
    kit.mesh(sphere(d.headR * 0.35), kit.flat(shade(p.skin, WHITE, 0.6), 0.55), headCenter, [-d.headR * 0.42, d.headR * 0.38, d.headR * 0.55], {
      scale: [1, 0.6, 0.4],
    });
  } else if (isRobot) {
    kit.mesh(roundedBox(d.torsoR * 2, d.torsoH, d.torsoR * 1.6, 0.04), kit.toon(torsoColor), spine, [0, d.torsoH / 2, 0], {
      outline: OUTLINE * 0.5,
    });
    kit.mesh(roundedBox(d.torsoR * 1.1, d.torsoH * 0.38, 0.02, 0.008), kit.toon(p.accent, { emissive: 0.7 }), spine, [
      0,
      d.torsoH * 0.6,
      d.torsoR * 0.8 + 0.006,
    ]);
  } else if (isBird) {
    kit.mesh(sphere(d.torsoR), kit.toon(p.skin), spine, [0, d.torsoH * 0.42, 0], { scale: [1, 1.15, 0.95], outline: OUTLINE * 0.5 });
    kit.mesh(sphere(d.torsoR * 0.78), kit.toon(p.secondary), spine, [0, d.torsoH * 0.38, d.torsoR * 0.3], { scale: [0.85, 1.05, 0.8] });
  } else {
    kit.mesh(capsule(d.torsoR, d.torsoH - d.torsoR * 2), kit.toon(torsoColor), spine, [0, d.torsoH / 2, 0], {
      scale: [1, 1, 0.78],
      outline: OUTLINE * 0.5,
    });
    if (furred && spec.outfit === "fur") {
      kit.mesh(sphere(d.torsoR * 0.75), kit.toon(shade(p.skin, WHITE, 0.55)), spine, [0, d.torsoH * 0.42, d.torsoR * 0.42], {
        scale: [0.8, 1.05, 0.5],
      });
    } else {
      dressTorso(kit, spec, spine, d);
    }
  }

  // ---- Arms ----------------------------------------------------------------
  const sleeveColor = bareOutfit || (furred && spec.outfit === "fur") ? p.skin : spec.outfit === "armor" ? p.secondary : outfitTorsoColor(spec);
  const armY = isSlime ? d.headR * 0.45 : d.torsoH * 0.42;
  const makeArm = (side: 1 | -1) => {
    const arm = joint(chest, side === 1 ? "armL" : "armR", side * d.shoulderX, armY, 0);
    const elbow = joint(arm, side === 1 ? "elbowL" : "elbowR", 0, -d.armUpper, 0);
    const hand = joint(elbow, side === 1 ? "handL" : "handR", 0, -d.armLower - 0.015, 0);
    if (isSlime) {
      kit.mesh(sphere(d.armR), kit.toon(p.skin, { opacity: 0.94 }), hand, [0, 0.02, 0], { scale: [1, 0.8, 1], outline: OUTLINE });
    } else if (isBird) {
      const wingMat = kit.toon(p.skin);
      kit.mesh(sphere(d.armUpper * 0.62), wingMat, arm, [side * 0.01, -d.armUpper * 0.55, -0.01], {
        scale: [0.35, 1.25, 0.9],
        outline: OUTLINE,
      });
      kit.mesh(sphere(d.armLower * 0.45), kit.toon(shade(p.skin, BLACK, 0.25)), elbow, [side * 0.005, -d.armLower * 0.35, -0.01], {
        scale: [0.3, 1.2, 0.8],
      });
    } else if (isRobot) {
      const limbMat = kit.toon(p.secondary);
      kit.mesh(cylinder(d.armR, d.armR * 0.9, d.armUpper), limbMat, arm, [0, -d.armUpper / 2, 0], { outline: OUTLINE });
      kit.mesh(sphere(d.armR * 1.25), kit.toon(p.skin), elbow, [0, 0, 0]);
      kit.mesh(cylinder(d.armR * 0.9, d.armR * 0.8, d.armLower), limbMat, elbow, [0, -d.armLower / 2, 0], { outline: OUTLINE });
      kit.mesh(sphere(d.armR * 1.35), kit.toon(p.skin), hand, [0, 0.01, 0], { outline: OUTLINE });
      kit.mesh(sphere(d.armR * 1.3), kit.toon(p.skin), arm, [0, 0, 0]);
    } else {
      const sleeve = kit.toon(sleeveColor);
      kit.mesh(capsule(d.armR, d.armUpper - d.armR), sleeve, arm, [0, -d.armUpper / 2, 0], { outline: OUTLINE });
      const forearmColor = sleeveColor === p.skin || spec.outfit === "kimono" || spec.outfit === "robe" || spec.outfit === "labcoat" || spec.outfit === "hoodie" || spec.outfit === "jacket" || spec.outfit === "suit" ? sleeveColor : p.skin;
      kit.mesh(capsule(d.armR * 0.92, d.armLower - d.armR), kit.toon(forearmColor), elbow, [0, -d.armLower / 2, 0], { outline: OUTLINE });
      kit.mesh(sphere(d.armR * 1.15), skinMat, hand, [0, 0.01, 0], { scale: [1, 1.05, 0.9], outline: OUTLINE });
      if (spec.outfit === "armor") {
        kit.mesh(sphere(d.armR * 1.75), kit.toon(p.primary), arm, [side * 0.006, -0.004, 0], {
          scale: [1.1, 0.8, 1.05],
          outline: OUTLINE,
        });
      }
    }
    return { arm, elbow, hand };
  };
  const left = makeArm(1);
  const right = makeArm(-1);

  // ---- Legs ----------------------------------------------------------------
  const pantsColor = legColor(spec);
  const makeLeg = (side: 1 | -1) => {
    const leg = joint(hips, side === 1 ? "legL" : "legR", side * d.torsoR * 0.48, -0.005, 0);
    const knee = joint(leg, side === 1 ? "kneeL" : "kneeR", 0, -d.legUpper, 0);
    if (isSlime) return { leg, knee };
    if (isBird) {
      const legMat = kit.toon(p.accent);
      kit.mesh(cylinder(0.011, 0.011, d.legUpper), legMat, leg, [0, -d.legUpper / 2, 0]);
      kit.mesh(cylinder(0.011, 0.011, d.legLower), legMat, knee, [0, -d.legLower / 2, 0]);
      for (const spread of [-0.5, 0, 0.5]) {
        kit.mesh(capsule(0.009, 0.04), legMat, knee, [Math.sin(spread) * 0.02, -d.legLower - 0.02, Math.cos(spread) * 0.02], {
          rotation: [Math.PI / 2, spread, 0],
        });
      }
      return { leg, knee };
    }
    const legMat = kit.toon(isRobot ? p.secondary : pantsColor);
    const lowerMat = kit.toon(isRobot ? p.secondary : lowerLegColor(spec));
    if (isRobot) {
      kit.mesh(cylinder(d.legR, d.legR * 0.9, d.legUpper), legMat, leg, [0, -d.legUpper / 2, 0], { outline: OUTLINE });
      kit.mesh(sphere(d.legR * 1.2), kit.toon(p.skin), knee, [0, 0, 0]);
      kit.mesh(cylinder(d.legR * 0.9, d.legR, d.legLower), lowerMat, knee, [0, -d.legLower / 2, 0], { outline: OUTLINE });
    } else {
      kit.mesh(capsule(d.legR, d.legUpper - d.legR), legMat, leg, [0, -d.legUpper / 2, 0], { outline: OUTLINE });
      kit.mesh(capsule(d.legR * 0.9, d.legLower - d.legR), lowerMat, knee, [0, -d.legLower / 2, 0], { outline: OUTLINE });
    }
    const shoeColor = furred || isRobot ? (isRobot ? p.skin : shade(p.skin, BLACK, 0.15)) : shade(p.accent, BLACK, 0.35);
    kit.mesh(roundedBox(d.legR * 2.1, 0.045, d.legR * 3, 0.02), kit.toon(shoeColor), knee, [0, -d.legLower - 0.012, d.legR * 0.55], {
      outline: OUTLINE,
    });
    return { leg, knee };
  };
  const leftLeg = makeLeg(1);
  const rightLeg = makeLeg(-1);

  // ---- Head ----------------------------------------------------------------
  const R = d.headR;
  if (isRobot) {
    kit.mesh(roundedBox(R * 2, R * 1.8, R * 1.7, R * 0.35), kit.toon(p.skin), headCenter, [0, 0, 0], { outline: OUTLINE * 0.6 });
    kit.mesh(roundedBox(R * 1.62, R * 1.1, 0.02, R * 0.2), kit.toon(shade(p.secondary, BLACK, 0.45)), headCenter, [0, -R * 0.05, R * 0.85 + 0.002]);
  } else if (!isSlime) {
    kit.mesh(sphere(R), kit.toon(p.skin), headCenter, [0, 0, 0], {
      scale: [isBeast ? 1.05 : 1, 0.94, 0.96],
      outline: OUTLINE * 0.6,
    });
  }
  const faceZ = (x: number, y: number): number => {
    if (isRobot) return R * 0.85 + 0.012;
    const sy = isSlime ? 0.8 : 0.94;
    const sz = 0.96;
    const nx = x / R;
    const ny = y / (R * sy);
    return R * sz * Math.sqrt(Math.max(1 - nx * nx - ny * ny, 0.05)) + 0.006;
  };
  const eyeY = isSlime ? R * 0.12 : -R * 0.05;

  if (isBeast) {
    const muzzleScale: Record<string, [number, number, number]> = {
      fox: [1.0, 0.75, 1.15],
      floppy: [1.15, 0.8, 1.05],
      cat: [0.95, 0.72, 0.8],
      bear: [1.2, 0.85, 0.85],
    };
    const scale = muzzleScale[spec.ears] ?? [1.05, 0.78, 0.85];
    const muzzleY = eyeY - R * 0.36;
    kit.mesh(sphere(R * 0.3), kit.toon(shade(p.skin, WHITE, 0.45)), headCenter, [0, muzzleY, R * 0.74], { scale, outline: OUTLINE });
    kit.mesh(sphere(R * 0.075), kit.toon(NOSE_COLOR), headCenter, [0, muzzleY + R * 0.12, R * 0.74 + R * 0.3 * scale[2]], {
      scale: [1.3, 0.9, 1],
    });
  }
  if (isBird) {
    kit.mesh(cone(R * 0.17, R * 0.42, 12), kit.toon(p.accent), headCenter, [0, eyeY - R * 0.28, R * 0.98], {
      rotation: [Math.PI / 2, 0, 0],
      outline: OUTLINE,
    });
  }

  // Eyes
  const eyes: THREE.Object3D[] = [];
  const eyeSize = R * (spec.eyes === "sparkle" ? 0.23 : 0.2);
  const irisMat = kit.flat(p.eyes);
  const pupilMat = kit.flat(shade(p.eyes, BLACK, 0.65));
  const shineMat = kit.flat("#ffffff");
  const rimMat = kit.flat(shade(p.eyes, BLACK, 0.8));
  const buildEye = (x: number, size: number, tilt: number) => {
    const group = joint(headCenter, "eye", x, eyeY, faceZ(x, eyeY));
    group.rotation.y = Math.asin(Math.max(-0.9, Math.min(0.9, x / R))) * 0.9;
    group.rotation.z = tilt;
    const squash = spec.eyes === "sharp" ? 0.68 : 1.18;
    kit.mesh(disc(size * 1.12), rimMat, group, [0, 0, -0.001], { scale: [0.86, squash, 1] });
    kit.mesh(disc(size), irisMat, group, [0, 0, 0], { scale: [0.8, squash, 1] });
    kit.mesh(disc(size * 0.48), pupilMat, group, [0, -size * 0.08, 0.001], { scale: [0.85, squash, 1] });
    kit.mesh(disc(size * 0.27), shineMat, group, [-size * 0.28, size * 0.36 * squash, 0.002]);
    kit.mesh(disc(size * 0.12), shineMat, group, [size * 0.25, -size * 0.3 * squash, 0.002]);
    if (spec.eyes === "sparkle") {
      kit.mesh(disc(size * 0.16, 0, Math.PI * 2), shineMat, group, [size * 0.32, size * 0.42, 0.003], { scale: [0.4, 1.4, 1] });
      kit.mesh(disc(size * 0.16, 0, Math.PI * 2), shineMat, group, [size * 0.32, size * 0.42, 0.003], { scale: [1.4, 0.4, 1] });
    }
    if (spec.eyes === "sharp" || spec.eyes === "sleepy") {
      kit.mesh(new THREE.PlaneGeometry(size * 2, size * 0.22), rimMat, group, [0, size * squash * 0.95, 0.003], {
        rotation: [0, 0, spec.eyes === "sharp" ? -tilt * 1.5 : 0],
      });
    }
    eyes.push(group);
  };
  if (spec.eyes === "visor") {
    const visor = joint(headCenter, "visor", 0, eyeY, faceZ(0, eyeY));
    kit.mesh(roundedBox(R * 1.35, eyeSize * 1.25, 0.02, eyeSize * 0.5), kit.toon(p.eyes, { emissive: 1.1 }), visor, [0, 0, 0]);
    eyes.push(visor);
  } else if (spec.eyes === "mono") {
    buildEye(0, eyeSize * 1.55, 0);
  } else {
    const eyeX = R * (isBird ? 0.42 : 0.36);
    const tilt = spec.eyes === "sharp" ? 0.18 : 0;
    buildEye(eyeX, eyeSize, tilt);
    buildEye(-eyeX, eyeSize, -tilt);
  }

  // Brows
  const brows: THREE.Object3D[] = [];
  if (!isRobot && !isSlime && spec.eyes !== "visor") {
    const browMat = kit.flat(furred ? shade(p.skin, BLACK, 0.45) : shade(p.hair, BLACK, 0.15));
    for (const side of [1, -1]) {
      const x = side * R * 0.36;
      const y = eyeY + eyeSize * 1.75;
      const brow = joint(headCenter, "brow", x, y, faceZ(x, y));
      brow.rotation.y = Math.asin(x / R) * 0.9;
      kit.mesh(roundedBox(R * 0.24, R * 0.045, 0.01, 0.004), browMat, brow, [0, 0, 0]);
      brows.push(brow);
    }
  }

  // Mouths
  const mouthY = isBeast ? eyeY - R * 0.48 : eyeY - R * (isSlime ? 0.32 : 0.4);
  const mouthZ = isBeast ? R * 0.74 + R * 0.28 : faceZ(0, mouthY);
  const mouthAnchor = joint(headCenter, "mouth", 0, mouthY, mouthZ);
  const mouthHex = isRobot ? p.eyes : MOUTH_COLOR;
  const mouthMat = isRobot ? kit.toon(mouthHex, { emissive: 1 }) : kit.flat(mouthHex);
  const mouthWidth = R * (isRobot ? 0.16 : 0.1);
  const mouths = {} as Record<MouthShape, THREE.Object3D>;
  for (const shape of MOUTH_SHAPES) {
    const holder = joint(mouthAnchor, `mouth:${shape}`);
    holder.visible = shape === "smile";
    mouths[shape] = holder;
    if (isBird) continue;
    switch (shape) {
      case "smile":
        kit.mesh(torus(mouthWidth, R * 0.018, Math.PI), mouthMat, holder, [0, mouthWidth * 0.5, 0], { rotation: [0, 0, Math.PI] });
        break;
      case "frown":
        kit.mesh(torus(mouthWidth * 0.85, R * 0.018, Math.PI), mouthMat, holder, [0, -mouthWidth * 0.55, 0]);
        break;
      case "grin":
        kit.mesh(disc(mouthWidth * 1.1, Math.PI, Math.PI), mouthMat, holder, [0, mouthWidth * 0.35, 0]);
        if (!isRobot) kit.mesh(disc(mouthWidth * 0.55, Math.PI, Math.PI), kit.flat("#e8707f"), holder, [0, mouthWidth * 0.05, 0.001]);
        break;
      case "open":
        kit.mesh(disc(mouthWidth * 0.62), mouthMat, holder, [0, 0, 0], { scale: [1, 1.25, 1] });
        break;
      case "flat":
        kit.mesh(roundedBox(mouthWidth * 1.3, R * 0.035, 0.008, 0.004), mouthMat, holder, [0, 0, 0]);
        break;
    }
  }

  // Blush
  const blush: THREE.MeshBasicMaterial[] = [];
  if (!isRobot) {
    const blushMat = new THREE.MeshBasicMaterial({ color: new THREE.Color(BLUSH_COLOR), transparent: true, opacity: 0.32, depthWrite: false });
    blush.push(blushMat);
    for (const side of [1, -1]) {
      const x = side * R * 0.58;
      const y = eyeY - R * 0.24;
      const cheek = joint(headCenter, "cheek", x, y, faceZ(x, y) - 0.002);
      cheek.rotation.y = Math.asin(x / R) * 0.9;
      kit.mesh(disc(R * 0.11), blushMat, cheek, [0, 0, 0], { scale: [1.3, 0.75, 1] });
    }
  }

  // ---- Hair, ears, tail, wings, accessories -------------------------------
  const hairSway = buildHair(kit, spec, headCenter, R, furred);
  const [earL, earR] = buildEars(kit, spec, headCenter, R, furred || isSlime);
  const tail = buildTail(kit, spec, hips, d);
  const [wingL, wingR] = isBird ? [null, null] : buildWings(kit, spec, chest, d);
  buildAccessories(kit, spec, { headCenter, chest, neck, spine, R, d, furred });

  const headTopY =
    d.hipY + (isSlime ? 0 : d.torsoH + 0.02) + d.headCenterY + R * (isRobot ? 0.9 : 0.94) + (spec.hair === "spiky" || spec.hair === "mohawk" ? R * 0.35 : 0);
  return {
    spec,
    root,
    body,
    hips,
    spine,
    chest,
    neck,
    head,
    armL: left.arm,
    armR: right.arm,
    elbowL: left.elbow,
    elbowR: right.elbow,
    legL: leftLeg.leg,
    legR: rightLeg.leg,
    kneeL: leftLeg.knee,
    kneeR: rightLeg.knee,
    handR: right.hand,
    tail,
    wingL,
    wingR,
    earL,
    earR,
    hairSway,
    eyes,
    eyeBaseOpen: spec.eyes === "sleepy" ? 0.6 : 1,
    mouths,
    brows,
    blush,
    headTopY,
    deskY: isSlime ? d.headR * 0.55 : d.hipY + d.torsoH * 0.2,
    hasLegs: !isSlime,
  };
}

function outfitTorsoColor(spec: CharacterSpec): string {
  switch (spec.outfit) {
    case "overalls":
      return spec.palette.secondary;
    case "armor":
      return spec.palette.secondary;
    default:
      return spec.palette.primary;
  }
}

function legColor(spec: CharacterSpec): string {
  const p = spec.palette;
  if (spec.body === "beast" && spec.outfit === "fur") return p.skin;
  switch (spec.outfit) {
    case "overalls":
      return p.primary;
    case "dress":
    case "robe":
    case "kimono":
      return shade(p.secondary, BLACK, 0.2);
    case "fur":
    case "plating":
      return p.skin;
    default:
      return p.secondary;
  }
}

function lowerLegColor(spec: CharacterSpec): string {
  if (spec.outfit === "dress") return spec.palette.skin;
  return legColor(spec);
}

function dressTorso(kit: Kit, spec: CharacterSpec, spine: THREE.Object3D, d: Dimensions): void {
  const p = spec.palette;
  const top = d.torsoH;
  const front = d.torsoR * 0.78;
  switch (spec.outfit) {
    case "hoodie": {
      kit.mesh(torus(d.torsoR * 0.72, d.torsoR * 0.22), kit.toon(p.secondary), spine, [0, top * 0.92, -d.torsoR * 0.12], {
        rotation: [Math.PI / 2 - 0.25, 0, 0],
        outline: OUTLINE,
      });
      kit.mesh(roundedBox(d.torsoR * 1.1, top * 0.22, 0.02, 0.01), kit.toon(p.secondary), spine, [0, top * 0.3, front]);
      for (const side of [1, -1]) {
        kit.mesh(cylinder(0.004, 0.004, top * 0.2, 6), kit.toon(p.accent), spine, [side * d.torsoR * 0.22, top * 0.72, front + 0.01]);
      }
      break;
    }
    case "jacket": {
      kit.mesh(roundedBox(0.008, top * 0.8, 0.012, 0.003), kit.toon(p.accent), spine, [0, top * 0.5, front + 0.004]);
      kit.mesh(torus(d.torsoR * 0.62, d.torsoR * 0.14), kit.toon(p.secondary), spine, [0, top * 0.92, 0], {
        rotation: [Math.PI / 2, 0, 0],
        outline: OUTLINE,
      });
      break;
    }
    case "suit": {
      kit.mesh(cone(d.torsoR * 0.42, top * 0.5, 3), kit.toon("#f8fafc"), spine, [0, top * 0.68, front - 0.004], {
        rotation: [0, 0, Math.PI],
        scale: [1, 1, 0.25],
      });
      kit.mesh(roundedBox(d.torsoR * 0.16, top * 0.42, 0.012, 0.005), kit.toon(p.accent), spine, [0, top * 0.6, front + 0.008]);
      break;
    }
    case "kimono":
    case "robe":
    case "dress":
    case "labcoat": {
      const skirtColor = spec.outfit === "labcoat" ? p.primary : p.primary;
      const length = spec.outfit === "dress" ? d.legUpper * 0.95 : spec.outfit === "labcoat" ? d.legUpper * 1.2 : d.legUpper + d.legLower * 0.7;
      kit.mesh(cylinder(d.torsoR * 0.95, d.torsoR * (spec.outfit === "dress" ? 1.75 : 1.4), length, 24), kit.toon(skirtColor), spine, [
        0,
        -length / 2 + 0.02,
        0,
      ], { scale: [1, 1, 0.85], outline: OUTLINE * 0.6 });
      kit.mesh(cylinder(d.torsoR * 1.02, d.torsoR * 1.02, top * 0.16, 24), kit.toon(spec.outfit === "kimono" ? p.secondary : p.accent), spine, [
        0,
        top * 0.22,
        0,
      ], { scale: [1, 1, 0.8] });
      if (spec.outfit === "kimono" || spec.outfit === "robe") {
        for (const side of [1, -1]) {
          kit.mesh(roundedBox(0.012, top * 0.6, 0.01, 0.004), kit.toon(p.accent), spine, [side * d.torsoR * 0.18, top * 0.62, front], {
            rotation: [0, 0, side * 0.45],
          });
        }
      }
      if (spec.outfit === "labcoat") {
        kit.mesh(roundedBox(0.008, top * 0.85, 0.01, 0.003), kit.toon(p.secondary), spine, [0, top * 0.45, front + 0.004]);
      }
      break;
    }
    case "armor": {
      kit.mesh(sphere(d.torsoR * 0.95), kit.toon(p.primary), spine, [0, top * 0.62, d.torsoR * 0.1], {
        scale: [1.05, 0.75, 0.85],
        outline: OUTLINE,
      });
      kit.mesh(cylinder(d.torsoR * 1.03, d.torsoR * 1.03, top * 0.12, 24), kit.toon(p.accent), spine, [0, top * 0.18, 0], { scale: [1, 1, 0.8] });
      break;
    }
    case "overalls": {
      kit.mesh(roundedBox(d.torsoR * 1.3, top * 0.5, 0.03, 0.012), kit.toon(p.primary), spine, [0, top * 0.38, front - 0.006]);
      for (const side of [1, -1]) {
        kit.mesh(roundedBox(0.018, top * 0.5, 0.012, 0.005), kit.toon(p.primary), spine, [side * d.torsoR * 0.45, top * 0.72, front - 0.004]);
        kit.mesh(sphere(0.01), kit.toon(p.accent), spine, [side * d.torsoR * 0.45, top * 0.6, front + 0.012]);
      }
      kit.mesh(cylinder(d.torsoR * 1.01, d.torsoR * 1.01, top * 0.24, 24), kit.toon(p.primary), spine, [0, top * 0.1, 0], { scale: [1, 1, 0.8] });
      break;
    }
    case "tunic": {
      kit.mesh(cylinder(d.torsoR * 0.98, d.torsoR * 1.2, d.legUpper * 0.6, 24), kit.toon(p.primary), spine, [0, -d.legUpper * 0.22, 0], {
        scale: [1, 1, 0.82],
        outline: OUTLINE * 0.6,
      });
      kit.mesh(cylinder(d.torsoR * 1.02, d.torsoR * 1.02, top * 0.1, 24), kit.toon(p.secondary), spine, [0, top * 0.2, 0], { scale: [1, 1, 0.8] });
      break;
    }
    default:
      break;
  }
}

function buildHair(kit: Kit, spec: CharacterSpec, headCenter: THREE.Object3D, R: number, furred: boolean): THREE.Object3D[] {
  const sway: THREE.Object3D[] = [];
  if (spec.hair === "none" || spec.body === "robot" || spec.body === "slime") return sway;
  const hairMat = kit.toon(spec.palette.hair);
  const capOnly = furred; // beasts and birds get a tuft rather than a full scalp
  const cap = () => {
    kit.mesh(new THREE.SphereGeometry(R * 1.07, 32, 16, 0, Math.PI * 2, 0, Math.PI * 0.52), hairMat, headCenter, [0, R * 0.02, -R * 0.03], {
      rotation: [-0.28, 0, 0],
      scale: [1.02, 0.96, 1],
      outline: OUTLINE * 0.5,
    });
  };
  const fringe = () => {
    for (const x of [-0.42, 0, 0.42]) {
      kit.mesh(sphere(R * 0.3), hairMat, headCenter, [x * R, R * 0.5, R * 0.72], { scale: [1.1, 0.65, 0.5], rotation: [0, 0, -x * 0.6] });
    }
  };
  const swayJoint = (x: number, y: number, z: number) => {
    const node = joint(headCenter, "hairSway", x, y, z);
    sway.push(node);
    return node;
  };
  if (capOnly) {
    const tuft = swayJoint(0, R * 0.88, R * 0.1);
    const count = spec.hair === "spiky" ? 5 : spec.hair === "curly" ? 4 : 3;
    for (let i = 0; i < count; i += 1) {
      const angle = (i - (count - 1) / 2) * 0.45;
      kit.mesh(cone(R * 0.12, R * 0.38, 8), hairMat, tuft, [Math.sin(angle) * R * 0.18, R * 0.12, 0], { rotation: [0.2, 0, -angle], outline: OUTLINE });
    }
    return sway;
  }
  switch (spec.hair) {
    case "short":
      cap();
      fringe();
      break;
    case "bob":
      cap();
      fringe();
      for (const side of [1, -1]) {
        kit.mesh(sphere(R * 0.5), hairMat, headCenter, [side * R * 0.78, -R * 0.2, -R * 0.08], { scale: [0.55, 1.15, 1.2], outline: OUTLINE * 0.5 });
      }
      kit.mesh(sphere(R * 0.9), hairMat, headCenter, [0, -R * 0.1, -R * 0.38], { scale: [1.12, 1.0, 0.75], outline: OUTLINE * 0.5 });
      break;
    case "long": {
      cap();
      fringe();
      const fall = swayJoint(0, R * 0.3, -R * 0.55);
      kit.mesh(capsule(R * 0.72, R * 1.2), hairMat, fall, [0, -R * 0.95, 0], { scale: [1.15, 1, 0.55], outline: OUTLINE * 0.5 });
      for (const side of [1, -1]) {
        kit.mesh(capsule(R * 0.22, R * 0.9), hairMat, headCenter, [side * R * 0.82, -R * 0.55, R * 0.1], { outline: OUTLINE * 0.5 });
      }
      break;
    }
    case "ponytail": {
      cap();
      fringe();
      const tail = swayJoint(0, R * 0.55, -R * 0.88);
      kit.mesh(torus(R * 0.14, R * 0.05), kit.toon(spec.palette.accent), tail, [0, 0, 0], { rotation: [0.6, 0, 0] });
      kit.mesh(capsule(R * 0.24, R * 0.95), hairMat, tail, [0, -R * 0.6, -R * 0.18], { rotation: [-0.35, 0, 0], outline: OUTLINE });
      break;
    }
    case "twintails":
      cap();
      fringe();
      for (const side of [1, -1]) {
        const tail = swayJoint(side * R * 0.86, R * 0.42, -R * 0.2);
        kit.mesh(sphere(R * 0.1), kit.toon(spec.palette.accent), tail, [0, 0, 0]);
        kit.mesh(capsule(R * 0.22, R * 1.0), hairMat, tail, [side * R * 0.12, -R * 0.62, 0], { rotation: [0, 0, side * 0.18], outline: OUTLINE });
      }
      break;
    case "spiky":
      cap();
      for (let i = 0; i < 8; i += 1) {
        const yaw = (i / 8) * Math.PI * 2;
        const lift = i % 2 === 0 ? 0.55 : 0.85;
        const direction = new THREE.Vector3(Math.sin(yaw) * Math.cos(lift), Math.sin(lift), Math.cos(yaw) * Math.cos(lift) - 0.3).normalize();
        const spike = new THREE.Mesh(cone(R * 0.2, R * 0.6, 10), hairMat);
        spike.position.copy(direction.clone().multiplyScalar(R * 0.98));
        spike.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), direction);
        headCenter.add(spike);
      }
      fringe();
      break;
    case "bun":
      cap();
      fringe();
      kit.mesh(sphere(R * 0.36), hairMat, headCenter, [0, R * 0.82, -R * 0.42], { outline: OUTLINE });
      break;
    case "mohawk":
      for (let i = 0; i < 5; i += 1) {
        const pitch = -0.9 + i * 0.45;
        const direction = new THREE.Vector3(0, Math.cos(pitch), Math.sin(pitch) * -1).normalize();
        const spike = new THREE.Mesh(cone(R * 0.14, R * 0.55, 10), hairMat);
        spike.position.copy(direction.clone().multiplyScalar(R * 0.95));
        spike.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), direction);
        headCenter.add(spike);
      }
      break;
    case "curly":
      for (let i = 0; i < 14; i += 1) {
        const yaw = (i / 14) * Math.PI * 2;
        const lift = i % 2 === 0 ? 0.35 : 0.85;
        const direction = new THREE.Vector3(Math.sin(yaw) * Math.cos(lift), Math.sin(lift), Math.cos(yaw) * Math.cos(lift) - 0.25).normalize();
        kit.mesh(sphere(R * 0.3), hairMat, headCenter, direction.multiplyScalar(R * 0.9).toArray() as [number, number, number], {
          outline: OUTLINE * 0.6,
        });
      }
      break;
    default:
      break;
  }
  return sway;
}

function buildEars(
  kit: Kit,
  spec: CharacterSpec,
  headCenter: THREE.Object3D,
  R: number,
  useSkin: boolean,
): [THREE.Object3D | null, THREE.Object3D | null] {
  if (spec.ears === "none") return [null, null];
  const p = spec.palette;
  const earColor = useSkin ? p.skin : spec.ears === "human" || spec.ears === "elf" ? p.skin : p.hair;
  const earMat = kit.toon(earColor);
  const innerMat = kit.toon(INNER_EAR_COLOR);
  const result: THREE.Object3D[] = [];
  for (const side of [1, -1] as const) {
    let ear: THREE.Object3D;
    switch (spec.ears) {
      case "human":
        ear = joint(headCenter, "ear", side * R * 0.97, -R * 0.06, 0);
        kit.mesh(sphere(R * 0.17), earMat, ear, [0, 0, 0], { scale: [0.5, 1, 0.7], outline: OUTLINE });
        break;
      case "elf":
        ear = joint(headCenter, "ear", side * R * 0.94, -R * 0.02, -R * 0.02);
        kit.mesh(cone(R * 0.13, R * 0.6, 8), earMat, ear, [side * R * 0.22, R * 0.1, 0], {
          rotation: [0, 0, -side * (Math.PI / 2 - 0.4)],
          scale: [1, 1, 0.45],
          outline: OUTLINE,
        });
        break;
      case "cat":
      case "fox": {
        const tall = spec.ears === "fox" ? 0.72 : 0.5;
        ear = joint(headCenter, "ear", side * R * 0.55, R * 0.72, -R * 0.05);
        ear.rotation.z = -side * 0.32;
        kit.mesh(cone(R * 0.32, R * tall, 4), earMat, ear, [0, R * tall * 0.4, 0], { rotation: [0, Math.PI / 4, 0], scale: [1, 1, 0.55], outline: OUTLINE });
        kit.mesh(cone(R * 0.18, R * tall * 0.65, 4), innerMat, ear, [0, R * tall * 0.33, R * 0.07], { rotation: [0, Math.PI / 4, 0], scale: [1, 1, 0.3] });
        break;
      }
      case "bunny":
        ear = joint(headCenter, "ear", side * R * 0.32, R * 0.8, -R * 0.05);
        ear.rotation.z = -side * 0.14;
        kit.mesh(capsule(R * 0.15, R * 0.85), earMat, ear, [0, R * 0.55, 0], { scale: [1, 1, 0.55], outline: OUTLINE });
        kit.mesh(capsule(R * 0.08, R * 0.65), innerMat, ear, [0, R * 0.55, R * 0.06], { scale: [1, 1, 0.3] });
        break;
      case "bear":
        ear = joint(headCenter, "ear", side * R * 0.68, R * 0.7, -R * 0.05);
        kit.mesh(sphere(R * 0.26), earMat, ear, [0, 0, 0], { scale: [1, 1, 0.6], outline: OUTLINE });
        kit.mesh(disc(R * 0.14), innerMat, ear, [0, 0, R * 0.16]);
        break;
      case "floppy":
        ear = joint(headCenter, "ear", side * R * 0.9, R * 0.35, -R * 0.05);
        ear.rotation.z = side * 0.3;
        kit.mesh(capsule(R * 0.17, R * 0.62), kit.toon(useSkin ? shade(p.skin, BLACK, 0.18) : p.hair), ear, [side * R * 0.06, -R * 0.42, 0], {
          scale: [0.7, 1, 1],
          outline: OUTLINE,
        });
        break;
      case "horns":
        ear = joint(headCenter, "ear", side * R * 0.48, R * 0.78, R * 0.05);
        ear.rotation.set(-0.25, 0, -side * 0.42);
        kit.mesh(cone(R * 0.11, R * 0.55, 10), kit.toon(BONE_COLOR), ear, [0, R * 0.25, 0], { outline: OUTLINE });
        break;
      case "antenna":
        ear = joint(headCenter, "ear", side * R * 0.42, R * 0.82, 0);
        ear.rotation.z = -side * 0.25;
        kit.mesh(cylinder(0.006, 0.006, R * 0.42, 6), kit.toon(shade(p.skin, BLACK, 0.35)), ear, [0, R * 0.21, 0]);
        kit.mesh(sphere(R * 0.09), kit.toon(p.accent, { emissive: 0.9 }), ear, [0, R * 0.45, 0]);
        break;
      case "fin":
        ear = joint(headCenter, "ear", side * R * 0.95, 0, -R * 0.08);
        kit.mesh(cone(R * 0.3, R * 0.5, 3), kit.toon(p.accent), ear, [side * R * 0.18, 0, 0], {
          rotation: [0, 0, -side * 1.25],
          scale: [1, 1, 0.22],
          outline: OUTLINE,
        });
        break;
      default:
        return [null, null];
    }
    result.push(ear);
  }
  return [result[0] ?? null, result[1] ?? null];
}

function buildTail(kit: Kit, spec: CharacterSpec, hips: THREE.Object3D, d: Dimensions): THREE.Object3D[] {
  if (spec.tail === "none" || spec.body === "slime") return [];
  const p = spec.palette;
  const furColor = spec.body === "beast" || spec.body === "bird" ? p.skin : p.hair;
  const chain: THREE.Object3D[] = [];
  let parent: THREE.Object3D = joint(hips, "tailRoot", 0, 0.03, -d.torsoR * 0.85);
  const segments = spec.tail === "bunny" ? 1 : spec.tail === "feather" ? 1 : 3;
  for (let i = 0; i < segments; i += 1) {
    const offset = i === 0 ? 0 : spec.tail === "fox" ? 0.07 : 0.06;
    const node = joint(parent, "tail", 0, offset * 0.6, -offset * 0.8);
    chain.push(node);
    parent = node;
  }
  const t = (i: number) => chain[Math.min(i, chain.length - 1)];
  switch (spec.tail) {
    case "cat":
      chain.forEach((node, i) => {
        node.rotation.x = -0.45;
        kit.mesh(capsule(0.016 - i * 0.002, 0.05), kit.toon(furColor), node, [0, 0.025, -0.02], { rotation: [-0.9, 0, 0], outline: OUTLINE });
      });
      break;
    case "fox":
      kit.mesh(sphere(0.06), kit.toon(furColor), t(1), [0, 0.03, -0.05], { scale: [0.8, 0.8, 1.5], outline: OUTLINE });
      kit.mesh(sphere(0.04), kit.toon(shade(furColor, WHITE, 0.85)), t(2), [0, 0.06, -0.08], { scale: [0.8, 0.8, 1.2], outline: OUTLINE });
      break;
    case "bunny":
      kit.mesh(sphere(0.035), kit.toon(shade(furColor, WHITE, 0.6)), t(0), [0, 0, 0], { outline: OUTLINE });
      break;
    case "dragon":
      chain.forEach((node, i) => {
        node.rotation.x = 0.25;
        kit.mesh(cone(0.03 - i * 0.008, 0.09, 10), kit.toon(spec.body === "humanoid" ? p.hair : p.skin), node, [0, -0.015, -0.04], {
          rotation: [-Math.PI / 2 - 0.3, 0, 0],
          outline: OUTLINE,
        });
        kit.mesh(cone(0.012, 0.025, 4), kit.toon(p.accent), node, [0, 0.012, -0.03]);
      });
      break;
    case "cable":
      chain.forEach((node, i) => {
        node.rotation.x = 0.3;
        kit.mesh(cylinder(0.008, 0.008, 0.07, 8), kit.toon("#1f2937"), node, [0, -0.01, -0.035], { rotation: [Math.PI / 2, 0, 0] });
        if (i === chain.length - 1) kit.mesh(roundedBox(0.03, 0.02, 0.035, 0.005), kit.toon(p.accent, { emissive: 0.6 }), node, [0, -0.01, -0.08]);
      });
      break;
    case "feather":
      for (const spread of [-0.45, 0, 0.45]) {
        kit.mesh(sphere(0.04), kit.toon(shade(furColor, BLACK, 0.15)), t(0), [Math.sin(spread) * 0.03, 0.01, -0.04], {
          rotation: [-0.6, spread, 0],
          scale: [0.45, 0.25, 1.4],
          outline: OUTLINE,
        });
      }
      break;
    default:
      break;
  }
  return chain;
}

function buildWings(kit: Kit, spec: CharacterSpec, chest: THREE.Object3D, d: Dimensions): [THREE.Object3D | null, THREE.Object3D | null] {
  if (spec.wings === "none") return [null, null];
  const p = spec.palette;
  const wings: THREE.Object3D[] = [];
  for (const side of [1, -1] as const) {
    const wing = joint(chest, side === 1 ? "wingL" : "wingR", side * d.torsoR * 0.35, d.torsoH * 0.25, -d.torsoR * 0.7);
    switch (spec.wings) {
      case "feather":
        for (let i = 0; i < 3; i += 1) {
          kit.mesh(sphere(0.07 - i * 0.012), kit.toon(i === 0 ? "#ffffff" : shade("#ffffff", BLACK, 0.06 * i)), wing, [side * (0.06 + i * 0.045), 0.04 - i * 0.04, -0.02], {
            rotation: [0, 0, side * (0.5 + i * 0.35)],
            scale: [1.9, 0.55, 0.25],
            outline: OUTLINE,
          });
        }
        break;
      case "bat": {
        const shape = new THREE.Shape();
        shape.moveTo(0, 0);
        shape.lineTo(0.2, 0.12);
        shape.quadraticCurveTo(0.16, 0.02, 0.19, -0.05);
        shape.quadraticCurveTo(0.12, -0.02, 0.1, -0.08);
        shape.quadraticCurveTo(0.06, -0.02, 0, -0.04);
        shape.lineTo(0, 0);
        const membrane = new THREE.ShapeGeometry(shape);
        kit.mesh(membrane, kit.toon(shade(p.accent, BLACK, 0.35), { side: THREE.DoubleSide }), wing, [0, 0, -0.01], {
          scale: [side, 1, 1],
        });
        break;
      }
      case "fairy":
        for (const [y, s] of [
          [0.05, 1],
          [-0.04, 0.7],
        ] as const) {
          kit.mesh(disc(0.08 * s), kit.toon(p.accent, { opacity: 0.45, side: THREE.DoubleSide, emissive: 0.4 }), wing, [side * 0.07 * s, y, -0.01], {
            rotation: [0, side * 0.3, side * (0.45 - y * 4)],
            scale: [1.4, 0.75, 1],
          });
        }
        break;
      case "jet":
        kit.mesh(cylinder(0.025, 0.03, 0.12, 14), kit.toon(shade(p.secondary, WHITE, 0.15)), wing, [side * 0.04, 0, -0.02], { outline: OUTLINE });
        kit.mesh(cone(0.022, 0.06, 12), kit.toon(p.accent, { emissive: 1.2, opacity: 0.85 }), wing, [side * 0.04, -0.09, -0.02], {
          rotation: [Math.PI, 0, 0],
        });
        break;
      default:
        break;
    }
    wings.push(wing);
  }
  return [wings[0] ?? null, wings[1] ?? null];
}

interface AccessoryAnchors {
  headCenter: THREE.Object3D;
  chest: THREE.Object3D;
  neck: THREE.Object3D;
  spine: THREE.Object3D;
  R: number;
  d: Dimensions;
  furred: boolean;
}

function buildAccessories(kit: Kit, spec: CharacterSpec, a: AccessoryAnchors): void {
  const p = spec.palette;
  const { headCenter, chest, neck, spine, R, d } = a;
  const isSlime = spec.body === "slime";
  const topY = isSlime ? R * 0.78 : R * 0.9;
  const eyeY = isSlime ? R * 0.12 : -R * 0.05;
  const faceFront = spec.body === "robot" ? R * 0.87 : R * 0.97;
  for (const accessory of spec.accessories) {
    switch (accessory) {
      case "glasses": {
        const mat = kit.toon("#1f2937");
        for (const side of [1, -1]) {
          kit.mesh(torus(R * 0.17, R * 0.022), mat, headCenter, [side * R * 0.36, eyeY, faceFront], { rotation: [0, side * 0.32, 0] });
        }
        kit.mesh(cylinder(R * 0.015, R * 0.015, R * 0.2, 6), mat, headCenter, [0, eyeY + R * 0.02, faceFront + 0.01], { rotation: [0, 0, Math.PI / 2] });
        break;
      }
      case "goggles": {
        const strap = kit.toon(shade(p.secondary, BLACK, 0.3));
        kit.mesh(torus(R * 1.0, R * 0.05), strap, headCenter, [0, R * 0.42, 0], { rotation: [Math.PI / 2 - 0.25, 0, 0] });
        for (const side of [1, -1]) {
          kit.mesh(cylinder(R * 0.17, R * 0.17, R * 0.12, 18), kit.toon(p.accent), headCenter, [side * R * 0.3, R * 0.55, R * 0.82], {
            rotation: [Math.PI / 2 - 0.5, 0, 0],
            outline: OUTLINE,
          });
          kit.mesh(disc(R * 0.13), kit.toon("#bae6fd", { emissive: 0.3 }), headCenter, [side * R * 0.3, R * 0.58, R * 0.89], { rotation: [-0.5, 0, 0] });
        }
        break;
      }
      case "headphones": {
        const mat = kit.toon(shade(p.accent, BLACK, 0.25));
        kit.mesh(torus(R * 1.05, R * 0.06, Math.PI), mat, headCenter, [0, R * 0.05, 0], { outline: OUTLINE });
        for (const side of [1, -1]) {
          kit.mesh(cylinder(R * 0.24, R * 0.24, R * 0.16, 20), kit.toon(p.accent), headCenter, [side * R * 1.02, R * 0.02, 0], {
            rotation: [0, 0, Math.PI / 2],
            outline: OUTLINE,
          });
        }
        break;
      }
      case "scarf": {
        const mat = kit.toon(p.accent);
        if (isSlime) {
          kit.mesh(torus(R * 0.85, R * 0.12), mat, headCenter, [0, -R * 0.35, 0], { rotation: [Math.PI / 2, 0, 0], outline: OUTLINE });
        } else {
          kit.mesh(torus(d.torsoR * 0.75, d.torsoR * 0.28), mat, neck, [0, -0.005, 0], { rotation: [Math.PI / 2, 0, 0], outline: OUTLINE });
          kit.mesh(roundedBox(d.torsoR * 0.4, d.torsoH * 0.5, 0.02, 0.008), mat, chest, [d.torsoR * 0.35, d.torsoH * 0.25, d.torsoR * 0.8], {
            rotation: [0.1, 0, 0.12],
          });
        }
        break;
      }
      case "beanie": {
        const mat = kit.toon(p.secondary);
        kit.mesh(new THREE.SphereGeometry(R * 1.1, 28, 14, 0, Math.PI * 2, 0, Math.PI * 0.48), mat, headCenter, [0, R * 0.1, -R * 0.02], {
          rotation: [-0.2, 0, 0],
          outline: OUTLINE * 0.6,
        });
        kit.mesh(torus(R * 1.02, R * 0.1), kit.toon(shade(p.secondary, WHITE, 0.25)), headCenter, [0, R * 0.3, R * 0.02], { rotation: [Math.PI / 2 - 0.2, 0, 0] });
        kit.mesh(sphere(R * 0.18), kit.toon(p.accent), headCenter, [0, R * 1.18, -R * 0.2], { outline: OUTLINE });
        break;
      }
      case "witch_hat": {
        const mat = kit.toon(shade(p.primary, BLACK, 0.25));
        kit.mesh(cylinder(R * 1.45, R * 1.45, R * 0.06, 32), mat, headCenter, [0, topY - R * 0.05, 0], { outline: OUTLINE * 0.5 });
        kit.mesh(cone(R * 0.75, R * 1.5, 24), mat, headCenter, [0, topY + R * 0.7, -R * 0.12], { rotation: [-0.25, 0, 0.12], outline: OUTLINE * 0.5 });
        kit.mesh(cylinder(R * 0.78, R * 0.76, R * 0.16, 28), kit.toon(p.accent), headCenter, [0, topY + R * 0.06, -R * 0.02]);
        break;
      }
      case "crown": {
        const gold = kit.toon(GOLD_COLOR, { emissive: 0.25 });
        kit.mesh(cylinder(R * 0.5, R * 0.45, R * 0.22, 24, ), gold, headCenter, [0, topY + R * 0.05, 0], { outline: OUTLINE });
        for (let i = 0; i < 5; i += 1) {
          const angle = (i / 5) * Math.PI * 2;
          kit.mesh(cone(R * 0.1, R * 0.26, 4), gold, headCenter, [Math.sin(angle) * R * 0.45, topY + R * 0.26, Math.cos(angle) * R * 0.45]);
        }
        kit.mesh(sphere(R * 0.07), kit.toon(p.accent, { emissive: 0.6 }), headCenter, [0, topY + R * 0.06, R * 0.48]);
        break;
      }
      case "halo":
        kit.mesh(torus(R * 0.6, R * 0.06), kit.toon(GOLD_COLOR, { emissive: 1.2 }), headCenter, [0, topY + R * 0.45, -R * 0.05], { rotation: [Math.PI / 2 - 0.2, 0, 0] });
        break;
      case "cape": {
        if (isSlime) break;
        const capeGeometry = new THREE.CylinderGeometry(d.torsoR * 1.05, d.torsoR * 1.9, d.torsoH + d.legUpper * 1.4, 24, 1, true, Math.PI * 0.5, Math.PI);
        kit.mesh(capeGeometry, kit.toon(p.accent === p.primary ? shade(p.primary, BLACK, 0.3) : shade(p.accent, BLACK, 0.2), { side: THREE.DoubleSide }), spine, [
          0,
          d.torsoH * 0.95 - (d.torsoH + d.legUpper * 1.4) / 2,
          -d.torsoR * 0.12,
        ], { scale: [1, 1, 0.75] });
        kit.mesh(sphere(0.014), kit.toon(GOLD_COLOR), chest, [d.torsoR * 0.55, d.torsoH * 0.5, d.torsoR * 0.72]);
        break;
      }
      case "backpack":
        if (isSlime) break;
        kit.mesh(roundedBox(d.torsoR * 1.5, d.torsoH * 0.75, d.torsoR * 0.8, 0.03), kit.toon(p.secondary), chest, [0, d.torsoH * 0.05, -d.torsoR * 1.05], {
          outline: OUTLINE,
        });
        kit.mesh(roundedBox(d.torsoR * 1.0, d.torsoH * 0.3, 0.03, 0.01), kit.toon(p.accent), chest, [0, -d.torsoH * 0.05, -d.torsoR * 1.5]);
        break;
      case "headband":
        kit.mesh(torus(R * 0.99, R * 0.06), kit.toon(p.accent), headCenter, [0, R * 0.32, 0], { rotation: [Math.PI / 2 - 0.12, 0, 0] });
        break;
      case "bow": {
        const mat = kit.toon(p.accent);
        const anchor = joint(headCenter, "bow", R * 0.55, R * 0.72, R * 0.15);
        anchor.rotation.z = -0.4;
        for (const side of [1, -1]) {
          kit.mesh(cone(R * 0.2, R * 0.32, 4), mat, anchor, [side * R * 0.16, 0, 0], { rotation: [0, 0, side * Math.PI / 2], scale: [1, 1, 0.5], outline: OUTLINE });
        }
        kit.mesh(sphere(R * 0.08), mat, anchor, [0, 0, 0]);
        break;
      }
      case "flower": {
        const anchor = joint(headCenter, "flower", -R * 0.62, R * 0.62, R * 0.32);
        anchor.rotation.set(0.3, -0.5, 0);
        for (let i = 0; i < 5; i += 1) {
          const angle = (i / 5) * Math.PI * 2;
          kit.mesh(sphere(R * 0.1), kit.toon(p.accent), anchor, [Math.cos(angle) * R * 0.11, Math.sin(angle) * R * 0.11, 0], { scale: [1, 1, 0.45] });
        }
        kit.mesh(sphere(R * 0.07), kit.toon("#fde047"), anchor, [0, 0, R * 0.03]);
        break;
      }
      case "bell": {
        const collar = kit.toon(p.accent);
        if (isSlime) {
          kit.mesh(sphere(R * 0.12), kit.toon(GOLD_COLOR, { emissive: 0.2 }), headCenter, [0, -R * 0.35, R * 0.9], { outline: OUTLINE });
        } else {
          kit.mesh(torus(d.torsoR * 0.62, 0.01), collar, neck, [0, -0.01, 0], { rotation: [Math.PI / 2, 0, 0] });
          kit.mesh(sphere(0.022), kit.toon(GOLD_COLOR, { emissive: 0.2 }), neck, [0, -0.03, d.torsoR * 0.66], { outline: OUTLINE });
        }
        break;
      }
      default:
        break;
    }
  }
}
