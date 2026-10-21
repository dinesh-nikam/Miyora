import { describe, it, expect, vi } from "vitest";
import * as THREE from "three";
import { disposeScene, disposeMaterial } from "../../avatar/dispose";
import { VRMUtils } from "@pixiv/three-vrm";

vi.mock("@pixiv/three-vrm", () => ({
  VRMUtils: {
    deepDispose: vi.fn(),
  },
  VRMLoaderPlugin: vi.fn(),
}));

describe("disposeScene and WebGL teardown", () => {
  it("disposes geometry and material on scene traversal", () => {
    const scene = new THREE.Scene();
    const geometry = new THREE.BoxGeometry(1, 1, 1);
    const material = new THREE.MeshBasicMaterial({ color: 0xff0000 });

    const geomDisposeSpy = vi.spyOn(geometry, "dispose");
    const matDisposeSpy = vi.spyOn(material, "dispose");

    const mesh = new THREE.Mesh(geometry, material);
    scene.add(mesh);

    const renderer = {
      dispose: vi.fn(),
      forceContextLoss: vi.fn(),
    } as unknown as THREE.WebGLRenderer;

    disposeScene(scene, renderer, null);

    expect(geomDisposeSpy).toHaveBeenCalled();
    expect(matDisposeSpy).toHaveBeenCalled();
    expect(renderer.dispose).toHaveBeenCalled();
    expect(renderer.forceContextLoss).toHaveBeenCalled();
  });

  it("disposes materials in an array", () => {
    const mat1 = new THREE.MeshBasicMaterial({ color: 0xff0000 });
    const mat2 = new THREE.MeshBasicMaterial({ color: 0x00ff00 });
    const spy1 = vi.spyOn(mat1, "dispose");
    const spy2 = vi.spyOn(mat2, "dispose");

    const scene = new THREE.Scene();
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), [mat1, mat2]);
    scene.add(mesh);

    const renderer = {
      dispose: vi.fn(),
      forceContextLoss: vi.fn(),
    } as unknown as THREE.WebGLRenderer;

    disposeScene(scene, renderer, null);

    expect(spy1).toHaveBeenCalled();
    expect(spy2).toHaveBeenCalled();
  });

  it("disposes attached textures on material", () => {
    const material = new THREE.MeshBasicMaterial();
    const texture = new THREE.Texture();
    const texSpy = vi.spyOn(texture, "dispose");
    material.map = texture;

    disposeMaterial(material);
    expect(texSpy).toHaveBeenCalled();
  });

  it("calls VRMUtils.deepDispose when VRM scene is present", () => {
    const scene = new THREE.Scene();
    const renderer = {
      dispose: vi.fn(),
      forceContextLoss: vi.fn(),
    } as unknown as THREE.WebGLRenderer;

    const mockVrm = {
      scene: new THREE.Group(),
    } as unknown as import("@pixiv/three-vrm").VRM;

    disposeScene(scene, renderer, mockVrm);

    expect(VRMUtils.deepDispose).toHaveBeenCalledWith(mockVrm.scene);
    expect(renderer.dispose).toHaveBeenCalled();
    expect(renderer.forceContextLoss).toHaveBeenCalled();
  });
});
