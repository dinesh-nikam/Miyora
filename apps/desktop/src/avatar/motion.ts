// State-driven motion for every avatar kind.
//
// `motionFor` turns a real pet-engine state into a target pose, facial
// expression, and prop flags. The stage eases the current pose toward the
// target every frame, so state changes blend instead of snapping. The same
// pose drives procedural rigs, VRM humanoids (via retargeting), and image
// sculpts (as whole-body motion).

import * as THREE from "three";
import type { VRM, VRMHumanBoneName } from "@pixiv/three-vrm";
import type { PetState } from "../components/CompanionRenderer";
import type { CharacterRig, MouthShape } from "./rig";

export type Vec3 = [number, number, number];

export interface Pose {
  rootY: number;
  turnY: number;
  leanX: number;
  spineX: number;
  chestX: number;
  chestZ: number;
  headX: number;
  headY: number;
  headZ: number;
  armL: Vec3;
  armR: Vec3;
  elbowL: number;
  elbowR: number;
  legL: number;
  legR: number;
  kneeL: number;
  kneeR: number;
  tailWag: number;
  wingFlap: number;
  squash: number;
}

export interface Expression {
  mouth: MouthShape;
  /** 0 = closed, 1 = fully open (before blinking). */
  eyeOpen: number;
  /** Raises (+) or lowers (-) the brows. */
  browLift: number;
  /** Positive = angry (inner ends down), negative = worried. */
  browTilt: number;
  blush: number;
}

export interface PropFlags {
  laptop: boolean;
  thought: boolean;
  confetti: boolean;
  sleepZ: boolean;
  food: boolean;
  glyph: string | null;
  glyphColor: string;
  /** Desaturate the stage (offline). */
  dim: boolean;
}

export interface StateMotion {
  pose: Pose;
  expression: Expression;
  props: PropFlags;
  /** Frames per second the stage needs for this state. */
  frameRate: number;
}

export const ACTIVE_FRAME_RATE = 30;
export const CALM_FRAME_RATE = 12;
/** Exponential easing rate toward the target pose, per second. */
export const POSE_EASE_RATE = 9;

const REST_ARM_SPLAY = 0.12;
const REST_ELBOW = -0.15;

export function restPose(): Pose {
  return {
    rootY: 0,
    turnY: 0,
    leanX: 0,
    spineX: 0,
    chestX: 0,
    chestZ: 0,
    headX: 0,
    headY: 0,
    headZ: 0,
    armL: [0, 0, REST_ARM_SPLAY],
    armR: [0, 0, -REST_ARM_SPLAY],
    elbowL: REST_ELBOW,
    elbowR: REST_ELBOW,
    legL: 0,
    legR: 0,
    kneeL: 0,
    kneeR: 0,
    tailWag: 0,
    wingFlap: 0,
    squash: 0,
  };
}

const NO_PROPS: PropFlags = {
  laptop: false,
  thought: false,
  confetti: false,
  sleepZ: false,
  food: false,
  glyph: null,
  glyphColor: "#f8fafc",
  dim: false,
};

/**
 * Computes the target motion for `state` at time `t` (seconds).
 * `energy` comes from the character spec and scales tempo.
 */
export function motionFor(state: PetState, t: number, energy = 1): StateMotion {
  const w = t * energy;
  const pose = restPose();
  const expression: Expression = { mouth: "smile", eyeOpen: 1, browLift: 0, browTilt: 0, blush: 0.3 };
  const props: PropFlags = { ...NO_PROPS };
  let frameRate = ACTIVE_FRAME_RATE;
  const breathe = Math.sin(w * 2);

  switch (state) {
    case "working": {
      const typeL = Math.sin(w * 16);
      const typeR = Math.sin(w * 16 + 1.7);
      Object.assign(pose, {
        leanX: 0.1,
        spineX: 0.06,
        headX: 0.3 + 0.02 * Math.sin(w * 3),
        headY: 0.08 * Math.sin(w * 0.6),
        armL: [-1.05 + 0.07 * typeL, 0, 0.2],
        armR: [-1.05 + 0.07 * typeR, 0, -0.2],
        elbowL: -0.55,
        elbowR: -0.55,
        tailWag: 0.15 * Math.sin(w * 2.5),
        squash: 0.015 * Math.sin(w * 8),
      });
      Object.assign(expression, { mouth: "flat", eyeOpen: 0.9, browLift: -0.15, blush: 0.15 });
      props.laptop = true;
      break;
    }
    case "thinking":
      Object.assign(pose, {
        headX: -0.22,
        headZ: 0.16,
        headY: 0.22 * Math.sin(w * 0.4),
        armR: [-1.55, 0.25, -0.35],
        elbowR: -1.95,
        armL: [-0.5, 0, -0.3],
        elbowL: -1.4,
        tailWag: 0.25 * Math.sin(w),
        squash: 0.02 * Math.sin(w * 1.2),
      });
      Object.assign(expression, { mouth: "flat", browLift: 0.25, browTilt: 0.1 });
      props.thought = true;
      break;
    case "waiting": {
      const tap = Math.max(0, Math.sin(w * 6));
      Object.assign(pose, {
        headY: 0.35 * Math.sin(w * 0.9),
        headX: -0.05,
        armL: [-0.6, 0, -0.28],
        armR: [-0.6, 0, 0.28],
        elbowL: -1.5,
        elbowR: -1.5,
        legR: -0.08 * tap,
        kneeR: 0.2 * tap,
        tailWag: 0.3 * Math.sin(w * 2),
        squash: 0.02 * tap,
      });
      Object.assign(expression, { mouth: "flat" });
      props.glyph = "...";
      break;
    }
    case "attention_required":
      Object.assign(pose, {
        rootY: 0.02 * Math.abs(Math.sin(w * 3.5)),
        headX: -0.15,
        armR: [-2.9, 0, -0.35 + 0.35 * Math.sin(w * 7)],
        elbowR: -0.3,
        armL: [0, 0, 0.15],
        tailWag: 0.5 * Math.sin(w * 4),
        wingFlap: 0.4 * Math.sin(w * 8),
        squash: 0.06 * Math.sin(w * 7),
      });
      Object.assign(expression, { mouth: "open", browLift: 0.35 });
      props.glyph = "!";
      props.glyphColor = "#fbbf24";
      break;
    case "success":
      Object.assign(pose, {
        rootY: 0.05 * Math.abs(Math.sin(w * 5)),
        armR: [-2.8 + 0.25 * Math.sin(w * 6), 0, -0.2],
        elbowR: -0.6,
        armL: [-0.3, 0, 0.25],
        tailWag: 0.6 * Math.sin(w * 6),
        wingFlap: 0.5 * Math.sin(w * 9),
        squash: 0.08 * Math.sin(w * 10),
      });
      Object.assign(expression, { mouth: "grin", blush: 0.55, browLift: 0.2 });
      props.confetti = true;
      break;
    case "celebrating":
      Object.assign(pose, {
        rootY: 0.08 * Math.abs(Math.sin(w * 6)),
        turnY: 0.4 * Math.sin(w * 3),
        armL: [-2.7 + 0.2 * Math.sin(w * 6), 0, 0.45],
        armR: [-2.7 + 0.2 * Math.sin(w * 6 + 1), 0, -0.45],
        elbowL: -0.4,
        elbowR: -0.4,
        tailWag: 0.7 * Math.sin(w * 8),
        wingFlap: 0.6 * Math.sin(w * 10),
        squash: 0.12 * Math.sin(w * 12),
      });
      Object.assign(expression, { mouth: "grin", blush: 0.6, browLift: 0.3 });
      props.confetti = true;
      break;
    case "level_up": {
      const phase = (w * 0.8) % 1;
      Object.assign(pose, {
        rootY: 0.12 * Math.sin(phase * Math.PI),
        turnY: phase * Math.PI * 2,
        armL: [-2.9, 0, 0.35],
        armR: [-2.9, 0, -0.35],
        elbowL: -0.2,
        elbowR: -0.2,
        tailWag: 0.6 * Math.sin(w * 8),
        wingFlap: 0.7 * Math.sin(w * 12),
        squash: 0.1 * Math.sin(phase * Math.PI * 2),
      });
      Object.assign(expression, { mouth: "grin", blush: 0.6, browLift: 0.35 });
      props.confetti = true;
      props.glyph = "LEVEL UP";
      props.glyphColor = "#fde047";
      break;
    }
    case "achievement":
      Object.assign(pose, {
        rootY: 0.03 * Math.abs(Math.sin(w * 4)),
        armL: [-2.6, 0, 0.25],
        armR: [-2.6, 0, -0.25],
        elbowL: -0.7,
        elbowR: -0.7,
        tailWag: 0.5 * Math.sin(w * 6),
        squash: 0.06 * Math.sin(w * 8),
      });
      Object.assign(expression, { mouth: "grin", blush: 0.5 });
      props.confetti = true;
      props.glyph = "★";
      props.glyphColor = "#fbbf24";
      break;
    case "error":
      Object.assign(pose, {
        leanX: -0.05,
        headY: 0.18 * Math.sin(w * 10),
        armL: [-2.5, 0, 0.55],
        armR: [-2.5, 0, -0.55],
        elbowL: -2.0,
        elbowR: -2.0,
        squash: -0.04,
      });
      Object.assign(expression, { mouth: "frown", browTilt: -0.4, blush: 0.05 });
      props.glyph = "×";
      props.glyphColor = "#f87171";
      break;
    case "sleeping":
    case "offline": {
      const slow = Math.sin(w * 0.9);
      Object.assign(pose, {
        leanX: 0.08,
        spineX: 0.12,
        chestX: 0.03 * slow,
        headX: 0.45,
        headZ: 0.2,
        armL: [0.05, 0, 0.08],
        armR: [0.05, 0, -0.08],
        elbowL: -0.1,
        elbowR: -0.1,
        squash: -0.05 + 0.04 * slow,
      });
      Object.assign(expression, { mouth: "flat", eyeOpen: 0, blush: 0.2 });
      props.sleepZ = state === "sleeping";
      props.dim = state === "offline";
      props.glyph = state === "offline" ? "OFFLINE" : null;
      props.glyphColor = "#94a3b8";
      frameRate = CALM_FRAME_RATE;
      break;
    }
    case "paused":
      Object.assign(pose, { headX: 0.1, armL: [0, 0, 0.1], armR: [0, 0, -0.1] });
      Object.assign(expression, { mouth: "flat", eyeOpen: 0.55 });
      props.glyph = "II";
      props.glyphColor = "#cbd5e1";
      frameRate = CALM_FRAME_RATE;
      break;
    case "hungry":
      Object.assign(pose, {
        spineX: 0.12,
        headX: 0.2,
        armL: [0.1, 0, 0.05],
        armR: [-0.7, 0, 0.1],
        elbowR: -1.2,
        squash: -0.06 + 0.02 * breathe,
      });
      Object.assign(expression, { mouth: "frown", browTilt: -0.3, eyeOpen: 0.8 });
      props.food = true;
      break;
    case "eating": {
      const chew = Math.sin(w * 5);
      Object.assign(pose, {
        headX: 0.05 + 0.05 * chew,
        armR: [-1.4, 0, -0.1],
        elbowR: -1.6 + 0.35 * chew,
        tailWag: 0.4 * Math.sin(w * 4),
        squash: 0.04 * chew,
      });
      Object.assign(expression, { mouth: chew > 0 ? "open" : "smile", blush: 0.5 });
      props.food = true;
      break;
    }
    case "idle":
    default:
      Object.assign(pose, {
        spineX: 0.02 * breathe,
        chestX: 0.015 * breathe,
        headX: -0.03 + 0.03 * Math.sin(w * 0.7),
        headY: 0.12 * Math.sin(w * 0.33),
        headZ: 0.04 * Math.sin(w * 0.5),
        armL: [0.05 * Math.sin(w * 1.1), 0, REST_ARM_SPLAY + 0.03 * breathe],
        armR: [0.05 * Math.sin(w * 1.1 + 1), 0, -REST_ARM_SPLAY - 0.03 * breathe],
        tailWag: 0.35 * Math.sin(w * 1.6),
        wingFlap: 0.15 * Math.sin(w * 1.5),
        squash: 0.03 * breathe,
      });
      break;
  }
  return { pose, expression, props, frameRate };
}

const lerp = (from: number, to: number, alpha: number) => from + (to - from) * alpha;

/** Eases `current` toward `target` in place. Turn angles take the short way. */
export function easePose(current: Pose, target: Pose, alpha: number): void {
  for (const key of Object.keys(current) as (keyof Pose)[]) {
    const value = current[key];
    const goal = target[key];
    if (Array.isArray(value) && Array.isArray(goal)) {
      for (let i = 0; i < 3; i += 1) value[i] = lerp(value[i], goal[i], alpha);
    } else if (typeof value === "number" && typeof goal === "number") {
      if (key === "turnY") {
        const delta = Math.atan2(Math.sin(goal - value), Math.cos(goal - value));
        current.turnY = value + delta * alpha;
      } else {
        (current as unknown as Record<string, number>)[key] = lerp(value, goal, alpha);
      }
    }
  }
}

export interface LookTarget {
  yaw: number;
  pitch: number;
}

/** Applies a pose and expression to a procedural rig. */
export function applyToRig(
  rig: CharacterRig,
  pose: Pose,
  expression: Expression,
  look: LookTarget,
  blink: number,
  time: number,
): void {
  rig.body.position.y = pose.rootY;
  rig.body.rotation.set(pose.leanX, pose.turnY, 0);
  rig.spine.rotation.x = pose.spineX;
  rig.chest.rotation.set(pose.chestX, 0, pose.chestZ);
  rig.head.rotation.set(pose.headX + look.pitch, pose.headY + look.yaw, pose.headZ);
  rig.armL.rotation.set(...pose.armL);
  rig.armR.rotation.set(...pose.armR);
  rig.elbowL.rotation.x = pose.elbowL;
  rig.elbowR.rotation.x = pose.elbowR;
  rig.legL.rotation.x = pose.legL;
  rig.legR.rotation.x = pose.legR;
  rig.kneeL.rotation.x = pose.kneeL;
  rig.kneeR.rotation.x = pose.kneeR;
  const squash = rig.spec.body === "slime" ? pose.squash * 2.2 : pose.squash * 0.35;
  rig.body.scale.set(1 - squash / 2, 1 + squash, 1 - squash / 2);
  rig.tail.forEach((segment, index) => {
    segment.rotation.y = pose.tailWag * (0.6 + 0.4 * index) * Math.cos(time * 0.5 + index * 0.6);
  });
  if (rig.wingL) rig.wingL.rotation.y = 0.25 + pose.wingFlap;
  if (rig.wingR) rig.wingR.rotation.y = -0.25 - pose.wingFlap;
  rig.hairSway.forEach((node, index) => {
    node.rotation.x = 0.06 * Math.sin(time * 1.3 + index) - pose.leanX * 0.6 - pose.headX * 0.3;
    node.rotation.z = -(pose.headZ + look.yaw * 0.2) * 0.6;
  });

  for (const shape of Object.keys(rig.mouths) as MouthShape[]) {
    rig.mouths[shape].visible = shape === expression.mouth;
  }
  const open = Math.max(0.06, expression.eyeOpen * rig.eyeBaseOpen * blink);
  for (const eye of rig.eyes) eye.scale.y = open;
  rig.brows.forEach((brow, index) => {
    const side = index === 0 ? 1 : -1;
    const baseY = (brow.userData.baseY as number | undefined) ?? brow.position.y;
    brow.userData.baseY = baseY;
    brow.position.y = baseY + expression.browLift * rig.spec.proportions.headScale * 0.02;
    brow.rotation.z = expression.browTilt * side;
  });
  for (const material of rig.blush) material.opacity = Math.min(0.75, expression.blush * 0.7);
}

/** Applies a pose to an image sculpt, which moves as one rigid body. */
export function applyToSculpt(model: THREE.Object3D, pose: Pose, look: LookTarget): void {
  const squash = pose.squash * 1.6;
  model.position.y = pose.rootY;
  model.rotation.set(pose.leanX + pose.headX * 0.25 + look.pitch * 0.3, pose.turnY + pose.headY * 0.4 + look.yaw * 0.5, pose.headZ * 0.6 + pose.chestZ);
  model.scale.set(1 - squash / 2, 1 + squash, 1 - squash / 2);
}

const DOWN = new THREE.Vector3(0, -1, 0);
const scratchEuler = new THREE.Euler();
const scratchQuat = new THREE.Quaternion();
const restToLeft = new THREE.Quaternion().setFromUnitVectors(DOWN, new THREE.Vector3(1, 0, 0));
const restToRight = new THREE.Quaternion().setFromUnitVectors(DOWN, new THREE.Vector3(-1, 0, 0));

function setBone(vrm: VRM, name: VRMHumanBoneName, quaternion: THREE.Quaternion): void {
  const node = vrm.humanoid.getNormalizedBoneNode(name);
  if (!node) return;
  node.quaternion.copy(quaternion);
  // VRM 0.x normalized bones live in the model's original -Z-facing frame.
  // Conjugating by a half turn around Y ([-x, y, -z, w]) maps our +Z-facing
  // rotations into that frame, including the left/right arm rest directions.
  if (vrm.meta.metaVersion === "0") {
    node.quaternion.set(-quaternion.x, quaternion.y, -quaternion.z, quaternion.w);
  }
}

function fromEuler(x: number, y: number, z: number): THREE.Quaternion {
  return scratchQuat.setFromEuler(scratchEuler.set(x, y, z)).clone();
}

/**
 * Retargets the procedural pose onto VRM normalized humanoid bones.
 * Our rig rests with arms hanging down; VRM rests in a T-pose, so arm
 * rotations are conjugated by the rest-direction change. Root motion goes on
 * `wrapper`, never on `vrm.scene`, whose transform holds the VRM 0.x turn and
 * the stage normalization.
 */
export function applyToVrm(vrm: VRM, wrapper: THREE.Object3D, pose: Pose, expression: Expression, look: LookTarget, blink: number): void {
  wrapper.position.y = pose.rootY;
  wrapper.rotation.y = pose.turnY;
  setBone(vrm, "spine", fromEuler(pose.spineX + pose.leanX, 0, 0));
  setBone(vrm, "chest", fromEuler(pose.chestX, 0, pose.chestZ));
  setBone(vrm, "neck", fromEuler((pose.headX + look.pitch) * 0.35, (pose.headY + look.yaw) * 0.35, pose.headZ * 0.35));
  setBone(vrm, "head", fromEuler((pose.headX + look.pitch) * 0.65, (pose.headY + look.yaw) * 0.65, pose.headZ * 0.65));

  const arm = (euler: Vec3, elbow: number, rest: THREE.Quaternion, upper: VRMHumanBoneName, lower: VRMHumanBoneName) => {
    const restInverse = rest.clone().invert();
    setBone(vrm, upper, fromEuler(...euler).multiply(restInverse));
    setBone(vrm, lower, rest.clone().multiply(fromEuler(elbow, 0, 0)).multiply(restInverse));
  };
  arm(pose.armL, pose.elbowL, restToLeft, "leftUpperArm", "leftLowerArm");
  arm(pose.armR, pose.elbowR, restToRight, "rightUpperArm", "rightLowerArm");
  setBone(vrm, "leftUpperLeg", fromEuler(pose.legL, 0, 0));
  setBone(vrm, "rightUpperLeg", fromEuler(pose.legR, 0, 0));
  setBone(vrm, "leftLowerLeg", fromEuler(pose.kneeL, 0, 0));
  setBone(vrm, "rightLowerLeg", fromEuler(pose.kneeR, 0, 0));

  const expressions = vrm.expressionManager;
  if (expressions) {
    const happy = expression.mouth === "grin" ? 1 : expression.mouth === "smile" ? 0.35 : 0;
    expressions.setValue("happy", happy);
    expressions.setValue("sad", expression.mouth === "frown" ? 0.6 : 0);
    expressions.setValue("aa", expression.mouth === "open" ? 0.6 : 0);
    expressions.setValue("surprised", expression.browLift > 0.3 ? 0.3 : 0);
    expressions.setValue("blink", Math.min(1, Math.max(0, 1 - expression.eyeOpen * blink)));
  }
}
