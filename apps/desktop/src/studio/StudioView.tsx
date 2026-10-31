import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { readOutput, studioApi } from "./api";
import { CreatePanel } from "./CreatePanel";
import { ErrorCallout } from "./ErrorCallout";
import { isActive } from "./format";
import { Inspector } from "./Inspector";
import { LibraryPanel } from "./LibraryPanel";
import { ModelViewport, type ViewMode } from "./ModelViewport";
import type { SceneStats } from "./sceneStats";
import { SettingsDialog } from "./SettingsDialog";
import { TaskQueue } from "./TaskQueue";
import type { AssetDetail, AssetSummary, ErrorReport, GenerationView, ProjectView, StudioOverview, TaskView } from "./types";
import "./studio.css";

/** Local DB refresh cadence while tasks are active; provider polling happens in Rust. */
const ACTIVE_REFRESH_MS = 2_000;
const CLOCK_TICK_MS = 1_000;
/** Recently viewed models kept in memory so switching versions does not re-read large files. */
const MODEL_CACHE_LIMIT = 3;

type LeftTab = "create" | "library";
type Outcome = { ok: true } | { ok: false; error: ErrorReport };

function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}

function defaultGeneration(detail: AssetDetail): GenerationView | null {
  return detail.generations.find((generation) => generation.versionId !== null) ?? detail.generations[0] ?? null;
}

export function StudioView() {
  const [overview, setOverview] = useState<StudioOverview | null>(null);
  const [projects, setProjects] = useState<ProjectView[]>([]);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [newProjectName, setNewProjectName] = useState<string | null>(null);
  const [assets, setAssets] = useState<AssetSummary[]>([]);
  const [detail, setDetail] = useState<AssetDetail | null>(null);
  const [generationId, setGenerationId] = useState<string | null>(null);
  const [tasks, setTasks] = useState<TaskView[]>([]);
  const [now, setNow] = useState(() => Date.now());
  const [model, setModel] = useState<{ bytes: ArrayBuffer; format: string } | null>(null);
  const [stats, setStats] = useState<SceneStats | null>(null);
  const [viewerError, setViewerError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("material");
  const [showGrid, setShowGrid] = useState(true);
  const [showBounds, setShowBounds] = useState(false);
  const [frameToken, setFrameToken] = useState(0);
  const [resetToken, setResetToken] = useState(0);
  const [leftTab, setLeftTab] = useState<LeftTab>("create");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [error, setError] = useState<ErrorReport | null>(null);
  const modelCache = useRef(new Map<number, ArrayBuffer>());
  const taskSignature = useRef("");
  const openAssetId = useRef<string | null>(null);
  openAssetId.current = detail?.id ?? null;

  const [providerId, setProviderId] = useState<string | null>(null);
  const provider =
    overview?.providers.find((entry) => entry.id === providerId) ??
    overview?.providers.find((entry) => entry.configured) ??
    overview?.providers[0] ??
    null;
  const generation = detail?.generations.find((entry) => entry.id === generationId) ?? null;
  const hasActive = useMemo(() => tasks.some(isActive), [tasks]);

  const succeeded = (result: Outcome) => {
    if (!result.ok) setError(result.error);
    return result.ok;
  };

  const loadOverview = useCallback(async () => {
    const result = await studioApi.overview();
    if (result.ok) setOverview(result.data);
    else setError(result.error);
  }, []);

  const loadProjects = useCallback(async () => {
    const result = await studioApi.listProjects();
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setProjects(result.data);
    setProjectId((current) => current ?? result.data[0]?.id ?? null);
  }, []);

  const loadAssets = useCallback(async () => {
    if (!projectId) return;
    const result = await studioApi.listAssets(projectId);
    if (result.ok) setAssets(result.data);
    else setError(result.error);
  }, [projectId]);

  const loadDetail = useCallback(async (assetId: string, preferredGeneration?: string) => {
    const result = await studioApi.assetDetail(assetId);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setDetail(result.data);
    setGenerationId((current) => {
      const wanted = preferredGeneration ?? current;
      return result.data.generations.some((entry) => entry.id === wanted)
        ? (wanted ?? null)
        : (defaultGeneration(result.data)?.id ?? null);
    });
  }, []);

  const loadTasks = useCallback(async () => {
    const result = await studioApi.listTasks();
    if (result.ok) setTasks(result.data);
  }, []);

  useEffect(() => {
    void loadOverview();
    void loadProjects();
    void loadTasks();
  }, [loadOverview, loadProjects, loadTasks]);

  useEffect(() => {
    setDetail(null);
    setGenerationId(null);
    void loadAssets();
  }, [loadAssets]);

  useEffect(() => {
    if (!hasActive) return;
    const refresh = window.setInterval(() => void loadTasks(), ACTIVE_REFRESH_MS);
    const clock = window.setInterval(() => setNow(Date.now()), CLOCK_TICK_MS);
    return () => {
      window.clearInterval(refresh);
      window.clearInterval(clock);
    };
  }, [hasActive, loadTasks]);

  // A task state change can add versions to the library and the open asset.
  useEffect(() => {
    const signature = tasks.map((task) => `${task.id}:${task.state}:${task.versionId ?? ""}`).join("|");
    if (signature === taskSignature.current) return;
    taskSignature.current = signature;
    setNow(Date.now());
    void loadAssets();
    if (openAssetId.current) void loadDetail(openAssetId.current);
  }, [tasks, loadAssets, loadDetail]);

  useEffect(() => {
    setViewerError(null);
    const output = generation?.outputs.find((entry) => entry.kind === "model");
    if (!output) {
      setModel(null);
      return;
    }
    const cached = modelCache.current.get(output.id);
    if (cached) {
      setModel({ bytes: cached, format: output.format });
      return;
    }
    let cancelled = false;
    readOutput(output.id)
      .then((bytes) => {
        if (cancelled) return;
        const cache = modelCache.current;
        cache.set(output.id, bytes);
        while (cache.size > MODEL_CACHE_LIMIT) {
          const oldest = cache.keys().next().value;
          if (oldest === undefined) break;
          cache.delete(oldest);
        }
        setModel({ bytes, format: output.format });
      })
      .catch((reason: unknown) => {
        if (!cancelled) setViewerError(`Model file could not be read: ${reason instanceof Error ? reason.message : String(reason)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [generation]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (isTypingTarget(event.target) || event.ctrlKey || event.metaKey || event.altKey) return;
      const actions: Record<string, () => void> = {
        f: () => setFrameToken((value) => value + 1),
        r: () => setResetToken((value) => value + 1),
        g: () => setShowGrid((value) => !value),
        b: () => setShowBounds((value) => !value),
        "1": () => setViewMode("material"),
        "2": () => setViewMode("solid"),
        "3": () => setViewMode("wireframe"),
      };
      const action = actions[event.key.toLowerCase()];
      if (action) {
        event.preventDefault();
        action();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const openAsset = (assetId: string, generationToOpen?: string) => void loadDetail(assetId, generationToOpen);

  const createProject = async () => {
    if (!newProjectName?.trim()) return;
    const result = await studioApi.createProject(newProjectName);
    if (!result.ok) {
      setError(result.error);
      return;
    }
    setNewProjectName(null);
    await loadProjects();
    setProjectId(result.data.id);
  };

  const onSubmitted = (task: TaskView) => {
    setError(null);
    setTasks((current) => [task, ...current.filter((entry) => entry.id !== task.id)]);
    void loadAssets();
    openAsset(task.assetId, task.generationId);
  };

  const resume = async (task: TaskView) => {
    if (succeeded(await studioApi.resumeTask(task.id))) void loadTasks();
  };

  const regenerate = async (generationIdToRepeat: string) => {
    const result = await studioApi.regenerate(generationIdToRepeat);
    if (result.ok) onSubmitted(result.data);
    else setError(result.error);
  };

  const selectedAsset = detail ? { id: detail.id, name: detail.name, versionId: generation?.versionId ?? null } : null;
  const viewModeLabels: Record<ViewMode, string> = { material: "Material", solid: "Solid", wireframe: "Wireframe" };

  return (
    <div className="studio-root">
      <header className="studio-header">
        <h2>Miyora 3D Studio</h2>
        <label className="studio-inline">
          <span>Project</span>
          <select value={projectId ?? ""} onChange={(event) => setProjectId(event.target.value)}>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name} ({project.assetCount})
              </option>
            ))}
          </select>
        </label>
        {newProjectName === null ? (
          <button onClick={() => setNewProjectName("")}>New project</button>
        ) : (
          <form
            className="studio-inline"
            onSubmit={(event) => {
              event.preventDefault();
              void createProject();
            }}
          >
            <input autoFocus value={newProjectName} placeholder="Project name" maxLength={120} onChange={(event) => setNewProjectName(event.target.value)} />
            <button type="submit" disabled={!newProjectName.trim()}>
              Create
            </button>
            <button type="button" onClick={() => setNewProjectName(null)}>
              Cancel
            </button>
          </form>
        )}
        <span className="studio-spacer" />
        {provider && (
          <span className={`studio-chip ${provider.configured ? "ok" : "warn"}`}>
            {provider.label}: {provider.configured ? "ready" : provider.requiresApiKey ? "no API key" : "not installed"}
          </span>
        )}
        <button onClick={() => setSettingsOpen(true)} disabled={!provider}>
          Settings
        </button>
      </header>

      {error && (
        <div className="studio-banner">
          <ErrorCallout error={error} />
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div className="studio-body">
        <aside className="studio-left">
          <div className="studio-segmented" role="tablist">
            {(["create", "library"] as const).map((tab) => (
              <button key={tab} role="tab" aria-selected={leftTab === tab} className={leftTab === tab ? "active" : ""} onClick={() => setLeftTab(tab)}>
                {tab === "create" ? "Create" : `Library (${assets.length})`}
              </button>
            ))}
          </div>
          {leftTab === "create" ? (
            provider ? (
              <CreatePanel
                provider={provider}
                providers={overview?.providers ?? []}
                onProviderChange={setProviderId}
                presets={overview?.presets ?? []} projectId={projectId} selectedAsset={selectedAsset} onSubmitted={onSubmitted} />
            ) : (
              <p className="studio-muted">Loading providers…</p>
            )
          ) : (
            <LibraryPanel assets={assets} selectedAssetId={detail?.id ?? null} onSelect={(asset) => openAsset(asset.id)} />
          )}
        </aside>

        <main className="studio-center">
          <div className="studio-toolbar" role="toolbar" aria-label="Viewport">
            {(["material", "solid", "wireframe"] as const).map((mode, index) => (
              <button key={mode} className={viewMode === mode ? "active" : ""} onClick={() => setViewMode(mode)} title={`Shortcut ${index + 1}`}>
                {viewModeLabels[mode]}
              </button>
            ))}
            <span className="studio-divider" />
            <button className={showGrid ? "active" : ""} onClick={() => setShowGrid(!showGrid)} title="Shortcut G">
              Grid
            </button>
            <button className={showBounds ? "active" : ""} onClick={() => setShowBounds(!showBounds)} title="Shortcut B">
              Bounds
            </button>
            <button onClick={() => setFrameToken(frameToken + 1)} title="Shortcut F">
              Frame
            </button>
            <button onClick={() => setResetToken(resetToken + 1)} title="Shortcut R">
              Reset view
            </button>
            {stats && (
              <span className="studio-muted studio-toolbar-stats">
                {stats.triangles.toLocaleString("en-US")} tris · {stats.vertices.toLocaleString("en-US")} verts · {stats.materials} mat
              </span>
            )}
          </div>
          <div className="studio-viewport">
            <ModelViewport
              bytes={model?.bytes ?? null}
              format={model?.format ?? null}
              viewMode={viewMode}
              showGrid={showGrid}
              showBounds={showBounds}
              frameToken={frameToken}
              resetToken={resetToken}
              onStats={setStats}
              onError={setViewerError}
            />
            {!model && !viewerError && (
              <p className="studio-viewport-empty">
                {generation?.task && isActive(generation.task)
                  ? "This version is still being generated."
                  : "Generate a model or open one from the library."}
              </p>
            )}
            {viewerError && <p className="studio-viewport-empty studio-warning">{viewerError}</p>}
          </div>
        </main>

        <aside className="studio-right">
          <Inspector
            detail={detail}
            generation={generation}
            stats={stats}
            onSelectGeneration={(entry) => setGenerationId(entry.id)}
            onRegenerate={(entry) => void regenerate(entry.id)}
            onVersionCreated={onSubmitted}
            onExported={() => {
              if (detail) void loadDetail(detail.id);
            }}
          />
        </aside>
      </div>

      <footer className="studio-queue-panel" aria-label="Task queue">
        <h3>Tasks &amp; history</h3>
        <TaskQueue
          tasks={tasks}
          nowMs={now}
          onOpen={(task) => openAsset(task.assetId, task.generationId)}
          onResume={(task) => void resume(task)}
          onRegenerate={(task) => void regenerate(task.generationId)}
        />
      </footer>

      {settingsOpen && provider && overview && (
        <SettingsDialog provider={provider} pollPolicy={overview.pollPolicy} onClose={() => setSettingsOpen(false)} onChanged={() => void loadOverview()} />
      )}
    </div>
  );
}
