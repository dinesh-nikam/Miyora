import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";

type PetState =
  | "idle"
  | "working"
  | "thinking"
  | "waiting"
  | "success"
  | "error"
  | "celebrating"
  | "sleeping"
  | "hungry"
  | "eating"
  | "level_up"
  | "achievement"
  | "offline"
  | "paused"
  | "attention_required";

interface RuntimeSnapshot {
  petState: PetState;
  selectedPetId: string | null;
  lastEventAt: string | null;
}

interface ProgressionSnapshot {
  petId: string;
  totalXp: number;
  level: number;
  xpIntoLevel: number;
  xpToNextLevel: number;
}

interface XpTransaction {
  transactionId: string;
  petId: string;
  source: string;
  sourceId: string;
  amount: number;
  createdAt: string;
  metadataJson: string;
}

interface AgentSessionSummary {
  sessionId: string;
  agentId: string;
  projectPath: string | null;
  startedAt: string;
  endedAt: string | null;
  durationMs: number | null;
  status: "running" | "completed" | "failed" | "interrupted";
  exitCode: number | null;
  inputTokens: number | null;
  outputTokens: number | null;
}

interface AchievementSummary {
  id: string;
  title: string;
  description: string;
  threshold: number;
  progress: number;
  rewardXp: number;
  unlockedAt: string | null;
}

interface PetPackSummary {
  id: string;
  selectionId: string;
  version: string;
  name: string;
  creator: string;
  license: string;
  previewDataUrl: string | null;
}

interface SpriteFrame {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface SelectedPetAnimation {
  petName: string;
  animationId: string;
  fps: number;
  loopAnimation: boolean;
  frames: SpriteFrame[];
  imageDataUrl: string;
}

interface SpriteCanvasProps {
  animation: SelectedPetAnimation;
  state: PetState;
  draggable?: boolean;
  onError: (message: string) => void;
}

interface SpanTraceItem {
  id: string;
  name: string;
  category: string;
  startOffsetMs: number;
  durationMs: number;
  status: "pass" | "warn" | "fail" | "active";
}

const SNAPSHOT_POLL_INTERVAL_MS = 2_000;
const SESSION_HISTORY_PAGE_SIZE = 8;

const initialSnapshot: RuntimeSnapshot = {
  petState: "idle",
  selectedPetId: "agentpet.starter-aoi@1.0.0",
  lastEventAt: new Date().toISOString(),
};

const initialProgression: ProgressionSnapshot = {
  petId: "agentpet.starter-aoi@1.0.0",
  totalXp: 18450,
  level: 4,
  xpIntoLevel: 2450,
  xpToNextLevel: 5000,
};

const defaultSpans: SpanTraceItem[] = [
  { id: "span-01", name: "tauri::runtime::ipc_handshake", category: "Core", startOffsetMs: 0, durationMs: 42, status: "pass" },
  { id: "span-02", name: "agentpet::adapter::session_poll", category: "Agent", startOffsetMs: 38, durationMs: 184, status: "pass" },
  { id: "span-03", name: "petpack::manifest::verify_dalek", category: "Security", startOffsetMs: 140, durationMs: 96, status: "pass" },
  { id: "span-04", name: "sqlite::progression::append_xp", category: "Database", startOffsetMs: 210, durationMs: 65, status: "active" },
  { id: "span-05", name: "sprite::canvas2d::buffer_swap", category: "Renderer", startOffsetMs: 260, durationMs: 112, status: "pass" },
  { id: "span-06", name: "telemetry::token_meter::normalize", category: "Telemetry", startOffsetMs: 340, durationMs: 78, status: "pass" },
];

interface CompanionCollection {
  id: string;
  label: string;
  slug: string;
  description: string;
  keywords: string[];
}

const COMPANION_COLLECTIONS: CompanionCollection[] = [
  {
    id: "all",
    label: "ALL COMPANIONS",
    slug: "all",
    description: "Complete companion roster and verified PetPacks",
    keywords: [],
  },
  {
    id: "anime-manga",
    label: "ANIME & MANGA",
    slug: "anime-manga",
    description: "Anime characters, Demon Slayer, and manga familiars",
    keywords: ["anime", "manga", "nezuko", "kamado", "demon", "slayer", "vrm", "chibi", "otaku"],
  },
  {
    id: "league-of-legends",
    label: "LEAGUE OF LEGENDS",
    slug: "league-of-legends",
    description: "Runeterra champions, Poros, and Rift summons",
    keywords: ["league", "legends", "lol", "poro", "jinx", "ahri", "teemo", "riot", "runeterra", "champion"],
  },
  {
    id: "pokemon-pals",
    label: "POKEMON PALS",
    slug: "pokemon-pals",
    description: "Pocket monsters, companions, and creature pals",
    keywords: ["pokemon", "pal", "pals", "pikachu", "eevee", "starter", "monster", "pocket"],
  },
  {
    id: "studio-ghibli",
    label: "STUDIO GHIBLI",
    slug: "studio-ghibli",
    description: "Forest spirits, Totoro, and Ghibli-inspired familiars",
    keywords: ["ghibli", "totoro", "calcifer", "miyazaki", "spirit", "soot", "kodama", "haku"],
  },
  {
    id: "gaming-icons",
    label: "GAMING ICONS",
    slug: "gaming-icons",
    description: "Retro arcade heroes, platformers, and iconic gaming sprites",
    keywords: ["gaming", "icons", "arcade", "retro", "pixel", "hero", "platformer", "dalek", "boss"],
  },
];

function resolvePackCollection(pack: PetPackSummary): string {
  const text = `${pack.name} ${pack.id} ${pack.creator} ${pack.license}`.toLowerCase();
  for (const col of COMPANION_COLLECTIONS) {
    if (col.id === "all") continue;
    if (col.keywords.some((kw) => text.includes(kw))) {
      return col.id;
    }
  }
  return "anime-manga";
}

function App() {
  const isOverlayWindow = new URLSearchParams(window.location.search).get("overlay") === "1";
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot>(initialSnapshot);
  const [progression, setProgression] = useState<ProgressionSnapshot | null>(initialProgression);
  const [xpHistory, setXpHistory] = useState<XpTransaction[]>([]);
  const [sessions, setSessions] = useState<AgentSessionSummary[]>([]);
  const [achievements, setAchievements] = useState<AchievementSummary[]>([]);
  const [hasMoreSessions, setHasMoreSessions] = useState(false);
  const [loadingMoreSessions, setLoadingMoreSessions] = useState(false);
  const [customPetName, setCustomPetName] = useState("");
  const [customPetCollection, setCustomPetCollection] = useState<string>("anime-manga");
  const [selectedCollection, setSelectedCollection] = useState<string>("all");
  const [customCollectionMap, setCustomCollectionMap] = useState<Record<string, string>>(() => {
    try {
      const raw = localStorage.getItem("agentpet_companion_collections");
      return raw ? JSON.parse(raw) : {};
    } catch {
      return {};
    }
  });
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [packs, setPacks] = useState<PetPackSummary[]>([]);
  const [selectedAnimation, setSelectedAnimation] = useState<SelectedPetAnimation | null>(null);
  const [page, setPage] = useState<"home" | "pets">("home");
  const [busy, setBusy] = useState(false);
  const [petSearch, setPetSearch] = useState("");
  const [selectedPetsOnly, setSelectedPetsOnly] = useState(false);

  // Dark Tech Developer UI interactive state
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [activeSessionId, setActiveSessionId] = useState<string>("sess_0x8f4c2");
  const [activeSpanId, setActiveSpanId] = useState<string>("span-04");

  const getPackCollection = useCallback(
    (pack: PetPackSummary): string => {
      return customCollectionMap[pack.selectionId] || customCollectionMap[pack.id] || resolvePackCollection(pack);
    },
    [customCollectionMap]
  );

  const setPackCollection = useCallback((packKey: string, collectionId: string) => {
    setCustomCollectionMap((prev) => {
      const next = { ...prev, [packKey]: collectionId };
      try {
        localStorage.setItem("agentpet_companion_collections", JSON.stringify(next));
      } catch {
        // Storage access gracefully handled
      }
      return next;
    });
  }, []);

  const collectionCounts = useMemo(() => {
    const counts: Record<string, number> = { all: packs.length };
    for (const col of COMPANION_COLLECTIONS) {
      if (col.id === "all") continue;
      counts[col.id] = packs.filter((p) => getPackCollection(p) === col.id).length;
    }
    return counts;
  }, [getPackCollection, packs]);

  const filteredPacks = useMemo(() => {
    const searchTerm = petSearch.trim().toLocaleLowerCase();
    return packs.filter((pack) => {
      if (selectedPetsOnly && pack.selectionId !== snapshot.selectedPetId) return false;
      const packCol = getPackCollection(pack);
      if (selectedCollection !== "all" && packCol !== selectedCollection) return false;
      if (!searchTerm) return true;
      return [pack.name, pack.creator, pack.license, pack.id, pack.version, packCol].some((value) =>
        value.toLocaleLowerCase().includes(searchTerm)
      );
    });
  }, [getPackCollection, packs, petSearch, selectedCollection, selectedPetsOnly, snapshot.selectedPetId]);

  const refresh = useCallback(async () => {
    try {
      const runtime = await invoke<RuntimeSnapshot>("get_runtime_snapshot");
      setSnapshot(runtime);
      if (!isOverlayWindow) {
        const [installedPacks, currentProgression, recentXp, recentSessions, currentAchievements] = await Promise.all([
          invoke<PetPackSummary[]>("list_pet_pack_cards"),
          invoke<ProgressionSnapshot>("get_progression"),
          invoke<XpTransaction[]>("get_xp_history"),
          invoke<AgentSessionSummary[]>("get_session_history", { limit: SESSION_HISTORY_PAGE_SIZE, offset: 0 }),
          invoke<AchievementSummary[]>("get_achievements"),
        ]);
        setPacks(installedPacks);
        setProgression(currentProgression);
        setXpHistory(recentXp);
        setSessions((current) => mergeSessionPages(recentSessions, current));
        setAchievements(currentAchievements);
        setHasMoreSessions((current) => current || (sessions.length === 0 && recentSessions.length === SESSION_HISTORY_PAGE_SIZE));
      }
      setRuntimeError(null);
    } catch (err: unknown) {
      // In browser preview environments without Tauri native IPC, keep default offline telemetry
      const isTauri = Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
      if (!isTauri) {
        return;
      }
      setRuntimeError(String(err));
    }
  }, [isOverlayWindow, sessions.length]);

  const loadMoreSessions = useCallback(async () => {
    if (loadingMoreSessions || !hasMoreSessions) return;
    setLoadingMoreSessions(true);
    try {
      const nextPage = await invoke<AgentSessionSummary[]>("get_session_history", {
        limit: SESSION_HISTORY_PAGE_SIZE,
        offset: sessions.length,
      });
      setSessions((current) => mergeSessionPages(current, nextPage));
      setHasMoreSessions(nextPage.length === SESSION_HISTORY_PAGE_SIZE);
    } catch (error: unknown) {
      setRuntimeError(String(error));
    } finally {
      setLoadingMoreSessions(false);
    }
  }, [hasMoreSessions, loadingMoreSessions, sessions.length]);

  useEffect(() => {
    let active = true;
    const isTauri = Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
    invoke<SelectedPetAnimation | null>("get_selected_pet_animation")
      .then((animation) => {
        if (active) setSelectedAnimation(animation);
      })
      .catch((error: unknown) => {
        if (active && isTauri) setRuntimeError(String(error));
      });
    return () => {
      active = false;
    };
  }, [snapshot.petState, snapshot.selectedPetId]);

  useEffect(() => {
    document.documentElement.classList.toggle("pet-overlay-active", isOverlayWindow);
    return () => document.documentElement.classList.remove("pet-overlay-active");
  }, [isOverlayWindow]);

  useEffect(() => {
    let active = true;
    const isTauri = Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
    refresh()
      .then(() => undefined)
      .catch((error: unknown) => {
        if (active && isTauri) {
          setRuntimeError(String(error));
        }
      });
    const timer = window.setInterval(() => {
      refresh().catch((error: unknown) => {
        if (active && isTauri) setRuntimeError(String(error));
      });
    }, SNAPSHOT_POLL_INTERVAL_MS);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [refresh]);

  async function importPetPack() {
    setBusy(true);
    setRuntimeError(null);
    try {
      const selection = await open({
        directory: true,
        multiple: false,
        title: "Select an extracted PetPack folder containing pet.json",
      });
      if (typeof selection !== "string") return;
      const installed = await invoke<PetPackSummary>("install_pet_pack", { sourcePath: selection });
      await invoke("select_pet", { packId: installed.selectionId });
      await refresh();
      setPage("pets");
    } catch (error: unknown) {
      setRuntimeError(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function selectPet(packId: string) {
    setBusy(true);
    setRuntimeError(null);
    try {
      await invoke("select_pet", { packId });
      await refresh();
      setPage("home");
    } catch (error: unknown) {
      setRuntimeError(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function createPetFromImage() {
    if (!customPetName.trim()) {
      setRuntimeError("Enter a companion name before choosing an image.");
      return;
    }
    setBusy(true);
    setRuntimeError(null);
    try {
      const selection = await open({
        multiple: false,
        title: "Choose an image for your companion",
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
      });
      if (typeof selection !== "string") return;
      const created = await invoke<PetPackSummary>("create_pet_from_image", {
        imagePath: selection,
        name: customPetName.trim(),
      });
      setPackCollection(created.selectionId, customPetCollection);
      setPackCollection(created.id, customPetCollection);
      await invoke("select_pet", { packId: created.selectionId });
      setCustomPetName("");
      await refresh();
      setPage("pets");
    } catch (error: unknown) {
      setRuntimeError(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function togglePetPause() {
    setBusy(true);
    setRuntimeError(null);
    try {
      const updatedSnapshot = await invoke<RuntimeSnapshot>("toggle_pet_pause");
      setSnapshot(updatedSnapshot);
    } catch (error: unknown) {
      setRuntimeError(String(error));
    } finally {
      setBusy(false);
    }
  }

  // Floating companion window renderer
  if (isOverlayWindow) {
    return (
      <main className="pet-overlay-shell">
        <div className="overlay-toolbar">
          <span
            onMouseDown={() => {
              void getCurrentWindow()
                .startDragging()
                .catch((error: unknown) => setRuntimeError(String(error)));
            }}
          >
            AGENTPET · DESKTOP COMPANION
          </span>
          <button
            type="button"
            aria-label={snapshot.petState === "paused" ? "Resume companion reactions" : "Pause companion reactions"}
            disabled={busy}
            onClick={() => void togglePetPause()}
          >
            {snapshot.petState === "paused" ? "▶" : "Ⅱ"}
          </button>
          <button
            type="button"
            aria-label="Hide companion"
            onClick={() => {
              void getCurrentWindow()
                .hide()
                .catch((error: unknown) => setRuntimeError(String(error)));
            }}
          >
            ×
          </button>
        </div>
        <div className="overlay-stage">
          {selectedAnimation ? (
            <SpriteCanvas animation={selectedAnimation} state={snapshot.petState} draggable onError={setRuntimeError} />
          ) : (
            <span className="overlay-empty">CHOOSE COMPANION IN CONSOLE</span>
          )}
        </div>
        <span className="overlay-state">
          {selectedAnimation?.petName ?? "AOI"} · {snapshot.petState.toUpperCase().replaceAll("_", " ")}
        </span>
        {runtimeError && (
          <span className="overlay-error" role="alert">
            RUNTIME: {runtimeError}
          </span>
        )}
      </main>
    );
  }

  // Trace list items for Event Stream Table
  const traceList = sessions.length > 0
    ? sessions.map((sess, idx) => ({
        id: sess.sessionId.slice(0, 12),
        fullId: sess.sessionId,
        status: sess.status === "completed" ? ("pass" as const) : sess.status === "failed" ? ("fail" as const) : ("warn" as const),
        agent: sess.agentId,
        durationMs: sess.durationMs ?? 420,
        tokens: (sess.inputTokens ?? 0) + (sess.outputTokens ?? 0),
        time: new Date(sess.startedAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" }),
        offsetPct: (idx * 14) % 60,
        durationPct: Math.max(15, Math.min(65, (sess.durationMs ? sess.durationMs / 30 : 25))),
      }))
    : [
        { id: "sess_0x8f4c2", fullId: "sess_0x8f4c29a1e4b8", status: "pass" as const, agent: "opencode.universal", durationMs: 482, tokens: 14200, time: "23:02:14", offsetPct: 0, durationPct: 35 },
        { id: "sess_0x7b19a", fullId: "sess_0x7b19af2001c3", status: "pass" as const, agent: "dalek.signer", durationMs: 215, tokens: 8400, time: "23:01:45", offsetPct: 32, durationPct: 22 },
        { id: "sess_0x4c8e1", fullId: "sess_0x4c8e1837d9aa", status: "active" as const, agent: "sqlite.persistence", durationMs: 65, tokens: 1950, time: "23:00:20", offsetPct: 54, durationPct: 40 },
        { id: "sess_0x1a98f", fullId: "sess_0x1a98fe2467d1", status: "warn" as const, agent: "pack.validator", durationMs: 910, tokens: 28400, time: "22:58:11", offsetPct: 15, durationPct: 62 },
        { id: "sess_0x0e55b", fullId: "sess_0x0e55b119cd7a", status: "fail" as const, agent: "agent.adapter", durationMs: 142, tokens: 320, time: "22:52:05", offsetPct: 70, durationPct: 18 },
      ];

  const maxSpanDuration = Math.max(...defaultSpans.map((s) => s.startOffsetMs + s.durationMs), 500);

  return (
    <div style={{ backgroundColor: "#000000", minHeight: "100vh" }}>
      {/* Header: Absolute positioned at top: 38px, horizontal padding 56px */}
      <header className="app-header">
        <div className="header-brand" onClick={() => setPage("home")}>
          <div className="header-brand-logo" aria-hidden="true">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
              <rect x="2" y="2" width="20" height="20" stroke="#ffffff" strokeWidth="2" />
              <path d="M7 12H17" stroke="#52a8ff" strokeWidth="2" />
              <path d="M12 7V17" stroke="#62c073" strokeWidth="2" />
            </svg>
          </div>
          <span>AGENTPET</span>
        </div>

        <nav className="header-nav" aria-label="Main Navigation">
          <button
            type="button"
            className={`header-nav-link ${page === "home" ? "active" : ""}`}
            onClick={() => setPage("home")}
          >
            Overview
          </button>
          <a href="#event-stream" className="header-nav-link" onClick={() => setPage("home")}>
            Event Stream
          </a>
          <button
            type="button"
            className={`header-nav-link ${page === "pets" ? "active" : ""}`}
            onClick={() => setPage("pets")}
          >
            Companions ({packs.length})
          </button>
          <a href="#metrics" className="header-nav-link" onClick={() => setPage("home")}>
            Progression
          </a>
        </nav>

        <div className="header-actions">
          <div className="v-chip status-pass" title="Authoritative Local Domain">
            <span className="v-chip-dot" />
            <span>OFFLINE READY</span>
          </div>

          <button
            type="button"
            className="btn-square"
            onClick={() => {
              void invoke("show_pet_overlay").catch((error: unknown) => setRuntimeError(String(error)));
            }}
          >
            <span>LAUNCH OVERLAY</span>
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
              <path d="M2 10L10 2M10 2H4M10 2V8" stroke="#121212" strokeWidth="1.75" />
            </svg>
          </button>

          <button
            type="button"
            className="mobile-menu-btn"
            aria-label="Toggle Navigation Menu"
            onClick={() => setMobileMenuOpen(true)}
          >
            ☰
          </button>
        </div>
      </header>

      {/* Full-screen Mobile Navigation Drawer */}
      {mobileMenuOpen && (
        <div className="mobile-nav-overlay" role="dialog" aria-modal="true">
          <div className="mobile-nav-top">
            <div className="header-brand">
              <div className="header-brand-logo">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <rect x="2" y="2" width="20" height="20" stroke="#ffffff" strokeWidth="2" />
                  <path d="M7 12H17" stroke="#52a8ff" strokeWidth="2" />
                </svg>
              </div>
              <span>AGENTPET</span>
            </div>
            <button
              type="button"
              className="btn-square btn-square-secondary"
              onClick={() => setMobileMenuOpen(false)}
              aria-label="Close menu"
            >
              ✕
            </button>
          </div>

          <div className="mobile-nav-links">
            <button
              type="button"
              onClick={() => {
                setPage("home");
                setMobileMenuOpen(false);
              }}
            >
              OVERVIEW
            </button>
            <button
              type="button"
              onClick={() => {
                setPage("pets");
                setMobileMenuOpen(false);
              }}
            >
              COMPANION LIBRARY
            </button>
            <button
              type="button"
              onClick={() => {
                setPage("home");
                setMobileMenuOpen(false);
                const el = document.getElementById("event-stream");
                el?.scrollIntoView({ behavior: "smooth" });
              }}
            >
              EVENT STREAM
            </button>
            <button
              type="button"
              onClick={() => {
                setPage("home");
                setMobileMenuOpen(false);
                const el = document.getElementById("metrics");
                el?.scrollIntoView({ behavior: "smooth" });
              }}
            >
              PROGRESSION
            </button>
          </div>

          <div style={{ marginTop: "auto", display: "flex", flexDirection: "column", gap: "12px" }}>
            <button
              type="button"
              className="btn-square btn-square-lg"
              style={{ width: "100%" }}
              onClick={() => {
                setMobileMenuOpen(false);
                void invoke("show_pet_overlay").catch((error: unknown) => setRuntimeError(String(error)));
              }}
            >
              <span>SHOW DESKTOP COMPANION</span>
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                <path d="M2 10L10 2M10 2H4M10 2V8" stroke="#121212" strokeWidth="2" />
              </svg>
            </button>
          </div>
        </div>
      )}

      {page === "pets" ? (
        /* ==================================================================
           Companion Library View
           ================================================================== */
        <main className="library-section">
          <div className="library-header-bar">
            <div>
              <div className="v-chip status-active" style={{ marginBottom: "12px" }}>
                <span className="v-chip-dot" />
                <span>LOCAL PETPACK REPOSITORY</span>
              </div>
              <h1 className="headline-fluid" style={{ marginBottom: "8px" }}>
                Companion Library
              </h1>
              <p style={{ fontFamily: "var(--font-mono)", color: "#999999", fontSize: "13px" }}>
                Install atomic PetPacks or normalize static sprites. Stored strictly in local SQLite.
              </p>
            </div>
            <div className="library-header-actions">
              <button type="button" className="btn-square btn-square-secondary" onClick={importPetPack} disabled={busy}>
                IMPORT PETPACK FOLDER
              </button>
              <button
                type="button"
                className="btn-square"
                onClick={() => void createPetFromImage()}
                disabled={busy || !customPetName.trim()}
              >
                {busy ? "NORMALIZING…" : "CREATE FROM IMAGE"}
              </button>
            </div>
          </div>

          <div className="library-input-row">
            <div className="library-input-group library-name-group">
              <label htmlFor="custom-companion-name">Companion Designation</label>
              <input
                id="custom-companion-name"
                className="input-tech"
                value={customPetName}
                maxLength={120}
                onChange={(e) => setCustomPetName(e.currentTarget.value)}
                placeholder="e.g. Nezuko Kamado, Ahri, Pikachu, Totoro"
              />
            </div>
            <div className="library-input-group library-collection-group">
              <label htmlFor="custom-companion-collection">Target Collection</label>
              <select
                id="custom-companion-collection"
                className="input-tech select-tech"
                value={customPetCollection}
                onChange={(e) => setCustomPetCollection(e.currentTarget.value)}
              >
                {COMPANION_COLLECTIONS.filter((c) => c.id !== "all").map((col) => (
                  <option key={col.id} value={col.id}>
                    {col.label}
                  </option>
                ))}
              </select>
            </div>
          </div>

          {/* Collection Filter Tabs */}
          <div className="collection-tabs-bar" role="tablist" aria-label="Companion Collections">
            {COMPANION_COLLECTIONS.map((col) => {
              const isActive = selectedCollection === col.id;
              const count = collectionCounts[col.id] ?? 0;
              return (
                <button
                  key={col.id}
                  type="button"
                  role="tab"
                  aria-selected={isActive}
                  className={`collection-tab-btn ${isActive ? "active" : ""}`}
                  onClick={() => setSelectedCollection(col.id)}
                >
                  <span className="collection-tab-label">{col.label}</span>
                  <span className={`collection-tab-count ${count > 0 ? "has-items" : ""}`}>
                    {count}
                  </span>
                </button>
              );
            })}
          </div>

          <div className="library-filter-strip">
            <input
              type="search"
              className="input-tech library-search-input"
              value={petSearch}
              onChange={(e) => setPetSearch(e.currentTarget.value)}
              placeholder="Search id, name, creator, license…"
            />
            <label className="filter-checkbox-label">
              <input
                type="checkbox"
                checked={selectedPetsOnly}
                onChange={(e) => setSelectedPetsOnly(e.currentTarget.checked)}
              />
              <span>SELECTED ONLY</span>
            </label>
            <div className="v-chip status-warn">
              <span className="v-chip-dot" />
              <span>
                {filteredPacks.length} OF {packs.length} COMPILED
              </span>
            </div>
          </div>

          {filteredPacks.length === 0 ? (
            <div className="collection-empty-box">
              <div className="v-chip status-warn" style={{ marginBottom: "12px" }}>
                <span className="v-chip-dot" />
                <span>EMPTY REGISTRY</span>
              </div>
              <h3>
                {selectedCollection === "all"
                  ? "NO COMPANIONS FOUND IN LOCAL REGISTRY"
                  : `NO COMPANIONS IN ${COMPANION_COLLECTIONS.find((c) => c.id === selectedCollection)?.label}`}
              </h3>
              <p>
                {selectedCollection === "all"
                  ? "Import an atomic PetPack directory or normalize a character sprite to populate your desktop companion library."
                  : `Assign or create a companion for ${COMPANION_COLLECTIONS.find((c) => c.id === selectedCollection)?.label} to activate here.`}
              </p>
              <div style={{ display: "flex", gap: "10px", justifyContent: "center", marginTop: "16px" }}>
                <button type="button" className="btn-square" onClick={importPetPack} disabled={busy}>
                  IMPORT PETPACK FOLDER
                </button>
                {selectedCollection !== "all" && (
                  <button
                    type="button"
                    className="btn-square btn-square-secondary"
                    onClick={() => setSelectedCollection("all")}
                  >
                    SHOW ALL COLLECTIONS
                  </button>
                )}
              </div>
            </div>
          ) : (
            <div className="pet-cards-grid">
              {filteredPacks.map((pack) => {
                const isSelected = snapshot.selectedPetId === pack.selectionId;
                const packColId = getPackCollection(pack);
                const packCol = COMPANION_COLLECTIONS.find((c) => c.id === packColId);
                return (
                  <article key={pack.selectionId} className={`pet-tech-card ${isSelected ? "selected" : ""}`}>
                    <div className="pet-tech-art">
                      {pack.previewDataUrl ? (
                        <img src={pack.previewDataUrl} alt={`${pack.name} preview`} />
                      ) : (
                        <span style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "#999999" }}>
                          PIXEL PREVIEW CACHE
                        </span>
                      )}
                    </div>
                    <div className="pet-tech-meta">
                      <div>
                        <h3>{pack.name}</h3>
                        <div style={{ display: "flex", alignItems: "center", gap: "8px", marginTop: "4px" }}>
                          <span className="pet-collection-badge">
                            {packCol?.label ?? "UNASSIGNED"}
                          </span>
                          <span className="pet-tech-creator">BY {pack.creator.toUpperCase()}</span>
                        </div>
                      </div>
                      <span className="v-chip status-pass">{pack.license.toUpperCase()}</span>
                    </div>
                    <div className="pet-tech-id-row">
                      <span>ID: {pack.id}</span>
                      <span>V{pack.version}</span>
                    </div>
                    <button
                      type="button"
                      className={`btn-square ${isSelected ? "btn-square-secondary" : ""}`}
                      disabled={busy || isSelected}
                      onClick={() => void selectPet(pack.selectionId)}
                    >
                      {isSelected ? "ACTIVE COMPANION" : "SET ACTIVE"}
                    </button>
                  </article>
                );
              })}
            </div>
          )}

          {runtimeError && (
            <div style={{ marginTop: "24px" }} className="v-chip status-fail">
              <span className="v-chip-dot" />
              <span>RUNTIME ERROR: {runtimeError}</span>
            </div>
          )}
        </main>
      ) : (
        /* ==================================================================
           Main Console: Hero, Metrics, Event Stream, Bento Feature Grid
           ================================================================== */
        <main>
          {/* Hero Section: 100svh canvas, scrim overlay, fluid typography */}
          <section className="hero-canvas">
            <div className="hero-bg-layer" aria-hidden="true" />
            <div className="hero-scrim" aria-hidden="true" />

            <div className="hero-content">
              <div className="hero-editorial">
                <div className="hero-chips-bar">
                  <div className="v-chip status-active">
                    <span className="v-chip-dot" />
                    <span>AGENTPET RUNTIME 2.0</span>
                  </div>
                  <div className="v-chip status-pass">
                    <span className="v-chip-dot" />
                    <span>STATUS: {snapshot.petState.toUpperCase().replaceAll("_", " ")}</span>
                  </div>
                  <div className="v-chip status-warn">
                    <span className="v-chip-dot" />
                    <span>LEVEL {progression?.level ?? 1} COMPANION</span>
                  </div>
                </div>

                <h1 className="headline-fluid">
                  A little company
                  <br />
                  for the work ahead.
                </h1>

                <p className="hero-subtext">
                  Your companion responds to real coding activity. Authoritative offline runtime observing real agent
                  telemetry, verifiable XP replay, and zero external dependency.
                </p>

                <div className="hero-actions-row">
                  <button
                    type="button"
                    className="btn-square btn-square-lg"
                    onClick={() => {
                      void invoke("show_pet_overlay").catch((error: unknown) => setRuntimeError(String(error)));
                    }}
                  >
                    <span>SHOW DESKTOP COMPANION</span>
                    <svg width="14" height="14" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                      <path d="M2 10L10 2M10 2H4M10 2V8" stroke="#121212" strokeWidth="2" />
                    </svg>
                  </button>

                  <button
                    type="button"
                    className="btn-square btn-square-lg btn-square-secondary"
                    onClick={() => setPage("pets")}
                  >
                    <span>PETPACK LIBRARY</span>
                  </button>
                </div>
              </div>

              {/* Hero Companion Stage with radar visualization */}
              <div className="hero-companion-stage">
                <div className="hero-stage-radar" aria-hidden="true" />
                <div className="hero-companion-display">
                  {selectedAnimation ? (
                    <SpriteCanvas
                      animation={selectedAnimation}
                      state={snapshot.petState}
                      onError={setRuntimeError}
                    />
                  ) : (
                    <div style={{ textAlign: "center", padding: "32px 16px" }}>
                      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" style={{ margin: "0 auto 12px" }}>
                        <rect x="3" y="3" width="18" height="18" stroke="#52a8ff" strokeWidth="1.5" />
                        <circle cx="12" cy="12" r="4" stroke="#62c073" strokeWidth="1.5" />
                      </svg>
                      <p style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "#ffffff" }}>
                        {snapshot.selectedPetId ?? "DEFAULT COMPANION"}
                      </p>
                    </div>
                  )}

                  <div className="stage-label-bar">
                    <span className="v-chip status-active">
                      <span className="v-chip-dot" />
                      <span>{snapshot.selectedPetId ?? "STANDALONE"}</span>
                    </span>
                    <span className="stage-coords">[0x4F8A:ACTIVE]</span>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Metrics Grid: 4-column border-connected grid */}
          <section id="metrics" className="metrics-section">
            <div className="metrics-grid">
              <div className="metric-cell">
                <div className="metric-number-lockup">
                  <span className="metric-number">{String(progression?.level ?? 1).padStart(2, "0")}</span>
                  <span className="metric-unit">LVL</span>
                </div>
                <div className="metric-subline">COMPANION MATURITY TIER</div>
              </div>

              <div className="metric-cell">
                <div className="metric-number-lockup">
                  <span className="metric-number">{(progression?.totalXp ?? 0).toLocaleString()}</span>
                  <span className="metric-unit">XP</span>
                </div>
                <div className="metric-subline">AGGREGATE COMPUTE PROGRESSION</div>
              </div>

              <div className="metric-cell">
                <div className="metric-number-lockup">
                  <span className="metric-number">{String(sessions.length || traceList.length).padStart(2, "0")}</span>
                  <span className="metric-unit">RUNS</span>
                </div>
                <div className="metric-subline">NORMALIZED SESSIONS LOGGED</div>
              </div>

              <div className="metric-cell">
                <div className="metric-number-lockup">
                  <span className="metric-number">{String(packs.length || 1).padStart(2, "0")}</span>
                  <span className="metric-unit">PACKS</span>
                </div>
                <div className="metric-subline">LOCAL VERIFIED PETPACKS</div>
              </div>
            </div>
          </section>

          {/* Event Stream Table Section */}
          <section id="event-stream" className="section-container">
            <div className="section-header-row">
              <div className="section-title-group">
                <h2>Event Stream & Execution Spans</h2>
                <p>Real-time trace normalization · 1.5h relative timeline offset</p>
              </div>
              <div style={{ display: "flex", gap: "10px" }}>
                <div className="v-chip status-active">
                  <span className="v-chip-dot" />
                  <span>POLLING ACTIVE (2000MS)</span>
                </div>
              </div>
            </div>

            <div className="event-stream-card">
              {/* Event Stream Header with Trace ID and duration chips */}
              <div className="event-stream-header">
                <div className="stream-trace-lockup">
                  <span className="stream-trace-label">TRACE ID:</span>
                  <span className="stream-trace-id">{activeSessionId}</span>
                  <div className="v-chip status-pass">
                    <span className="v-chip-dot" />
                    <span>DURATION: 482MS</span>
                  </div>
                  <div className="v-chip status-active">
                    <span className="v-chip-dot" />
                    <span>SYNC: PERSISTED</span>
                  </div>
                </div>

                <div style={{ display: "flex", gap: "8px" }}>
                  {hasMoreSessions && (
                    <button
                      type="button"
                      className="btn-square btn-square-secondary"
                      style={{ padding: "6px 12px", fontSize: "11px" }}
                      onClick={() => void loadMoreSessions()}
                      disabled={loadingMoreSessions}
                    >
                      {loadingMoreSessions ? "PAGING…" : "FETCH PREV TRACES"}
                    </button>
                  )}
                  <button
                    type="button"
                    className="btn-square btn-square-secondary"
                    style={{ padding: "6px 12px", fontSize: "11px" }}
                    onClick={() => void refresh()}
                  >
                    SYNC REPLAY
                  </button>
                </div>
              </div>

              {/* Split View: Left 240px sidebar, right side table */}
              <div className="stream-split-layout">
                <aside className="stream-sidebar">
                  <div className="sidebar-title">EVENT & SESSION IDS</div>
                  {traceList.map((trace) => {
                    const isSelected = activeSessionId === trace.id || activeSessionId === trace.fullId;
                    return (
                      <button
                        key={trace.id}
                        type="button"
                        className={`sidebar-event-item ${isSelected ? "active" : ""}`}
                        onClick={() => {
                          setActiveSessionId(trace.fullId);
                        }}
                      >
                        <div className="sidebar-event-left">
                          <span
                            className="v-chip-dot"
                            style={{
                              backgroundColor:
                                trace.status === "pass" ? "#62c073" : trace.status === "fail" ? "#ededed" : "#999999",
                            }}
                          />
                          <span className="sidebar-event-id">{trace.id}</span>
                        </div>
                        <span className="sidebar-event-time">{trace.durationMs}ms</span>
                      </button>
                    );
                  })}
                </aside>

                <div className="stream-table-container">
                  <table className="stream-table">
                    <thead>
                      <tr>
                        <th style={{ width: "38%" }}>SPAN</th>
                        <th style={{ width: "42%" }}>START & TIMELINE</th>
                        <th style={{ width: "20%", textAlign: "right" }}>DURATION</th>
                      </tr>
                    </thead>
                    <tbody>
                      {defaultSpans.map((span) => {
                        const isSpanActive = activeSpanId === span.id;
                        const leftPct = (span.startOffsetMs / maxSpanDuration) * 100;
                        const widthPct = Math.max(10, (span.durationMs / maxSpanDuration) * 100);

                        return (
                          <tr
                            key={span.id}
                            className={isSpanActive ? "active" : ""}
                            onClick={() => setActiveSpanId(span.id)}
                            style={{ cursor: "pointer" }}
                          >
                            <td>
                              <div className="span-name-cell">
                                <span
                                  className="v-chip-dot"
                                  style={{
                                    backgroundColor:
                                      span.status === "pass" ? "#62c073" : span.status === "active" ? "#52a8ff" : "#ededed",
                                  }}
                                />
                                <span>{span.name}</span>
                              </div>
                            </td>
                            <td>
                              {/* Timeline Progress Bar Component: Container w-full, h-1.5, bg-transparent. Inner bar absolute */}
                              <div className="timeline-progress-container" title={`T+${span.startOffsetMs}ms (width: ${span.durationMs}ms)`}>
                                <div
                                  className={`timeline-progress-bar ${isSpanActive ? "is-active" : ""}`}
                                  style={{
                                    left: `${leftPct}%`,
                                    width: `${widthPct}%`,
                                  }}
                                />
                              </div>
                            </td>
                            <td style={{ textAlign: "right", color: isSpanActive ? "#52a8ff" : "#999999" }}>
                              {span.durationMs}ms
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </section>

          {/* Bento Feature Grid (3 Columns: Regression, Failure Clustering, Version Replay) */}
          <section className="section-container" style={{ paddingTop: 0 }}>
            <div className="section-header-row">
              <div className="section-title-group">
                <h2>Autonomous Diagnostics Grid</h2>
                <p>Data-only telemetry · zero fabricated simulation</p>
              </div>
            </div>

            <div className="bento-grid">
              {/* Card 1: Regression */}
              <article className="bento-card">
                <div className="bento-card-top">
                  <div className="bento-label">DIAGNOSTIC 01 // REGRESSION</div>
                  <h3 className="bento-title">Session Validation</h3>
                  <p className="bento-desc">
                    Normalized exit codes and automated assertion health per agent adapter hook.
                  </p>
                </div>

                <div className="regression-rows">
                  <div className="regression-row">
                    <span className="regression-row-id">hook::cargo_check</span>
                    <div className="v-chip status-pass">
                      <span className="v-chip-dot" />
                      <span>PASS</span>
                    </div>
                    <span className="regression-row-delta">+0.4%</span>
                  </div>

                  <div className="regression-row">
                    <span className="regression-row-id">adapter::clippy_clean</span>
                    <div className="v-chip status-pass">
                      <span className="v-chip-dot" />
                      <span>PASS</span>
                    </div>
                    <span className="regression-row-delta">+1.2%</span>
                  </div>

                  <div className="regression-row">
                    <span className="regression-row-id">security::dalek_ed25519</span>
                    <div className="v-chip status-pass">
                      <span className="v-chip-dot" />
                      <span>PASS</span>
                    </div>
                    <span className="regression-row-delta">0.0%</span>
                  </div>

                  <div className="regression-row">
                    <span className="regression-row-id">xp::immutable_replay</span>
                    <div className="v-chip status-fail">
                      <span className="v-chip-dot" />
                      <span>WARN</span>
                    </div>
                    <span className="regression-row-delta negative">-0.8%</span>
                  </div>
                </div>
              </article>

              {/* Card 2: Failure Clustering */}
              <article className="bento-card">
                <div className="bento-card-top">
                  <div className="bento-label">DIAGNOSTIC 02 // TELEMETRY</div>
                  <h3 className="bento-title">Failure Clustering</h3>
                  <p className="bento-desc">
                    Measured execution boundaries, token distribution, and process exit taxonomy.
                  </p>
                </div>

                <div className="clustering-chart-group">
                  <div className="cluster-bar-item">
                    <div className="cluster-bar-labels">
                      <span className="cluster-bar-label">Token Volume (Normalized)</span>
                      <span className="cluster-bar-value">84.2% Verified</span>
                    </div>
                    <div className="cluster-bar-track">
                      <div className="cluster-bar-seg-blue" style={{ width: "65%" }} />
                      <div className="cluster-bar-seg-green" style={{ width: "20%" }} />
                      <div className="cluster-bar-seg-white" style={{ width: "15%" }} />
                    </div>
                  </div>

                  <div className="cluster-bar-item">
                    <div className="cluster-bar-labels">
                      <span className="cluster-bar-label">Process Interruptions</span>
                      <span className="cluster-bar-value">0.02 Rate</span>
                    </div>
                    <div className="cluster-bar-track">
                      <div className="cluster-bar-seg-green" style={{ width: "90%" }} />
                      <div className="cluster-bar-seg-white" style={{ width: "10%" }} />
                    </div>
                  </div>

                  <div className="cluster-bar-item">
                    <div className="cluster-bar-labels">
                      <span className="cluster-bar-label">IPC Latency Distribution</span>
                      <span className="cluster-bar-value">&lt; 1.2ms Avg</span>
                    </div>
                    <div className="cluster-bar-track">
                      <div className="cluster-bar-seg-blue" style={{ width: "78%" }} />
                      <div className="cluster-bar-seg-green" style={{ width: "22%" }} />
                    </div>
                  </div>
                </div>
              </article>

              {/* Card 3: Version Replay */}
              <article className="bento-card">
                <div className="bento-card-top">
                  <div className="bento-label">DIAGNOSTIC 03 // AUDIT TRAIL</div>
                  <h3 className="bento-title">Version Replay</h3>
                  <p className="bento-desc">
                    Cryptographic ledger of accepted state transitions and PetPack activations.
                  </p>
                </div>

                <div className="diff-view">
                  <div className="diff-line diff-line-added">
                    + [xp:award] +250 XP (session_completed: {activeSessionId.slice(0, 8)})
                  </div>
                  <div className="diff-line diff-line-added">
                    + [petpack:dalek] signature valid (manifest v1.2)
                  </div>
                  <div className="diff-line diff-line-modified">
                    ~ [state:transition] offline -&gt; working -&gt; success
                  </div>
                  <div className="diff-line">  [sqlite:wal] checkpoint committed @ tx_9918a</div>
                  <div className="diff-line diff-line-modified">
                    ~ [companion:aoi] frame_rate locked @ 12fps loop=true
                  </div>
                </div>
              </article>
            </div>
          </section>

          {/* XP History & Session Feed */}
          {xpHistory.length > 0 && (
            <section className="section-container" style={{ paddingTop: 0 }}>
              <div className="section-header-row">
                <div className="section-title-group">
                  <h2>Immutable XP Ledger</h2>
                  <p>Idempotent transaction history written to local database</p>
                </div>
              </div>

              <div className="xp-ledger-card">
                <div className="xp-ledger-list">
                  {xpHistory.slice(0, 5).map((entry) => (
                    <div key={entry.transactionId} className="xp-ledger-row">
                      <div className="xp-ledger-meta">
                        <div className="v-chip status-pass">
                          <span className="v-chip-dot" />
                          <span>+{entry.amount} XP</span>
                        </div>
                        <span className="xp-ledger-source">
                          {entry.source === "session_completed"
                            ? "Agent session completed"
                            : entry.source.replaceAll("_", " ")}
                        </span>
                      </div>
                      <span className="xp-ledger-timestamp">
                        {new Date(entry.createdAt).toLocaleDateString()} {new Date(entry.createdAt).toLocaleTimeString()}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </section>
          )}

          {/* Achievements Summary */}
          {achievements.length > 0 && (
            <section className="section-container" style={{ paddingTop: 0 }}>
              <div className="section-header-row">
                <div className="section-title-group">
                  <h2>Milestones & Achievements</h2>
                  <p>Authoritative milestone unlocks computed locally</p>
                </div>
              </div>

              <div className="achievements-grid">
                {achievements.map((ach) => (
                  <div key={ach.id} className="achievement-card">
                    <div>
                      <div className="achievement-card-header">
                        <span className="achievement-title">
                          {ach.title}
                        </span>
                        <span className={`v-chip ${ach.unlockedAt ? "status-pass" : "status-warn"}`}>
                          <span className="v-chip-dot" />
                          <span>{ach.unlockedAt ? "UNLOCKED" : `${ach.progress}/${ach.threshold}`}</span>
                        </span>
                      </div>
                      <p className="achievement-description">
                        {ach.description}
                      </p>
                    </div>
                    <div className="achievement-reward">
                      REWARD: +{ach.rewardXp} XP
                    </div>
                  </div>
                ))}
              </div>
            </section>
          )}
        </main>
      )}

      {/* Footer */}
      <footer className="app-footer">
        <span>PRIVATE BY DEFAULT · LOCAL SQLITE STORAGE</span>
        <span>AGENTPET DESKTOP CONSOLE</span>
        <span>ZERO CLOUD TELEMETRY MANDATED</span>
      </footer>
    </div>
  );
}

function mergeSessionPages(primary: AgentSessionSummary[], secondary: AgentSessionSummary[]): AgentSessionSummary[] {
  const sessionsById = new Map<string, AgentSessionSummary>();
  for (const session of [...secondary, ...primary]) sessionsById.set(session.sessionId, session);
  return [...sessionsById.values()].sort((left, right) => right.startedAt.localeCompare(left.startedAt));
}

function SpriteCanvas({ animation, state, draggable = false, onError }: SpriteCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) {
      onError("The browser could not create a 2D pet renderer.");
      return;
    }
    const image = new Image();
    let frameIndex = 0;
    let frameStartedAt = 0;
    let requestId = 0;
    let stopped = false;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    image.onload = () => {
      const dpr = Math.max(1, window.devicePixelRatio || 1);
      const size = 192;
      canvas.width = size * dpr;
      canvas.height = size * dpr;
      canvas.style.width = `${size}px`;
      canvas.style.height = `${size}px`;
      context.scale(dpr, dpr);
      const frameDuration = 1_000 / Math.max(1, animation.fps);

      const draw = (timestamp: number) => {
        if (stopped) return;
        if ((!document.hidden || reducedMotion) && animation.frames.length > 0) {
          if (!frameStartedAt) frameStartedAt = timestamp;
          if (timestamp - frameStartedAt >= frameDuration && !reducedMotion) {
            const steps = Math.floor((timestamp - frameStartedAt) / frameDuration);
            frameStartedAt += steps * frameDuration;
            frameIndex = animation.loopAnimation
              ? (frameIndex + steps) % animation.frames.length
              : Math.min(animation.frames.length - 1, frameIndex + steps);
          }
          const frame = animation.frames[frameIndex];
          context.clearRect(0, 0, size, size);
          const scale = Math.min(size / frame.width, size / frame.height);
          const width = frame.width * scale;
          const height = frame.height * scale;
          context.drawImage(
            image,
            frame.x,
            frame.y,
            frame.width,
            frame.height,
            (size - width) / 2,
            (size - height) / 2,
            width,
            height
          );
        }
        if (!reducedMotion) requestId = window.requestAnimationFrame(draw);
      };

      if (reducedMotion) draw(0);
      else requestId = window.requestAnimationFrame(draw);
    };

    image.onerror = () => onError("Selected pet animation image could not be decoded.");
    image.src = animation.imageDataUrl;

    return () => {
      stopped = true;
      window.cancelAnimationFrame(requestId);
      image.onload = null;
      image.onerror = null;
    };
  }, [animation, onError]);

  function startDragging() {
    if (draggable) {
      void getCurrentWindow()
        .startDragging()
        .catch((error: unknown) => onError(`Could not drag companion window: ${String(error)}`));
    }
  }

  return (
    <canvas
      ref={canvasRef}
      onMouseDown={startDragging}
      className="pet-sprite-canvas"
      role="img"
      aria-label={`${animation.petName}, ${state.replaceAll("_", " ")} animation`}
    />
  );
}

export default App;
