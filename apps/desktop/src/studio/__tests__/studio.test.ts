// @vitest-environment node
import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { formatBytes, formatElapsed, isActive, parseStudioError, taskStatusLabel } from "../format";
import { computeSceneStats } from "../sceneStats";

describe("parseStudioError", () => {
  it("reads the structured report sent by Rust commands", () => {
    const raw = JSON.stringify({
      kind: "insufficient_credits",
      message: "Provider rejected the request: no credit",
      suggestion: "Top up credits with the provider, then retry.",
      requestId: "req_abc123",
      providerCode: 2010,
    });
    expect(parseStudioError(raw)).toEqual({
      kind: "insufficient_credits",
      message: "Provider rejected the request: no credit",
      suggestion: "Top up credits with the provider, then retry.",
      requestId: "req_abc123",
      providerCode: 2010,
    });
  });

  it.each(["command studio_x not found", "{\"kind\":1}", "null", "[]"])("wraps unstructured errors: %s", (raw) => {
    const report = parseStudioError(raw);
    expect(report.kind).toBe("unknown");
    expect(report.message).toBe(raw);
    expect(report.requestId).toBeNull();
  });
});

describe("task labels", () => {
  it("never reports success before the local state is SUCCEEDED", () => {
    expect(taskStatusLabel({ state: "RUNNING", stage: "downloading" })).toBe("Downloading…");
    expect(taskStatusLabel({ state: "RUNNING", stage: "generating" })).toBe("Generating…");
    expect(taskStatusLabel({ state: "RUNNING", stage: "retrying" })).toBe("Reconnecting…");
    expect(taskStatusLabel({ state: "QUEUED", stage: "submitted" })).toBe("Queued");
    expect(taskStatusLabel({ state: "SUCCEEDED", stage: null })).toBe("Completed");
    expect(isActive({ state: "RUNNING" })).toBe(true);
    expect(isActive({ state: "FAILED" })).toBe(false);
  });
});

describe("formatting", () => {
  it("formats elapsed time against completion or now", () => {
    expect(formatElapsed("2026-10-29T10:00:00Z", "2026-10-29T10:01:05Z", 0)).toBe("1m 05s");
    expect(formatElapsed("2026-10-29T10:00:00Z", null, Date.parse("2026-10-29T12:03:00Z"))).toBe("2h 03m");
    expect(formatElapsed("bad", null, 0)).toBe("—");
  });

  it("formats byte sizes", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(20 * 1024 * 1024)).toBe("20.0 MB");
  });
});

describe("computeSceneStats", () => {
  it("counts geometry, materials, textures and dimensions from the loaded scene", () => {
    const texture = new THREE.DataTexture(new Uint8Array(4 * 4 * 4), 4, 4);
    const material = new THREE.MeshStandardMaterial({ map: texture, normalMap: texture });
    const box = new THREE.Mesh(new THREE.BoxGeometry(2, 1, 0.5), material);
    box.name = "Body";
    const nonIndexed = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1).toNonIndexed(), new THREE.MeshBasicMaterial());
    nonIndexed.position.set(3, 0, 0);
    const root = new THREE.Group();
    const child = new THREE.Group();
    child.add(nonIndexed);
    root.add(box, child);

    const stats = computeSceneStats(root);
    expect(stats.meshes).toBe(2);
    expect(stats.triangles).toBe(24);
    expect(stats.vertices).toBe(24 + 36);
    expect(stats.materials).toBe(2);
    expect(stats.textures).toEqual([{ slot: "map", width: 4, height: 4 }]);
    expect(stats.size.y).toBeCloseTo(1);
    expect(stats.size.x).toBeCloseTo(4.5);
    expect(stats.hierarchy.map((node) => [node.name, node.depth])).toEqual([
      ["Body", 0],
      ["(unnamed)", 0],
      ["(unnamed)", 1],
    ]);
  });

  it("reports an empty scene as zero rather than inventing data", () => {
    const stats = computeSceneStats(new THREE.Group());
    expect(stats).toMatchObject({ meshes: 0, vertices: 0, triangles: 0, materials: 0, textures: [] });
    expect(stats.size).toEqual({ x: 0, y: 0, z: 0 });
  });
});
