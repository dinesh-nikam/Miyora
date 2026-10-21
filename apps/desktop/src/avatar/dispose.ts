import * as THREE from "three";
import { VRM, VRMUtils } from "@pixiv/three-vrm";

export function disposeMaterial(material: THREE.Material): void {
  for (const key of Object.keys(material)) {
    const value = (material as unknown as Record<string, unknown>)[key];
    if (value && typeof value === "object" && "isTexture" in value && (value as THREE.Texture).isTexture) {
      (value as THREE.Texture).dispose();
    }
  }
  material.dispose();
}

/** Releases GPU resources held by every mesh, point cloud, and sprite under `root`. */
export function disposeObject(root: THREE.Object3D): void {
  root.traverse((object) => {
    const drawable = object as THREE.Mesh | THREE.Points | THREE.Sprite;
    if ((drawable as THREE.Mesh).isMesh || (drawable as THREE.Points).isPoints || (drawable as THREE.Sprite).isSprite) {
      drawable.geometry?.dispose();
      const material = drawable.material;
      if (Array.isArray(material)) {
        material.forEach(disposeMaterial);
      } else if (material) {
        disposeMaterial(material);
      }
    }
  });
}

export function disposeScene(scene: THREE.Scene, renderer: THREE.WebGLRenderer, vrm: VRM | null = null): void {
  if (vrm) {
    try {
      VRMUtils.deepDispose(vrm.scene);
    } catch (error: unknown) {
      console.warn("[avatar] VRM dispose failed", error);
    }
  }
  disposeObject(scene);
  renderer.dispose();
  try {
    renderer.forceContextLoss();
  } catch (error: unknown) {
    // The context may already be lost when the window closes.
    console.warn("[avatar] forceContextLoss failed", error);
  }
}
