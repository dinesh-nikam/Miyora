import { useEffect, useRef } from "react";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { VRM, VRMLoaderPlugin, VRMUtils } from "@pixiv/three-vrm";
import { invoke } from "@tauri-apps/api/core";
import type { PetState } from "../components/CompanionRenderer";
import { disposeObject, disposeScene } from "./dispose";
import { applyToRig, applyToSculpt, applyToVrm, easePose, motionFor, POSE_EASE_RATE, restPose, type LookTarget } from "./motion";
import { StageProps } from "./props";
import { buildCharacter, type CharacterRig } from "./rig";
import { subjectKey, type AvatarHud, type StageSubject } from "./types";

export interface AvatarStageProps {
  subject: StageSubject | null;
  state: PetState;
  hud?: AvatarHud | null;
  size?: number;
  interactive?: boolean;
  onDoubleClick?: () => void;
  onError?: (message: string) => void;
}

const FIELD_OF_VIEW = 30;
const MODEL_TARGET_HEIGHT = 0.92;
const STAGE_WIDTH = 1.0;
const LOOK_YAW_LIMIT = 0.45;
const LOOK_PITCH_LIMIT = 0.22;
const DRAG_SENSITIVITY = 0.012;
const BLINK_MIN_INTERVAL = 2.4;
const BLINK_MAX_INTERVAL = 5.6;
const BLINK_DURATION = 0.14;
const MODEL_CACHE_LIMIT = 4;
const DIM_LIGHT_FACTOR = 0.45;
/** Upper bound on one animation step, so a stalled tab cannot jump the pose. */
const MAX_FRAME_STEP = 0.1;
/** Camera aim as a fraction of the framed height; below half leaves room for the XP label. */
const FRAME_TARGET_RATIO = 0.36;

const modelCache = new Map<string, ArrayBuffer>();

async function fetchModel(avatarId: string, sha256: string): Promise<ArrayBuffer> {
  const cached = modelCache.get(sha256);
  if (cached) return cached;
  const bytes = await invoke<ArrayBuffer>("get_avatar_model", { avatarId });
  if (!(bytes instanceof ArrayBuffer)) {
    throw new Error("AVATAR_MODEL_READ_FAILED: model response was not binary");
  }
  modelCache.set(sha256, bytes);
  while (modelCache.size > MODEL_CACHE_LIMIT) {
    const oldest = modelCache.keys().next().value;
    if (oldest === undefined) break;
    modelCache.delete(oldest);
  }
  return bytes;
}

type LoadedAvatar =
  | { kind: "rig"; rig: CharacterRig; object: THREE.Object3D; headTopY: number; deskY: number; energy: number; accent: string }
  | { kind: "vrm"; vrm: VRM; object: THREE.Object3D; headTopY: number; deskY: number; energy: number; accent: string }
  | { kind: "sculpt"; model: THREE.Object3D; object: THREE.Object3D; headTopY: number; deskY: number; energy: number; accent: string };

function normalizeModel(object: THREE.Object3D): number {
  const box = new THREE.Box3().setFromObject(object);
  const size = box.getSize(new THREE.Vector3());
  const height = Math.max(size.y, 1e-3);
  const scale = MODEL_TARGET_HEIGHT / height;
  object.scale.multiplyScalar(scale);
  const scaled = new THREE.Box3().setFromObject(object);
  const center = scaled.getCenter(new THREE.Vector3());
  object.position.x -= center.x;
  object.position.z -= center.z;
  object.position.y -= scaled.min.y;
  return MODEL_TARGET_HEIGHT;
}

async function loadModelAvatar(subject: Extract<StageSubject, { kind: "image_sculpt" | "vrm" }>): Promise<LoadedAvatar> {
  const bytes = await fetchModel(subject.avatarId, subject.modelSha256);
  const loader = new GLTFLoader();
  loader.register((parser) => new VRMLoaderPlugin(parser));
  const gltf = await loader.parseAsync(bytes.slice(0), "");
  const vrm = gltf.userData.vrm as VRM | undefined;
  if (vrm) {
    VRMUtils.removeUnnecessaryVertices(gltf.scene);
    VRMUtils.rotateVRM0(vrm);
    const wrapper = new THREE.Group();
    wrapper.add(vrm.scene);
    const height = normalizeModel(vrm.scene);
    return { kind: "vrm", vrm, object: wrapper, headTopY: height, deskY: height * 0.45, energy: 1, accent: "#38bdf8" };
  }
  const pivot = new THREE.Group();
  pivot.add(gltf.scene);
  const height = normalizeModel(gltf.scene);
  return { kind: "sculpt", model: pivot, object: pivot, headTopY: height, deskY: height * 0.4, energy: 1, accent: "#38bdf8" };
}

function loadProcedural(subject: Extract<StageSubject, { kind: "procedural" }>): LoadedAvatar {
  const rig = buildCharacter(subject.spec);
  return {
    kind: "rig",
    rig,
    object: rig.root,
    headTopY: rig.headTopY,
    deskY: rig.deskY,
    energy: subject.spec.energy,
    accent: subject.spec.palette.accent,
  };
}

function disposeAvatar(avatar: LoadedAvatar | null): void {
  if (!avatar) return;
  avatar.object.parent?.remove(avatar.object);
  if (avatar.kind === "vrm") {
    VRMUtils.deepDispose(avatar.vrm.scene);
  } else {
    disposeObject(avatar.object);
  }
}

export function AvatarStage({ subject, state, hud = null, size = 240, interactive = true, onDoubleClick, onError }: AvatarStageProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const stateRef = useRef<PetState>(state);
  stateRef.current = state;
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;
  const stageRef = useRef<{
    scene: THREE.Scene;
    camera: THREE.PerspectiveCamera;
    renderer: THREE.WebGLRenderer;
    holder: THREE.Group;
    props: StageProps;
    lights: THREE.Light[];
    baseIntensity: number[];
    avatar: LoadedAvatar | null;
  } | null>(null);
  const key = subjectKey(subject);
  const subjectRef = useRef(subject);
  subjectRef.current = subject;

  // Renderer, scene, and animation loop live for the component lifetime.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true, powerPreference: "low-power" });
    } catch (error: unknown) {
      onErrorRef.current?.(`AVATAR_WEBGL_UNAVAILABLE: ${String(error)}`);
      return;
    }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    renderer.setSize(size, size);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    container.appendChild(renderer.domElement);

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(FIELD_OF_VIEW, 1, 0.05, 20);
    const hemisphere = new THREE.HemisphereLight(0xffffff, 0x334155, 1.7);
    const keyLight = new THREE.DirectionalLight(0xffffff, 1.5);
    keyLight.position.set(1.2, 2.2, 2.4);
    const rim = new THREE.DirectionalLight(0x7dd3fc, 1.1);
    rim.position.set(-1.6, 1.4, -1.8);
    const lights: THREE.Light[] = [hemisphere, keyLight, rim];
    lights.forEach((light) => scene.add(light));
    const holder = new THREE.Group();
    scene.add(holder);
    const props = new StageProps("#38bdf8");
    scene.add(props.group, props.hudGroup);
    stageRef.current = { scene, camera, renderer, holder, props, lights, baseIntensity: lights.map((l) => l.intensity), avatar: null };

    let orbit = 0;
    let dragging = false;
    let lastX = 0;
    const look: LookTarget = { yaw: 0, pitch: 0 };
    const lookGoal: LookTarget = { yaw: 0, pitch: 0 };
    const onPointerMove = (event: PointerEvent) => {
      const rect = container.getBoundingClientRect();
      const nx = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      const ny = ((event.clientY - rect.top) / rect.height) * 2 - 1;
      lookGoal.yaw = THREE.MathUtils.clamp(nx * LOOK_YAW_LIMIT, -LOOK_YAW_LIMIT, LOOK_YAW_LIMIT);
      lookGoal.pitch = THREE.MathUtils.clamp(ny * LOOK_PITCH_LIMIT, -LOOK_PITCH_LIMIT, LOOK_PITCH_LIMIT);
      if (dragging) {
        orbit += (event.clientX - lastX) * DRAG_SENSITIVITY;
        lastX = event.clientX;
      }
    };
    const onPointerDown = (event: PointerEvent) => {
      dragging = true;
      lastX = event.clientX;
    };
    const onPointerUp = () => {
      dragging = false;
    };
    const onPointerLeave = () => {
      lookGoal.yaw = 0;
      lookGoal.pitch = 0;
    };
    if (interactive) {
      container.addEventListener("pointermove", onPointerMove);
      container.addEventListener("pointerdown", onPointerDown);
      container.addEventListener("pointerleave", onPointerLeave);
      window.addEventListener("pointerup", onPointerUp);
    }

    const pose = restPose();
    let lastTimestamp = performance.now();
    let elapsed = 0;
    let sinceRender = Infinity;
    let nextBlink = BLINK_MIN_INTERVAL;
    let blinkStart = -1;
    let frame = 0;

    const tick = () => {
      frame = requestAnimationFrame(tick);
      const now = performance.now();
      const dt = Math.min((now - lastTimestamp) / 1000, MAX_FRAME_STEP);
      lastTimestamp = now;
      if (document.hidden) return;
      const stage = stageRef.current;
      if (!stage) return;
      const avatar = stage.avatar;
      const motion = motionFor(stateRef.current, elapsed, avatar?.energy ?? 1);
      sinceRender += dt;
      if (sinceRender < 1 / motion.frameRate) return;
      const step = Math.min(sinceRender, MAX_FRAME_STEP);
      sinceRender = 0;
      elapsed += step;

      easePose(pose, motion.pose, 1 - Math.exp(-POSE_EASE_RATE * step));
      const lookWeight = motion.expression.eyeOpen > 0.2 ? 1 : 0;
      look.yaw += (lookGoal.yaw * lookWeight - look.yaw) * Math.min(1, step * 6);
      look.pitch += (lookGoal.pitch * lookWeight - look.pitch) * Math.min(1, step * 6);

      if (elapsed >= nextBlink) {
        blinkStart = elapsed;
        nextBlink = elapsed + BLINK_MIN_INTERVAL + Math.random() * (BLINK_MAX_INTERVAL - BLINK_MIN_INTERVAL);
      }
      const blinkProgress = blinkStart >= 0 ? (elapsed - blinkStart) / BLINK_DURATION : 1;
      const blink = blinkProgress < 1 ? Math.abs(1 - 2 * blinkProgress) : 1;

      stage.holder.rotation.y = orbit;
      if (avatar?.kind === "rig") {
        applyToRig(avatar.rig, pose, motion.expression, look, blink, elapsed);
        if (avatar.rig.earL && avatar.rig.earR) {
          const twitch = Math.max(0, Math.sin(elapsed * 0.9) - 0.96) * 6;
          avatar.rig.earL.rotation.x = -twitch * 0.3;
          avatar.rig.earR.rotation.x = -twitch * 0.15;
        }
      } else if (avatar?.kind === "vrm") {
        applyToVrm(avatar.vrm, avatar.object, pose, motion.expression, look, blink);
        avatar.vrm.update(step);
      } else if (avatar?.kind === "sculpt") {
        applyToSculpt(avatar.model, pose, look);
      }
      stage.props.group.rotation.y = orbit;
      stage.props.update(motion.props, step, elapsed);
      stage.lights.forEach((light, index) => {
        light.intensity = stage.baseIntensity[index] * (motion.props.dim ? DIM_LIGHT_FACTOR : 1);
      });
      stage.renderer.render(stage.scene, stage.camera);
    };
    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);
      if (interactive) {
        container.removeEventListener("pointermove", onPointerMove);
        container.removeEventListener("pointerdown", onPointerDown);
        container.removeEventListener("pointerleave", onPointerLeave);
        window.removeEventListener("pointerup", onPointerUp);
      }
      const stage = stageRef.current;
      stageRef.current = null;
      if (stage) {
        if (stage.avatar?.kind === "vrm") {
          const vrm = stage.avatar.vrm;
          stage.holder.remove(stage.avatar.object);
          disposeScene(stage.scene, stage.renderer, vrm);
        } else {
          disposeScene(stage.scene, stage.renderer, null);
        }
      }
      renderer.domElement.remove();
    };
  }, [interactive, size]);

  // Swap the character whenever the subject changes.
  useEffect(() => {
    const stage = stageRef.current;
    const current = subjectRef.current;
    if (!stage) return;
    let cancelled = false;
    const install = (avatar: LoadedAvatar | null) => {
      const live = stageRef.current;
      if (cancelled || !live) {
        disposeAvatar(avatar);
        return;
      }
      disposeAvatar(live.avatar);
      live.avatar = avatar;
      const headTopY = avatar?.headTopY ?? 0.9;
      if (avatar) live.holder.add(avatar.object);
      live.props.setAccent(avatar?.accent ?? "#38bdf8");
      live.props.setLayout({ headTopY, deskY: avatar?.deskY ?? 0.35 });
      const halfFov = THREE.MathUtils.degToRad(FIELD_OF_VIEW / 2);
      const frameHeight = headTopY + 0.38;
      const distance = Math.max(frameHeight / (2 * Math.tan(halfFov)), STAGE_WIDTH / (2 * Math.tan(halfFov)));
      const targetY = frameHeight * FRAME_TARGET_RATIO;
      live.camera.position.set(0, targetY + distance * 0.12, distance);
      live.camera.lookAt(0, targetY, 0);
    };
    if (!current) {
      install(null);
      return () => {
        cancelled = true;
      };
    }
    if (current.kind === "procedural") {
      try {
        install(loadProcedural(current));
      } catch (error: unknown) {
        onErrorRef.current?.(`AVATAR_BUILD_FAILED: ${String(error)}`);
        install(null);
      }
    } else {
      loadModelAvatar(current)
        .then(install)
        .catch((error: unknown) => {
          // A superseded load is not an error the user needs to see.
          if (cancelled) return;
          onErrorRef.current?.(`AVATAR_MODEL_LOAD_FAILED: ${String(error)}`);
          install(null);
        });
    }
    return () => {
      cancelled = true;
    };
  }, [key, interactive, size]);

  useEffect(() => {
    stageRef.current?.props.setHud(hud);
  }, [hud, key, interactive, size]);

  return (
    <div
      ref={containerRef}
      onDoubleClick={onDoubleClick}
      title={interactive ? "Drag to turn the character. Double-click for the command deck." : undefined}
      style={{ width: size, height: size, cursor: interactive ? "grab" : "default", userSelect: "none", position: "relative", touchAction: "none" }}
    />
  );
}
