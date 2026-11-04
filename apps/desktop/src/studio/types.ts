// Mirrors the serde shapes in crates/studio. Keep in sync with Rust.

export type TaskState = "QUEUED" | "RUNNING" | "SUCCEEDED" | "FAILED" | "CANCELLED" | "EXPIRED";
export type GeometryQuality = "standard" | "detailed";
export type TextureQuality = "standard" | "detailed" | "extreme";
export type ExportOrientation = "+x" | "-x" | "+y" | "-y";

export interface ErrorReport {
  kind: string;
  message: string;
  suggestion: string;
  requestId: string | null;
  providerCode: number | null;
}

export interface ModelInfo {
  id: string;
  label: string;
  source: string;
  supportsGeometryQuality: boolean;
  maxFacesStandard: number;
  maxFacesDetailed: number | null;
}

export interface ProviderLimits {
  promptMaxChars: number;
  negativePromptMaxChars: number;
  imageMaxBytes: number;
  quadMaxFaces: number;
}

export interface ProviderInfo {
  id: string;
  label: string;
  requiresApiKey: boolean;
  acceptsApiKey: boolean;
  hasApiKey: boolean;
  /** Null when usable; otherwise why not (e.g. local engine not installed). */
  unavailableReason: string | null;
  /** GenerationSettings keys this provider honours; others must not be sent. */
  supportedSettings: string[];
  /** Ready to generate: key stored (cloud) or engine installed (local). */
  configured: boolean;
  capabilities: string[];
  models: ModelInfo[];
  defaultModel: string;
  limits: ProviderLimits;
}

export interface PresetDefaults {
  faceLimit: number | null;
  texture: boolean | null;
  pbr: boolean | null;
  smartLowPoly: boolean | null;
  quad: boolean | null;
  autoSize: boolean | null;
  geometryQuality: GeometryQuality | null;
  textureQuality: TextureQuality | null;
}

export interface Preset {
  id: string;
  label: string;
  styleHint: string;
  topologyHint: string;
  negativeHint: string;
  defaults: PresetDefaults;
}

export interface PollPolicy {
  intervalSecs: number;
  maxIntervalSecs: number;
  timeoutSecs: number;
  maxConsecutiveErrors: number;
}

export interface StudioOverview {
  providers: ProviderInfo[];
  presets: Preset[];
  pollPolicy: PollPolicy;
}

export interface GenerationSettings {
  model?: string;
  modelSeed?: number;
  textureSeed?: number;
  imageSeed?: number;
  faceLimit?: number;
  texture?: boolean;
  pbr?: boolean;
  textureQuality?: TextureQuality;
  geometryQuality?: GeometryQuality;
  autoSize?: boolean;
  quad?: boolean;
  smartLowPoly?: boolean;
  exportUv?: boolean;
  compressGeometry?: boolean;
  exportOrientation?: ExportOrientation;
  /** Recorded by local retopology. */
  sourceTriangles?: number;
  resultTriangles?: number;
}

export interface PromptIntent {
  objectIdentity: string;
  structuralComponents: string;
  materials: string;
  visualStyle: string;
  dimensions: string;
  pose: string;
  details: string;
  topologyIntent: string;
  exclusions: string;
}

export interface BuiltPrompt {
  prompt: string;
  negativePrompt: string;
  truncated: boolean;
  sanitized: boolean;
}

export interface ImageCheck {
  mime: string;
  extension: string;
  width: number;
  height: number;
  byteSize: number;
  sha256: string;
  warnings: string[];
}

export interface ImageInspection {
  fileName: string;
  check: ImageCheck;
  previewDataUrl: string;
}

export interface AccountBalance {
  available: number;
  frozen: number;
}

export interface ProjectView {
  id: string;
  name: string;
  createdAt: string;
  assetCount: number;
}

export interface AssetSummary {
  id: string;
  projectId: string;
  name: string;
  createdAt: string;
  versionCount: number;
  latestVersionId: string | null;
  thumbnailOutputId: number | null;
}

export interface TaskView {
  id: string;
  generationId: string;
  assetId: string;
  assetName: string;
  provider: string;
  providerTaskId: string;
  operation: string;
  state: TaskState;
  stage: string | null;
  progress: number | null;
  createdAt: string;
  startedAt: string | null;
  completedAt: string | null;
  updatedAt: string;
  attempts: number;
  error: ErrorReport | null;
  canResume: boolean;
  versionId: string | null;
}

export interface PbrChannels {
  baseColor: boolean;
  metallicRoughness: boolean;
  normal: boolean;
  occlusion: boolean;
  emissive: boolean;
}

export interface ModelReport {
  format: string;
  byteSize: number;
  meshes: number | null;
  vertices: number | null;
  triangles: number | null;
  materials: number | null;
  textures: number | null;
  images: number | null;
  animations: number | null;
  skins: number | null;
  hasUv: boolean | null;
  hasVertexColors: boolean | null;
  pbr: PbrChannels | null;
  extensionsRequired: string[];
}

export interface OutputView {
  id: number;
  kind: "model" | "preview_image";
  label: string;
  format: string;
  byteSize: number;
  sha256: string;
  report: ModelReport | null;
}

export interface InputView {
  role: string;
  originalName: string;
  mime: string;
  width: number;
  height: number;
  byteSize: number;
  sha256: string;
}

export type LocalExportFormat = "original" | "obj" | "stl" | "ply";

export interface ExportOptions {
  scale: number;
  zUp: boolean;
}

export interface ExportView {
  id: string;
  format: LocalExportFormat;
  destination: string;
  files: string[];
  byteSize: number;
  triangles: number | null;
  createdAt: string;
}

export interface GenerationView {
  id: string;
  operation: string;
  provider: string;
  model: string;
  prompt: string | null;
  negativePrompt: string | null;
  intent: PromptIntent | null;
  settings: GenerationSettings;
  parentVersionId: string | null;
  createdAt: string;
  inputs: InputView[];
  outputs: OutputView[];
  task: TaskView | null;
  usage: { credits: number; source: "provider" | "estimated" } | null;
  versionId: string | null;
  versionNumber: number | null;
  exports: ExportView[];
}

export interface AssetDetail {
  id: string;
  projectId: string;
  name: string;
  createdAt: string;
  generations: GenerationView[];
}

export interface TextSubmission {
  projectId: string;
  assetId: string | null;
  assetName: string | null;
  provider: string;
  prompt: string;
  negativePrompt: string | null;
  intent: PromptIntent | null;
  settings: GenerationSettings;
  parentVersionId: string | null;
}

export interface ImageSubmission {
  projectId: string;
  assetId: string | null;
  assetName: string | null;
  provider: string;
  imagePath: string;
  settings: GenerationSettings;
  parentVersionId: string | null;
}
