import * as THREE from "three";

export interface TextureInfo {
  slot: string;
  width: number | null;
  height: number | null;
}

export interface HierarchyNode {
  name: string;
  type: string;
  depth: number;
}

export interface SceneStats {
  meshes: number;
  vertices: number;
  triangles: number;
  materials: number;
  textures: TextureInfo[];
  size: { x: number; y: number; z: number };
  hierarchy: HierarchyNode[];
  hierarchyTruncated: boolean;
}

const TEXTURE_SLOTS = ["map", "normalMap", "roughnessMap", "metalnessMap", "aoMap", "emissiveMap"] as const;
const MAX_HIERARCHY_NODES = 200;
const MAX_HIERARCHY_DEPTH = 4;

function imageSize(texture: THREE.Texture): { width: number | null; height: number | null } {
  const image: unknown = texture.image;
  if (typeof image === "object" && image !== null && "width" in image && "height" in image) {
    const { width, height } = image as { width: unknown; height: unknown };
    if (typeof width === "number" && typeof height === "number") return { width, height };
  }
  return { width: null, height: null };
}

/** Counts what the renderer actually holds, so numbers reflect the loaded scene. */
export function computeSceneStats(root: THREE.Object3D): SceneStats {
  let meshes = 0;
  let vertices = 0;
  let triangles = 0;
  const materials = new Set<string>();
  const textures = new Map<string, TextureInfo>();
  root.updateMatrixWorld(true);
  root.traverse((object) => {
    const mesh = object as THREE.Mesh;
    if (!mesh.isMesh) return;
    meshes += 1;
    const position = mesh.geometry.getAttribute("position");
    const count = position ? position.count : 0;
    vertices += count;
    const index = mesh.geometry.getIndex();
    triangles += Math.floor((index ? index.count : count) / 3);
    const list = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    for (const material of list) {
      materials.add(material.uuid);
      const slots = material as unknown as Partial<Record<(typeof TEXTURE_SLOTS)[number], THREE.Texture | null>>;
      for (const slot of TEXTURE_SLOTS) {
        const texture = slots[slot];
        if (texture && !textures.has(texture.uuid)) {
          textures.set(texture.uuid, { slot, ...imageSize(texture) });
        }
      }
    }
  });
  const box = new THREE.Box3().setFromObject(root);
  const size = box.isEmpty() ? new THREE.Vector3() : box.getSize(new THREE.Vector3());

  const hierarchy: HierarchyNode[] = [];
  let truncated = false;
  const walk = (node: THREE.Object3D, depth: number) => {
    if (hierarchy.length >= MAX_HIERARCHY_NODES) {
      truncated = true;
      return;
    }
    hierarchy.push({ name: node.name || "(unnamed)", type: node.type, depth });
    if (depth >= MAX_HIERARCHY_DEPTH) {
      truncated ||= node.children.length > 0;
      return;
    }
    node.children.forEach((child) => walk(child, depth + 1));
  };
  root.children.forEach((child) => walk(child, 0));

  return {
    meshes,
    vertices,
    triangles,
    materials: materials.size,
    textures: [...textures.values()],
    size: { x: size.x, y: size.y, z: size.z },
    hierarchy,
    hierarchyTruncated: truncated,
  };
}
