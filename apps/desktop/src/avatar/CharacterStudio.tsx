import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invokeSafe } from "../utils/ipc";
import type { PetState } from "../components/CompanionRenderer";
import { AvatarStage } from "./AvatarStage";
import { renderThumbnail } from "./thumbnails";
import {
  ACCESSORIES,
  BODY_TYPES,
  EAR_TYPES,
  EYE_STYLES,
  HAIR_STYLES,
  OUTFITS,
  PALETTE_KEYS,
  PATTERNS,
  SPEC_LIMITS,
  TAIL_TYPES,
  WING_TYPES,
  type Accessory,
  type AvatarCatalogEntry,
  type AvatarHud,
  type AvatarSource,
  type CharacterSpec,
  type CustomAvatarSummary,
  type ResolvedAvatar,
  type StageSubject,
} from "./types";

export interface CharacterStudioProps {
  liveState: PetState;
  hud: AvatarHud | null;
  activeAvatar: ResolvedAvatar | null;
  onAvatarChanged: (avatar: ResolvedAvatar) => void;
}

type StudioTab = "library" | "customize" | "import";
type Viewing = { source: AvatarSource; id: string };

const PREVIEW_STATES: PetState[] = [
  "idle",
  "working",
  "thinking",
  "waiting",
  "attention_required",
  "success",
  "celebrating",
  "level_up",
  "achievement",
  "error",
  "hungry",
  "eating",
  "sleeping",
  "paused",
  "offline",
];
const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "webp", "gif"];
const MODEL_EXTENSIONS = ["vrm", "glb"];

const panel: CSSProperties = {
  background: "#0c1322",
  border: "1px solid rgba(255, 255, 255, 0.08)",
  borderRadius: "12px",
  padding: "16px",
};
const label: CSSProperties = { fontSize: "11px", color: "#94a3b8", display: "flex", flexDirection: "column", gap: "4px" };
const input: CSSProperties = {
  background: "#0a101d",
  border: "1px solid rgba(148, 163, 184, 0.25)",
  borderRadius: "6px",
  color: "#e2e8f0",
  padding: "6px 8px",
  fontSize: "12px",
};

function button(kind: "primary" | "ghost" | "danger", disabled = false): CSSProperties {
  const palette = {
    primary: { bg: "rgba(56, 189, 248, 0.18)", border: "rgba(56, 189, 248, 0.5)", color: "#7dd3fc" },
    ghost: { bg: "transparent", border: "rgba(148, 163, 184, 0.3)", color: "#cbd5e1" },
    danger: { bg: "rgba(248, 113, 113, 0.12)", border: "rgba(248, 113, 113, 0.45)", color: "#fca5a5" },
  }[kind];
  return {
    background: palette.bg,
    border: `1px solid ${palette.border}`,
    color: palette.color,
    padding: "6px 12px",
    borderRadius: "6px",
    fontSize: "12px",
    fontWeight: 600,
    cursor: disabled ? "not-allowed" : "pointer",
    opacity: disabled ? 0.5 : 1,
  };
}

function humanize(value: string): string {
  return value.replace(/_/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function SpecThumbnail({ spec, size = 84 }: { spec: CharacterSpec; size?: number }) {
  const ref = useRef<HTMLDivElement>(null);
  const [url, setUrl] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let cancelled = false;
    const start = () => {
      renderThumbnail(spec)
        .then((result) => {
          if (!cancelled) setUrl(result);
        })
        .catch((error: unknown) => {
          console.error("[avatar] thumbnail failed", error);
          if (!cancelled) setFailed(true);
        });
    };
    const node = ref.current;
    if (!node || typeof IntersectionObserver === "undefined") {
      start();
      return () => {
        cancelled = true;
      };
    }
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        observer.disconnect();
        start();
      }
    });
    observer.observe(node);
    return () => {
      cancelled = true;
      observer.disconnect();
    };
  }, [spec]);
  return (
    <div ref={ref} style={{ width: size, height: size, display: "flex", alignItems: "center", justifyContent: "center" }}>
      {url ? (
        <img src={url} alt={spec.name} width={size} height={size} style={{ objectFit: "contain" }} />
      ) : (
        <span style={{ fontSize: "10px", color: failed ? "#f87171" : "#475569" }}>{failed ? "Render failed" : "Rendering..."}</span>
      )}
    </div>
  );
}

const ANIME_TAG = "anime";
const ANIME_COLLECTION = "collection:anime";
const BADGE_STYLE: CSSProperties = {
  fontSize: "9px",
  fontWeight: 700,
  letterSpacing: "0.04em",
  padding: "1px 6px",
  borderRadius: "999px",
  textTransform: "uppercase",
};

interface CharacterCardProps {
  spec: CharacterSpec;
  selected: boolean;
  status: "active" | "customized" | null;
  onSelect: () => void;
}

/** A catalog card themed from the character's own palette. */
function CharacterCard({ spec, selected, status, onSelect }: CharacterCardProps) {
  const { palette } = spec;
  const anime = spec.tags.includes(ANIME_TAG);
  const style = {
    "--card-accent": palette.accent,
    "--card-tint": `${palette.primary}40`,
  } as CSSProperties;
  return (
    <button
      type="button"
      className={`character-card${selected ? " is-selected" : ""}`}
      style={style}
      onClick={onSelect}
      aria-pressed={selected}
      aria-label={`${spec.name}, ${spec.title}${status ? `, ${status}` : ""}`}
      title={spec.description}
    >
      <SpecThumbnail spec={spec} />
      <span className="character-card__name">{spec.name}</span>
      <span className="character-card__title">{spec.title}</span>
      <span
        className="character-card__swatches"
        aria-label={`Palette — hair: ${palette.hair}, eyes: ${palette.eyes}, primary: ${palette.primary}, accent: ${palette.accent}`}
      >
        {[palette.hair, palette.eyes, palette.primary, palette.accent].map((color, index) => (
          <span key={index} style={{ background: color }} aria-hidden="true" />
        ))}
      </span>
      <span className="character-card__badges">
        {anime && <span style={{ ...BADGE_STYLE, background: "rgba(244, 114, 182, 0.18)", color: "#f9a8d4" }}>Anime</span>}
        <span style={{ ...BADGE_STYLE, background: "rgba(148, 163, 184, 0.14)", color: "#cbd5e1" }}>{humanize(spec.body)}</span>
        {status === "active" && <span style={{ ...BADGE_STYLE, background: "rgba(56, 189, 248, 0.2)", color: "#7dd3fc" }}>Active</span>}
        {status === "customized" && <span style={{ ...BADGE_STYLE, background: "rgba(251, 191, 36, 0.18)", color: "#fcd34d" }}>Edited</span>}
      </span>
    </button>
  );
}

function Field({ title, children }: { title: string; children: ReactNode }) {
  return (
    <label style={label}>
      {title}
      {children}
    </label>
  );
}

function SelectField<T extends string>({ title, value, options, onChange }: { title: string; value: T; options: readonly T[]; onChange: (value: T) => void }) {
  return (
    <Field title={title}>
      <select style={input} value={value} onChange={(event) => onChange(event.target.value as T)}>
        {options.map((option) => (
          <option key={option} value={option}>
            {humanize(option)}
          </option>
        ))}
      </select>
    </Field>
  );
}

function SliderField({ title, value, range, onChange }: { title: string; value: number; range: readonly [number, number]; onChange: (value: number) => void }) {
  return (
    <Field title={`${title}: ${value.toFixed(2)}`}>
      <input type="range" min={range[0]} max={range[1]} step={0.01} value={value} onChange={(event) => onChange(Number(event.target.value))} />
    </Field>
  );
}

export function CharacterStudio({ liveState, hud, activeAvatar, onAvatarChanged }: CharacterStudioProps) {
  const [tab, setTab] = useState<StudioTab>("library");
  const [catalog, setCatalog] = useState<AvatarCatalogEntry[]>([]);
  const [customs, setCustoms] = useState<CustomAvatarSummary[]>([]);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [viewing, setViewing] = useState<Viewing | null>(null);
  const [draft, setDraft] = useState<CharacterSpec | null>(null);
  const [search, setSearch] = useState("");
  const [bodyFilter, setBodyFilter] = useState<string>("all");
  const [previewState, setPreviewState] = useState<PetState | "live">("live");
  const [status, setStatus] = useState<{ tone: "info" | "error"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [imageName, setImageName] = useState("My Character");

  const reportError = useCallback((text: string) => setStatus({ tone: "error", text }), []);

  const refresh = useCallback(async () => {
    const [catalogResult, customResult] = await Promise.all([
      invokeSafe<AvatarCatalogEntry[]>("list_avatar_catalog"),
      invokeSafe<CustomAvatarSummary[]>("list_custom_avatars"),
    ]);
    if (catalogResult.ok) setCatalog(catalogResult.data);
    else reportError(`Could not load the character library: ${catalogResult.error}`);
    if (customResult.ok) {
      setCustoms(customResult.data);
      const sculpts = customResult.data.filter((avatar) => avatar.kind === "image_sculpt");
      const loaded = await Promise.all(
        sculpts.map(async (avatar) => {
          const preview = await invokeSafe<string | null>("get_avatar_preview", { avatarId: avatar.avatarId });
          return [avatar.avatarId, preview.ok ? preview.data : null] as const;
        }),
      );
      setPreviews(Object.fromEntries(loaded.filter((entry): entry is readonly [string, string] => entry[1] !== null)));
    } else {
      reportError(`Could not load your custom characters: ${customResult.error}`);
    }
  }, [reportError]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Start by viewing whatever the companion currently is.
  useEffect(() => {
    if (!viewing && activeAvatar) setViewing({ source: activeAvatar.source, id: activeAvatar.avatarId });
  }, [activeAvatar, viewing]);

  const viewedCatalog = viewing?.source === "catalog" ? catalog.find((entry) => entry.spec.id === viewing.id) ?? null : null;
  const viewedCustom = viewing?.source === "custom" ? customs.find((avatar) => avatar.avatarId === viewing.id) ?? null : null;
  const baseSpec = viewedCatalog?.spec ?? viewedCustom?.spec ?? null;

  const baseJson = baseSpec ? JSON.stringify(baseSpec) : "";
  const viewKey = viewing ? `${viewing.source}:${viewing.id}` : "";
  const dirty = draft !== null && JSON.stringify(draft) !== baseJson;
  const synced = useRef({ viewKey: "", baseJson: "" });
  const draftRef = useRef(draft);
  draftRef.current = draft;

  // Load the base spec into the editor when the viewed character changes, or
  // when its saved version changes while the editor holds no unsaved edits.
  // Library refreshes rebuild objects, so compare by content, not identity.
  useEffect(() => {
    const previous = synced.current;
    const current = draftRef.current;
    const untouched = current === null || JSON.stringify(current) === previous.baseJson;
    if (viewKey !== previous.viewKey || untouched) {
      setDraft(baseJson ? (JSON.parse(baseJson) as CharacterSpec) : null);
    }
    synced.current = { viewKey, baseJson };
  }, [viewKey, baseJson]);

  const subject: StageSubject | null = useMemo(() => {
    if (draft) return { kind: "procedural", spec: draft };
    if (viewedCustom && viewedCustom.kind !== "procedural" && viewedCustom.modelSha256) {
      return { kind: viewedCustom.kind, avatarId: viewedCustom.avatarId, modelSha256: viewedCustom.modelSha256, name: viewedCustom.name };
    }
    return null;
  }, [draft, viewedCustom]);

  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    return catalog.filter(({ spec }) => {
      if (bodyFilter === ANIME_COLLECTION) {
        if (!spec.tags.includes(ANIME_TAG)) return false;
      } else if (bodyFilter !== "all" && spec.body !== bodyFilter) {
        return false;
      }
      if (!query) return true;
      return [spec.name, spec.title, spec.description, ...spec.tags].some((text) => text.toLowerCase().includes(query));
    });
  }, [catalog, search, bodyFilter]);

  const update = (patch: Partial<CharacterSpec>) => {
    setDraft((current) => (current ? { ...current, ...patch } : current));
  };

  const run = async (work: () => Promise<void>) => {
    setBusy(true);
    setStatus(null);
    try {
      await work();
    } finally {
      setBusy(false);
    }
  };

  const useAsCompanion = () =>
    run(async () => {
      if (!viewing) return;
      if (dirty) {
        reportError("Save or revert your changes before using this character.");
        return;
      }
      const result = await invokeSafe<ResolvedAvatar>("select_avatar", { source: viewing.source, avatarId: viewing.id });
      if (!result.ok) return reportError(result.error);
      onAvatarChanged(result.data);
      setStatus({ tone: "info", text: `${result.data.name} is now your companion.` });
    });

  const saveDraft = () =>
    run(async () => {
      if (!draft) return;
      const result = await invokeSafe<null>("save_avatar_customization", { spec: draft });
      if (!result.ok) return reportError(result.error);
      await refresh();
      if (activeAvatar?.avatarId === draft.id) {
        const live = await invokeSafe<ResolvedAvatar>("get_avatar");
        if (live.ok) onAvatarChanged(live.data);
      }
      setStatus({ tone: "info", text: "Customization saved." });
    });

  const saveAsNew = () =>
    run(async () => {
      if (!draft) return;
      const result = await invokeSafe<CustomAvatarSummary>("create_custom_character", { spec: draft });
      if (!result.ok) return reportError(result.error);
      await refresh();
      setViewing({ source: "custom", id: result.data.avatarId });
      setStatus({ tone: "info", text: `Saved as a new character: ${result.data.name}.` });
    });

  const resetCustomization = () =>
    run(async () => {
      if (!viewedCatalog) return;
      const result = await invokeSafe<boolean>("reset_avatar_customization", { catalogId: viewedCatalog.spec.id });
      if (!result.ok) return reportError(result.error);
      await refresh();
      if (activeAvatar?.avatarId === viewedCatalog.spec.id) {
        const live = await invokeSafe<ResolvedAvatar>("get_avatar");
        if (live.ok) onAvatarChanged(live.data);
      }
      setStatus({ tone: "info", text: "Restored the original design." });
    });

  const deleteCustom = (avatar: CustomAvatarSummary) =>
    run(async () => {
      if (!window.confirm(`Delete "${avatar.name}"? Its files are removed from this computer and cannot be recovered.`)) return;
      const result = await invokeSafe<null>("delete_custom_avatar", { avatarId: avatar.avatarId });
      if (!result.ok) return reportError(result.error);
      if (viewing?.id === avatar.avatarId) setViewing(null);
      await refresh();
      const live = await invokeSafe<ResolvedAvatar>("get_avatar");
      if (live.ok) onAvatarChanged(live.data);
      setStatus({ tone: "info", text: `Deleted ${avatar.name}.` });
    });

  const importImage = () =>
    run(async () => {
      const selected = await open({ multiple: false, directory: false, filters: [{ name: "Image", extensions: IMAGE_EXTENSIONS }] });
      if (!selected || typeof selected !== "string") return;
      setStatus({ tone: "info", text: "Sculpting your image into 3D..." });
      const result = await invokeSafe<CustomAvatarSummary[]>("create_avatar_from_image", { imagePath: selected, name: imageName });
      if (!result.ok) return reportError(result.error);
      await refresh();
      const sculpt = result.data[0];
      if (sculpt) setViewing({ source: "custom", id: sculpt.avatarId });
      setTab("library");
      const triangles = sculpt?.triangleCount ?? 0;
      setStatus({ tone: "info", text: `Created a 3D sculpt (${triangles.toLocaleString()} triangles) and a styled character from your image.` });
    });

  const importModel = () =>
    run(async () => {
      const selected = await open({ multiple: false, directory: false, filters: [{ name: "VRM avatar", extensions: MODEL_EXTENSIONS }] });
      if (!selected || typeof selected !== "string") return;
      const result = await invokeSafe<CustomAvatarSummary>("import_vrm_avatar", { vrmPath: selected });
      if (!result.ok) return reportError(result.error);
      await refresh();
      setViewing({ source: "custom", id: result.data.avatarId });
      setTab("library");
      setStatus({ tone: "info", text: `Imported ${result.data.name}.` });
    });

  const isActive = (source: AvatarSource, id: string) => activeAvatar?.source === source && activeAvatar.avatarId === id;
  const vrmMeta = viewedCustom?.kind === "vrm" ? viewedCustom.metadata : null;
  const metaText = (key: string) => {
    const value = vrmMeta?.[key];
    return typeof value === "string" || typeof value === "number" || typeof value === "boolean" ? String(value) : "not declared";
  };

  return (
    <div style={{ display: "grid", gridTemplateColumns: "minmax(280px, 320px) 1fr", gap: "16px", alignItems: "start", minWidth: 0 }}>
      {/* Preview */}
      <div style={{ ...panel, display: "flex", flexDirection: "column", alignItems: "center", gap: "10px", position: "sticky", top: 0 }}>
        <div style={{ alignSelf: "stretch" }}>
          <div style={{ fontSize: "16px", fontWeight: 700, color: "#e2e8f0" }}>{draft?.name ?? viewedCustom?.name ?? "Select a character"}</div>
          <div style={{ fontSize: "11px", color: "#64748b" }}>
            {draft?.title ?? (viewedCustom?.kind === "image_sculpt" ? "3D sculpt from your image" : viewedCustom?.kind === "vrm" ? "Imported VRM model" : "")}
          </div>
        </div>
        <AvatarStage subject={subject} state={previewState === "live" ? liveState : previewState} hud={hud} size={300} onError={reportError} />
        <Field title="Pose preview (visual only; agent data is unchanged)">
          <select style={input} value={previewState} onChange={(event) => setPreviewState(event.target.value as PetState | "live")}>
            <option value="live">Live agent state ({humanize(liveState)})</option>
            {PREVIEW_STATES.map((state) => (
              <option key={state} value={state}>
                {humanize(state)}
              </option>
            ))}
          </select>
        </Field>
        {vrmMeta && (
          <div style={{ alignSelf: "stretch", fontSize: "11px", color: "#94a3b8", lineHeight: 1.6 }}>
            <div>Author: {metaText("author")}</div>
            <div>License: {metaText("licenseName")}</div>
            <div>Allowed user: {metaText("allowedUser")}</div>
            <div>Triangles: {metaText("triangleCount")}</div>
          </div>
        )}
        {viewedCustom?.kind === "image_sculpt" && viewedCustom.triangleCount !== null && (
          <div style={{ alignSelf: "stretch", fontSize: "11px", color: "#94a3b8" }}>
            Mesh: {viewedCustom.triangleCount.toLocaleString()} triangles, inflated from the image silhouette.
          </div>
        )}
        <div style={{ display: "flex", flexWrap: "wrap", gap: "8px", alignSelf: "stretch" }}>
          <button style={button("primary", busy || !viewing)} disabled={busy || !viewing} onClick={() => void useAsCompanion()}>
            {viewing && isActive(viewing.source, viewing.id) ? "Active companion" : "Use as companion"}
          </button>
          {draft && (
            <>
              <button style={button("ghost", busy || !dirty)} disabled={busy || !dirty} onClick={() => void saveDraft()}>
                Save changes
              </button>
              <button style={button("ghost", busy)} disabled={busy} onClick={() => void saveAsNew()}>
                Save as new
              </button>
              {dirty && (
                <button style={button("ghost", busy)} disabled={busy} onClick={() => setDraft(baseJson ? (JSON.parse(baseJson) as CharacterSpec) : null)}>
                  Revert
                </button>
              )}
              {viewedCatalog?.customized && !dirty && (
                <button style={button("danger", busy)} disabled={busy} onClick={() => void resetCustomization()}>
                  Restore original
                </button>
              )}
            </>
          )}
        </div>
        {status && (
          <div role="status" style={{ alignSelf: "stretch", fontSize: "12px", color: status.tone === "error" ? "#fca5a5" : "#7dd3fc" }}>
            {status.text}
          </div>
        )}
      </div>

      {/* Tabs */}
      <div style={{ display: "flex", flexDirection: "column", gap: "12px", minWidth: 0, overflow: "hidden" }}>
        <div
          role="tablist"
          aria-label="Character Studio modes"
          style={{
            display: "inline-flex",
            alignSelf: "flex-start",
            gap: "6px",
            background: "#0c1322",
            padding: "4px",
            borderRadius: "8px",
            border: "1px solid rgba(255, 255, 255, 0.08)",
          }}
        >
          {(["library", "customize", "import"] as StudioTab[]).map((name) => {
            const isSelected = tab === name;
            return (
              <button
                key={name}
                role="tab"
                aria-selected={isSelected}
                tabIndex={isSelected ? 0 : -1}
                style={{
                  background: isSelected ? "rgba(56, 189, 248, 0.2)" : "transparent",
                  border: isSelected ? "1px solid rgba(56, 189, 248, 0.55)" : "1px solid transparent",
                  color: isSelected ? "#7dd3fc" : "#94a3b8",
                  padding: "6px 14px",
                  borderRadius: "6px",
                  fontSize: "12px",
                  fontWeight: isSelected ? 700 : 600,
                  cursor: "pointer",
                  transition: "background 0.15s ease, color 0.15s ease",
                }}
                onClick={() => setTab(name)}
              >
                {name === "library" ? `Library (${catalog.length + customs.length})` : humanize(name)}
              </button>
            );
          })}
        </div>

        {tab === "library" && (
          <>
            {customs.length > 0 && (
              <div style={panel}>
                <div style={{ fontSize: "13px", fontWeight: 700, color: "#e2e8f0", marginBottom: "10px" }}>Your characters</div>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(120px, 1fr))", gap: "10px" }}>
                  {customs.map((avatar) => (
                    <div
                      key={avatar.avatarId}
                      onClick={() => setViewing({ source: "custom", id: avatar.avatarId })}
                      style={{
                        background: "#0a101d",
                        border: viewing?.id === avatar.avatarId ? "2px solid #38bdf8" : "1px solid rgba(255,255,255,0.08)",
                        borderRadius: "10px",
                        padding: "8px",
                        cursor: "pointer",
                        display: "flex",
                        flexDirection: "column",
                        alignItems: "center",
                        gap: "4px",
                      }}
                    >
                      {avatar.spec ? (
                        <SpecThumbnail spec={avatar.spec} />
                      ) : previews[avatar.avatarId] ? (
                        <img src={previews[avatar.avatarId]} alt={avatar.name} width={84} height={84} style={{ objectFit: "contain" }} />
                      ) : (
                        <div style={{ width: 84, height: 84, display: "flex", alignItems: "center", justifyContent: "center", color: "#a78bfa", fontWeight: 800 }}>
                          {avatar.kind === "vrm" ? "VRM" : "3D"}
                        </div>
                      )}
                      <div style={{ fontSize: "12px", fontWeight: 600, color: "#e2e8f0", textAlign: "center" }}>{avatar.name}</div>
                      <div style={{ fontSize: "10px", color: "#64748b" }}>
                        {avatar.kind === "image_sculpt" ? "Image sculpt" : avatar.kind === "vrm" ? "VRM model" : avatar.sourceAvatarId ? "Styled from image" : "Custom"}
                        {isActive("custom", avatar.avatarId) ? " - active" : ""}
                      </div>
                      <button
                        style={{ ...button("danger", busy), padding: "2px 8px", fontSize: "10px" }}
                        disabled={busy}
                        onClick={(event) => {
                          event.stopPropagation();
                          void deleteCustom(avatar);
                        }}
                      >
                        Delete
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}
            <div style={panel}>
              <div style={{ display: "flex", gap: "8px", marginBottom: "12px", flexWrap: "wrap" }}>
                <input style={{ ...input, flex: 1, minWidth: "160px" }} placeholder="Search name, role, or tag" value={search} onChange={(event) => setSearch(event.target.value)} />
                <select style={input} value={bodyFilter} onChange={(event) => setBodyFilter(event.target.value)}>
                  <option value="all">All characters</option>
                  <option value={ANIME_COLLECTION}>Anime style ({catalog.filter(({ spec }) => spec.tags.includes(ANIME_TAG)).length})</option>
                  {BODY_TYPES.map((body) => (
                    <option key={body} value={body}>
                      {humanize(body)} (species)
                    </option>
                  ))}
                </select>
              </div>
              <div style={{ fontSize: "11px", color: "#64748b", marginBottom: "8px" }}>
                {filtered.length} of {catalog.length} original characters
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(120px, 1fr))", gap: "10px" }}>
                {filtered.map(({ spec, customized }) => (
                  <CharacterCard
                    key={spec.id}
                    spec={spec}
                    selected={viewing?.id === spec.id}
                    status={isActive("catalog", spec.id) ? "active" : customized ? "customized" : null}
                    onSelect={() => setViewing({ source: "catalog", id: spec.id })}
                  />
                ))}
              </div>
            </div>
          </>
        )}

        {tab === "customize" &&
          (draft ? (
            <div style={{ ...panel, display: "flex", flexDirection: "column", gap: "14px" }}>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(150px, 1fr))", gap: "10px" }}>
                <Field title="Name">
                  <input style={input} maxLength={SPEC_LIMITS.maxNameChars} value={draft.name} onChange={(event) => update({ name: event.target.value })} />
                </Field>
                <SelectField title="Body" value={draft.body} options={BODY_TYPES} onChange={(body) => update({ body })} />
                <SelectField title="Ears" value={draft.ears} options={EAR_TYPES} onChange={(ears) => update({ ears })} />
                <SelectField title="Tail" value={draft.tail} options={TAIL_TYPES} onChange={(tail) => update({ tail })} />
                <SelectField title="Wings" value={draft.wings} options={WING_TYPES} onChange={(wings) => update({ wings })} />
                <SelectField title="Hair" value={draft.hair} options={HAIR_STYLES} onChange={(hair) => update({ hair })} />
                <SelectField title="Eyes" value={draft.eyes} options={EYE_STYLES} onChange={(eyes) => update({ eyes })} />
                <SelectField title="Outfit" value={draft.outfit} options={OUTFITS} onChange={(outfit) => update({ outfit })} />
                <SelectField title="Fabric pattern" value={draft.pattern ?? "none"} options={PATTERNS} onChange={(pattern) => update({ pattern })} />
              </div>
              <div>
                <div style={{ ...label, marginBottom: "6px" }}>
                  Accessories ({draft.accessories.length}/{SPEC_LIMITS.maxAccessories})
                </div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: "6px" }}>
                  {ACCESSORIES.map((accessory: Accessory) => {
                    const on = draft.accessories.includes(accessory);
                    const blocked = !on && draft.accessories.length >= SPEC_LIMITS.maxAccessories;
                    return (
                      <button
                        key={accessory}
                        disabled={blocked}
                        style={{ ...button(on ? "primary" : "ghost", blocked), padding: "4px 10px", fontSize: "11px" }}
                        onClick={() =>
                          update({ accessories: on ? draft.accessories.filter((item) => item !== accessory) : [...draft.accessories, accessory] })
                        }
                      >
                        {humanize(accessory)}
                      </button>
                    );
                  })}
                </div>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(110px, 1fr))", gap: "10px" }}>
                <Field title="Hair tips">
                  <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
                    <input
                      type="checkbox"
                      aria-label="Fade hair to a tip color"
                      checked={draft.hairTip !== undefined}
                      onChange={(event) => update({ hairTip: event.target.checked ? draft.palette.accent : undefined })}
                    />
                    <input
                      type="color"
                      disabled={draft.hairTip === undefined}
                      value={draft.hairTip ?? draft.palette.hair}
                      style={{ ...input, padding: "2px", height: "32px", flex: 1 }}
                      onChange={(event) => update({ hairTip: event.target.value })}
                    />
                  </div>
                </Field>
                {PALETTE_KEYS.map((key) => (
                  <Field key={key} title={humanize(key)}>
                    <input
                      type="color"
                      value={draft.palette[key]}
                      style={{ ...input, padding: "2px", height: "32px" }}
                      onChange={(event) => update({ palette: { ...draft.palette, [key]: event.target.value } })}
                    />
                  </Field>
                ))}
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))", gap: "10px" }}>
                <SliderField title="Height" value={draft.proportions.height} range={SPEC_LIMITS.height} onChange={(height) => update({ proportions: { ...draft.proportions, height } })} />
                <SliderField title="Head size" value={draft.proportions.headScale} range={SPEC_LIMITS.headScale} onChange={(headScale) => update({ proportions: { ...draft.proportions, headScale } })} />
                <SliderField title="Build" value={draft.proportions.build} range={SPEC_LIMITS.build} onChange={(build) => update({ proportions: { ...draft.proportions, build } })} />
                <SliderField title="Limb length" value={draft.proportions.limbLength} range={SPEC_LIMITS.limbLength} onChange={(limbLength) => update({ proportions: { ...draft.proportions, limbLength } })} />
                <SliderField title="Energy" value={draft.energy} range={SPEC_LIMITS.energy} onChange={(energy) => update({ energy })} />
              </div>
              <div style={{ fontSize: "11px", color: "#64748b" }}>
                Changes preview live. "Save changes" stores them for this character; "Save as new" keeps the original and adds a copy to your characters.
              </div>
            </div>
          ) : (
            <div style={{ ...panel, fontSize: "12px", color: "#94a3b8" }}>
              Image sculpts and VRM models keep their own geometry. Select a library character, or the styled character created from your image, to customize it.
            </div>
          ))}

        {tab === "import" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            <div style={{ ...panel, display: "flex", flexDirection: "column", gap: "10px" }}>
              <div style={{ fontSize: "13px", fontWeight: 700, color: "#e2e8f0" }}>Image to 3D character</div>
              <div style={{ fontSize: "12px", color: "#94a3b8", lineHeight: 1.6 }}>
                Your image is processed on this computer. The subject is cut out (using transparency, or a plain background), its silhouette is inflated
                into a closed, textured 3D mesh, and its colors are sampled to style a fully rigged character you can customize. Works best with one subject on a
                transparent or plain background. Depth comes from the silhouette, so flat details such as faces stay painted on the surface.
              </div>
              <Field title="Character name">
                <input style={input} maxLength={SPEC_LIMITS.maxNameChars} value={imageName} onChange={(event) => setImageName(event.target.value)} />
              </Field>
              <div>
                <button style={button("primary", busy || !imageName.trim())} disabled={busy || !imageName.trim()} onClick={() => void importImage()}>
                  {busy ? "Working..." : "Choose image..."}
                </button>
              </div>
            </div>
            <div style={{ ...panel, display: "flex", flexDirection: "column", gap: "10px" }}>
              <div style={{ fontSize: "13px", fontWeight: 700, color: "#e2e8f0" }}>Import a VRM model</div>
              <div style={{ fontSize: "12px", color: "#94a3b8", lineHeight: 1.6 }}>
                Bring a fully modeled avatar from VRoid Studio, Blender, or Unity (.vrm or .glb, up to 50 MB, 100,000 triangles, 16 textures, no external
                files). Its humanoid bones follow the same agent-driven poses. Respect the license declared by the model's author.
              </div>
              <div>
                <button style={button("primary", busy)} disabled={busy} onClick={() => void importModel()}>
                  Choose model...
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
