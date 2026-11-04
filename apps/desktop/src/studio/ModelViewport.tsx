import { useEffect, useRef, useState } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { FBXLoader } from "three/examples/jsm/loaders/FBXLoader.js";
import { MeshoptDecoder } from "three/examples/jsm/libs/meshopt_decoder.module.js";
import { disposeObject, disposeScene } from "../avatar/dispose";
import { computeSceneStats, type SceneStats } from "./sceneStats";

export type ViewMode = "material" | "solid" | "wireframe";

interface ModelViewportProps {
  bytes: ArrayBuffer | null;
  format: string | null;
  viewMode: ViewMode;
  showGrid: boolean;
  showBounds: boolean;
  /** Increment to frame the model keeping the current view direction. */
  frameToken: number;
  /** Increment to reset the camera to the default three-quarter view. */
  resetToken: number;
  onStats: (stats: SceneStats | null) => void;
  onError: (message: string | null) => void;
}

interface ViewportContext {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: OrbitControls;
  grid: THREE.GridHelper;
  bounds: THREE.Box3Helper | null;
  model: THREE.Object3D | null;
  originals: Map<THREE.Mesh, THREE.Material | THREE.Material[]>;
  overrides: Record<Exclude<ViewMode, "material">, THREE.Material>;
  render: () => void;
}

const DEFAULT_DIRECTION = new THREE.Vector3(1, 0.7, 1).normalize();
/** Extra room around the model's bounding sphere when framing. */
const FRAME_MARGIN = 1.15;

function frame(context: ViewportContext, direction: THREE.Vector3): void {
  if (!context.model) return;
  const box = new THREE.Box3().setFromObject(context.model);
  if (box.isEmpty()) return;
  const center = box.getCenter(new THREE.Vector3());
  const radius = Math.max(box.getSize(new THREE.Vector3()).length() / 2, 1e-3);
  const fov = THREE.MathUtils.degToRad(context.camera.fov);
  const distance = (radius / Math.sin(fov / 2)) * FRAME_MARGIN;
  context.camera.position.copy(center).addScaledVector(direction, distance);
  context.camera.near = Math.max(distance / 1000, 1e-4);
  context.camera.far = distance * 100;
  context.camera.updateProjectionMatrix();
  context.controls.target.copy(center);
  context.controls.update();
  context.grid.position.y = box.min.y;
  context.grid.scale.setScalar(Math.max(radius, 0.5));
  context.render();
}

function applyViewMode(context: ViewportContext, mode: ViewMode): void {
  context.originals.forEach((original, mesh) => {
    mesh.material = mode === "material" ? original : context.overrides[mode];
  });
  context.render();
}

async function parseModel(bytes: ArrayBuffer, format: string): Promise<THREE.Object3D> {
  if (format === "fbx") {
    return new FBXLoader().parse(bytes.slice(0), "");
  }
  const loader = new GLTFLoader();
  // Provider geometry compression uses meshopt.
  loader.setMeshoptDecoder(MeshoptDecoder);
  const gltf = await loader.parseAsync(bytes.slice(0), "");
  return gltf.scene;
}

export function ModelViewport(props: ModelViewportProps) {
  const { bytes, format, viewMode, showGrid, showBounds, frameToken, resetToken, onStats, onError } = props;
  const hostRef = useRef<HTMLDivElement | null>(null);
  const contextRef = useRef<ViewportContext | null>(null);
  const viewModeRef = useRef(viewMode);
  viewModeRef.current = viewMode;
  // Bumped after each async load so overlays (bounds) attach to the new model.
  const [loadedModels, setLoadedModels] = useState(0);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true });
    } catch (error: unknown) {
      onError(`3D viewport unavailable: ${error instanceof Error ? error.message : String(error)}`);
      return;
    }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    host.appendChild(renderer.domElement);

    const scene = new THREE.Scene();
    scene.background = new THREE.Color("#15171c");
    const pmrem = new THREE.PMREMGenerator(renderer);
    const environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    pmrem.dispose();
    scene.environment = environment;
    const keyLight = new THREE.DirectionalLight(0xffffff, 1.1);
    keyLight.position.set(3, 5, 2);
    scene.add(keyLight, new THREE.AmbientLight(0xffffff, 0.15));
    const grid = new THREE.GridHelper(4, 20, 0x3b4150, 0x262a33);
    scene.add(grid);

    const camera = new THREE.PerspectiveCamera(40, 1, 0.01, 1000);
    camera.position.copy(DEFAULT_DIRECTION).multiplyScalar(4);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.screenSpacePanning = true;
    const render = () => renderer.render(scene, camera);
    // Render on demand: no idle GPU work while nothing moves.
    controls.addEventListener("change", render);

    const context: ViewportContext = {
      renderer,
      scene,
      camera,
      controls,
      grid,
      bounds: null,
      model: null,
      originals: new Map(),
      overrides: {
        solid: new THREE.MeshStandardMaterial({ color: 0xb8bcc6, roughness: 0.65, metalness: 0 }),
        wireframe: new THREE.MeshBasicMaterial({ color: 0x7dd3fc, wireframe: true }),
      },
      render,
    };
    contextRef.current = context;

    const resize = () => {
      const width = Math.max(host.clientWidth, 1);
      const height = Math.max(host.clientHeight, 1);
      renderer.setSize(width, height);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
      render();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(host);
    resize();

    return () => {
      observer.disconnect();
      controls.removeEventListener("change", render);
      controls.dispose();
      context.originals.forEach((original, mesh) => {
        mesh.material = original;
      });
      Object.values(context.overrides).forEach((material) => material.dispose());
      context.bounds?.dispose();
      environment.dispose();
      disposeScene(scene, renderer);
      renderer.domElement.remove();
      contextRef.current = null;
    };
  }, [onError]);

  useEffect(() => {
    const context = contextRef.current;
    if (!context) return;
    let cancelled = false;
    const clear = () => {
      if (context.bounds) {
        context.scene.remove(context.bounds);
        context.bounds.dispose();
        context.bounds = null;
      }
      if (context.model) {
        context.originals.forEach((original, mesh) => {
          mesh.material = original;
        });
        context.scene.remove(context.model);
        disposeObject(context.model);
        context.model = null;
        context.originals.clear();
      }
    };
    clear();
    onStats(null);
    context.render();
    if (!bytes || !format) return;
    onError(null);
    parseModel(bytes, format)
      .then((model) => {
        if (cancelled) {
          disposeObject(model);
          return;
        }
        context.model = model;
        model.traverse((object) => {
          const mesh = object as THREE.Mesh;
          if (mesh.isMesh) context.originals.set(mesh, mesh.material);
        });
        context.scene.add(model);
        frame(context, DEFAULT_DIRECTION);
        applyViewMode(context, viewModeRef.current);
        onStats(computeSceneStats(model));
        setLoadedModels((count) => count + 1);
      })
      .catch((error: unknown) => {
        if (!cancelled) onError(`Model could not be displayed: ${error instanceof Error ? error.message : String(error)}`);
      });
    return () => {
      cancelled = true;
      clear();
    };
  }, [bytes, format, onError, onStats]);

  useEffect(() => {
    if (contextRef.current) applyViewMode(contextRef.current, viewMode);
  }, [viewMode]);

  useEffect(() => {
    const context = contextRef.current;
    if (!context) return;
    context.grid.visible = showGrid;
    if (context.bounds) {
      context.scene.remove(context.bounds);
      context.bounds.dispose();
      context.bounds = null;
    }
    if (showBounds && context.model) {
      context.bounds = new THREE.Box3Helper(new THREE.Box3().setFromObject(context.model), 0xfbbf24);
      context.scene.add(context.bounds);
    }
    context.render();
  }, [showGrid, showBounds, loadedModels]);

  useEffect(() => {
    const context = contextRef.current;
    if (!context || frameToken === 0) return;
    const direction = context.camera.position.clone().sub(context.controls.target).normalize();
    frame(context, direction.lengthSq() > 0 ? direction : DEFAULT_DIRECTION);
  }, [frameToken]);

  useEffect(() => {
    if (contextRef.current && resetToken > 0) frame(contextRef.current, DEFAULT_DIRECTION);
  }, [resetToken]);

  return <div ref={hostRef} className="studio-viewport-canvas" aria-label="3D model viewport" role="img" />;
}
