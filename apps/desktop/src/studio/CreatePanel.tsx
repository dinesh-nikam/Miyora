import { useCallback, useEffect, useState, type KeyboardEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { studioApi } from "./api";
import { ErrorCallout } from "./ErrorCallout";
import { formatBytes } from "./format";
import type {
  BuiltPrompt,
  ErrorReport,
  ExportOrientation,
  GenerationSettings,
  ImageInspection,
  Preset,
  PromptIntent,
  ProviderInfo,
  TaskView,
} from "./types";

const EMPTY_INTENT: PromptIntent = {
  objectIdentity: "",
  structuralComponents: "",
  materials: "",
  visualStyle: "",
  dimensions: "",
  pose: "",
  details: "",
  topologyIntent: "",
  exclusions: "",
};
const INTENT_FIELDS: { key: keyof PromptIntent; label: string; placeholder: string }[] = [
  { key: "structuralComponents", label: "Structure", placeholder: "lid, iron hinges, side handles" },
  { key: "materials", label: "Materials", placeholder: "weathered oak, wrought iron" },
  { key: "visualStyle", label: "Style", placeholder: "stylized, hand-painted" },
  { key: "dimensions", label: "Scale", placeholder: "about 60 cm wide" },
  { key: "pose", label: "Pose", placeholder: "closed, standing upright" },
  { key: "details", label: "Details", placeholder: "rusty lock, scratches" },
  { key: "topologyIntent", label: "Topology", placeholder: "clean quads, game-ready" },
  { key: "exclusions", label: "Avoid", placeholder: "text, floating parts" },
];
const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "webp"];
const PROMPT_BUILD_DEBOUNCE_MS = 300;
const ORIENTATIONS: ExportOrientation[] = ["+x", "-x", "+y", "-y"];

type Mode = "text" | "image";
type BoolSetting = "texture" | "pbr" | "autoSize" | "quad" | "smartLowPoly" | "exportUv" | "compressGeometry";
/** Documented provider defaults, shown when the user has not overridden them. */
const BOOL_DEFAULTS: Record<BoolSetting, boolean> = {
  texture: true,
  pbr: true,
  exportUv: true,
  autoSize: false,
  quad: false,
  smartLowPoly: false,
  compressGeometry: false,
};

interface CreatePanelProps {
  provider: ProviderInfo;
  providers: ProviderInfo[];
  onProviderChange: (providerId: string) => void;
  presets: Preset[];
  projectId: string | null;
  selectedAsset: { id: string; name: string; versionId: string | null } | null;
  onSubmitted: (task: TaskView) => void;
}

function presetSettings(preset: Preset | undefined): GenerationSettings {
  if (!preset) return {};
  const settings: GenerationSettings = {};
  const { defaults } = preset;
  if (defaults.faceLimit !== null) settings.faceLimit = defaults.faceLimit;
  if (defaults.texture !== null) settings.texture = defaults.texture;
  if (defaults.pbr !== null) settings.pbr = defaults.pbr;
  if (defaults.smartLowPoly !== null) settings.smartLowPoly = defaults.smartLowPoly;
  if (defaults.quad !== null) settings.quad = defaults.quad;
  if (defaults.autoSize !== null) settings.autoSize = defaults.autoSize;
  if (defaults.geometryQuality !== null) settings.geometryQuality = defaults.geometryQuality;
  if (defaults.textureQuality !== null) settings.textureQuality = defaults.textureQuality;
  return settings;
}

/** Keeps only settings the provider honours, so nothing is sent that it would ignore. */
function supportedOnly(settings: GenerationSettings, supported: string[]): GenerationSettings {
  return Object.fromEntries(
    Object.entries(settings).filter(([key]) => supported.includes(key)),
  ) as GenerationSettings;
}

function optionalNumber(value: string): number | undefined {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

export function CreatePanel({ provider, providers, onProviderChange, presets, projectId, selectedAsset, onSubmitted }: CreatePanelProps) {
  const [mode, setMode] = useState<Mode>("text");
  const [presetId, setPresetId] = useState<string>("");
  const [intent, setIntent] = useState<PromptIntent>(EMPTY_INTENT);
  const [built, setBuilt] = useState<BuiltPrompt | null>(null);
  const [prompt, setPrompt] = useState("");
  const [negativePrompt, setNegativePrompt] = useState("");
  const [promptEdited, setPromptEdited] = useState(false);
  const [settings, setSettings] = useState<GenerationSettings>({});
  const [assetName, setAssetName] = useState("");
  const [asNewVersion, setAsNewVersion] = useState(false);
  const [image, setImage] = useState<{ path: string; inspection: ImageInspection } | null>(null);
  const [error, setError] = useState<ErrorReport | null>(null);
  const [busy, setBusy] = useState(false);

  const model = provider.models.find((entry) => entry.id === (settings.model ?? provider.defaultModel));
  const supports = (key: keyof GenerationSettings) => provider.supportedSettings.includes(key);
  const canText = provider.capabilities.includes("textTo3d");
  const advanced = model?.supportsGeometryQuality ?? false;

  // Switching provider drops settings and modes it cannot honour.
  useEffect(() => {
    setSettings((current) => supportedOnly(current, provider.supportedSettings));
    if (!canText) setMode("image");
    setImage(null);
    setError(null);
  }, [provider, canText]);
  const faceCap =
    settings.geometryQuality === "detailed" && model?.maxFacesDetailed ? model.maxFacesDetailed : model?.maxFacesStandard;

  useEffect(() => {
    if (promptEdited || mode !== "text") return;
    if (!intent.objectIdentity.trim()) {
      setBuilt(null);
      setPrompt("");
      setNegativePrompt("");
      return;
    }
    const timer = window.setTimeout(async () => {
      const result = await studioApi.buildPrompt(provider.id, intent, presetId || null);
      if (result.ok) {
        setBuilt(result.data);
        setPrompt(result.data.prompt);
        setNegativePrompt(result.data.negativePrompt);
      }
    }, PROMPT_BUILD_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [intent, presetId, promptEdited, mode, provider.id]);

  const inspect = useCallback(
    async (path: string) => {
      setError(null);
      const result = await studioApi.inspectImage(provider.id, path);
      if (result.ok) {
        setImage({ path, inspection: result.data });
      } else {
        setImage(null);
        setError(result.error);
      }
    },
    [provider.id],
  );

  useEffect(() => {
    if (mode !== "image") return;
    let unlisten: (() => void) | null = null;
    let disposed = false;
    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          if (event.payload.type === "drop" && event.payload.paths.length > 0) {
            void inspect(event.payload.paths[0]);
          }
        })
        .then((stop) => {
          if (disposed) stop();
          else unlisten = stop;
        })
        .catch((reason: unknown) => console.warn("[studio] drag-and-drop unavailable", reason));
    } catch (reason: unknown) {
      // The plain-browser preview harness has no Tauri webview.
      console.warn("[studio] drag-and-drop unavailable", reason);
    }
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [mode, inspect]);

  const chooseImage = async () => {
    const selected = await open({ multiple: false, filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }] });
    if (typeof selected === "string") await inspect(selected);
  };

  const applyPreset = (id: string) => {
    setPresetId(id);
    const preset = presets.find((entry) => entry.id === id);
    setSettings((current) => supportedOnly({ model: current.model, ...presetSettings(preset) }, provider.supportedSettings));
  };

  const update = <K extends keyof GenerationSettings>(key: K, value: GenerationSettings[K]) =>
    setSettings((current) => {
      const next = { ...current };
      if (value === undefined) delete next[key];
      else next[key] = value;
      return next;
    });

  const canSubmit =
    provider.configured &&
    projectId !== null &&
    !busy &&
    (mode === "text" ? prompt.trim().length > 0 && prompt.length <= provider.limits.promptMaxChars : image !== null);

  const submit = async () => {
    if (!projectId || !canSubmit) return;
    setBusy(true);
    setError(null);
    const target = asNewVersion && selectedAsset ? selectedAsset : null;
    const common = {
      projectId,
      assetId: target?.id ?? null,
      assetName: target ? null : assetName.trim() || null,
      provider: provider.id,
      settings: supportedOnly(settings, provider.supportedSettings),
      parentVersionId: target?.versionId ?? null,
    };
    const result =
      mode === "text"
        ? await studioApi.submitText({
            ...common,
            prompt,
            negativePrompt: negativePrompt.trim() || null,
            intent: intent.objectIdentity.trim() ? intent : null,
          })
        : await studioApi.submitImage({ ...common, imagePath: image?.path ?? "" });
    setBusy(false);
    if (result.ok) onSubmitted(result.data);
    else setError(result.error);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      void submit();
    }
  };

  // Cloud defaults come from provider docs; the local engine bakes textures only when asked.
  const boolDefault = (key: BoolSetting) => (supports("pbr") ? BOOL_DEFAULTS[key] : false);

  const checkbox = (key: BoolSetting, label: string, hint?: string) => (
    <label className="studio-check" title={hint}>
      <input type="checkbox" checked={settings[key] ?? boolDefault(key)} onChange={(event) => update(key, event.target.checked)} />
      {label}
    </label>
  );

  return (
    <section className="studio-panel studio-create" aria-label="Create" onKeyDown={onKeyDown}>
      <label className="studio-field">
        <span>Engine</span>
        <select value={provider.id} onChange={(event) => onProviderChange(event.target.value)}>
          {providers.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.label}
              {entry.configured ? "" : entry.requiresApiKey ? " — needs API key" : " — not installed"}
            </option>
          ))}
        </select>
      </label>
      <div className="studio-segmented" role="tablist">
        {(canText ? (["text", "image"] as const) : (["image"] as const)).map((entry) => (
          <button key={entry} role="tab" aria-selected={mode === entry} className={mode === entry ? "active" : ""} onClick={() => setMode(entry)}>
            {entry === "text" ? "Text → 3D" : "Image → 3D"}
          </button>
        ))}
      </div>

      <label className="studio-field">
        <span>Preset</span>
        <select value={presetId} onChange={(event) => applyPreset(event.target.value)}>
          <option value="">None</option>
          {presets.map((preset) => (
            <option key={preset.id} value={preset.id}>
              {preset.label}
            </option>
          ))}
        </select>
      </label>

      {mode === "text" ? (
        <>
          <label className="studio-field">
            <span>Object *</span>
            <textarea
              rows={2}
              value={intent.objectIdentity}
              placeholder="A medieval wooden treasure chest"
              onChange={(event) => setIntent({ ...intent, objectIdentity: event.target.value })}
            />
          </label>
          <details className="studio-details">
            <summary>Structure, materials, style…</summary>
            {INTENT_FIELDS.map((field) => (
              <label key={field.key} className="studio-field">
                <span>{field.label}</span>
                <input
                  value={intent[field.key]}
                  placeholder={field.placeholder}
                  onChange={(event) => setIntent({ ...intent, [field.key]: event.target.value })}
                />
              </label>
            ))}
          </details>
          <label className="studio-field">
            <span>
              Final prompt{" "}
              <small className="studio-muted">
                {prompt.length}/{provider.limits.promptMaxChars}
              </small>
            </span>
            <textarea
              rows={4}
              value={prompt}
              onChange={(event) => {
                setPromptEdited(true);
                setPrompt(event.target.value);
              }}
            />
          </label>
          <label className="studio-field">
            <span>
              Negative prompt{" "}
              <small className="studio-muted">
                {negativePrompt.length}/{provider.limits.negativePromptMaxChars}
              </small>
            </span>
            <input
              value={negativePrompt}
              maxLength={provider.limits.negativePromptMaxChars}
              onChange={(event) => {
                setPromptEdited(true);
                setNegativePrompt(event.target.value);
              }}
            />
          </label>
          {promptEdited && (
            <button className="studio-link" onClick={() => setPromptEdited(false)}>
              Rebuild from fields
            </button>
          )}
          {built?.truncated && <p className="studio-warning">Prompt was shortened to the provider limit.</p>}
          {built?.sanitized && <p className="studio-warning">Emoji and control characters were removed.</p>}
        </>
      ) : (
        <div className="studio-dropzone">
          {image ? (
            <>
              <img src={image.inspection.previewDataUrl} alt={`Preview of ${image.inspection.fileName}`} />
              <p>
                {image.inspection.fileName} · {image.inspection.check.width}×{image.inspection.check.height} ·{" "}
                {image.inspection.check.mime} · {formatBytes(image.inspection.check.byteSize)}
              </p>
              {image.inspection.check.warnings.map((warning) => (
                <p key={warning} className="studio-warning">
                  {warning}
                </p>
              ))}
            </>
          ) : (
            <p className="studio-muted">
              Drop a PNG, JPEG or WebP image here (max {formatBytes(provider.limits.imageMaxBytes)}).
            </p>
          )}
          <button onClick={() => void chooseImage()}>{image ? "Replace image…" : "Choose image…"}</button>
        </div>
      )}

      <details className="studio-details" open>
        <summary>Generation settings</summary>
        <label className="studio-field">
          <span>Model</span>
          <select value={settings.model ?? provider.defaultModel} onChange={(event) => update("model", event.target.value)}>
            {provider.models.map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.label}
              </option>
            ))}
          </select>
        </label>
        {advanced && (
          <label className="studio-field">
            <span>Geometry quality</span>
            <select
              value={settings.geometryQuality ?? ""}
              onChange={(event) => {
                const value = event.target.value;
                update("geometryQuality", value === "standard" || value === "detailed" ? value : undefined);
              }}
            >
              <option value="">Provider default</option>
              <option value="standard">Standard</option>
              <option value="detailed">Detailed (Ultra)</option>
            </select>
          </label>
        )}
        {supports("faceLimit") && (
        <label className="studio-field">
          <span>
            Polygon budget {faceCap ? <small className="studio-muted">max {faceCap.toLocaleString("en-US")}</small> : null}
          </span>
          <input
            type="number"
            min={1}
            max={faceCap}
            placeholder="Adaptive"
            value={settings.faceLimit ?? ""}
            onChange={(event) => update("faceLimit", optionalNumber(event.target.value))}
          />
        </label>
        )}
        <div className="studio-checks">
          {supports("texture") && checkbox("texture", supports("pbr") ? "Texture" : "Bake UV texture (slower)")}
          {supports("pbr") && checkbox("pbr", "PBR materials")}
          {supports("exportUv") && checkbox("exportUv", "UV unwrap")}
          {advanced && checkbox("autoSize", "Real-world scale")}
          {advanced && checkbox("quad", "Quad topology", "Forces FBX output")}
          {advanced && checkbox("smartLowPoly", "Smart low-poly")}
          {advanced && checkbox("compressGeometry", "Meshopt compression")}
        </div>
        {advanced && (
          <label className="studio-field">
            <span>Texture quality</span>
            <select
              value={settings.textureQuality ?? ""}
              onChange={(event) => {
                const value = event.target.value;
                update("textureQuality", value === "standard" || value === "detailed" || value === "extreme" ? value : undefined);
              }}
            >
              <option value="">Provider default</option>
              <option value="standard">Standard</option>
              <option value="detailed">Detailed</option>
              <option value="extreme">Extreme</option>
            </select>
          </label>
        )}
        {supports("exportOrientation") && (
        <label className="studio-field">
          <span>Export orientation</span>
          <select
            value={settings.exportOrientation ?? ""}
            onChange={(event) => update("exportOrientation", ORIENTATIONS.find((entry) => entry === event.target.value))}
          >
            <option value="">Provider default (+X)</option>
            <option value="+x">+X forward</option>
            <option value="-x">−X forward</option>
            <option value="+y">+Y forward</option>
            <option value="-y">−Y forward</option>
          </select>
        </label>
        )}
        {supports("modelSeed") && (
        <label className="studio-field">
          <span>
            Model seed <small className="studio-muted">blank = random, recorded</small>
          </span>
          <input type="number" min={0} value={settings.modelSeed ?? ""} onChange={(event) => update("modelSeed", optionalNumber(event.target.value))} />
        </label>
        )}
      </details>

      {selectedAsset && (
        <label className="studio-check">
          <input type="checkbox" checked={asNewVersion} onChange={(event) => setAsNewVersion(event.target.checked)} />
          Save as new version of “{selectedAsset.name}”
        </label>
      )}
      {!(asNewVersion && selectedAsset) && (
        <label className="studio-field">
          <span>Asset name</span>
          <input value={assetName} placeholder="Derived from prompt or file name" onChange={(event) => setAssetName(event.target.value)} />
        </label>
      )}

      {!provider.configured && (
        <p className="studio-warning">
          {provider.requiresApiKey ? `Add a ${provider.label} API key in Settings to generate.` : provider.unavailableReason}
        </p>
      )}
      {provider.configured && !provider.requiresApiKey && provider.acceptsApiKey && !provider.hasApiKey && (
        <p className="studio-muted">Running anonymously with a small free quota. Add a free Hugging Face token in Settings for more.</p>
      )}
      {error && <ErrorCallout error={error} title="Generation was not submitted" />}
      <button className="studio-primary" disabled={!canSubmit} onClick={() => void submit()}>
        {busy ? "Submitting…" : "Generate"} <kbd>Ctrl+Enter</kbd>
      </button>
      <p className="studio-muted">Uses provider credits. The task appears in the queue once the provider accepts it.</p>
    </section>
  );
}
