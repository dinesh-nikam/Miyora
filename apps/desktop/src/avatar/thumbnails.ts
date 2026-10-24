// Catalog thumbnails rendered with one shared offscreen WebGL context.
//
// Browsers cap live WebGL contexts, so a grid of 100+ characters cannot give
// each card its own renderer. Thumbnails are rendered one at a time into a
// single canvas, cached as PNG data URLs, and the rig is disposed right away.

import * as THREE from "three";
import { disposeObject } from "./dispose";
import { applyToRig, motionFor } from "./motion";
import { buildCharacter } from "./rig";
import type { CharacterSpec } from "./types";

const THUMBNAIL_SIZE = 168;
const THUMBNAIL_FOV = 28;
const CACHE_LIMIT = 400;

interface Job {
  key: string;
  spec: CharacterSpec;
  resolve: (url: string) => void;
  reject: (error: Error) => void;
}

let renderer: THREE.WebGLRenderer | null = null;
let scene: THREE.Scene | null = null;
let camera: THREE.PerspectiveCamera | null = null;
const cache = new Map<string, string>();
const pending = new Map<string, Promise<string>>();
const queue: Job[] = [];
let draining = false;

function ensureRenderer(): { renderer: THREE.WebGLRenderer; scene: THREE.Scene; camera: THREE.PerspectiveCamera } {
  if (!renderer || !scene || !camera) {
    renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true, preserveDrawingBuffer: true, powerPreference: "low-power" });
    renderer.setPixelRatio(1);
    renderer.setSize(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    scene = new THREE.Scene();
    scene.add(new THREE.HemisphereLight(0xffffff, 0x334155, 1.7));
    const key = new THREE.DirectionalLight(0xffffff, 1.5);
    key.position.set(1.2, 2.2, 2.4);
    scene.add(key);
    camera = new THREE.PerspectiveCamera(THUMBNAIL_FOV, 1, 0.05, 10);
  }
  return { renderer, scene, camera };
}

function renderNow(spec: CharacterSpec): string {
  const context = ensureRenderer();
  const rig = buildCharacter(spec);
  try {
    const motion = motionFor("idle", 0, spec.energy);
    applyToRig(rig, motion.pose, motion.expression, { yaw: -0.15, pitch: 0 }, 1, 0);
    rig.root.rotation.y = 0.35;
    context.scene.add(rig.root);
    const height = rig.headTopY + 0.06;
    const distance = height / (2 * Math.tan(THREE.MathUtils.degToRad(THUMBNAIL_FOV / 2))) + 0.25;
    context.camera.position.set(0, height * 0.55, distance);
    context.camera.lookAt(0, height * 0.5, 0);
    context.renderer.render(context.scene, context.camera);
    return context.renderer.domElement.toDataURL("image/png");
  } finally {
    context.scene.remove(rig.root);
    disposeObject(rig.root);
  }
}

function drain(): void {
  if (draining) return;
  draining = true;
  const step = () => {
    const job = queue.shift();
    if (!job) {
      draining = false;
      return;
    }
    try {
      const url = renderNow(job.spec);
      cache.set(job.key, url);
      if (cache.size > CACHE_LIMIT) {
        const oldest = cache.keys().next().value;
        if (oldest !== undefined) cache.delete(oldest);
      }
      job.resolve(url);
    } catch (error: unknown) {
      job.reject(error instanceof Error ? error : new Error(String(error)));
    } finally {
      pending.delete(job.key);
    }
    // Yield between thumbnails so the UI stays responsive.
    window.setTimeout(step, 0);
  };
  step();
}

const THUMBNAIL_CACHE_VERSION = "v3_detailed";

/** Renders (or returns the cached) thumbnail for `spec` as a PNG data URL. */
export function renderThumbnail(spec: CharacterSpec): Promise<string> {
  const key = `${THUMBNAIL_CACHE_VERSION}:${JSON.stringify(spec)}`;
  const cached = cache.get(key);
  if (cached) return Promise.resolve(cached);
  const inFlight = pending.get(key);
  if (inFlight) return inFlight;
  const promise = new Promise<string>((resolve, reject) => {
    queue.push({ key, spec, resolve, reject });
  });
  pending.set(key, promise);
  drain();
  return promise;
}
