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
import type { CharacterSpec, Pattern } from "./types";

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

// Four soft cel bands: core shadow, shadow, light, highlight.
const TOON_STEPS = [96, 158, 214, 255];
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
const FLAME_CORE_COLOR = "#fde68a";
const STAR_POINTS = 5;

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

  constructor(
    private readonly pattern: Pattern = "none",
    private readonly motif = "#ffffff",
  ) {}

  /** Toon material whose color comes from vertex colors (hair gradients). */
  vertexToon(): THREE.MeshToonMaterial {
    const cached = this.materials.get("vertex-toon");
    if (cached) return cached as THREE.MeshToonMaterial;
    const material = new THREE.MeshToonMaterial({ color: 0xffffff, vertexColors: true, gradientMap: gradientMap() });
    this.materials.set("vertex-toon", material);
    return material;
  }

  /** Outfit cloth: patterned when the spec declares a pattern. */
  cloth(hex: string, options: ToonOptions = {}): THREE.MeshToonMaterial {
    if (this.pattern === "none") return this.toon(hex, options);
    const key = `cloth:${hex}:${this.pattern}:${this.motif}:${options.side ?? THREE.FrontSide}`;
    const cached = this.materials.get(key);
    if (cached) return cached as THREE.MeshToonMaterial;
    const material = new THREE.MeshToonMaterial({
      color: 0xffffff,
      map: patternTexture(this.pattern, hex, motifColor(hex, this.motif)),
      gradientMap: gradientMap(),
      side: options.side ?? THREE.FrontSide,
    });
    this.materials.set(key, material);
    return material;
  }

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
    options: { rotation?: [number, number, number]; scale?: [number, number, number]; outline?: number; ink?: string } = {},
  ): THREE.Mesh {
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.set(...position);
    if (options.rotation) mesh.rotation.set(...options.rotation);
    if (options.scale) mesh.scale.set(...options.scale);
    if (options.outline && "color" in material) {
      const hex = options.ink ?? `#${(material as THREE.MeshToonMaterial).color.getHexString()}`;
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

const OUTLINE = 0.032;
/** Bangs reach from the hairline down to the brows. */
const BANG_LENGTH_SCALE = 1.5;
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

/** Vertex colors fading from `from` to `to` along +Y, kicking in past `start`. */
function verticalGradient(geometry: THREE.BufferGeometry, from: string, to: string, start = 0.45): THREE.BufferGeometry {
  geometry.computeBoundingBox();
  const box = geometry.boundingBox;
  const position = geometry.getAttribute("position");
  const colors = new Float32Array(position.count * 3);
  const a = new THREE.Color(from);
  const b = new THREE.Color(to);
  const mixed = new THREE.Color();
  const span = box ? Math.max(box.max.y - box.min.y, 1e-6) : 1;
  for (let i = 0; i < position.count; i += 1) {
    const t = box ? (position.getY(i) - box.min.y) / span : 0;
    mixed.copy(a).lerp(b, THREE.MathUtils.smoothstep(t, start, 1));
    colors.set([mixed.r, mixed.g, mixed.b], i * 3);
  }
  geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
  return geometry;
}

/** Radial folds that deepen toward the hem of a skirt-like cylinder. */
function pleated(geometry: THREE.BufferGeometry, folds: number, depth: number): THREE.BufferGeometry {
  geometry.computeBoundingBox();
  const box = geometry.boundingBox;
  if (!box) return geometry;
  const position = geometry.getAttribute("position");
  const height = Math.max(box.max.y - box.min.y, 1e-6);
  for (let i = 0; i < position.count; i += 1) {
    const x = position.getX(i);
    const z = position.getZ(i);
    const toHem = (box.max.y - position.getY(i)) / height;
    const scale = 1 + depth * toHem * Math.sin(Math.atan2(z, x) * folds);
    position.setXYZ(i, x * scale, position.getY(i), z * scale);
  }
  position.needsUpdate = true;
  geometry.computeVertexNormals();
  return geometry;
}

/** Sphere reshaped into an anime face: narrower jaw, softly pointed chin. */
function animeHead(radius: number): THREE.BufferGeometry {
  const geometry = new THREE.SphereGeometry(radius, 40, 30);
  const position = geometry.getAttribute("position");
  for (let i = 0; i < position.count; i += 1) {
    const y = position.getY(i);
    if (y >= 0) continue;
    const t = -y / radius;
    const front = Math.max(0, position.getZ(i)) / radius;
    position.setX(i, position.getX(i) * (1 - 0.24 * Math.pow(t, 1.4)));
    position.setY(i, y * (1 + 0.06 * front * t));
    position.setZ(i, position.getZ(i) * (1 - 0.08 * t));
  }
  position.needsUpdate = true;
  geometry.computeVertexNormals();
  return geometry;
}

const PATTERN_TEXTURE_SIZE = 96;
const PATTERN_REPEAT = 2;

function hexBytes(hex: string): [number, number, number] {
  const value = Number.parseInt(hex.slice(1), 16);
  return [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
}

/** Picks a motif color with enough contrast against the cloth. */
function motifColor(base: string, accent: string): string {
  const [r1, g1, b1] = hexBytes(base);
  const [r2, g2, b2] = hexBytes(accent);
  const distance = Math.hypot(r1 - r2, g1 - g2, b1 - b2);
  if (distance > 90) return accent;
  const luminance = 0.2126 * r1 + 0.7152 * g1 + 0.0722 * b1;
  return luminance > 140 ? shade(base, BLACK, 0.35) : shade(base, WHITE, 0.55);
}

/** True where the motif covers tile-space coordinate (u, v) in [0, 1). */
function motifAt(pattern: Pattern, u: number, v: number): boolean {
  const cell = (value: number, cells: number) => (value * cells) % 1;
  switch (pattern) {
    case "dots": {
      const dx = cell(u, 4) - 0.5;
      const dy = cell(v + (Math.floor(u * 4) % 2) * 0.125, 4) - 0.5;
      return dx * dx + dy * dy < 0.045;
    }
    case "stripes":
      return cell(u + v, 5) < 0.28;
    case "waves":
      return cell(v * 5 + 0.09 * Math.sin(u * Math.PI * 2 * 3), 1) < 0.16;
    case "petals": {
      const dx = cell(u, 3) - 0.5;
      const dy = cell(v + (Math.floor(u * 3) % 2) * 0.17, 3) - 0.5;
      const radius = Math.hypot(dx, dy);
      const angle = Math.atan2(dy, dx);
      return radius > 0.04 && radius < 0.3 * (0.55 + 0.45 * Math.abs(Math.cos(2.5 * angle)));
    }
    case "stars": {
      const dx = Math.abs(cell(u, 4) - 0.5);
      const dy = Math.abs(cell(v + (Math.floor(u * 4) % 2) * 0.125, 4) - 0.5);
      return Math.sqrt(dx) + Math.sqrt(dy) < 0.5;
    }
    case "diamonds": {
      const dx = Math.abs(cell(u, 4) - 0.5);
      const dy = Math.abs(cell(v, 4) - 0.5);
      const ring = dx + dy;
      return ring > 0.36 && ring < 0.46;
    }
    default:
      return false;
  }
}

/** Builds a tiling cloth texture on the CPU (no canvas needed). */
function patternTexture(pattern: Pattern, base: string, motif: string): THREE.DataTexture {
  const size = PATTERN_TEXTURE_SIZE;
  const data = new Uint8Array(size * size * 4);
  const baseBytes = hexBytes(base);
  const motifBytes = hexBytes(motif);
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const color = motifAt(pattern, (x + 0.5) / size, (y + 0.5) / size) ? motifBytes : baseBytes;
      data.set([color[0], color[1], color[2], 255], (y * size + x) * 4);
    }
  }
  const texture = new THREE.DataTexture(data, size, size, THREE.RGBAFormat);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.repeat.set(PATTERN_REPEAT, PATTERN_REPEAT);
  texture.magFilter = THREE.LinearFilter;
  texture.needsUpdate = true;
  return texture;
}

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
  const p = spec.palette;
  const kit = new Kit(spec.pattern ?? "none", p.accent);
  const d = dimensions(spec);
  const isBeast = spec.body === "beast";
  const isRobot = spec.body === "robot";
  const isSlime = spec.body === "slime";
  const isBird = spec.body === "bird";
  const furred = isBeast || isBird;
  const bareOutfit = spec.outfit === "fur" || spec.outfit === "plating";
  const isGenos = spec.id.endsWith("genos");
  const isPikachu = spec.id.endsWith("pikachu");
  const isKodama = spec.id.endsWith("kodama-spirit");

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
    kit.mesh(capsule(d.torsoR, d.torsoH - d.torsoR * 2), torsoColor === p.skin ? kit.toon(torsoColor) : kit.cloth(torsoColor), spine, [0, d.torsoH / 2, 0], {
      scale: [1, 1, 0.78],
      outline: OUTLINE * 0.5,
      ink: torsoColor,
    });
    if (furred && spec.outfit === "fur") {
      if (!isPikachu) {
        kit.mesh(sphere(d.torsoR * 0.75), kit.toon(shade(p.skin, WHITE, 0.55)), spine, [0, d.torsoH * 0.42, d.torsoR * 0.42], {
          scale: [0.8, 1.05, 0.5],
        });
      } else {
        kit.mesh(roundedBox(d.torsoR * 1.5, 0.018, 0.01, 0.004), kit.toon("#78350f"), spine, [0, d.torsoH * 0.55, -d.torsoR * 0.72], { outline: OUTLINE * 0.4 });
        kit.mesh(roundedBox(d.torsoR * 1.3, 0.018, 0.01, 0.004), kit.toon("#78350f"), spine, [0, d.torsoH * 0.35, -d.torsoR * 0.72], { outline: OUTLINE * 0.4 });
      }
    } else if (!isKodama) {
      dressTorso(kit, spec, spine, d);
    }
  }

  // ---- Arms ----------------------------------------------------------------
  const sleeveColor = isGenos
    ? p.secondary
    : bareOutfit || (furred && spec.outfit === "fur")
      ? p.skin
      : spec.outfit === "armor"
        ? p.secondary
        : outfitTorsoColor(spec);
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
      const sleeve = sleeveColor === p.skin ? kit.toon(sleeveColor) : kit.cloth(sleeveColor);
      kit.mesh(capsule(d.armR, d.armUpper - d.armR), sleeve, arm, [0, -d.armUpper / 2, 0], { outline: OUTLINE, ink: sleeveColor });
      const longSleeves = ["kimono", "robe", "labcoat", "hoodie", "jacket", "suit", "hakama", "coat"].includes(spec.outfit);
      const forearmColor = sleeveColor === p.skin || longSleeves ? sleeveColor : p.skin;
      kit.mesh(capsule(d.armR * 0.92, d.armLower - d.armR), forearmColor === p.skin ? skinMat : kit.cloth(forearmColor), elbow, [0, -d.armLower / 2, 0], {
        outline: OUTLINE,
        ink: forearmColor,
      });
      if (spec.outfit === "kimono" || spec.outfit === "hakama") {
        // Wide hanging sleeve.
        kit.mesh(roundedBox(d.armR * 2.6, d.armLower * 0.9, d.armR * 2.2, d.armR * 0.6), kit.cloth(sleeveColor), elbow, [0, -d.armLower * 0.35, -d.armR * 0.3], {
          outline: OUTLINE,
          ink: sleeveColor,
        });
      }
      kit.mesh(sphere(d.armR * 1.15), isGenos ? kit.toon(p.secondary) : skinMat, hand, [0, 0.01, 0], { scale: [1, 1.05, 0.9], outline: OUTLINE });
      if (isGenos) {
        kit.mesh(torus(d.armR * 1.1, 0.004), kit.toon("#ffd700"), arm, [0, -d.armUpper * 0.45, 0], { rotation: [Math.PI / 2, 0, 0] });
        kit.mesh(torus(d.armR * 1.05, 0.004), kit.toon("#ffd700"), elbow, [0, -d.armLower * 0.45, 0], { rotation: [Math.PI / 2, 0, 0] });
      }
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
  } else if (spec.body === "humanoid") {
    kit.mesh(animeHead(R), kit.toon(p.skin), headCenter, [0, 0, 0], { scale: [1, 0.94, 0.96], outline: OUTLINE * 0.6 });
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
  if (spec.body === "humanoid" && !isKodama) {
    const noseY = eyeY - R * 0.27;
    kit.mesh(disc(R * 0.028), kit.flat(shade(p.skin, BLACK, 0.3)), headCenter, [0, noseY, faceZ(0, noseY) + 0.004], { scale: [0.6, 1, 1] });
  }

  if (isBeast) {
    if (isPikachu) {
      const noseY = eyeY - R * 0.22;
      kit.mesh(disc(R * 0.032), kit.flat("#1c1917"), headCenter, [0, noseY, faceZ(0, noseY) + 0.004], { scale: [1, 0.75, 1] });
      for (const side of [1, -1] as const) {
        const cheekX = side * R * 0.54;
        const cheekY = eyeY - R * 0.26;
        const cheek = joint(headCenter, "cheekPouch", cheekX, cheekY, faceZ(cheekX, cheekY) + 0.005);
        cheek.rotation.y = Math.asin(cheekX / R) * 0.9;
        kit.mesh(disc(R * 0.18), kit.flat("#dc2626"), cheek, [0, 0, 0], { scale: [1.05, 0.95, 1] });
      }
    } else {
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
  }
  if (isBird) {
    kit.mesh(cone(R * 0.17, R * 0.42, 12), kit.toon(p.accent), headCenter, [0, eyeY - R * 0.28, R * 0.98], {
      rotation: [Math.PI / 2, 0, 0],
      outline: OUTLINE,
    });
  }

  // Eyes
  const eyes: THREE.Object3D[] = [];
  const anime = spec.body === "humanoid";
  const eyeSize = R * (spec.eyes === "sparkle" ? 0.24 : anime ? 0.22 : 0.2);
  const isPower = spec.id.endsWith("power");
  const isGojo = spec.id.endsWith("gojo");
  const irisMat = isGenos ? kit.flat("#facc15") : isGojo ? kit.flat("#38bdf8") : kit.flat(p.eyes);
  const pupilMat = isGenos ? kit.toon("#f97316", { emissive: 1.2 }) : kit.flat(shade(p.eyes, BLACK, 0.65));
  const shineMat = kit.flat("#ffffff");
  const rimMat = kit.flat(shade(p.eyes, BLACK, 0.8));
  const lashMat = kit.flat("#1b1220");
  const scleraMat = isGenos ? kit.flat("#0f172a") : kit.flat("#fbfbff");
  const irisDarkMat = isGenos ? kit.flat("#020617") : kit.flat(shade(p.eyes, BLACK, 0.5));
  const irisLightMat = isGenos ? kit.flat("#fef08a") : kit.flat(shade(p.eyes, WHITE, 0.4));
  const buildEye = (x: number, size: number, tilt: number) => {
    const group = joint(headCenter, "eye", x, eyeY, faceZ(x, eyeY));
    group.rotation.y = Math.asin(Math.max(-0.9, Math.min(0.9, x / R))) * 0.9;
    group.rotation.z = tilt;
    const squash = spec.eyes === "sharp" ? 0.72 : spec.eyes === "sleepy" ? 1.0 : 1.2;
    if (isRobot) {
      kit.mesh(disc(size * 1.12), rimMat, group, [0, 0, -0.001], { scale: [0.86, squash, 1] });
      kit.mesh(disc(size), irisMat, group, [0, 0, 0], { scale: [0.8, squash, 1] });
      kit.mesh(disc(size * 0.48), pupilMat, group, [0, -size * 0.08, 0.001], { scale: [0.85, squash, 1] });
      kit.mesh(disc(size * 0.27), shineMat, group, [-size * 0.28, size * 0.36 * squash, 0.002]);
      eyes.push(group);
      return;
    }
    const side = x === 0 ? 1 : Math.sign(x);
    // Layered anime eye, back to front.
    kit.mesh(disc(size * 1.06), scleraMat, group, [0, 0, -0.003], { scale: [1, squash * 0.94, 1] });
    kit.mesh(disc(size * 0.88), irisDarkMat, group, [0, -size * 0.04, -0.002], { scale: [0.8, squash, 1] });
    kit.mesh(disc(size * 0.74), irisMat, group, [0, -size * 0.07, -0.001], { scale: [0.8, squash * 0.98, 1] });
    kit.mesh(disc(size * 0.56, Math.PI, Math.PI), irisLightMat, group, [0, -size * 0.12, 0], { scale: [0.8, squash * 0.95, 1] });
    if (isPower) {
      kit.mesh(roundedBox(size * 0.42, size * 0.1, 0.005, 0.002), kit.flat("#ef4444"), group, [0, -size * 0.03, 0.002]);
      kit.mesh(roundedBox(size * 0.1, size * 0.42, 0.005, 0.002), kit.flat("#ef4444"), group, [0, -size * 0.03, 0.002]);
      kit.mesh(disc(size * 0.12), kit.flat("#fbbf24"), group, [0, -size * 0.03, 0.0025]);
    } else {
      kit.mesh(disc(size * 0.34), pupilMat, group, [0, -size * 0.03, 0.001], { scale: [0.6, squash * 1.05, 1] });
    }
    kit.mesh(disc(size * 0.24), shineMat, group, [-side * size * 0.24, size * 0.3 * squash, 0.002]);
    kit.mesh(disc(size * 0.1), shineMat, group, [side * size * 0.22, -size * 0.34 * squash, 0.002]);
    if (spec.eyes === "sparkle") {
      kit.mesh(disc(size * 0.16), shineMat, group, [side * size * 0.3, size * 0.4, 0.003], { scale: [0.35, 1.5, 1] });
      kit.mesh(disc(size * 0.16), shineMat, group, [side * size * 0.3, size * 0.4, 0.003], { scale: [1.5, 0.35, 1] });
    }
    // Upper lash line with an outer flick, and a short lower lash.
    kit.mesh(torus(size * 0.98, size * 0.13, Math.PI * 0.86), lashMat, group, [0, 0, 0.003], {
      rotation: [0, 0, Math.PI * 0.07],
      scale: [1.05, squash * 0.95, 0.35],
    });
    kit.mesh(cone(size * 0.11, size * 0.34, 6), lashMat, group, [side * size * 1.0, size * 0.22 * squash, 0.003], {
      rotation: [0, 0, -side * 1.15],
      scale: [1, 1, 0.3],
    });
    kit.mesh(torus(size * 0.92, size * 0.035, Math.PI * 0.3), lashMat, group, [0, 0, 0.003], {
      rotation: [0, 0, side > 0 ? -Math.PI * 0.45 : -Math.PI * 0.85],
      scale: [1, squash * 0.95, 0.35],
    });
    if (spec.eyes === "sleepy") {
      kit.mesh(disc(size * 1.12, 0, Math.PI), kit.flat(p.skin), group, [0, size * 0.12, 0.004], { scale: [1.02, 0.7, 1] });
    }
    eyes.push(group);
  };
  if (spec.eyes === "visor") {
    const visor = joint(headCenter, "visor", 0, eyeY, faceZ(0, eyeY));
    kit.mesh(roundedBox(R * 1.35, eyeSize * 1.25, 0.02, eyeSize * 0.5), kit.toon(p.eyes, { emissive: 1.1 }), visor, [0, 0, 0]);
    eyes.push(visor);
  } else if (spec.eyes === "mono") {
    buildEye(0, eyeSize * 1.55, 0);
  } else if (!isKodama) {
    const eyeX = R * (isBird ? 0.42 : anime ? 0.38 : 0.36);
    const tilt = spec.eyes === "sharp" ? 0.18 : 0;
    buildEye(eyeX, eyeSize, tilt);
    buildEye(-eyeX, eyeSize, -tilt);
  } else {
    for (const side of [-1, 1]) {
      const eye = joint(headCenter, `eye:${side}`, side * R * 0.28, eyeY, faceZ(side * R * 0.28, eyeY));
      kit.mesh(disc(R * 0.16), kit.flat("#111827"), eye, [0, 0, 0.007], { scale: [0.9, 1.25, 1] });
      eyes.push(eye);
    }
  }

  // Brows
  const brows: THREE.Object3D[] = [];
  if (!isRobot && !isSlime && !isKodama && spec.eyes !== "visor") {
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
  const mouthWidth = R * (isRobot ? 0.16 : anime ? 0.085 : 0.1);
  const mouths = {} as Record<MouthShape, THREE.Object3D>;
  for (const shape of MOUTH_SHAPES) {
    const holder = joint(mouthAnchor, `mouth:${shape}`);
    holder.visible = shape === "smile";
    mouths[shape] = holder;
    if (isBird || isKodama) continue;
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
  if (!isRobot && !isPikachu && !isKodama) {
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
  buildAccessories(kit, spec, { headCenter, chest, neck, spine, handR: right.hand, handL: left.hand, R, d, furred });

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
    eyeBaseOpen: spec.eyes === "sleepy" && isRobot ? 0.6 : 1,
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
    case "coat":
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
    case "sailor":
      return p.skin;
    default:
      return p.secondary;
  }
}

function lowerLegColor(spec: CharacterSpec): string {
  if (spec.outfit === "dress") return spec.palette.skin;
  if (spec.outfit === "sailor") return shade(spec.palette.secondary, BLACK, 0.3);
  return legColor(spec);
}

function dressTorso(kit: Kit, spec: CharacterSpec, spine: THREE.Object3D, d: Dimensions): void {
  const p = spec.palette;
  const top = d.torsoH;
  const front = d.torsoR * 0.78;
  switch (spec.outfit) {
    case "hakama": {
      for (const side of [1, -1]) {
        kit.mesh(roundedBox(0.012, top * 0.62, 0.01, 0.004), kit.toon(p.accent), spine, [side * d.torsoR * 0.16, top * 0.66, front], {
          rotation: [0, 0, side * 0.5],
        });
      }
      const length = d.legUpper + d.legLower + 0.02;
      const trousers = pleated(new THREE.CylinderGeometry(d.torsoR * 1.0, d.torsoR * 1.75, length, 48, 4), 8, 0.07);
      kit.mesh(trousers, kit.cloth(p.secondary), spine, [0, top * 0.22 - length / 2, 0], { scale: [1, 1, 0.82], outline: OUTLINE * 0.6, ink: p.secondary });
      kit.mesh(cylinder(d.torsoR * 1.03, d.torsoR * 1.03, top * 0.12, 24), kit.toon(p.accent), spine, [0, top * 0.24, 0], { scale: [1, 1, 0.8] });
      break;
    }
    case "sailor": {
      kit.mesh(roundedBox(d.torsoR * 1.7, top * 0.36, 0.012, 0.006), kit.toon(p.secondary), spine, [0, top * 0.8, -d.torsoR * 0.72], {
        rotation: [0.25, 0, 0],
      });
      for (const side of [1, -1]) {
        kit.mesh(roundedBox(0.02, top * 0.42, 0.01, 0.004), kit.toon(p.secondary), spine, [side * d.torsoR * 0.22, top * 0.74, front], {
          rotation: [0, 0, side * 0.55],
        });
      }
      kit.mesh(cone(d.torsoR * 0.22, top * 0.3, 4), kit.toon(p.accent), spine, [0, top * 0.56, front + 0.012], { rotation: [0, 0, Math.PI], scale: [1, 1, 0.3] });
      const skirtLength = d.legUpper * 0.78;
      const skirt = pleated(new THREE.CylinderGeometry(d.torsoR * 0.98, d.torsoR * 1.55, skirtLength, 64, 3), 14, 0.07);
      kit.mesh(skirt, kit.cloth(p.secondary), spine, [0, -skirtLength / 2 + 0.02, 0], { scale: [1, 1, 0.85], outline: OUTLINE * 0.6, ink: p.secondary });
      break;
    }
    case "coat": {
      const length = top * 0.95 + d.legUpper * 1.1;
      const coat = pleated(new THREE.CylinderGeometry(d.torsoR * 1.07, d.torsoR * 1.5, length, 48, 4, true, 0.38, Math.PI * 2 - 0.76), 6, 0.04);
      kit.mesh(coat, kit.cloth(p.primary, { side: THREE.DoubleSide }), spine, [0, top - length / 2, 0], { scale: [1, 1, 0.85] });
      for (const side of [1, -1]) {
        kit.mesh(roundedBox(0.03, top * 0.4, 0.012, 0.006), kit.toon(shade(p.primary, BLACK, 0.2)), spine, [side * d.torsoR * 0.3, top * 0.78, front + 0.006], {
          rotation: [0, 0, side * 0.35],
        });
      }
      break;
    }
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
      const skirtColor = p.primary;
      const length = spec.outfit === "dress" ? d.legUpper * 0.95 : spec.outfit === "labcoat" ? d.legUpper * 1.2 : d.legUpper + d.legLower * 0.7;
      const skirt = pleated(new THREE.CylinderGeometry(d.torsoR * 0.95, d.torsoR * (spec.outfit === "dress" ? 1.75 : 1.4), length, 48, 4), spec.outfit === "dress" ? 9 : 7, 0.06);
      kit.mesh(skirt, kit.cloth(skirtColor), spine, [
        0,
        -length / 2 + 0.02,
        0,
      ], { scale: [1, 1, 0.85], outline: OUTLINE * 0.6, ink: skirtColor });
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
      kit.mesh(pleated(new THREE.CylinderGeometry(d.torsoR * 0.98, d.torsoR * 1.2, d.legUpper * 0.6, 40, 3), 6, 0.05), kit.cloth(p.primary), spine, [0, -d.legUpper * 0.22, 0], {
        scale: [1, 1, 0.82],
        outline: OUTLINE * 0.6,
        ink: p.primary,
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
  const hairHex = spec.palette.hair;
  const tipHex = spec.hairTip ?? null;
  const hairMat = kit.toon(hairHex);
  const lockMat = tipHex ? kit.vertexToon() : hairMat;
  const swayJoint = (x: number, y: number, z: number) => {
    const node = joint(headCenter, "hairSway", x, y, z);
    sway.push(node);
    return node;
  };
  /** A tapered, slightly flattened lock hanging from its root along -Y. */
  const lock = (parent: THREE.Object3D, position: [number, number, number], length: number, width: number, rotation: [number, number, number], flat = 0.5) => {
    const geometry = new THREE.ConeGeometry(width, length, 8, 3);
    if (tipHex) verticalGradient(geometry, hairHex, tipHex, 0.35);
    geometry.rotateX(Math.PI);
    geometry.translate(0, -length / 2, 0);
    kit.mesh(geometry, lockMat, parent, position, { rotation, scale: [1, 1, flat], outline: OUTLINE * 0.8, ink: hairHex });
  };
  const cap = () => {
    kit.mesh(new THREE.SphereGeometry(R * 1.07, 36, 18, 0, Math.PI * 2, 0, Math.PI * 0.52), hairMat, headCenter, [0, R * 0.02, -R * 0.03], {
      rotation: [-0.28, 0, 0],
      scale: [1.02, 0.96, 1],
      outline: OUTLINE * 0.5,
    });
  };
  const bangs = (count: number, length: number) => {
    for (let i = 0; i < count; i += 1) {
      const t = count === 1 ? 0 : i / (count - 1) - 0.5;
      const x = t * R * 1.15;
      // Roots sit on the hair cap; tips angle forward so they clear the face.
      const rootY = R * (0.62 - Math.abs(t) * 0.12);
      const rootZ = Math.sqrt(Math.max((R * 1.07) ** 2 - rootY * rootY - x * x, 0)) * 0.97;
      lock(headCenter, [x, rootY, rootZ], R * length * BANG_LENGTH_SCALE * (1 - Math.abs(t) * 0.3), R * 0.23, [-0.6, 0, -t * 0.8]);
    }
  };
  const sideLocks = (length: number) => {
    for (const side of [1, -1]) {
      lock(headCenter, [side * R * 0.99, R * 0.25, R * 0.22], R * length, R * 0.17, [-0.08, 0, side * 0.14]);
    }
  };
  const backFall = (count: number, length: number) => {
    const fall = swayJoint(0, R * 0.35, -R * 0.2);
    for (let i = 0; i < count; i += 1) {
      const angle = (i / (count - 1) - 0.5) * Math.PI * 0.95;
      lock(fall, [Math.sin(angle) * R * 0.78, 0, -Math.cos(angle) * R * 0.62], R * length * (1 - Math.abs(angle) * 0.12), R * 0.28, [0.18, 0, -angle * 0.18], 0.6);
    }
  };
  const bundle = (parent: THREE.Object3D, length: number) => {
    for (const [dx, dz, scale] of [
      [0, 0, 1],
      [R * 0.07, -R * 0.05, 0.85],
      [-R * 0.07, -R * 0.04, 0.9],
    ] as const) {
      lock(parent, [dx, 0, dz], R * length * scale, R * 0.2, [0.15, 0, 0], 0.7);
    }
  };

  if (furred) {
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
      bangs(5, 0.5);
      sideLocks(0.5);
      break;
    case "bob":
      cap();
      bangs(5, 0.55);
      sideLocks(0.95);
      backFall(7, 0.95);
      break;
    case "long":
      cap();
      bangs(5, 0.6);
      sideLocks(1.7);
      backFall(9, 2.4);
      break;
    case "hime": {
      cap();
      kit.mesh(roundedBox(R * 1.45, R * 0.42, R * 0.3, R * 0.06), hairMat, headCenter, [0, R * 0.46, R * 0.66], { rotation: [-0.25, 0, 0], outline: OUTLINE * 0.6 });
      for (const side of [1, -1]) {
        kit.mesh(roundedBox(R * 0.3, R * 1.35, R * 0.16, R * 0.05), hairMat, headCenter, [side * R * 0.84, -R * 0.35, R * 0.34], { outline: OUTLINE * 0.6 });
      }
      backFall(9, 2.5);
      break;
    }
    case "ponytail": {
      cap();
      bangs(5, 0.5);
      sideLocks(0.7);
      const tail = swayJoint(0, R * 0.62, -R * 0.88);
      kit.mesh(torus(R * 0.14, R * 0.05), kit.toon(spec.palette.accent), tail, [0, 0, 0], { rotation: [0.6, 0, 0] });
      bundle(tail, 1.6);
      break;
    }
    case "twintails":
      cap();
      bangs(5, 0.5);
      for (const side of [1, -1]) {
        const tail = swayJoint(side * R * 0.88, R * 0.45, -R * 0.2);
        kit.mesh(sphere(R * 0.11), kit.toon(spec.palette.accent), tail, [0, 0, 0]);
        bundle(tail, 1.8);
      }
      break;
    case "side_tail": {
      cap();
      bangs(5, 0.5);
      sideLocks(0.7);
      const tail = swayJoint(-R * 0.82, R * 0.6, -R * 0.35);
      kit.mesh(sphere(R * 0.12), kit.toon(spec.palette.accent), tail, [0, 0, 0]);
      bundle(tail, 1.7);
      break;
    }
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
      bangs(4, 0.45);
      break;
    case "bun":
      cap();
      bangs(5, 0.5);
      sideLocks(0.8);
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
      bangs(3, 0.35);
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
      case "bunny": {
        ear = joint(headCenter, "ear", side * R * 0.32, R * 0.8, -R * 0.05);
        ear.rotation.z = -side * 0.14;
        kit.mesh(capsule(R * 0.15, R * 0.85), earMat, ear, [0, R * 0.55, 0], { scale: [1, 1, 0.55], outline: OUTLINE });
        if (spec.id.endsWith("pikachu")) {
          kit.mesh(cone(R * 0.155, R * 0.38, 12), kit.toon("#1c1917"), ear, [0, R * 0.84, 0], { scale: [1, 1, 0.55] });
        } else {
          kit.mesh(capsule(R * 0.08, R * 0.65), innerMat, ear, [0, R * 0.55, R * 0.06], { scale: [1, 1, 0.3] });
        }
        break;
      }
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
      case "horns": {
        const isAnya = spec.id.endsWith("anya");
        const isPower = spec.id.endsWith("power");
        if (spec.body === "humanoid") {
          const humanEar = joint(headCenter, "earHuman", side * R * 0.97, -R * 0.06, 0);
          kit.mesh(sphere(R * 0.17), kit.toon(p.skin), humanEar, [0, 0, 0], { scale: [0.5, 1, 0.7], outline: OUTLINE });
        }
        if (isAnya) {
          ear = joint(headCenter, "ear", side * R * 0.78, R * 0.36, R * 0.22);
          ear.rotation.set(-0.1, 0, -side * 0.85);
          kit.mesh(cone(R * 0.22, R * 0.52, 12), kit.toon("#0f172a"), ear, [0, R * 0.24, 0], { outline: OUTLINE });
          kit.mesh(torus(R * 0.2, R * 0.035), kit.toon("#ca8a04", { emissive: 0.3 }), ear, [0, R * 0.08, 0], { rotation: [Math.PI / 2, 0, 0] });
        } else if (isPower) {
          ear = joint(headCenter, "ear", side * R * 0.28, R * 0.88, R * 0.1);
          ear.rotation.set(-0.2, 0, -side * 0.18);
          kit.mesh(cone(R * 0.1, R * 0.68, 10), kit.toon(p.accent), ear, [0, R * 0.32, 0], { outline: OUTLINE });
        } else {
          ear = joint(headCenter, "ear", side * R * 0.48, R * 0.78, R * 0.05);
          ear.rotation.set(-0.25, 0, -side * 0.42);
          const hornColor = p.accent !== p.skin && p.accent !== "#000000" ? p.accent : BONE_COLOR;
          kit.mesh(cone(R * 0.11, R * 0.55, 10), kit.toon(hornColor), ear, [0, R * 0.25, 0], { outline: OUTLINE });
        }
        break;
      }
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
      case "leaf":
        if (spec.id.endsWith("kodama-spirit")) return [null, null];
        ear = joint(headCenter, "ear", side * R * 0.6, R * 0.68, -R * 0.05);
        ear.rotation.z = -side * 0.55;
        kit.mesh(sphere(R * 0.3), earMat, ear, [0, R * 0.32, 0], { scale: [0.55, 1.35, 0.16], outline: OUTLINE });
        kit.mesh(cylinder(R * 0.018, R * 0.018, R * 0.62, 6), kit.toon(shade(earColor, BLACK, 0.3)), ear, [0, R * 0.32, R * 0.05]);
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
    case "flame":
      chain.forEach((node, i) => {
        node.rotation.x = -0.35;
        kit.mesh(capsule(0.03 - i * 0.005, 0.05), kit.toon(furColor), node, [0, 0.02, -0.025], { rotation: [-1.0, 0, 0], outline: OUTLINE });
      });
      kit.mesh(cone(0.06, 0.15, 12), kit.toon(p.accent, { emissive: 1.1, opacity: 0.9 }), t(2), [0, 0.12, -0.06], { outline: OUTLINE * 0.5 });
      kit.mesh(cone(0.032, 0.09, 10), kit.toon(FLAME_CORE_COLOR, { emissive: 1.4 }), t(2), [0, 0.1, -0.045]);
      break;
    case "leaf": {
      const stem = kit.toon(shade(p.hair, BLACK, 0.2));
      chain.forEach((node) => {
        node.rotation.x = -0.4;
        kit.mesh(cylinder(0.009, 0.009, 0.06, 6), stem, node, [0, 0.02, -0.025], { rotation: [-1.0, 0, 0] });
      });
      kit.mesh(sphere(0.09), kit.toon(p.hair), t(2), [0, 0.11, -0.06], { rotation: [-0.5, 0, 0], scale: [0.6, 1.5, 0.15], outline: OUTLINE });
      break;
    }
    case "star": {
      if (spec.id.endsWith("pikachu")) {
        chain.forEach((node, i) => {
          node.rotation.x = -0.3;
          kit.mesh(roundedBox(0.025 - i * 0.004, 0.06, 0.012, 0.003), kit.toon(i === 0 ? "#78350f" : p.skin), node, [0, 0.02, -0.02], { rotation: [-0.8, 0, 0] });
        });
        const lightning = new THREE.Shape();
        lightning.moveTo(0, 0);
        lightning.lineTo(0.04, 0.08);
        lightning.lineTo(0.01, 0.08);
        lightning.lineTo(0.06, 0.16);
        lightning.lineTo(0.02, 0.16);
        lightning.lineTo(0.09, 0.26);
        lightning.lineTo(0.01, 0.26);
        lightning.lineTo(-0.04, 0.15);
        lightning.lineTo(0.0, 0.15);
        lightning.lineTo(-0.03, 0.07);
        lightning.closePath();
        kit.mesh(new THREE.ExtrudeGeometry(lightning, { depth: 0.012, bevelEnabled: false }), kit.toon("#facc15"), t(2), [0, 0.08, -0.06], {
          outline: OUTLINE * 0.5,
          scale: [1.2, 1.2, 1.2],
        });
        break;
      }
      chain.forEach((node, i) => {
        node.rotation.x = -0.4;
        kit.mesh(capsule(0.018 - i * 0.003, 0.05), kit.toon(furColor), node, [0, 0.02, -0.025], { rotation: [-1.0, 0, 0], outline: OUTLINE });
      });
      const star = new THREE.Shape();
      for (let i = 0; i < STAR_POINTS * 2; i += 1) {
        const angle = (i / (STAR_POINTS * 2)) * Math.PI * 2 + Math.PI / 2;
        const radius = i % 2 === 0 ? 0.09 : 0.04;
        if (i === 0) star.moveTo(Math.cos(angle) * radius, Math.sin(angle) * radius);
        else star.lineTo(Math.cos(angle) * radius, Math.sin(angle) * radius);
      }
      star.closePath();
      kit.mesh(new THREE.ExtrudeGeometry(star, { depth: 0.012, bevelEnabled: false }), kit.toon(p.accent, { emissive: 0.9 }), t(2), [0, 0.12, -0.06], {
        outline: OUTLINE * 0.5,
      });
      break;
    }
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
  handR: THREE.Object3D;
  handL?: THREE.Object3D;
  chest: THREE.Object3D;
  neck: THREE.Object3D;
  spine: THREE.Object3D;
  R: number;
  d: Dimensions;
  furred: boolean;
}

function buildAccessories(kit: Kit, spec: CharacterSpec, a: AccessoryAnchors): void {
  const p = spec.palette;
  const { headCenter, chest, neck, spine, handR, handL, R, d } = a;
  const hasHands = spec.body !== "slime" && spec.body !== "bird";
  const isSlime = spec.body === "slime";
  const topY = isSlime ? R * 0.78 : R * 0.9;
  const eyeY = isSlime ? R * 0.12 : -R * 0.05;
  const faceFront = spec.body === "robot" ? R * 0.87 : R * 0.97;
  const faceZ = (x: number, y: number): number => {
    if (spec.body === "robot") return R * 0.85 + 0.012;
    const sy = isSlime ? 0.8 : 0.94;
    const sz = 0.96;
    const nx = x / R;
    const ny = y / (R * sy);
    return R * sz * Math.sqrt(Math.max(1 - nx * nx - ny * ny, 0.05)) + 0.006;
  };
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
      case "katana": {
        if (isSlime) break;
        const sword = joint(spine, "katana", d.torsoR * 0.95, d.torsoH * 0.16, 0.01);
        sword.rotation.set(1.1, 0, 0.18);
        kit.mesh(cylinder(0.013, 0.011, 0.36, 10), kit.toon("#1c1917"), sword, [0, -0.13, 0], { outline: OUTLINE });
        kit.mesh(cylinder(0.03, 0.03, 0.008, 16), kit.toon(GOLD_COLOR), sword, [0, 0.055, 0]);
        kit.mesh(cylinder(0.011, 0.011, 0.1, 8), kit.toon(shade(p.accent, BLACK, 0.3)), sword, [0, 0.11, 0], { outline: OUTLINE });
        break;
      }
      case "wand": {
        if (!hasHands) break;
        const wand = joint(handR, "wand", 0, -0.005, 0.01);
        wand.rotation.x = 1.25;
        kit.mesh(cylinder(0.006, 0.008, 0.16, 8), kit.toon(shade(p.secondary, WHITE, 0.3)), wand, [0, 0.07, 0]);
        kit.mesh(new THREE.OctahedronGeometry(0.026), kit.toon(p.accent, { emissive: 0.9 }), wand, [0, 0.16, 0], { scale: [1, 1.3, 0.5] });
        break;
      }
      case "staff": {
        if (!hasHands) break;
        const staff = joint(handR, "staff", 0, 0, 0.01);
        const length = d.height * 0.78;
        kit.mesh(cylinder(0.009, 0.011, length, 10), kit.toon("#7c4a23"), staff, [0, length * 0.18, 0], { outline: OUTLINE });
        kit.mesh(sphere(0.032), kit.toon(p.accent, { emissive: 0.9 }), staff, [0, length * 0.68 + 0.02, 0], { outline: OUTLINE });
        kit.mesh(torus(0.036, 0.006), kit.toon(GOLD_COLOR), staff, [0, length * 0.68, 0], { rotation: [Math.PI / 2, 0, 0] });
        break;
      }
      case "kitsune_mask": {
        const mask = joint(headCenter, "kitsuneMask", -R * 0.74, R * 0.38, R * 0.42);
        mask.rotation.set(0, -0.95, 0.25);
        kit.mesh(sphere(R * 0.36), kit.toon("#fafaf9"), mask, [0, 0, 0], { scale: [0.85, 1, 0.42], outline: OUTLINE });
        kit.mesh(cone(R * 0.12, R * 0.26, 10), kit.toon("#fafaf9"), mask, [0, -R * 0.12, R * 0.16], { rotation: [Math.PI / 2, 0, 0] });
        for (const side of [1, -1]) {
          kit.mesh(cone(R * 0.11, R * 0.24, 4), kit.toon("#fafaf9"), mask, [side * R * 0.2, R * 0.34, 0], { rotation: [0, 0, -side * 0.35] });
          kit.mesh(roundedBox(R * 0.16, R * 0.035, 0.006, 0.003), kit.flat("#dc2626"), mask, [side * R * 0.13, R * 0.05, R * 0.15], {
            rotation: [0, 0, side * 0.35],
          });
        }
        kit.mesh(disc(R * 0.03), kit.flat("#dc2626"), mask, [0, R * 0.2, R * 0.152]);
        break;
      }
      case "shell": {
        if (isSlime) break;
        const shell = joint(chest, "shell", 0, d.torsoH * 0.05, -d.torsoR * 0.75);
        const dome = new THREE.SphereGeometry(d.torsoR * 1.25, 28, 14, 0, Math.PI * 2, 0, Math.PI * 0.5);
        kit.mesh(dome, kit.toon(p.secondary), shell, [0, 0, 0], { rotation: [-Math.PI / 2, 0, 0], scale: [1, 1.15, 0.7], outline: OUTLINE });
        kit.mesh(torus(d.torsoR * 1.22, d.torsoR * 0.1), kit.toon(shade(p.secondary, WHITE, 0.35)), shell, [0, 0, 0], { scale: [1, 1.15, 1] });
        kit.mesh(sphere(d.torsoR * 0.32), kit.toon(p.accent), shell, [0, 0, -d.torsoR * 0.82], { scale: [1, 1, 0.35] });
        break;
      }
      case "gem": {
        const gem = joint(headCenter, "gem", 0, isSlime ? R * 0.5 : R * 0.42, faceFront * 0.93);
        kit.mesh(new THREE.OctahedronGeometry(R * 0.11), kit.toon(p.accent, { emissive: 1.0 }), gem, [0, 0, 0], { scale: [1, 1.2, 0.5], outline: OUTLINE * 0.5 });
        break;
      }
      default:
        break;
    }
  }

  // Character-specific signature details
  const id = spec.id;
  const isPikachu = id.endsWith("pikachu");
  const isKodama = id.endsWith("kodama-spirit");
  const isBramble = id.endsWith("woodland-guardian");
  const isYor = id.endsWith("yor");
  const isFrieren = id.endsWith("frieren");
  const isDenji = id.endsWith("denji");
  const isPower = id.endsWith("power");
  const isAnya = id.endsWith("anya");
  const isGenos = id.endsWith("genos");
  const isZoro = id.endsWith("zoro");
  const isNezuko = id.endsWith("nezuko");
  const isTanjiro = id.endsWith("tanjiro");
  const isLuffy = id.endsWith("luffy");
  const isNaruto = id.endsWith("naruto");
  const isSunWukong = id.endsWith("sun-wukong");
  const isKrishna = id.endsWith("krishna");
  const isGanesha = id.endsWith("ganesha");
  const isHanuman = id.endsWith("hanuman");
  const isShiva = id.endsWith("shiva");
  const isRama = id.endsWith("rama");
  const isGojo = id.endsWith("gojo");
  const isNoa = id.endsWith("witch-apprentice");
  const isRen = id.endsWith("valley-flyer");
  const isNinjaCat = id.endsWith("ninja-cat");

  // Pikachu: Electric spark held in hand
  if (isPikachu) {
    const spark = joint(handR, "electricSpark", 0, 0.02, 0.02);
    kit.mesh(sphere(0.028), kit.toon("#fde047", { emissive: 1.5, opacity: 0.9 }), spark, [0, 0, 0], { outline: OUTLINE * 0.3 });
  }

  // Yor: Crimson roses on ears with radiating golden stiletto needle hairpins, choker, dual stiletto weapons
  if (isYor) {
    for (const side of [1, -1] as const) {
      const rose = joint(headCenter, `yorRose${side}`, side * R * 0.88, R * 0.26, R * 0.18);
      kit.mesh(sphere(R * 0.18), kit.toon("#b91c1c"), rose, [0, 0, 0], { scale: [0.6, 1, 1], outline: OUTLINE * 0.4 });
      kit.mesh(sphere(R * 0.12), kit.toon("#991b1b"), rose, [side * 0.01, 0, 0], { scale: [0.5, 0.8, 0.8] });
      kit.mesh(sphere(R * 0.06), kit.toon("#ffd700"), rose, [side * 0.02, 0, 0]);
      for (let i = 0; i < 4; i++) {
        const pin = joint(rose, `yorPin${i}`, 0, 0, 0);
        pin.rotation.z = (i * Math.PI) / 2 + Math.PI / 4;
        kit.mesh(cone(0.006, 0.28, 6), kit.toon("#ffd700", { emissive: 0.35 }), pin, [0, 0.14, 0], { outline: OUTLINE * 0.3 });
      }
    }
    kit.mesh(torus(d.torsoR * 0.65, 0.012), kit.toon("#0f172a"), neck, [0, 0, 0], { rotation: [Math.PI / 2, 0, 0] });
    kit.mesh(sphere(0.014), kit.toon("#ffd700"), neck, [0, -0.01, d.torsoR * 0.68]);
    kit.mesh(roundedBox(d.torsoR * 0.75, d.torsoH * 0.55, 0.01, 0.004), kit.toon("#b91c1c"), chest, [0, -d.torsoH * 0.22, d.torsoR * 0.62], { outline: OUTLINE * 0.4 });
    const hands = [handR, handL].filter(Boolean) as THREE.Object3D[];
    for (const h of hands) {
      const needle = joint(h, "stiletto", 0, 0, 0.01);
      kit.mesh(cylinder(0.006, 0.003, 0.32, 8), kit.toon("#ffd700", { emissive: 0.35 }), needle, [0, 0.11, 0], { outline: OUTLINE * 0.3 });
      kit.mesh(torus(0.02, 0.004), kit.toon("#ffd700"), needle, [0, -0.04, 0]);
    }
  }

  // Frieren: Elven teardrop ruby earrings, shoulder mantle capelet with ruby brooch, crescent mage staff
  if (isFrieren) {
    for (const side of [1, -1] as const) {
      const fe = joint(headCenter, `frierenEar${side}`, side * R * 0.95, -R * 0.04, -R * 0.05);
      fe.rotation.set(0.1, side * 0.2, -side * 0.35);
      kit.mesh(cone(R * 0.15, R * 0.85, 8), kit.toon(p.skin), fe, [side * R * 0.35, 0, 0], { rotation: [0, 0, (-side * Math.PI) / 2], outline: OUTLINE });
      const earring = joint(fe, "earring", side * R * 0.18, -R * 0.12, 0);
      kit.mesh(cylinder(0.003, 0.003, 0.05, 6), kit.toon("#ffd700"), earring, [0, 0.02, 0]);
      kit.mesh(sphere(0.024), kit.toon("#dc2626", { emissive: 0.4 }), earring, [0, -0.02, 0], { scale: [0.75, 1.4, 0.75], outline: OUTLINE * 0.3 });
    }
    const mGeom = new THREE.CylinderGeometry(d.torsoR * 0.95, d.torsoR * 1.55, d.torsoH * 0.45, 20, 1, true);
    kit.mesh(mGeom, kit.toon("#f8fafc", { side: THREE.DoubleSide }), chest, [0, d.torsoH * 0.28, 0], { outline: OUTLINE * 0.4 });
    kit.mesh(torus(d.torsoR * 1.54, 0.008), kit.toon("#f59e0b"), chest, [0, d.torsoH * 0.06, 0], { rotation: [Math.PI / 2, 0, 0] });
    kit.mesh(sphere(0.02), kit.toon("#dc2626", { emissive: 0.5 }), chest, [0, d.torsoH * 0.5, d.torsoR * 0.76], { scale: [1, 1.3, 0.6] });
    const cStaff = joint(handR, "crescentStaff", 0, 0, 0.01);
    const csLen = d.height * 0.88;
    kit.mesh(cylinder(0.009, 0.011, csLen, 10), kit.toon("#451a03"), cStaff, [0, csLen * 0.2, 0], { outline: OUTLINE });
    kit.mesh(torus(0.09, 0.014, Math.PI * 1.5), kit.toon("#ffd700", { emissive: 0.35 }), cStaff, [0, csLen * 0.72, 0], { rotation: [0, 0, Math.PI * 0.25], outline: OUTLINE * 0.4 });
    kit.mesh(sphere(0.038), kit.toon("#dc2626", { emissive: 1.2 }), cStaff, [0, csLen * 0.72, 0], { outline: OUTLINE * 0.3 });
  }

  // Suzu: Kodama hollow mouth socket, clover sprout, lotus leaf umbrella
  if (isKodama) {
    kit.mesh(disc(R * 0.12), kit.flat("#111827"), headCenter, [0, eyeY - R * 0.32, faceZ(0, eyeY - R * 0.32) + 0.007], { scale: [1.1, 0.85, 1] });
    const sprout = joint(headCenter, "kodamaSprout", 0, R * 0.9, 0);
    kit.mesh(cylinder(0.006, 0.008, R * 0.32, 8), kit.toon("#15803d"), sprout, [0, R * 0.15, 0]);
    for (let i = 0; i < 3; i++) {
      const ang = (i / 3) * Math.PI * 2;
      kit.mesh(sphere(R * 0.12), kit.toon("#22c55e"), sprout, [Math.cos(ang) * R * 0.11, R * 0.32, Math.sin(ang) * R * 0.11], { scale: [1, 0.3, 1.2], rotation: [0.25, ang, 0] });
    }
    const u = joint(handR, "kodamaUmbrella", 0, 0, 0.01);
    const uLen = d.height * 0.82;
    kit.mesh(cylinder(0.008, 0.01, uLen, 10), kit.toon("#3f2b1d"), u, [0, uLen * 0.2, 0], { outline: OUTLINE * 0.4 });
    const leafGeom = new THREE.SphereGeometry(d.headR * 1.35, 24, 12, 0, Math.PI * 2, 0, Math.PI * 0.46);
    kit.mesh(leafGeom, kit.toon("#16a34a", { side: THREE.DoubleSide }), u, [0, uLen * 0.72, 0], { rotation: [Math.PI, 0, 0], outline: OUTLINE * 0.5 });
  }

  // Bramble: Totoro white belly patch with grey chevrons, branching moss antlers, giant leaf umbrella
  if (isBramble) {
    kit.mesh(sphere(d.torsoR * 1.25), kit.toon("#f5f5f4"), spine, [0, d.torsoH * 0.4, d.torsoR * 0.45], { scale: [1.05, 1.2, 0.6] });
    for (const chevron of [-d.torsoR * 0.35, 0, d.torsoR * 0.35]) {
      kit.mesh(roundedBox(0.024, 0.008, 0.004, 0.002), kit.toon("#78716c"), spine, [chevron, d.torsoH * 0.55, d.torsoR * 0.98 + 0.004], { rotation: [0, 0, 0.5] });
      kit.mesh(roundedBox(0.024, 0.008, 0.004, 0.002), kit.toon("#78716c"), spine, [chevron + 0.016, d.torsoH * 0.55, d.torsoR * 0.98 + 0.004], { rotation: [0, 0, -0.5] });
    }
    for (const side of [1, -1] as const) {
      const antler = joint(headCenter, `brambleAntler${side}`, side * R * 0.45, R * 0.82, -R * 0.05);
      antler.rotation.set(-0.2, 0, -side * 0.45);
      kit.mesh(cylinder(R * 0.08, R * 0.12, R * 0.65, 8), kit.toon("#78350f"), antler, [0, R * 0.3, 0], { outline: OUTLINE * 0.4 });
      kit.mesh(sphere(R * 0.09), kit.toon("#15803d"), antler, [0, R * 0.62, 0]);
      kit.mesh(cone(R * 0.06, R * 0.32, 6), kit.toon("#78350f"), antler, [side * R * 0.12, R * 0.28, 0], { rotation: [0, 0, -side * 0.7] });
      kit.mesh(sphere(R * 0.06), kit.toon("#15803d"), antler, [side * R * 0.22, R * 0.38, 0]);
    }
    const bu = joint(handR, "brambleUmbrella", 0, 0, 0.01);
    const buLen = d.height * 0.85;
    kit.mesh(cylinder(0.01, 0.012, buLen, 10), kit.toon("#3f2b1d"), bu, [0, buLen * 0.2, 0], { outline: OUTLINE * 0.4 });
    const bLeafGeom = new THREE.SphereGeometry(d.headR * 1.5, 24, 12, 0, Math.PI * 2, 0, Math.PI * 0.48);
    kit.mesh(bLeafGeom, kit.toon("#15803d", { side: THREE.DoubleSide }), bu, [0, buLen * 0.72, 0], { rotation: [Math.PI, 0, 0], outline: OUTLINE * 0.5 });
  }

  // Denji: Chest pull cord + companion Pochita with chainsaw blade
  if (isDenji) {
    kit.mesh(torus(0.018, 0.004), kit.toon("#111827"), chest, [0, d.torsoH * 0.48, d.torsoR * 0.8], { rotation: [0.3, 0, 0], outline: OUTLINE * 0.4 });
    const pochita = joint(chest, "pochita", 0, d.torsoH * 0.18, d.torsoR * 0.95);
    kit.mesh(sphere(0.065), kit.toon("#f97316"), pochita, [0, 0, 0], { scale: [1, 1.1, 1.25], outline: OUTLINE * 0.4 });
    kit.mesh(roundedBox(0.012, 0.065, 0.14, 0.004), kit.toon("#94a3b8"), pochita, [0, 0.04, 0.08], { outline: OUTLINE * 0.3 });
    kit.mesh(torus(0.022, 0.005), kit.toon("#111827"), pochita, [0, 0.05, -0.06], { rotation: [Math.PI / 2, 0, 0] });
    kit.mesh(sphere(0.016), kit.flat("#111827"), pochita, [-0.035, 0.01, 0.05]);
    kit.mesh(sphere(0.016), kit.flat("#111827"), pochita, [0.035, 0.01, 0.05]);
  }

  // Power: Giant blood war hammer
  if (isPower) {
    const hammer = joint(handR, "bloodHammer", 0, 0, 0.01);
    kit.mesh(cylinder(0.012, 0.014, 0.75, 8), kit.toon("#991b1b"), hammer, [0, 0.2, 0], { outline: OUTLINE });
    kit.mesh(roundedBox(0.14, 0.14, 0.24, 0.025), kit.toon("#dc2626", { emissive: 0.3 }), hammer, [0, 0.52, 0], { outline: OUTLINE * 0.5 });
  }

  // Anya: Ahoge sprout + green Chimera plushie
  if (isAnya) {
    const ahoge = joint(headCenter, "ahoge", 0, R * 0.95, 0);
    kit.mesh(cylinder(0.005, 0.008, R * 0.28, 6), kit.toon(p.hair), ahoge, [0.02, R * 0.12, 0], { rotation: [0, 0, -0.4] });
    const chimera = joint(chest, "chimera", 0, d.torsoH * 0.18, d.torsoR * 0.9);
    kit.mesh(sphere(0.055), kit.toon("#16a34a"), chimera, [0, 0, 0], { scale: [1.1, 1, 1], outline: OUTLINE * 0.4 });
    kit.mesh(sphere(0.04), kit.toon("#eab308"), chimera, [0, 0.04, 0.02]);
    kit.mesh(cone(0.015, 0.05, 6), kit.toon("#f97316"), chimera, [-0.03, 0.07, 0.02], { rotation: [0, 0, 0.3] });
    kit.mesh(cone(0.015, 0.05, 6), kit.toon("#f97316"), chimera, [0.03, 0.07, 0.02], { rotation: [0, 0, -0.3] });
  }

  // Genos: Incinerator core on chest and palms
  if (isGenos) {
    kit.mesh(disc(0.024), kit.toon("#f97316", { emissive: 1.5 }), chest, [0, d.torsoH * 0.52, d.torsoR * 0.8 + 0.004]);
    kit.mesh(torus(0.028, 0.005), kit.toon("#ffd700"), chest, [0, d.torsoH * 0.52, d.torsoR * 0.8 + 0.002]);
    const hands = [handR, handL].filter(Boolean) as THREE.Object3D[];
    for (const h of hands) {
      kit.mesh(disc(0.016), kit.toon("#f97316", { emissive: 1.5 }), h, [0, 0.01, 0.02], { rotation: [0.5, 0, 0] });
    }
  }

  // Zoro: Three katanas, 3 gold hoop earrings, haramaki sash
  if (isZoro) {
    for (let i = 0; i < 3; i++) {
      const kColor = ["#f8fafc", "#991b1b", "#1c1917"][i];
      const sword = joint(spine, `katana${i}`, d.torsoR * 0.95, d.torsoH * (0.12 + i * 0.08), -0.02 + i * 0.03);
      sword.rotation.set(1.15, 0, 0.15 + i * 0.06);
      kit.mesh(cylinder(0.012, 0.01, 0.42, 8), kit.toon(kColor), sword, [0, -0.15, 0], { outline: OUTLINE * 0.4 });
      kit.mesh(cylinder(0.028, 0.028, 0.007, 12), kit.toon("#ffd700"), sword, [0, 0.06, 0]);
      kit.mesh(cylinder(0.01, 0.01, 0.11, 8), kit.toon("#111827"), sword, [0, 0.12, 0], { outline: OUTLINE * 0.4 });
    }
    for (let i = 0; i < 3; i++) {
      kit.mesh(torus(0.016, 0.004), kit.toon("#ffd700"), headCenter, [R * 0.97, -R * (0.15 + i * 0.07), 0], { rotation: [Math.PI / 2, 0, 0] });
    }
    kit.mesh(cylinder(d.torsoR * 1.05, d.torsoR * 1.05, d.torsoH * 0.45, 18), kit.toon("#15803d"), spine, [0, d.torsoH * 0.26, 0], { outline: OUTLINE * 0.4 });
  }

  // Nezuko: Green bamboo muzzle with red tie cords
  if (isNezuko) {
    kit.mesh(cylinder(R * 0.095, R * 0.095, R * 0.62, 16), kit.toon("#16a34a"), headCenter, [0, eyeY - R * 0.4, faceZ(0, eyeY - R * 0.4) + 0.014], { rotation: [0, 0, Math.PI / 2], outline: OUTLINE * 0.6 });
    kit.mesh(torus(R * 0.1, R * 0.014), kit.toon("#14532d"), headCenter, [-R * 0.18, eyeY - R * 0.4, faceZ(0, eyeY - R * 0.4) + 0.014], { rotation: [0, Math.PI / 2, 0] });
    kit.mesh(torus(R * 0.1, R * 0.014), kit.toon("#14532d"), headCenter, [R * 0.18, eyeY - R * 0.4, faceZ(0, eyeY - R * 0.4) + 0.014], { rotation: [0, Math.PI / 2, 0] });
    kit.mesh(roundedBox(R * 1.35, R * 0.035, 0.008, 0.002), kit.toon("#dc2626"), headCenter, [0, eyeY - R * 0.4, faceZ(0, eyeY - R * 0.4) + 0.006]);
  }

  // Tanjiro: Hanafuda earrings + forehead scar
  if (isTanjiro) {
    for (const side of [1, -1] as const) {
      kit.mesh(roundedBox(R * 0.08, R * 0.22, 0.004, 0.002), kit.toon("#f8fafc"), headCenter, [side * R * 0.96, -R * 0.22, 0], { outline: OUTLINE * 0.4 });
      kit.mesh(disc(R * 0.035), kit.flat("#dc2626"), headCenter, [side * R * 0.96, -R * 0.18, side * 0.003]);
    }
    kit.mesh(disc(R * 0.09), kit.flat("#991b1b"), headCenter, [R * 0.32, eyeY + R * 0.42, faceZ(R * 0.32, eyeY + R * 0.42) + 0.005], { scale: [1, 0.8, 1] });
  }

  // Luffy: Straw hat with red ribbon + chest scar
  if (isLuffy) {
    kit.mesh(cylinder(R * 1.5, R * 1.5, R * 0.04, 32), kit.toon("#eab308"), headCenter, [0, R * 0.88, -R * 0.05], { rotation: [-0.25, 0, 0], outline: OUTLINE * 0.5 });
    kit.mesh(sphere(R * 0.65), kit.toon("#eab308"), headCenter, [0, R * 1.08, -R * 0.14], { scale: [1, 0.6, 1], outline: OUTLINE * 0.5 });
    kit.mesh(torus(R * 0.66, R * 0.06), kit.toon("#dc2626"), headCenter, [0, R * 0.96, -R * 0.12], { rotation: [Math.PI / 2 - 0.25, 0, 0] });
    kit.mesh(roundedBox(R * 0.08, R * 0.015, 0.002, 0.001), kit.flat("#1c1917"), headCenter, [-R * 0.38, eyeY - R * 0.16, faceZ(-R * 0.38, eyeY - R * 0.16) + 0.004]);
  }

  // Naruto: Whisker marks + Rasengan in hand
  if (isNaruto) {
    for (const side of [1, -1] as const) {
      for (let i = 0; i < 3; i += 1) {
        const wy = eyeY - R * (0.2 + i * 0.07);
        const wx = side * R * 0.42;
        kit.mesh(roundedBox(R * 0.1, R * 0.016, 0.002, 0.001), kit.flat("#1f2937"), headCenter, [wx, wy, faceZ(wx, wy) + 0.004], {
          rotation: [0, Math.asin(wx / R) * 0.9, side * (i - 1) * 0.18],
        });
      }
    }
    const rasengan = joint(handR, "rasengan", 0, 0.1, 0.06);
    kit.mesh(sphere(0.042), kit.toon("#38bdf8", { emissive: 1.6, opacity: 0.82 }), rasengan, [0, 0, 0]);
    kit.mesh(sphere(0.02), kit.toon("#ffffff", { emissive: 2.0 }), rasengan, [0, 0, 0]);
  }

  // Sun Wukong: Golden fillet circlet, cloud pauldrons, Ruyi Jingu Bang
  if (isSunWukong) {
    const fillet = joint(headCenter, "fillet", 0, R * 0.35, 0);
    kit.mesh(torus(R * 1.01, R * 0.06), kit.toon("#ffd700", { emissive: 0.4 }), fillet, [0, 0, 0], { rotation: [Math.PI / 2 - 0.12, 0, 0] });
    kit.mesh(cone(R * 0.12, R * 0.45, 6), kit.toon("#ffd700", { emissive: 0.5 }), fillet, [-R * 0.28, R * 0.28, R * 0.88], { rotation: [0.3, -0.4, 0.3] });
    kit.mesh(cone(R * 0.12, R * 0.45, 6), kit.toon("#ffd700", { emissive: 0.5 }), fillet, [R * 0.28, R * 0.28, R * 0.88], { rotation: [0.3, 0.4, -0.3] });
    const wukongStaff = joint(handR, "ruyiJinguBang", 0, 0, 0.01);
    const wsLen = d.height * 0.95;
    kit.mesh(cylinder(0.012, 0.012, wsLen, 10), kit.toon("#991b1b"), wukongStaff, [0, wsLen * 0.18, 0], { outline: OUTLINE });
    kit.mesh(cylinder(0.016, 0.016, 0.12, 12), kit.toon("#ffd700", { emissive: 0.4 }), wukongStaff, [0, wsLen * 0.62, 0]);
    kit.mesh(cylinder(0.016, 0.016, 0.12, 12), kit.toon("#ffd700", { emissive: 0.4 }), wukongStaff, [0, -wsLen * 0.26, 0]);
  }

  // Krishna: Peacock feather in golden crown + golden Bansuri flute
  if (isKrishna) {
    const kCrown = joint(headCenter, "krishnaCrown", 0, topY + R * 0.08, 0);
    kit.mesh(cylinder(R * 0.55, R * 0.48, R * 0.28, 20), kit.toon("#ffd700", { emissive: 0.3 }), kCrown, [0, 0, 0], { outline: OUTLINE * 0.4 });
    const feather = joint(kCrown, "peacockFeather", R * 0.18, R * 0.22, -R * 0.05);
    feather.rotation.set(-0.2, 0.3, 0.4);
    kit.mesh(sphere(R * 0.22), kit.toon("#059669"), feather, [0, R * 0.18, 0], { scale: [0.55, 1.4, 0.2], outline: OUTLINE * 0.3 });
    kit.mesh(sphere(R * 0.1), kit.toon("#0284c7"), feather, [0, R * 0.24, 0.01], { scale: [0.6, 1.1, 0.2] });
    const flute = joint(chest, "flute", 0, d.torsoH * 0.48, d.torsoR * 0.85);
    flute.rotation.set(0.15, 0, 0.35);
    kit.mesh(cylinder(0.007, 0.007, 0.38, 8), kit.toon("#ffd700", { emissive: 0.4 }), flute, [0, 0, 0], { rotation: [0, 0, Math.PI / 2], outline: OUTLINE * 0.3 });
  }

  // Ganesha: Elephant trunk + golden modak sweet
  if (isGanesha) {
    const trunk = joint(headCenter, "ganeshaTrunk", 0, eyeY - R * 0.25, faceFront);
    kit.mesh(cylinder(R * 0.14, R * 0.08, R * 0.48, 12), kit.toon(p.skin), trunk, [0, -R * 0.16, R * 0.08], { rotation: [0.4, 0, 0], outline: OUTLINE * 0.4 });
    kit.mesh(sphere(R * 0.09), kit.toon(p.skin), trunk, [0, -R * 0.32, R * 0.22], { scale: [0.9, 0.8, 1.2] });
    const modak = joint(handR, "modak", 0, 0.02, 0.01);
    kit.mesh(cone(0.028, 0.055, 8), kit.toon("#ffd700", { emissive: 0.3 }), modak, [0, 0.02, 0]);
  }

  // Hanuman: Golden gada mace
  if (isHanuman) {
    const hMace = joint(handR, "hanumanGada", 0, 0, 0.01);
    kit.mesh(cylinder(0.011, 0.013, 0.45, 8), kit.toon("#ffd700"), hMace, [0, 0.12, 0], { outline: OUTLINE });
    kit.mesh(sphere(0.065), kit.toon("#ffd700", { emissive: 0.3 }), hMace, [0, 0.36, 0], { outline: OUTLINE * 0.4 });
    kit.mesh(cone(0.028, 0.06, 8), kit.toon("#ffd700"), hMace, [0, 0.44, 0]);
  }

  // Shiva: Golden Trishula trident
  if (isShiva) {
    const sStaff = joint(handR, "trishula", 0, 0, 0.01);
    const sLen = d.height * 0.95;
    kit.mesh(cylinder(0.01, 0.01, sLen, 8), kit.toon("#475569"), sStaff, [0, sLen * 0.2, 0], { outline: OUTLINE });
    kit.mesh(torus(0.055, 0.009, Math.PI), kit.toon("#ffd700", { emissive: 0.3 }), sStaff, [0, sLen * 0.65, 0], { rotation: [0, 0, Math.PI] });
    kit.mesh(cone(0.012, 0.12, 6), kit.toon("#ffd700", { emissive: 0.3 }), sStaff, [0, sLen * 0.72, 0]);
    kit.mesh(cone(0.01, 0.09, 6), kit.toon("#ffd700"), sStaff, [-0.055, sLen * 0.68, 0]);
    kit.mesh(cone(0.01, 0.09, 6), kit.toon("#ffd700"), sStaff, [0.055, sLen * 0.68, 0]);
  }

  // Rama: Kodanda longbow + quiver
  if (isRama) {
    const bow = joint(handR, "kodandaBow", 0, 0, 0.01);
    kit.mesh(torus(0.24, 0.009, Math.PI * 0.75), kit.toon("#b45309"), bow, [0, 0.08, 0], { rotation: [0, 0, 0.4], outline: OUTLINE * 0.4 });
    const quiver = joint(spine, "quiver", -d.torsoR * 0.5, d.torsoH * 0.45, -d.torsoR * 0.85);
    quiver.rotation.set(0.3, 0, -0.4);
    kit.mesh(cylinder(0.024, 0.02, 0.32, 8), kit.toon("#78350f"), quiver, [0, 0, 0], { outline: OUTLINE * 0.4 });
    kit.mesh(cone(0.015, 0.08, 6), kit.toon("#ffd700"), quiver, [0, 0.18, 0]);
  }

  // Gojo: Blue cursed energy orb
  if (isGojo) {
    const blueEnergy = joint(handR, "cursedEnergy", 0, 0.12, 0.08);
    kit.mesh(sphere(0.045), kit.toon("#38bdf8", { emissive: 1.8, opacity: 0.85 }), blueEnergy, [0, 0, 0], { outline: OUTLINE * 0.3 });
    kit.mesh(sphere(0.025), kit.toon("#ffffff", { emissive: 2.0 }), blueEnergy, [0, 0, 0]);
  }

  // Ren: Squirrel on left shoulder
  if (isRen) {
    const squirrel = joint(chest, "squirrel", d.shoulderX * 0.85, d.torsoH * 0.65, 0);
    kit.mesh(sphere(0.035), kit.toon("#92400e"), squirrel, [0, 0, 0], { scale: [1, 1.2, 0.9], outline: OUTLINE * 0.3 });
    kit.mesh(sphere(0.024), kit.toon("#92400e"), squirrel, [0, 0.03, 0.02]);
    kit.mesh(cone(0.012, 0.05, 6), kit.toon("#78350f"), squirrel, [0, 0.01, -0.03], { rotation: [-1.2, 0, 0] });
  }

  // Noa: Delivery twig broom + tiny black cat
  if (isNoa) {
    const broom = joint(handR, "broom", 0, 0, 0.01);
    kit.mesh(cylinder(0.008, 0.008, 0.65, 8), kit.toon("#78350f"), broom, [0, 0.15, 0], { outline: OUTLINE * 0.4 });
    kit.mesh(cone(0.05, 0.22, 10), kit.toon("#ca8a04"), broom, [0, 0.48, 0], { outline: OUTLINE * 0.4 });
    const cat = joint(chest, "cat", -d.shoulderX * 0.85, d.torsoH * 0.62, 0);
    kit.mesh(sphere(0.03), kit.toon("#111827"), cat, [0, 0, 0], { outline: OUTLINE * 0.3 });
    kit.mesh(sphere(0.02), kit.toon("#111827"), cat, [0, 0.025, 0.015]);
  }

  // Ninja Cat: Crossed dual ninjato swords on back
  if (isNinjaCat) {
    for (const side of [1, -1] as const) {
      const ninjato = joint(spine, `ninjato${side}`, side * 0.03, d.torsoH * 0.4, -d.torsoR * 0.85);
      ninjato.rotation.set(0.3, 0, side * 0.65);
      kit.mesh(cylinder(0.008, 0.008, 0.38, 8), kit.toon("#00f0ff", { emissive: 0.8 }), ninjato, [0, -0.1, 0], { outline: OUTLINE * 0.3 });
      kit.mesh(cylinder(0.01, 0.01, 0.1, 8), kit.toon("#111927"), ninjato, [0, 0.12, 0]);
    }
  }
}
