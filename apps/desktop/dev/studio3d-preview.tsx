// [SIMULATED - DEV ONLY] Design harness for the 3D Studio UI in a plain browser.
// Not part of `vite build` (only index.html is an entry) and never shipped.
// Mocks read-only studio IPC; the viewer loads a real local GLB from output/.
// No provider is contacted and no task here is real.
import glbUrl from "../../output/anime-rigged-catalog-v1/ember/model.glb?url";
import type { AssetDetail, StudioOverview, TaskView } from "../src/studio/types";

// `?model=<name>` loads output/smoke/<name>.glb (real engine results copied there for review).
const smokeModels = import.meta.glob<string>("../../output/smoke/*.glb", { query: "?url", import: "default", eager: true });
const requested = new URLSearchParams(window.location.search).get("model");
const modelUrl =
  Object.entries(smokeModels).find(([path]) => requested !== null && path.endsWith(`/${requested}.glb`))?.[1] ?? glbUrl;
const NOW = new Date().toISOString();
const overview: StudioOverview = {
  providers: [
    {
      id: "tripo",
      label: "Tripo API (cloud, paid credits)",
      requiresApiKey: true,
      acceptsApiKey: true,
      hasApiKey: false,
      unavailableReason: null,
      supportedSettings: ["model", "modelSeed", "textureSeed", "imageSeed", "faceLimit", "texture", "pbr", "textureQuality", "geometryQuality", "autoSize", "quad", "smartLowPoly", "exportUv", "compressGeometry", "exportOrientation"],
      configured: false,
      capabilities: ["textTo3d", "imageTo3d", "taskStatus", "upload", "accountBalance"],
      models: [
        { id: "v3.1-20260211", label: "v3.1 (latest, best quality)", source: "documentation", supportsGeometryQuality: true, maxFacesStandard: 1_500_000, maxFacesDetailed: 2_000_000 },
        { id: "v2.5-20250123", label: "v2.5 (balanced)", source: "documentation", supportsGeometryQuality: false, maxFacesStandard: 500_000, maxFacesDetailed: null },
      ],
      defaultModel: "v3.1-20260211",
      limits: { promptMaxChars: 1024, negativePromptMaxChars: 255, imageMaxBytes: 20 * 1024 * 1024, quadMaxFaces: 150_000 },
    },
    {
      id: "local-triposr",
      label: "Local TripoSR (free, offline)",
      requiresApiKey: false,
      acceptsApiKey: false,
      hasApiKey: false,
      unavailableReason: null,
      supportedSettings: ["model", "texture"],
      configured: true,
      capabilities: ["imageTo3d", "taskStatus", "upload"],
      models: [{ id: "triposr-1.0", label: "TripoSR 1.0 (local)", source: "local engine", supportsGeometryQuality: false, maxFacesStandard: 0, maxFacesDetailed: null }],
      defaultModel: "triposr-1.0",
      limits: { promptMaxChars: 0, negativePromptMaxChars: 0, imageMaxBytes: 20 * 1024 * 1024, quadMaxFaces: 0 },
    },
  ],
  presets: [
    { id: "low-poly", label: "Low Poly", styleHint: "stylized low-poly", topologyHint: "minimal faceted geometry", negativeHint: "high-frequency detail", defaults: { faceLimit: 5000, texture: null, pbr: null, smartLowPoly: true, quad: null, autoSize: null, geometryQuality: null, textureQuality: null } },
  ],
  pollPolicy: { intervalSecs: 3, maxIntervalSecs: 30, timeoutSecs: 1800, maxConsecutiveErrors: 6 },
};
const task: TaskView = {
  id: "sim-task", generationId: "sim-gen", assetId: "sim-asset", assetName: "[SIMULATED] Ember", provider: "simulated",
  providerTaskId: "SIMULATED-DEV-ONLY", operation: "text_to_3d", state: "SUCCEEDED", stage: null, progress: 100,
  createdAt: NOW, startedAt: NOW, completedAt: NOW, updatedAt: NOW, attempts: 1, error: null, canResume: false, versionId: "sim-v1",
};
const detail: AssetDetail = {
  id: "sim-asset", projectId: "sim-project", name: "[SIMULATED] Ember", createdAt: NOW,
  generations: [{
    id: "sim-gen", operation: "text_to_3d", provider: "simulated", model: "local-file", prompt: "[SIMULATED - DEV ONLY] local GLB from output/",
    negativePrompt: null, intent: null, settings: {}, parentVersionId: null, createdAt: NOW, inputs: [],
    outputs: [{ id: 1, kind: "model", label: "model_url", format: "glb", byteSize: 0, sha256: "", report: null }],
    task, usage: null, versionId: "sim-v1", versionNumber: 1, exports: [],
  }],
};
const responses: Record<string, () => unknown> = {
  studio_overview: () => overview,
  studio_list_projects: () => [{ id: "sim-project", name: "[SIMULATED] Harness", createdAt: NOW, assetCount: 1 }],
  studio_list_assets: () => [{ id: "sim-asset", projectId: "sim-project", name: "[SIMULATED] Ember", createdAt: NOW, versionCount: 1, latestVersionId: "sim-v1", thumbnailOutputId: null }],
  studio_asset_detail: () => detail,
  studio_list_tasks: () => [task],
};
(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  invoke: async (cmd: string) => {
    if (cmd === "studio_read_output") return (await fetch(modelUrl)).arrayBuffer();
    const respond = responses[cmd];
    if (respond) return respond();
    throw new Error(JSON.stringify({ kind: "simulated", message: `${cmd} is not available in the dev harness.`, suggestion: "Run the Tauri app for real provider calls.", requestId: null, providerCode: null }));
  },
  transformCallback: () => 0,
};
const [{ createRoot }, { StudioView }] = await Promise.all([import("react-dom/client"), import("../src/studio/StudioView")]);
createRoot(document.getElementById("root") as HTMLElement).render(
  <>
    <p style={{ color: "#fbbf24", font: "600 13px system-ui", margin: "0 0 8px" }}>[SIMULATED - DEV ONLY] UI harness: no provider calls, data below is mocked.</p>
    <StudioView />
  </>,
);
