import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { invokeSafe } from "./utils/ipc";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { CompanionRenderer, PetState, SelectedPetAnimation } from "./components/CompanionRenderer";
import { CommandDeck, ProviderReport } from "./components/CommandDeck";
import { CharacterStudio } from "./avatar/CharacterStudio";
import type { AvatarHud, ResolvedAvatar } from "./avatar/types";

interface RuntimeSnapshot {
  petState: PetState;
  selectedPetId: string | null;
  lastEventAt: string | null;
  isSimulated?: boolean;
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

interface StoredAgentEvent {
  id: string;
  eventType: string;
  occurredAt: string;
  source: string;
  provider: string;
  sessionId: string | null;
  projectId: string | null;
  payloadJson: string | null;
  latencyMs: number | null;
}

interface LedgerVerificationResult {
  isValid: boolean;
  totalEntries: number;
  headHash: string;
  brokenAtIndex: number | null;
  errorMessage: string | null;
}

const SNAPSHOT_POLL_INTERVAL_MS = 1_500;

export default function App() {
  const isOverlayWindow = new URLSearchParams(window.location.search).get("overlay") === "1";

  // Navigation State
  const [activeTab, setActiveTab] = useState<"den" | "sessions" | "companions" | "progress" | "settings">("den");
  const [isCommandDeckOpen, setIsCommandDeckOpen] = useState(false);

  // Core Data
  const [renderMode, setRenderMode] = useState<"3d" | "2d">(() => {
    try {
      const saved = localStorage.getItem("agentpet_render_mode");
      if (saved === "2d" || saved === "3d") return saved;
    } catch {
      // ignore
    }
    return "3d";
  });
  const [snapshot, setSnapshot] = useState<RuntimeSnapshot>({
    petState: "idle",
    selectedPetId: null,
    lastEventAt: null,
  });
  const isSimulated = snapshot?.isSimulated ?? false;
  const [progression, setProgression] = useState<ProgressionSnapshot | null>(null);
  const [xpHistory, setXpHistory] = useState<XpTransaction[]>([]);
  const [sessions, setSessions] = useState<AgentSessionSummary[]>([]);
  const [achievements, setAchievements] = useState<AchievementSummary[]>([]);
  const [installedPacks, setInstalledPacks] = useState<PetPackSummary[]>([]);
  const [selectedAnimation, setSelectedAnimation] = useState<SelectedPetAnimation | null>(null);
  const [avatar, setAvatar] = useState<ResolvedAvatar | null>(null);
  const [avatarHud, setAvatarHud] = useState<AvatarHud | null>(null);
  const [realEvents, setRealEvents] = useState<StoredAgentEvent[]>([]);
  const [providers, setProviders] = useState<ProviderReport[]>([]);
  const [ledgerVerification, setLedgerVerification] = useState<LedgerVerificationResult | null>(null);
  const [selectedEventJson, setSelectedEventJson] = useState<string | null>(null);

  // UI state
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [panelErrors, setPanelErrors] = useState<Record<string, string | null>>({});
  const [busy, setBusy] = useState(false);
  const [ledgerVerifying, setLedgerVerifying] = useState(false);

  // Poll runtime data
  const refreshCoreData = useCallback(async () => {
    try {
      const snapRes = await invokeSafe<RuntimeSnapshot>("get_runtime_snapshot");
      if (snapRes.ok) {
        setSnapshot(snapRes.data);
        setPanelErrors((prev) => ({ ...prev, den: null }));
      } else {
        setPanelErrors((prev) => ({ ...prev, den: `Failed to load runtime snapshot: ${snapRes.error}` }));
      }

      const animRes = await invokeSafe<SelectedPetAnimation | null>("get_selected_pet_animation");
      if (animRes.ok) {
        setSelectedAnimation((prev) => {
          if (!prev && !animRes.data) return null;
          if (
            prev &&
            animRes.data &&
            prev.petName === animRes.data.petName &&
            prev.animationId === animRes.data.animationId
          ) {
            return prev;
          }
          return animRes.data;
        });
      }

      const [avatarRes, hudRes] = await Promise.all([
        invokeSafe<ResolvedAvatar>("get_avatar"),
        invokeSafe<AvatarHud>("get_avatar_hud"),
      ]);
      if (avatarRes.ok) {
        setAvatar((previous) => (JSON.stringify(previous) === JSON.stringify(avatarRes.data) ? previous : avatarRes.data));
        setPanelErrors((prev) => ({ ...prev, avatar: null }));
      } else {
        setPanelErrors((prev) => ({ ...prev, avatar: `Failed to load the 3D companion: ${avatarRes.error}` }));
      }
      if (hudRes.ok) {
        setAvatarHud((previous) => (JSON.stringify(previous) === JSON.stringify(hudRes.data) ? previous : hudRes.data));
      }

      // Only query heavy items in main window
      if (!isOverlayWindow) {
        const [progRes, eventsRes, provsRes] = await Promise.all([
          invokeSafe<ProgressionSnapshot>("get_progression"),
          invokeSafe<StoredAgentEvent[]>("get_real_events", { limit: 25 }),
          invokeSafe<ProviderReport[]>("get_providers_status"),
        ]);
        if (progRes.ok) {
          setProgression(progRes.data);
          setPanelErrors((prev) => ({ ...prev, progression: null }));
        } else {
          setPanelErrors((prev) => ({ ...prev, progression: `Failed to load progression: ${progRes.error}` }));
        }

        if (eventsRes.ok) {
          setRealEvents(eventsRes.data);
          setPanelErrors((prev) => ({ ...prev, events: null }));
        } else {
          setPanelErrors((prev) => ({ ...prev, events: `Failed to load real events: ${eventsRes.error}` }));
        }

        if (provsRes.ok) {
          setProviders(provsRes.data);
          setPanelErrors((prev) => ({ ...prev, providers: null }));
        } else {
          setPanelErrors((prev) => ({ ...prev, providers: `Failed to load providers: ${provsRes.error}` }));
        }
      }
    } catch (err: unknown) {
      setRuntimeError(String(err));
    }
  }, [isOverlayWindow]);

  const loadPacksData = useCallback(async () => {
    if (isOverlayWindow) return;
    const packsRes = await invokeSafe<PetPackSummary[]>("list_pet_pack_cards");
    if (packsRes.ok) {
      setInstalledPacks(packsRes.data);
      setPanelErrors((prev) => ({ ...prev, companions: null }));
    } else {
      setPanelErrors((prev) => ({ ...prev, companions: `Failed to load pet packs: ${packsRes.error}` }));
    }
  }, [isOverlayWindow]);

  useEffect(() => {
    void refreshCoreData();
    const interval = setInterval(refreshCoreData, SNAPSHOT_POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [refreshCoreData]);

  useEffect(() => {
    if (!isOverlayWindow) {
      void loadPacksData();
    }
  }, [isOverlayWindow, loadPacksData]);

  // Load tab-specific data
  const loadSessionsData = useCallback(async () => {
    const res = await invokeSafe<AgentSessionSummary[]>("get_session_history", { limit: 20, offset: 0 });
    if (res.ok) {
      setSessions(res.data);
      setPanelErrors((prev) => ({ ...prev, sessions: null }));
    } else {
      setPanelErrors((prev) => ({ ...prev, sessions: `Failed to load session history: ${res.error}` }));
    }
  }, []);

  const loadProgressData = useCallback(async () => {
    const [xpRes, achRes, verRes] = await Promise.all([
      invokeSafe<XpTransaction[]>("get_xp_history"),
      invokeSafe<AchievementSummary[]>("get_achievements"),
      invokeSafe<LedgerVerificationResult>("verify_ledger"),
    ]);
    if (xpRes.ok) {
      setXpHistory(xpRes.data);
      setPanelErrors((prev) => ({ ...prev, progress: null }));
    } else {
      setPanelErrors((prev) => ({ ...prev, progress: `Failed to load XP history: ${xpRes.error}` }));
    }
    if (achRes.ok) {
      setAchievements(achRes.data);
    }
    if (verRes.ok) {
      setLedgerVerification(verRes.data);
    } else {
      setPanelErrors((prev) => ({ ...prev, progress: `Ledger verification failed: ${verRes.error}` }));
    }
  }, []);

  useEffect(() => {
    if (activeTab === "sessions") {
      void loadSessionsData();
    } else if (activeTab === "progress") {
      void loadProgressData();
    } else if (activeTab === "companions") {
      void loadPacksData();
    }
  }, [activeTab, loadSessionsData, loadProgressData, loadPacksData]);

  const retryPanel = useCallback(
    (panelKey: string) => {
      setPanelErrors((prev) => ({ ...prev, [panelKey]: null }));
      if (panelKey === "sessions") {
        void loadSessionsData();
      } else if (panelKey === "progress") {
        void loadProgressData();
      } else {
        void refreshCoreData();
      }
    },
    [loadSessionsData, loadProgressData, refreshCoreData]
  );

  const renderPanelError = (panelKey: string) => {
    const error = panelErrors[panelKey];
    if (!error) return null;
    return (
      <div
        className="edge shadow-pop-sm"
        style={{
          background: "#ff2db7",
          color: "#ffffff",
          padding: "12px 18px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "16px",
          fontFamily: "'Space Grotesk', sans-serif",
          fontWeight: 700,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "10px", fontSize: "13px" }}>
          <span style={{ fontSize: "16px" }}>⚠️</span>
          <span>{error}</span>
        </div>
        <button
          onClick={() => retryPanel(panelKey)}
          className="shadow-pop-sm shadow-pop-hover"
          style={{
            background: "#0a0a0a",
            color: "#ffffff",
            border: "2px solid #0a0a0a",
            borderRadius: "999px",
            padding: "4px 14px",
            fontSize: "11px",
            fontWeight: 800,
            textTransform: "uppercase",
            cursor: "pointer",
          }}
        >
          Retry
        </button>
      </div>
    );
  };

  // Actions
  const handleVerifyLedger = useCallback(async () => {
    setLedgerVerifying(true);
    try {
      const result = await invokeSafe<LedgerVerificationResult>("verify_ledger");
      if (result.ok) {
        setLedgerVerification(result.data);
        setPanelErrors((prev) => ({ ...prev, progress: null }));
      } else {
        setPanelErrors((prev) => ({ ...prev, progress: `Ledger verification failed: ${result.error}` }));
        setRuntimeError(`Ledger verification failed: ${result.error}`);
      }
    } finally {
      setLedgerVerifying(false);
    }
  }, []);

  const handleToggleSimulated = useCallback(async () => {
    try {
      const next = await invokeSafe<boolean>("toggle_demo_mode");
      if (next.ok) {
        await refreshCoreData();
      } else {
        setRuntimeError(`Toggle simulation failed: ${next.error}`);
      }
    } catch (err: unknown) {
      setRuntimeError(String(err));
    }
  }, [refreshCoreData]);

  const handleSimulateEvent = useCallback(
    async (eventType: string) => {
      try {
        const res = await invokeSafe("simulate_agent_event", { eventType });
        if (res.ok) {
          await refreshCoreData();
        } else {
          setRuntimeError(`Simulate event failed: ${res.error}`);
        }
      } catch (err: unknown) {
        setRuntimeError(String(err));
      }
    },
    [refreshCoreData]
  );

  const handleSelectPet = useCallback(
    async (selectionId: string) => {
      try {
        const res = await invokeSafe("select_pet", { packId: selectionId });
        if (res.ok) {
          await refreshCoreData();
        } else {
          setRuntimeError(`Select pet failed: ${res.error}`);
        }
      } catch (err: unknown) {
        setRuntimeError(String(err));
      }
    },
    [refreshCoreData]
  );

  const handleTogglePause = useCallback(async () => {
    setBusy(true);
    try {
      const updated = await invokeSafe<RuntimeSnapshot>("toggle_pet_pause");
      if (updated.ok) {
        setSnapshot(updated.data);
      } else {
        setRuntimeError(`Toggle pause failed: ${updated.error}`);
      }
    } catch (err: unknown) {
      setRuntimeError(String(err));
    } finally {
      setBusy(false);
    }
  }, []);

  const handleImportPack = useCallback(async () => {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        filters: [{ name: "PetPack Archive", extensions: ["tar.gz", "tgz", "tar"] }],
      });
      if (selected && typeof selected === "string") {
        const res = await invokeSafe("install_pet_pack", { sourcePath: selected });
        if (res.ok) {
          await Promise.all([refreshCoreData(), loadPacksData()]);
        } else {
          setRuntimeError(`Pack install failed: ${res.error}`);
        }
      }
    } catch (err: unknown) {
      setRuntimeError(String(err));
    }
  }, [refreshCoreData, loadPacksData]);

  const activePetName = selectedAnimation?.petName ?? "Aoi";
  const claudeProvider = providers.find((p) => p.providerId === "claude_code");
  const quotaWindow = claudeProvider?.quotaStatus?.fiveHour;
  const quotaProvenance = claudeProvider?.quotaProvenance ?? "N/A";

  // Real today metrics computed from real local evidence
  const totalCostToday = claudeProvider?.costUsd ?? null;
  const totalInTokensToday = claudeProvider?.inputTokens ?? null;
  const totalOutTokensToday = claudeProvider?.outputTokens ?? null;
  const totalTokensToday =
    totalInTokensToday !== null || totalOutTokensToday !== null
      ? (totalInTokensToday ?? 0) + (totalOutTokensToday ?? 0)
      : null;

  const [soundMuted, setSoundMuted] = useState<boolean>(() => {
    return localStorage.getItem("agentpet_sound_muted") === "true";
  });

  const toggleSound = () => {
    const next = !soundMuted;
    setSoundMuted(next);
    localStorage.setItem("agentpet_sound_muted", String(next));
  };

  const handleNextPet = useCallback(async () => {
    if (installedPacks.length <= 1) return;
    const currentId = snapshot.selectedPetId;
    const currentIndex = installedPacks.findIndex(
      (p) => p.selectionId === currentId || p.id === currentId
    );
    const nextIndex = (currentIndex + 1) % installedPacks.length;
    const nextPack = installedPacks[nextIndex];
    if (nextPack) {
      await handleSelectPet(nextPack.selectionId);
    }
  }, [installedPacks, snapshot.selectedPetId, handleSelectPet]);

  const handleWakePet = useCallback(async () => {
    await refreshCoreData();
  }, [refreshCoreData]);

  // -------------------------------------------------------------
  // RENDER: Overlay Window
  // -------------------------------------------------------------
  if (isOverlayWindow) {
    return (
      <main
        className="agentpet-overlay-window"
        style={{
          width: "100vw",
          height: "100vh",
          backgroundColor: "transparent",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "6px",
          boxSizing: "border-box",
          overflow: "hidden",
        }}
      >
        {/* Overlay Draggable Titlebar */}
        <div
          className="edge shadow-pop-sm"
          style={{
            width: "100%",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            background: "#ffffff",
            padding: "6px 10px",
            cursor: "grab",
            fontSize: "11px",
            color: "#0a0a0a",
            boxSizing: "border-box",
            fontFamily: "'Space Grotesk', sans-serif",
          }}
          onMouseDown={() => {
            void getCurrentWindow()
              .startDragging()
              .catch((err) => {
                setRuntimeError(`Failed to drag window: ${err}`);
              });
          }}
        >
          <span style={{ fontWeight: 900, color: "#0a0a0a", letterSpacing: "0.5px" }}>
            AGENTPET {isSimulated ? "· SIMULATED" : ""}
          </span>
          <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
            <button
              onClick={toggleSound}
              aria-label={soundMuted ? "Unmute sounds" : "Mute sounds"}
              title={soundMuted ? "Unmute Sounds" : "Mute Sounds"}
              style={{
                background: soundMuted ? "#ffffff" : "#ffd000",
                border: "2px solid #0a0a0a",
                borderRadius: "4px",
                color: "#0a0a0a",
                fontSize: "11px",
                padding: "2px 5px",
                cursor: "pointer",
                fontWeight: 800,
              }}
            >
              {soundMuted ? "🔇" : "🔊"}
            </button>
            <button
              onClick={() => setIsCommandDeckOpen(true)}
              aria-label="Open Command Deck"
              title="Open Command Deck"
              style={{
                background: "#ffd000",
                border: "2px solid #0a0a0a",
                borderRadius: "4px",
                color: "#0a0a0a",
                fontSize: "11px",
                padding: "2px 5px",
                cursor: "pointer",
                fontWeight: 900,
              }}
            >
              ⚡
            </button>
            <button
              onClick={handleTogglePause}
              disabled={busy}
              aria-label={snapshot.petState === "paused" ? "Resume companion" : "Pause companion"}
              title={snapshot.petState === "paused" ? "Resume" : "Pause"}
              style={{
                background: "#00d4ff",
                border: "2px solid #0a0a0a",
                borderRadius: "4px",
                color: "#0a0a0a",
                fontSize: "10px",
                padding: "2px 6px",
                cursor: "pointer",
                fontWeight: 900,
              }}
            >
              {snapshot.petState === "paused" ? "▶" : "Ⅱ"}
            </button>
            <button
              onClick={() => void getCurrentWindow().hide()}
              aria-label="Hide overlay window"
              title="Hide Overlay"
              style={{
                background: "#ff2db7",
                border: "2px solid #0a0a0a",
                borderRadius: "4px",
                color: "#ffffff",
                fontSize: "11px",
                padding: "2px 6px",
                cursor: "pointer",
                fontWeight: 900,
              }}
            >
              ×
            </button>
          </div>
        </div>

        {/* Center Companion Stage with Unified Status & Controls */}
        <CompanionRenderer
          state={snapshot.petState}
          animation={selectedAnimation}
          avatar={avatar}
          hud={avatarHud}
          quotaWindow={quotaWindow}
          quotaProvenance={quotaProvenance}
          size={180}
          mode={renderMode}
          onModeToggle={setRenderMode}
          interactive={true}
          onDoubleClick={() => setIsCommandDeckOpen(true)}
          onNextPet={handleNextPet}
          onWakePet={handleWakePet}
          onError={setRuntimeError}
        />

        {/* Command Deck Popover */}
        <CommandDeck
          isOpen={isCommandDeckOpen}
          onClose={() => setIsCommandDeckOpen(false)}
          providers={providers}
          activePetName={activePetName}
          activeState={snapshot.petState}
          isSimulated={isSimulated}
          onToggleSimulated={handleToggleSimulated}
          onSelectPet={handleSelectPet}
          installedPacks={installedPacks}
          onVerifyLedger={handleVerifyLedger}
          selectedPetId={snapshot.selectedPetId}
          ledgerVerification={ledgerVerification}
        />
      </main>
    );
  }

  // -------------------------------------------------------------
  // RENDER: Full Desktop Application Window
  // -------------------------------------------------------------
  return (
    <div
      className="grain"
      style={{
        backgroundColor: "#ffffff",
        color: "#0a0a0a",
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        fontFamily: "'Archivo', sans-serif",
      }}
    >
      {/* SIMULATED MODE BANNER (Mandatory Ground Rule 1) */}
      {isSimulated && (
        <div
          style={{
            backgroundColor: "#ffd000",
            color: "#0a0a0a",
            fontWeight: 900,
            fontSize: "12px",
            letterSpacing: "0.08em",
            padding: "8px 24px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            borderBottom: "4px solid #0a0a0a",
            fontFamily: "'Space Grotesk', sans-serif",
            textTransform: "uppercase",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <span style={{ fontSize: "16px" }}>⚠️</span>
            <span>SIMULATED DEMO MODE ACTIVE · DB: agentpet_simulated.sqlite3 · ZERO REAL XP AWARDED</span>
          </div>
          <button
            onClick={handleToggleSimulated}
            className="shadow-pop-sm shadow-pop-hover"
            style={{
              background: "#0a0a0a",
              color: "#ffffff",
              padding: "4px 14px",
              border: "2px solid #0a0a0a",
              borderRadius: "999px",
              fontSize: "11px",
              fontWeight: 800,
              cursor: "pointer",
            }}
          >
            EXIT TO LIVE
          </button>
        </div>
      )}

      {/* Runtime Diagnostics Banner */}
      {runtimeError && (
        <div
          style={{
            backgroundColor: "#ff2db7",
            color: "#ffffff",
            padding: "8px 24px",
            fontSize: "12px",
            fontWeight: 800,
            fontFamily: "'Space Grotesk', sans-serif",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            borderBottom: "4px solid #0a0a0a",
          }}
        >
          <span>🚨 RUNTIME DIAGNOSTIC: {runtimeError}</span>
          <button
            onClick={() => setRuntimeError(null)}
            style={{
              background: "#0a0a0a",
              color: "#ffffff",
              border: "2px solid #ffffff",
              padding: "2px 8px",
              borderRadius: "4px",
              fontWeight: 900,
              cursor: "pointer",
            }}
          >
            ✕
          </button>
        </div>
      )}

      {/* Sticky White Nav */}
      <header
        style={{
          position: "sticky",
          top: 0,
          zIndex: 50,
          backgroundColor: "#ffffff",
          borderBottom: "4px solid #0a0a0a",
        }}
      >
        <div
          style={{
            maxWidth: "1400px",
            margin: "0 auto",
            padding: "0 24px",
            height: "72px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          {/* Brand Left: 3-bar logo glyph + display wordmark */}
          <div style={{ display: "flex", alignItems: "center", gap: "14px" }}>
            <div style={{ display: "flex", gap: "4px", alignItems: "center" }}>
              <span
                style={{
                  width: "10px",
                  height: "26px",
                  backgroundColor: "#ff2db7",
                  border: "2px solid #0a0a0a",
                  display: "inline-block",
                  boxShadow: "2px 2px 0 #0a0a0a",
                }}
              />
              <span
                style={{
                  width: "10px",
                  height: "26px",
                  backgroundColor: "#00d4ff",
                  border: "2px solid #0a0a0a",
                  display: "inline-block",
                  boxShadow: "2px 2px 0 #0a0a0a",
                }}
              />
              <span
                style={{
                  width: "10px",
                  height: "26px",
                  backgroundColor: "#ffd000",
                  border: "2px solid #0a0a0a",
                  display: "inline-block",
                  boxShadow: "2px 2px 0 #0a0a0a",
                }}
              />
            </div>
            <div>
              <div className="display" style={{ fontSize: "22px", color: "#0a0a0a", lineHeight: 1 }}>
                AGENTPET
              </div>
              <div
                className="font-grotesk"
                style={{
                  fontSize: "10px",
                  color: "#0a0a0a",
                  fontWeight: 800,
                  letterSpacing: "0.15em",
                  textTransform: "uppercase",
                  marginTop: "2px",
                }}
              >
                COMPANION · <span style={{ color: "#ff2db7" }}>{activePetName}</span>
              </div>
            </div>
          </div>

          {/* Navigation Center */}
          <nav style={{ display: "flex", gap: "8px" }}>
            {(
              [
                { id: "den", label: "Den", icon: "🏛️" },
                { id: "sessions", label: "Sessions", icon: "📊" },
                { id: "companions", label: "Companions", icon: "✨" },
                { id: "progress", label: "Progress", icon: "🔒" },
                { id: "settings", label: "Settings", icon: "⚙️" },
              ] as const
            ).map((tab) => {
              const isActive = activeTab === tab.id;
              return (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                  className="font-grotesk shadow-pop-hover"
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "6px",
                    padding: "8px 16px",
                    borderRadius: "999px",
                    fontSize: "13px",
                    fontWeight: 800,
                    textTransform: "uppercase",
                    letterSpacing: "0.08em",
                    cursor: "pointer",
                    backgroundColor: isActive ? "#00d4ff" : "#ffffff",
                    color: "#0a0a0a",
                    border: "3px solid #0a0a0a",
                    boxShadow: isActive ? "4px 4px 0 #0a0a0a" : "none",
                  }}
                >
                  <span>{tab.icon}</span>
                  <span>{tab.label}</span>
                </button>
              );
            })}
          </nav>

          {/* Header Right Actions */}
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <button
              onClick={toggleSound}
              className="font-grotesk shadow-pop-hover"
              style={{
                width: "40px",
                height: "40px",
                borderRadius: "50%",
                backgroundColor: soundMuted ? "#ffffff" : "#ffd000",
                border: "3px solid #0a0a0a",
                boxShadow: "3px 3px 0 #0a0a0a",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: "15px",
                cursor: "pointer",
              }}
              title={soundMuted ? "Unmute Audio" : "Mute Audio"}
            >
              {soundMuted ? "🔇" : "🔊"}
            </button>

            <button
              onClick={() => setIsCommandDeckOpen(true)}
              className="font-grotesk shadow-pop-hover"
              style={{
                padding: "10px 18px",
                backgroundColor: "#0a0a0a",
                color: "#ffffff",
                border: "3px solid #0a0a0a",
                borderRadius: "999px",
                boxShadow: "4px 4px 0 #0a0a0a",
                fontSize: "12px",
                fontWeight: 900,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: "6px",
              }}
            >
              <span style={{ color: "#ffd000" }}>⚡</span>
              <span>COMMAND DECK</span>
            </button>

            <button
              onClick={() => void invoke("show_pet_overlay").catch((err) => setRuntimeError(`Failed to show companion window: ${err}`))}
              className="font-grotesk shadow-pop-hover"
              style={{
                padding: "10px 16px",
                backgroundColor: "#ff2db7",
                color: "#ffffff",
                border: "3px solid #0a0a0a",
                borderRadius: "999px",
                boxShadow: "4px 4px 0 #0a0a0a",
                fontSize: "12px",
                fontWeight: 900,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                cursor: "pointer",
              }}
            >
              DESK OVERLAY
            </button>
          </div>
        </div>
      </header>

      {/* Slogan Marquee (Forward) */}
      <div
        className="marquee-mask"
        style={{
          backgroundColor: "#0a0a0a",
          color: "#ffffff",
          padding: "12px 0",
          borderBottom: "4px solid #0a0a0a",
        }}
      >
        <div className="marquee-strip">
          {[1, 2].map((idx) => (
            <div
              key={idx}
              aria-hidden={idx === 2}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "28px",
                paddingRight: "28px",
              }}
            >
              <span className="display" style={{ fontSize: "22px" }}>
                ZERO FABRICATED METRICS
              </span>
              <span style={{ color: "#ff2db7", fontSize: "24px", fontWeight: 900 }}>✦</span>
              <span className="display" style={{ fontSize: "22px", color: "#00d4ff" }}>
                TAMPER-EVIDENT LEDGER
              </span>
              <span style={{ color: "#ffd000", fontSize: "24px", fontWeight: 900 }}>✦</span>
              <span className="display" style={{ fontSize: "22px" }}>
                3D PROCEDURAL RIG
              </span>
              <span style={{ color: "#ff2db7", fontSize: "24px", fontWeight: 900 }}>✦</span>
              <span className="display" style={{ fontSize: "22px", color: "#ffd000" }}>
                TAURI 2 & RUST DOMAIN
              </span>
              <span style={{ color: "#00d4ff", fontSize: "24px", fontWeight: 900 }}>✦</span>
              <span className="display" style={{ fontSize: "22px" }}>
                LOCAL-FIRST ARCHITECTURE
              </span>
              <span style={{ color: "#ff2db7", fontSize: "24px", fontWeight: 900 }}>✦</span>
            </div>
          ))}
        </div>
      </div>

      {/* Secondary Client / Agent Marquee (Reverse) */}
      <div
        className="marquee-mask"
        style={{
          backgroundColor: "#ffd000",
          color: "#0a0a0a",
          padding: "8px 0",
          borderBottom: "4px solid #0a0a0a",
        }}
      >
        <div className="marquee-rev-strip">
          {[1, 2].map((idx) => (
            <div
              key={idx}
              aria-hidden={idx === 2}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "24px",
                paddingRight: "24px",
              }}
            >
              {[
                "CLAUDE CODE",
                "•",
                "COPILOT",
                "•",
                "CURSOR",
                "•",
                "CODEX",
                "•",
                "OPENAI",
                "•",
                "AOI",
                "•",
                "MIRA",
                "•",
                "ROWAN",
                "•",
                "HANA",
                "•",
                "IVO",
                "•",
                "TETSU",
                "•",
                "SUMIRE",
                "•",
                "MOMIJI",
              ].map((token, i) => (
                <span
                  key={i}
                  className="font-grotesk"
                  style={{
                    fontSize: "14px",
                    fontWeight: 900,
                    textTransform: "uppercase",
                    letterSpacing: "0.15em",
                    color: token === "•" ? "#ff2db7" : "#0a0a0a",
                  }}
                >
                  {token}
                </span>
              ))}
            </div>
          ))}
        </div>
      </div>

      {/* Main Tab Content */}
      <main
        style={{
          flex: 1,
          padding: "36px 24px",
          maxWidth: "1400px",
          width: "100%",
          margin: "0 auto",
          boxSizing: "border-box",
        }}
      >
        {/* =========================================================
            TAB 1: DEN
           ========================================================= */}
        {activeTab === "den" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
            {renderPanelError("den")}

            {/* Top Eyebrow Row */}
            <div
              className="font-grotesk"
              style={{
                display: "flex",
                flexWrap: "wrap",
                justifyContent: "space-between",
                alignItems: "center",
                fontSize: "12px",
                fontWeight: 800,
                textTransform: "uppercase",
                letterSpacing: "0.18em",
                color: "#0a0a0a",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span
                  style={{
                    width: "10px",
                    height: "10px",
                    borderRadius: "50%",
                    backgroundColor: "#ff2db7",
                    display: "inline-block",
                    boxShadow: "0 0 8px #ff2db7",
                  }}
                />
                <span>REAL-TIME COMPANION TELEMETRY · LOCAL RUNTIME</span>
              </div>
              <div>INDEPENDENT DESKTOP CORE · TAURI 2 & RUST</div>
            </div>

            {/* Hero Giant Headline */}
            <div>
              <h1
                className="display"
                style={{
                  fontSize: "clamp(44px, 7vw, 96px)",
                  color: "#0a0a0a",
                  lineHeight: 0.88,
                  margin: "8px 0 20px 0",
                }}
              >
                COMPANIONS THAT <span style={{ color: "#ff2db7" }}>REFUSE</span> TO BE{" "}
                <span className="highlight-cyan">IGNORED</span>
              </h1>
              <div
                style={{
                  display: "flex",
                  flexWrap: "wrap",
                  justifyContent: "space-between",
                  alignItems: "flex-end",
                  gap: "24px",
                }}
              >
                <p
                  className="font-grotesk"
                  style={{
                    fontSize: "18px",
                    lineHeight: 1.4,
                    color: "rgba(10, 10, 10, 0.85)",
                    maxWidth: "640px",
                    fontWeight: 600,
                    margin: 0,
                  }}
                >
                  Local desktop companion for autonomous coding agents. Zero fake data, tamper-evident SHA-256 hash
                  chaining, real process observation, and a living 3D presence that reacts to your workflow.
                </p>
                <div style={{ display: "flex", gap: "12px", flexWrap: "wrap" }}>
                  <button
                    onClick={() => setIsCommandDeckOpen(true)}
                    className="font-grotesk shadow-pop shadow-pop-hover"
                    style={{
                      backgroundColor: "#ff2db7",
                      color: "#ffffff",
                      padding: "14px 28px",
                      border: "3px solid #0a0a0a",
                      borderRadius: "999px",
                      fontSize: "14px",
                      fontWeight: 900,
                      cursor: "pointer",
                      textTransform: "uppercase",
                      letterSpacing: "0.08em",
                    }}
                  >
                    OPEN COMMAND DECK ⚡
                  </button>
                  <button
                    onClick={() => void invoke("show_pet_overlay").catch((err) => setRuntimeError(`Failed to show companion window: ${err}`))}
                    className="font-grotesk shadow-pop shadow-pop-hover"
                    style={{
                      backgroundColor: "#ffffff",
                      color: "#0a0a0a",
                      padding: "14px 24px",
                      border: "3px solid #0a0a0a",
                      borderRadius: "999px",
                      fontSize: "14px",
                      fontWeight: 900,
                      cursor: "pointer",
                      textTransform: "uppercase",
                      letterSpacing: "0.08em",
                    }}
                  >
                    DESK OVERLAY
                  </button>
                </div>
              </div>
            </div>

            {/* Stage Showcase Card */}
            <div
              className="edge-thick shadow-pop-lg"
              style={{
                backgroundColor: "#ffffff",
                padding: "40px 24px",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                position: "relative",
              }}
            >
              {/* Top-Right Stage Mode Toggle */}
              <div style={{ position: "absolute", top: "20px", right: "20px", zIndex: 10 }}>
                <button
                  type="button"
                  onClick={() => {
                    const next = renderMode === "3d" ? "2d" : "3d";
                    setRenderMode(next);
                    try {
                      localStorage.setItem("agentpet_render_mode", next);
                    } catch {}
                  }}
                  className="font-grotesk shadow-pop-sm shadow-pop-hover"
                  style={{
                    backgroundColor: renderMode === "3d" ? "#ffd000" : "#ffffff",
                    color: "#0a0a0a",
                    border: "3px solid #0a0a0a",
                    borderRadius: "999px",
                    padding: "8px 18px",
                    fontSize: "12px",
                    fontWeight: 900,
                    letterSpacing: "0.05em",
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    gap: "8px",
                  }}
                >
                  <span>{renderMode === "3d" ? "🎮 3D AVATAR ACTIVE" : "🖼️ 2D SPRITE ACTIVE"}</span>
                  <span style={{ opacity: 0.6 }}>⇄ Toggle</span>
                </button>
              </div>

              {/* Pet Renderer Stage */}
              <CompanionRenderer
                state={snapshot.petState}
                animation={selectedAnimation}
                avatar={avatar}
                hud={avatarHud}
                quotaWindow={quotaWindow}
                quotaProvenance={quotaProvenance}
                size={290}
                mode={renderMode}
                onModeToggle={setRenderMode}
                interactive={true}
                onDoubleClick={() => setIsCommandDeckOpen(true)}
                onError={setRuntimeError}
              />

              {/* Status Ribbon */}
              <div
                style={{
                  marginTop: "24px",
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  gap: "10px",
                }}
              >
                <div className="display" style={{ fontSize: "36px", color: "#0a0a0a" }}>
                  {activePetName}
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "12px", flexWrap: "wrap", justifyContent: "center" }}>
                  <span
                    className="font-grotesk shadow-pop-sm"
                    style={{
                      backgroundColor: "#00d4ff",
                      color: "#0a0a0a",
                      border: "3px solid #0a0a0a",
                      borderRadius: "999px",
                      padding: "4px 18px",
                      fontSize: "13px",
                      fontWeight: 900,
                      letterSpacing: "0.08em",
                      textTransform: "uppercase",
                    }}
                  >
                    STATE: {snapshot.petState.toUpperCase().replaceAll("_", " ")}
                  </span>
                  <span className="font-grotesk" style={{ fontSize: "12px", color: "#0a0a0a", fontWeight: 700 }}>
                    ⚡ Double-click companion to summon Command Deck
                  </span>
                </div>
              </div>
            </div>

            {/* Today Summary 4-Cell Strip */}
            <div
              className="edge-thick shadow-pop-lg"
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))",
                backgroundColor: "#0a0a0a",
                gap: "4px",
              }}
            >
              {/* 1. Real Tokens Today (Magenta) */}
              <div
                style={{
                  backgroundColor: "#ff2db7",
                  color: "#0a0a0a",
                  padding: "24px",
                  display: "flex",
                  flexDirection: "column",
                  justifyContent: "space-between",
                  minHeight: "150px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span className="font-grotesk" style={{ fontSize: "12px", fontWeight: 900, letterSpacing: "0.1em" }}>
                    TOKENS PROCESSED
                  </span>
                  <span
                    className="font-grotesk"
                    style={{
                      fontSize: "10px",
                      padding: "2px 8px",
                      backgroundColor: "#0a0a0a",
                      color: "#ffffff",
                      border: "2px solid #0a0a0a",
                      fontWeight: 900,
                    }}
                  >
                    LOCAL
                  </span>
                </div>
                <div className="display" style={{ fontSize: "38px", color: "#0a0a0a" }}>
                  {totalTokensToday !== null ? totalTokensToday.toLocaleString() : "N/A"}
                </div>
                <div className="font-grotesk" style={{ fontSize: "11px", fontWeight: 700, color: "rgba(10,10,10,0.8)" }}>
                  Computed from local Claude session logs
                </div>
              </div>

              {/* 2. Real Cost Today (Cyan) */}
              <div
                style={{
                  backgroundColor: "#00d4ff",
                  color: "#0a0a0a",
                  padding: "24px",
                  display: "flex",
                  flexDirection: "column",
                  justifyContent: "space-between",
                  minHeight: "150px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span className="font-grotesk" style={{ fontSize: "12px", fontWeight: 900, letterSpacing: "0.1em" }}>
                    ESTIMATED SESSION COST (USD)
                  </span>
                  <span
                    className="font-grotesk"
                    style={{
                      fontSize: "10px",
                      padding: "2px 8px",
                      backgroundColor: "#0a0a0a",
                      color: "#ffffff",
                      border: "2px solid #0a0a0a",
                      fontWeight: 900,
                    }}
                  >
                    LOCAL
                  </span>
                </div>
                <div className="display" style={{ fontSize: "38px", color: "#0a0a0a" }}>
                  {totalCostToday !== null ? `$${totalCostToday.toFixed(2)}` : "N/A"}
                </div>
                <div className="font-grotesk" style={{ fontSize: "11px", fontWeight: 700, color: "rgba(10,10,10,0.8)" }}>
                  Estimated ({claudeProvider?.costProvenance ?? "ESTIMATED"}) from .claude/metrics/costs.jsonl
                </div>
              </div>

              {/* 3. Real Level & Total XP (Yellow) */}
              <div
                style={{
                  backgroundColor: "#ffd000",
                  color: "#0a0a0a",
                  padding: "24px",
                  display: "flex",
                  flexDirection: "column",
                  justifyContent: "space-between",
                  minHeight: "150px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span className="font-grotesk" style={{ fontSize: "12px", fontWeight: 900, letterSpacing: "0.1em" }}>
                    PROGRESSION XP
                  </span>
                  <span
                    className="font-grotesk"
                    style={{
                      fontSize: "10px",
                      padding: "2px 8px",
                      backgroundColor: "#0a0a0a",
                      color: "#ffffff",
                      border: "2px solid #0a0a0a",
                      fontWeight: 900,
                    }}
                  >
                    LOCAL
                  </span>
                </div>
                <div className="display" style={{ fontSize: "34px", color: "#0a0a0a" }}>
                  LVL {progression?.level ?? 1} · {(progression?.totalXp ?? 0).toLocaleString()} XP
                </div>
                <div className="font-grotesk" style={{ fontSize: "11px", fontWeight: 700, color: "rgba(10,10,10,0.8)" }}>
                  Tamper-evident append-only ledger
                </div>
              </div>

              {/* 4. 5-Hour Rate Limit Quota (Ink) */}
              <div
                style={{
                  backgroundColor: "#0a0a0a",
                  color: "#ffffff",
                  padding: "24px",
                  display: "flex",
                  flexDirection: "column",
                  justifyContent: "space-between",
                  minHeight: "150px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span className="font-grotesk" style={{ fontSize: "12px", fontWeight: 900, letterSpacing: "0.1em" }}>
                    5H RATE LIMIT
                  </span>
                  <span
                    className="font-grotesk"
                    style={{
                      fontSize: "10px",
                      padding: "2px 8px",
                      backgroundColor: "#00d4ff",
                      color: "#0a0a0a",
                      border: "2px solid #00d4ff",
                      fontWeight: 900,
                    }}
                  >
                    {quotaProvenance}
                  </span>
                </div>
                <div className="display" style={{ fontSize: "38px", color: "#00d4ff" }}>
                  {quotaWindow?.usedPercentage !== undefined
                    ? `${(100 - quotaWindow.usedPercentage).toFixed(1)}% REM`
                    : "N/A"}
                </div>
                <div className="font-grotesk" style={{ fontSize: "11px", color: "rgba(255,255,255,0.7)" }}>
                  {claudeProvider?.quotaUpdatedSecondsAgo !== undefined &&
                  claudeProvider?.quotaUpdatedSecondsAgo !== null
                    ? `Updated ${claudeProvider.quotaUpdatedSecondsAgo}s ago`
                    : "No statusline hook payload received"}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* =========================================================
            TAB 2: SESSIONS & REAL EVENT STREAM
           ========================================================= */}
        {activeTab === "sessions" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
            {renderPanelError("sessions")}

            <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", flexWrap: "wrap", gap: "16px" }}>
              <div>
                <h2 className="display" style={{ fontSize: "clamp(32px, 5vw, 64px)", color: "#0a0a0a", margin: 0 }}>
                  NORMALIZED AGENT <span className="highlight-cyan">EVENTS</span>
                </h2>
                <p className="font-grotesk" style={{ fontSize: "15px", color: "#0a0a0a", fontWeight: 600, margin: "8px 0 0 0" }}>
                  Live event stream delivered to AgentPet with measured event-to-render latency. Click any row to inspect raw payload JSON.
                </p>
              </div>
              <span
                className="font-grotesk shadow-pop-sm"
                style={{
                  fontSize: "13px",
                  fontWeight: 900,
                  backgroundColor: "#ffd000",
                  color: "#0a0a0a",
                  padding: "6px 16px",
                  border: "3px solid #0a0a0a",
                  borderRadius: "999px",
                }}
              >
                TOTAL CAPTURED: {realEvents.length}
              </span>
            </div>

            {realEvents.length === 0 ? (
              <div
                className="edge-thick shadow-pop"
                style={{
                  backgroundColor: "#ffffff",
                  padding: "54px 24px",
                  textAlign: "center",
                  color: "#0a0a0a",
                }}
              >
                <div style={{ fontSize: "36px", marginBottom: "12px" }}>📡</div>
                <div className="display" style={{ fontSize: "24px" }}>
                  NO AGENT EVENTS CAPTURED YET
                </div>
                <p className="font-grotesk" style={{ fontSize: "14px", fontWeight: 600, maxWidth: "460px", margin: "12px auto 0 auto" }}>
                  Attach native hooks in Settings or run your agents via <code>agentpet run -- claude</code> to start streaming real events.
                </p>
              </div>
            ) : (
              <div className="edge-thick shadow-pop-lg" style={{ backgroundColor: "#ffffff", overflow: "hidden" }}>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "12px", textAlign: "left" }}>
                  <thead>
                    <tr
                      className="font-grotesk"
                      style={{
                        backgroundColor: "#0a0a0a",
                        color: "#ffffff",
                        borderBottom: "4px solid #0a0a0a",
                        fontWeight: 900,
                        letterSpacing: "0.08em",
                        textTransform: "uppercase",
                      }}
                    >
                      <th style={{ padding: "14px 16px" }}>EVENT ID</th>
                      <th style={{ padding: "14px 16px" }}>TYPE</th>
                      <th style={{ padding: "14px 16px" }}>PROVIDER</th>
                      <th style={{ padding: "14px 16px" }}>SOURCE</th>
                      <th style={{ padding: "14px 16px" }}>TIMESTAMP</th>
                      <th style={{ padding: "14px 16px" }}>LATENCY</th>
                      <th style={{ padding: "14px 16px" }}>ACTION</th>
                    </tr>
                  </thead>
                  <tbody>
                    {realEvents.map((evt) => (
                      <tr
                        key={evt.id}
                        style={{ borderBottom: "2px solid #0a0a0a", cursor: "pointer", transition: "background 0.1s ease" }}
                        onClick={() => setSelectedEventJson(evt.payloadJson)}
                        className="hover:bg-[#ffd000]/20"
                      >
                        <td style={{ padding: "12px 16px", fontFamily: "monospace", fontWeight: 700, color: "#0a0a0a" }}>
                          {evt.id.slice(0, 10)}...
                        </td>
                        <td style={{ padding: "12px 16px" }}>
                          <span
                            className="font-grotesk shadow-pop-sm"
                            style={{
                              padding: "4px 10px",
                              borderRadius: "4px",
                              fontSize: "11px",
                              fontWeight: 900,
                              textTransform: "uppercase",
                              border: "2px solid #0a0a0a",
                              backgroundColor: evt.eventType.includes("completed")
                                ? "#00d4ff"
                                : evt.eventType.includes("failed")
                                ? "#ff2db7"
                                : "#ffd000",
                              color: evt.eventType.includes("failed") ? "#ffffff" : "#0a0a0a",
                            }}
                          >
                            {evt.eventType.toUpperCase()}
                          </span>
                        </td>
                        <td style={{ padding: "12px 16px", fontWeight: 700, color: "#0a0a0a" }}>{evt.provider}</td>
                        <td style={{ padding: "12px 16px", color: "rgba(10,10,10,0.7)", fontWeight: 600 }}>{evt.source}</td>
                        <td style={{ padding: "12px 16px", fontWeight: 700, color: "#0a0a0a" }}>
                          {new Date(evt.occurredAt).toLocaleTimeString()}
                        </td>
                        <td style={{ padding: "12px 16px", fontFamily: "monospace", fontWeight: 800, color: "#0a0a0a" }}>
                          {evt.latencyMs !== null ? `${evt.latencyMs}ms` : "—"}
                        </td>
                        <td style={{ padding: "12px 16px" }}>
                          <button
                            className="font-grotesk shadow-pop-sm shadow-pop-hover"
                            style={{
                              background: "#0a0a0a",
                              color: "#ffffff",
                              border: "2px solid #0a0a0a",
                              padding: "4px 10px",
                              borderRadius: "4px",
                              fontSize: "11px",
                              fontWeight: 800,
                              textTransform: "uppercase",
                              cursor: "pointer",
                            }}
                          >
                            Inspect JSON
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {/* JSON Inspector Modal */}
            {selectedEventJson && (
              <div
                style={{
                  position: "fixed",
                  inset: 0,
                  backgroundColor: "rgba(10, 10, 10, 0.75)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  zIndex: 9999,
                  backdropFilter: "blur(2px)",
                }}
                onClick={() => setSelectedEventJson(null)}
              >
                <div
                  className="edge-thick shadow-pop-lg"
                  style={{
                    background: "#ffffff",
                    width: "85%",
                    maxWidth: "680px",
                    maxHeight: "80vh",
                    padding: "24px",
                    display: "flex",
                    flexDirection: "column",
                    gap: "16px",
                  }}
                  onClick={(e) => e.stopPropagation()}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "3px solid #0a0a0a", paddingBottom: "12px" }}>
                    <div className="display" style={{ fontSize: "20px", color: "#0a0a0a" }}>
                      RAW EVENT PAYLOAD JSON
                    </div>
                    <button
                      onClick={() => setSelectedEventJson(null)}
                      className="shadow-pop-sm shadow-pop-hover"
                      style={{
                        background: "#ff2db7",
                        color: "#ffffff",
                        border: "2px solid #0a0a0a",
                        width: "32px",
                        height: "32px",
                        borderRadius: "50%",
                        fontWeight: 900,
                        cursor: "pointer",
                      }}
                    >
                      ✕
                    </button>
                  </div>
                  <pre
                    style={{
                      background: "#0a0a0a",
                      color: "#00d4ff",
                      padding: "16px",
                      borderRadius: "0px",
                      border: "3px solid #0a0a0a",
                      overflow: "auto",
                      fontSize: "12px",
                      fontFamily: "monospace",
                      lineHeight: 1.4,
                    }}
                  >
                    {(() => {
                      try {
                        return JSON.stringify(JSON.parse(selectedEventJson), null, 2);
                      } catch {
                        return selectedEventJson;
                      }
                    })()}
                  </pre>
                </div>
              </div>
            )}

            {/* Completed Sessions History */}
            <div style={{ marginTop: "16px" }}>
              <h3 className="display" style={{ fontSize: "28px", color: "#0a0a0a", marginBottom: "12px" }}>
                COMPLETED AGENT SESSIONS
              </h3>
              {sessions.length === 0 ? (
                <div className="font-grotesk" style={{ fontSize: "13px", fontWeight: 700, color: "rgba(10,10,10,0.6)" }}>
                  No completed sessions recorded yet.
                </div>
              ) : (
                <div className="edge-thick shadow-pop" style={{ backgroundColor: "#ffffff", overflow: "hidden" }}>
                  <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "12px", textAlign: "left" }}>
                    <thead>
                      <tr
                        className="font-grotesk"
                        style={{
                          backgroundColor: "#ffd000",
                          color: "#0a0a0a",
                          borderBottom: "4px solid #0a0a0a",
                          fontWeight: 900,
                          textTransform: "uppercase",
                          letterSpacing: "0.08em",
                        }}
                      >
                        <th style={{ padding: "12px 16px" }}>SESSION ID</th>
                        <th style={{ padding: "12px 16px" }}>AGENT</th>
                        <th style={{ padding: "12px 16px" }}>STATUS</th>
                        <th style={{ padding: "12px 16px" }}>DURATION</th>
                        <th style={{ padding: "12px 16px" }}>TOKENS</th>
                        <th style={{ padding: "12px 16px" }}>EXIT CODE</th>
                      </tr>
                    </thead>
                    <tbody>
                      {sessions.map((s) => (
                        <tr key={s.sessionId} style={{ borderBottom: "2px solid #0a0a0a" }}>
                          <td style={{ padding: "12px 16px", fontFamily: "monospace", fontWeight: 700, color: "#0a0a0a" }}>
                            {s.sessionId.slice(0, 12)}...
                          </td>
                          <td style={{ padding: "12px 16px", fontWeight: 700, color: "#0a0a0a" }}>{s.agentId}</td>
                          <td style={{ padding: "12px 16px" }}>
                            <span
                              className="font-grotesk shadow-pop-sm"
                              style={{
                                padding: "2px 8px",
                                borderRadius: "4px",
                                fontSize: "10px",
                                fontWeight: 900,
                                border: "2px solid #0a0a0a",
                                background: s.status === "completed" ? "#00d4ff" : "#ff2db7",
                                color: s.status === "completed" ? "#0a0a0a" : "#ffffff",
                              }}
                            >
                              {s.status.toUpperCase()}
                            </span>
                          </td>
                          <td style={{ padding: "12px 16px", fontWeight: 700, color: "#0a0a0a" }}>
                            {s.durationMs !== null ? `${(s.durationMs / 1000).toFixed(1)}s` : "—"}
                          </td>
                          <td style={{ padding: "12px 16px", fontWeight: 800, color: "#ff2db7" }}>
                            {((s.inputTokens || 0) + (s.outputTokens || 0)).toLocaleString()}
                          </td>
                          <td style={{ padding: "12px 16px", fontFamily: "monospace", fontWeight: 900, color: s.exitCode === 0 ? "#0a0a0a" : "#ff2db7" }}>
                            {s.exitCode !== null ? s.exitCode : "—"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          </div>
        )}

        {/* =========================================================
            TAB 3: COMPANIONS
           ========================================================= */}
        {activeTab === "companions" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "32px" }}>
            {renderPanelError("avatar")}
            <div>
              <h2 className="display" style={{ fontSize: "clamp(32px, 5vw, 64px)", color: "#0a0a0a", margin: "0 0 8px 0" }}>
                3D CHARACTER <span className="highlight-magenta">STUDIO</span>
              </h2>
              <p className="font-grotesk" style={{ fontSize: "15px", color: "#0a0a0a", fontWeight: 600, margin: "0 0 18px 0" }}>
                Pick one of the original 3D characters, customize it, sculpt one from your own image, or import a VRM model. 3D mode renders the character you choose here.
              </p>
              <div className="edge-thick shadow-pop-lg studio-outer" style={{ backgroundColor: "#070c18", padding: "20px" }}>
                <CharacterStudio liveState={snapshot.petState} hud={avatarHud} activeAvatar={avatar} onAvatarChanged={setAvatar} />
              </div>
            </div>

            {renderPanelError("companions")}
            <div style={{ display: "flex", alignItems: "flex-end", justifyContent: "space-between", flexWrap: "wrap", gap: "16px" }}>
              <div>
                <h2 className="display" style={{ fontSize: "clamp(28px, 4.5vw, 52px)", color: "#0a0a0a", margin: 0 }}>
                  2D SPRITE <span className="highlight-yellow">PACKS</span>
                </h2>
                <p className="font-grotesk" style={{ fontSize: "14px", color: "#0a0a0a", fontWeight: 600, margin: "6px 0 0 0" }}>
                  Verified PetPacks with pure data-driven spritesheets, shown in 2D mode. Click to activate companion.
                </p>
              </div>
              <button
                onClick={handleImportPack}
                className="font-grotesk shadow-pop shadow-pop-hover"
                style={{
                  background: "#ff2db7",
                  border: "3px solid #0a0a0a",
                  color: "#ffffff",
                  padding: "10px 20px",
                  borderRadius: "999px",
                  fontSize: "13px",
                  fontWeight: 900,
                  textTransform: "uppercase",
                  letterSpacing: "0.08em",
                  cursor: "pointer",
                }}
              >
                + Import PetPack Archive
              </button>
            </div>

            {/* Pack Cards Grid (Selected Work Tile System) */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: "24px" }}>
              {installedPacks.map((pack) => {
                const isSelected = snapshot.selectedPetId
                  ? snapshot.selectedPetId === pack.selectionId || snapshot.selectedPetId === pack.id
                  : false;
                return (
                  <div
                    key={pack.selectionId}
                    className="edge-thick shadow-pop shadow-pop-hover"
                    style={{
                      backgroundColor: "#ffffff",
                      display: "flex",
                      flexDirection: "column",
                      overflow: "hidden",
                    }}
                  >
                    {/* Abstract Image / Graphic Half */}
                    <div
                      style={{
                        width: "100%",
                        height: "170px",
                        backgroundColor: "#0a0a0a",
                        borderBottom: "4px solid #0a0a0a",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        position: "relative",
                        overflow: "hidden",
                      }}
                    >
                      {/* Diagonal background accent badge */}
                      {isSelected && (
                        <div
                          className="font-grotesk shadow-pop-sm"
                          style={{
                            position: "absolute",
                            top: "12px",
                            right: "12px",
                            backgroundColor: "#00d4ff",
                            color: "#0a0a0a",
                            fontSize: "11px",
                            fontWeight: 900,
                            padding: "4px 10px",
                            border: "2px solid #0a0a0a",
                            letterSpacing: "0.08em",
                            textTransform: "uppercase",
                            zIndex: 2,
                          }}
                        >
                          ACTIVE COMPANION
                        </div>
                      )}

                      {pack.previewDataUrl ? (
                        <img
                          src={pack.previewDataUrl}
                          alt={pack.name}
                          style={{
                            maxHeight: "100%",
                            maxWidth: "100%",
                            objectFit: "contain",
                            imageRendering: "pixelated",
                            filter: "drop-shadow(4px 4px 0 #00d4ff)",
                          }}
                        />
                      ) : (
                        <span style={{ fontSize: "56px", filter: "drop-shadow(4px 4px 0 #ff2db7)" }}>
                          {pack.name.includes("Arjuna")
                            ? "🏹"
                            : pack.name.includes("Wukong")
                            ? "👑"
                            : pack.name.includes("Ninja")
                            ? "🐱"
                            : "⚡"}
                        </span>
                      )}
                    </div>

                    {/* Content Half */}
                    <div style={{ padding: "20px", display: "flex", flexDirection: "column", gap: "12px", flex: 1, justifyContent: "space-between" }}>
                      <div>
                        <div className="font-grotesk" style={{ fontSize: "11px", fontWeight: 800, color: "#ff2db7", textTransform: "uppercase", letterSpacing: "0.15em" }}>
                          0{installedPacks.indexOf(pack) + 1} / PETPACK
                        </div>
                        <div className="display" style={{ fontSize: "24px", color: "#0a0a0a", marginTop: "4px" }}>
                          {pack.name}
                        </div>
                        {/* Structured metadata — version / author / license in a fixed-height grid
                            so every card reserves identical space and no field is ever truncated */}
                        <div
                          className="petpack-meta"
                          aria-label={`Version ${pack.version}, by ${pack.creator}, license ${pack.license}`}
                        >
                          <span className="petpack-meta__label">Version</span>
                          <span className="petpack-meta__value">v{pack.version}</span>
                          <span className="petpack-meta__label">Author</span>
                          <span className="petpack-meta__value" title={pack.creator}>{pack.creator}</span>
                          <span className="petpack-meta__label">License</span>
                          <span className="petpack-meta__value">
                            <span
                              className={`petpack-meta__license-badge ${
                                pack.license === "CC0-1.0"
                                  ? "cc0"
                                  : pack.license.startsWith("CC-BY")
                                  ? "cc-by"
                                  : "restricted"
                              }`}
                            >
                              {pack.license}
                            </span>
                          </span>
                        </div>
                      </div>

                      {/* Select Button */}
                      <button
                        onClick={() => handleSelectPet(pack.selectionId)}
                        disabled={isSelected}
                        className="font-grotesk shadow-pop-sm shadow-pop-hover"
                        style={{
                          width: "100%",
                          padding: "10px",
                          backgroundColor: isSelected ? "#00d4ff" : "#0a0a0a",
                          color: isSelected ? "#0a0a0a" : "#ffffff",
                          border: "3px solid #0a0a0a",
                          fontSize: "12px",
                          fontWeight: 900,
                          textTransform: "uppercase",
                          letterSpacing: "0.08em",
                          cursor: isSelected ? "default" : "pointer",
                        }}
                      >
                        {isSelected ? "Currently Active" : "Select Companion ->"}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* =========================================================
            TAB 4: PROGRESS & LEDGER INTEGRITY
           ========================================================= */}
        {activeTab === "progress" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
            {renderPanelError("progress")}

            {/* Tamper-Evident Ledger Proof Banner (Cyan Block) */}
            <div
              className="edge-thick shadow-pop-lg"
              style={{
                backgroundColor: ledgerVerification
                  ? ledgerVerification.isValid
                    ? "#00d4ff"
                    : "#ff4444"
                  : "#00d4ff",
                color: "#0a0a0a",
                padding: "28px",
                display: "flex",
                flexWrap: "wrap",
                alignItems: "center",
                justifyContent: "space-between",
                gap: "20px",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "18px", maxWidth: "700px" }}>
                <span style={{ fontSize: "42px", filter: "drop-shadow(3px 3px 0 #0a0a0a)" }}>
                  {ledgerVerification && !ledgerVerification.isValid ? "⚠️" : "🛡️"}
                </span>
                <div>
                  <div className="display" style={{ fontSize: "26px", color: "#0a0a0a" }}>
                    {ledgerVerification
                      ? ledgerVerification.isValid
                        ? "TAMPER-EVIDENT APPEND-ONLY LEDGER · VERIFIED"
                        : "LEDGER VERIFICATION FAILED · TAMPERING DETECTED"
                      : "TAMPER-EVIDENT APPEND-ONLY LEDGER"}
                  </div>
                  <div className="font-grotesk" style={{ fontSize: "13px", fontWeight: 700, color: "#0a0a0a", marginTop: "4px" }}>
                    Every XP transaction is cryptographically linked with SHA-256 hash chaining and protected by SQLite triggers.
                  </div>
                  {ledgerVerification && (
                    <div style={{ fontSize: "11px", fontWeight: 800, marginTop: "6px", fontFamily: "monospace", color: "#0a0a0a" }}>
                      {ledgerVerification.isValid ? (
                        <>TOTAL ENTRIES: {ledgerVerification.totalEntries} · HEAD: {ledgerVerification.headHash.slice(0, 18)}...</>
                      ) : (
                        <>BROKEN AT INDEX: {ledgerVerification.brokenAtIndex ?? "Unknown"} · ERROR: {ledgerVerification.errorMessage ?? "Hash mismatch"}</>
                      )}
                    </div>
                  )}
                </div>
              </div>
              <button
                onClick={handleVerifyLedger}
                disabled={ledgerVerifying}
                className="font-grotesk shadow-pop shadow-pop-hover"
                style={{
                  backgroundColor: "#ffd000",
                  color: "#0a0a0a",
                  fontWeight: 900,
                  fontSize: "13px",
                  textTransform: "uppercase",
                  letterSpacing: "0.08em",
                  padding: "12px 24px",
                  border: "3px solid #0a0a0a",
                  borderRadius: "999px",
                  cursor: "pointer",
                }}
              >
                {ledgerVerifying ? "Verifying Hash Chain..." : "Verify Hash Chain 🔒"}
              </button>
            </div>

            {/* XP Transactions Table */}
            <div>
              <h3 className="display" style={{ fontSize: "28px", color: "#0a0a0a", marginBottom: "12px" }}>
                RECENT IMMUTABLE TRANSACTIONS
              </h3>
              {xpHistory.length === 0 ? (
                <div className="font-grotesk" style={{ color: "rgba(10,10,10,0.6)", fontSize: "13px", fontWeight: 700 }}>
                  No XP transactions recorded yet. Complete coding sessions to earn XP.
                </div>
              ) : (
                <div className="edge-thick shadow-pop" style={{ backgroundColor: "#ffffff", overflow: "hidden" }}>
                  <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "12px", textAlign: "left" }}>
                    <thead>
                      <tr
                        className="font-grotesk"
                        style={{
                          backgroundColor: "#0a0a0a",
                          color: "#ffffff",
                          borderBottom: "4px solid #0a0a0a",
                          fontWeight: 900,
                          textTransform: "uppercase",
                          letterSpacing: "0.08em",
                        }}
                      >
                        <th style={{ padding: "12px 16px" }}>TRANSACTION ID</th>
                        <th style={{ padding: "12px 16px" }}>SOURCE</th>
                        <th style={{ padding: "12px 16px" }}>AMOUNT</th>
                        <th style={{ padding: "12px 16px" }}>CREATED AT</th>
                      </tr>
                    </thead>
                    <tbody>
                      {xpHistory.map((tx) => (
                        <tr key={tx.transactionId} style={{ borderBottom: "2px solid #0a0a0a" }}>
                          <td style={{ padding: "12px 16px", fontFamily: "monospace", fontWeight: 700, color: "#0a0a0a" }}>
                            {tx.transactionId.slice(0, 14)}...
                          </td>
                          <td style={{ padding: "12px 16px", fontWeight: 700, color: "#0a0a0a" }}>{tx.source}</td>
                          <td style={{ padding: "12px 16px", color: "#ff2db7", fontWeight: 900, fontSize: "13px" }}>
                            +{tx.amount} XP
                          </td>
                          <td style={{ padding: "12px 16px", fontWeight: 700, color: "rgba(10,10,10,0.7)" }}>
                            {new Date(tx.createdAt).toLocaleString()}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>

            {/* Achievements */}
            <div>
              <h3 className="display" style={{ fontSize: "28px", color: "#0a0a0a", marginBottom: "12px" }}>
                ACHIEVEMENTS
              </h3>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "16px" }}>
                {achievements.map((ach) => (
                  <div
                    key={ach.id}
                    className="edge-thick shadow-pop shadow-pop-hover"
                    style={{
                      backgroundColor: ach.unlockedAt ? "#ffd000" : "#ffffff",
                      padding: "18px",
                      display: "flex",
                      flexDirection: "column",
                      justifyContent: "space-between",
                      gap: "8px",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <span className="display" style={{ fontSize: "16px", color: "#0a0a0a" }}>
                        {ach.title}
                      </span>
                      <span
                        className="font-grotesk shadow-pop-sm"
                        style={{
                          fontSize: "11px",
                          color: "#ffffff",
                          backgroundColor: "#0a0a0a",
                          padding: "2px 8px",
                          fontWeight: 900,
                        }}
                      >
                        +{ach.rewardXp} XP
                      </span>
                    </div>
                    <div className="font-grotesk" style={{ fontSize: "12px", fontWeight: 600, color: "rgba(10,10,10,0.8)" }}>
                      {ach.description}
                    </div>
                    <div
                      className="font-grotesk"
                      style={{
                        fontSize: "11px",
                        fontWeight: 900,
                        color: "#0a0a0a",
                        marginTop: "4px",
                        textTransform: "uppercase",
                      }}
                    >
                      {ach.unlockedAt
                        ? `★ UNLOCKED: ${new Date(ach.unlockedAt).toLocaleDateString()}`
                        : `PROGRESS: ${ach.progress} / ${ach.threshold}`}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* =========================================================
            TAB 5: SETTINGS
           ========================================================= */}
        {activeTab === "settings" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
            {renderPanelError("settings")}

            {/* Simulation Controls (DEV ONLY) */}
            {import.meta.env.DEV && (
              <div
                className="edge-thick shadow-pop-lg"
                style={{
                  backgroundColor: "#ffd000",
                  color: "#0a0a0a",
                  padding: "24px",
                  display: "flex",
                  flexDirection: "column",
                  gap: "14px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: "12px" }}>
                  <div>
                    <div className="display" style={{ fontSize: "22px" }}>
                      SIMULATION & TEST MODE (DEV ONLY)
                    </div>
                    <div className="font-grotesk" style={{ fontSize: "12px", fontWeight: 700, marginTop: "4px" }}>
                      Isolated testing database (<code>agentpet_simulated.sqlite3</code>). Grants 0 XP to live ledger.
                    </div>
                  </div>
                  <button
                    onClick={handleToggleSimulated}
                    className="font-grotesk shadow-pop shadow-pop-hover"
                    style={{
                      padding: "10px 20px",
                      backgroundColor: isSimulated ? "#0a0a0a" : "#ffffff",
                      color: isSimulated ? "#ffffff" : "#0a0a0a",
                      border: "3px solid #0a0a0a",
                      borderRadius: "999px",
                      fontWeight: 900,
                      fontSize: "12px",
                      textTransform: "uppercase",
                      letterSpacing: "0.08em",
                      cursor: "pointer",
                    }}
                  >
                    {isSimulated ? "Simulated Mode Active" : "Enable Simulated Mode"}
                  </button>
                </div>

                {/* Event Triggers */}
                {isSimulated && (
                  <div style={{ display: "flex", gap: "10px", flexWrap: "wrap", marginTop: "8px" }}>
                    {(
                      [
                        { id: "started", label: "Agent Started" },
                        { id: "working", label: "Agent Working" },
                        { id: "thinking", label: "Agent Thinking" },
                        { id: "needs_input", label: "Input Needed" },
                        { id: "completed", label: "Agent Completed" },
                        { id: "failed", label: "Agent Failed" },
                      ] as const
                    ).map((btn) => (
                      <button
                        key={btn.id}
                        onClick={() => handleSimulateEvent(btn.id)}
                        className="font-grotesk shadow-pop-sm shadow-pop-hover"
                        style={{
                          padding: "8px 14px",
                          backgroundColor: "#ffffff",
                          color: "#0a0a0a",
                          border: "2px solid #0a0a0a",
                          borderRadius: "4px",
                          fontSize: "11px",
                          fontWeight: 800,
                          textTransform: "uppercase",
                          cursor: "pointer",
                        }}
                      >
                        {btn.label}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Provider Hook Manager */}
            <div
              className="edge-thick shadow-pop-lg"
              style={{
                backgroundColor: "#ffffff",
                padding: "24px",
                display: "flex",
                flexDirection: "column",
                gap: "14px",
              }}
            >
              <div className="display" style={{ fontSize: "24px", color: "#0a0a0a" }}>
                CODING AGENT PROVIDER ADAPTERS
              </div>
              <div className="font-grotesk" style={{ fontSize: "13px", fontWeight: 600, color: "rgba(10,10,10,0.7)" }}>
                Inspect real machine paths and hook configurations. Documented in <code>docs/PROVIDERS.md</code>.
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "12px", marginTop: "8px" }}>
                {providers.map((p) => (
                  <div
                    key={p.providerId}
                    className="edge shadow-pop-sm"
                    style={{
                      backgroundColor: "#ffffff",
                      padding: "14px 18px",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      gap: "12px",
                    }}
                  >
                    <div>
                      <div className="font-grotesk" style={{ fontWeight: 900, fontSize: "14px", color: "#0a0a0a" }}>
                        {p.displayName}
                      </div>
                      <div className="font-grotesk" style={{ fontSize: "11px", color: "rgba(10,10,10,0.6)", marginTop: "2px", fontWeight: 600 }}>
                        Config: {p.configPath ?? "Default directory"}
                      </div>
                    </div>
                    <span
                      className="font-grotesk shadow-pop-sm"
                      style={{
                        fontSize: "11px",
                        fontWeight: 900,
                        padding: "4px 12px",
                        borderRadius: "999px",
                        backgroundColor: p.detected ? "#00d4ff" : "#ffffff",
                        color: "#0a0a0a",
                        border: "2px solid #0a0a0a",
                        textTransform: "uppercase",
                        letterSpacing: "0.08em",
                      }}
                    >
                      {p.detected ? "DETECTED & SUPPORTED" : "NOT FOUND"}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Closing CTA Section (Cyan Closing CTA) */}
      <section
        style={{
          backgroundColor: "#00d4ff",
          borderTop: "4px solid #0a0a0a",
          borderBottom: "4px solid #0a0a0a",
          position: "relative",
          overflow: "hidden",
          padding: "72px 24px",
          textAlign: "center",
        }}
      >
        {/* Decorative Floating Shapes */}
        <div
          style={{
            position: "absolute",
            top: "-40px",
            right: "5%",
            width: "160px",
            height: "160px",
            borderRadius: "50%",
            backgroundColor: "#ff2db7",
            border: "4px solid #0a0a0a",
            boxShadow: "8px 8px 0 #0a0a0a",
            pointerEvents: "none",
          }}
        />
        <div
          style={{
            position: "absolute",
            bottom: "20px",
            left: "5%",
            width: "80px",
            height: "80px",
            backgroundColor: "#ffd000",
            border: "4px solid #0a0a0a",
            transform: "rotate(14deg)",
            boxShadow: "6px 6px 0 #0a0a0a",
            pointerEvents: "none",
          }}
        />

        <div style={{ maxWidth: "800px", margin: "0 auto", position: "relative", zIndex: 1 }}>
          <div
            className="font-grotesk"
            style={{
              fontSize: "13px",
              fontWeight: 900,
              textTransform: "uppercase",
              letterSpacing: "0.2em",
              color: "#0a0a0a",
              marginBottom: "12px",
            }}
          >
            LOCAL-FIRST · AUTONOMOUS AGENT COMPANION
          </div>
          <h2
            className="display"
            style={{
              fontSize: "clamp(36px, 6vw, 76px)",
              color: "#0a0a0a",
              lineHeight: 0.9,
              margin: "0 0 28px 0",
            }}
          >
            GOT AN AGENT THAT NEEDS TO <span className="highlight-magenta">SHIP?</span>
          </h2>
          <div style={{ display: "flex", justifyContent: "center", gap: "16px", flexWrap: "wrap" }}>
            <button
              onClick={() => setIsCommandDeckOpen(true)}
              className="font-grotesk shadow-pop shadow-pop-hover"
              style={{
                backgroundColor: "#0a0a0a",
                color: "#ffffff",
                padding: "16px 36px",
                borderRadius: "999px",
                border: "3px solid #0a0a0a",
                fontSize: "14px",
                fontWeight: 900,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <span style={{ color: "#ffd000" }}>⚡</span>
              <span>LAUNCH COMMAND DECK</span>
            </button>
            <button
              onClick={() => void invoke("show_pet_overlay").catch((err) => setRuntimeError(`Failed to show companion window: ${err}`))}
              className="font-grotesk shadow-pop shadow-pop-hover"
              style={{
                backgroundColor: "#ffffff",
                color: "#0a0a0a",
                padding: "16px 32px",
                borderRadius: "999px",
                border: "3px solid #0a0a0a",
                fontSize: "14px",
                fontWeight: 900,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                cursor: "pointer",
              }}
            >
              SPAWN DESK OVERLAY
            </button>
          </div>
        </div>
      </section>

      {/* Neo-Brutalist 4-Column Footer */}
      <footer
        style={{
          backgroundColor: "#0a0a0a",
          color: "#ffffff",
          borderTop: "4px solid #0a0a0a",
          padding: "54px 24px 32px 24px",
        }}
      >
        <div
          style={{
            maxWidth: "1400px",
            margin: "0 auto",
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
            gap: "36px",
            paddingBottom: "40px",
            borderBottom: "2px solid rgba(255, 255, 255, 0.15)",
          }}
        >
          {/* Col 1: Brand & Bio */}
          <div>
            <div style={{ display: "flex", gap: "4px", alignItems: "center", marginBottom: "12px" }}>
              <span style={{ width: "8px", height: "20px", backgroundColor: "#ff2db7", border: "1px solid #ffffff", display: "inline-block" }} />
              <span style={{ width: "8px", height: "20px", backgroundColor: "#00d4ff", border: "1px solid #ffffff", display: "inline-block" }} />
              <span style={{ width: "8px", height: "20px", backgroundColor: "#ffd000", border: "1px solid #ffffff", display: "inline-block" }} />
              <span className="display" style={{ fontSize: "20px", color: "#ffffff", marginLeft: "6px" }}>
                AGENTPET
              </span>
            </div>
            <p className="font-grotesk" style={{ fontSize: "13px", color: "rgba(255, 255, 255, 0.6)", lineHeight: 1.5, margin: 0 }}>
              Maximalist, local-first companion observing AI coding agents in real-time. Zero fake telemetry, tamper-evident ledger proofs, and living rigs.
            </p>
          </div>

          {/* Col 2: Navigation */}
          <div>
            <div className="font-grotesk" style={{ fontSize: "12px", fontWeight: 900, textTransform: "uppercase", letterSpacing: "0.15em", color: "#00d4ff", marginBottom: "14px" }}>
              WORKSPACE
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
              {["Den", "Sessions", "Companions", "Progress", "Settings"].map((item) => (
                <button
                  key={item}
                  onClick={() => setActiveTab(item.toLowerCase() as any)}
                  className="font-grotesk"
                  style={{
                    background: "none",
                    border: "none",
                    padding: 0,
                    textAlign: "left",
                    color: "rgba(255,255,255,0.7)",
                    fontSize: "13px",
                    fontWeight: 700,
                    cursor: "pointer",
                  }}
                >
                  {item}
                </button>
              ))}
            </div>
          </div>

          {/* Col 3: Engine */}
          <div>
            <div className="font-grotesk" style={{ fontSize: "12px", fontWeight: 900, textTransform: "uppercase", letterSpacing: "0.15em", color: "#ffd000", marginBottom: "14px" }}>
              ARCHITECTURE
            </div>
            <div className="font-grotesk" style={{ display: "flex", flexDirection: "column", gap: "6px", fontSize: "13px", color: "rgba(255, 255, 255, 0.7)", fontWeight: 600 }}>
              <div>Tauri 2 Desktop Shell</div>
              <div>Rust Domain Crates</div>
              <div>Append-Only SQLite Hash Chain</div>
              <div>Three.js Procedural Rigging</div>
              <div>Pure Data-Driven PetPacks</div>
            </div>
          </div>

          {/* Col 4: Telemetry */}
          <div>
            <div className="font-grotesk" style={{ fontSize: "12px", fontWeight: 900, textTransform: "uppercase", letterSpacing: "0.15em", color: "#ff2db7", marginBottom: "14px" }}>
              TELEMETRY PROOF
            </div>
            <div className="font-grotesk" style={{ fontSize: "13px", color: "rgba(255, 255, 255, 0.7)", lineHeight: 1.5 }}>
              Status: <span style={{ color: "#00d4ff", fontWeight: 900 }}>LOCAL & LIVE</span><br />
              Active Companion: <span style={{ color: "#ffd000", fontWeight: 900 }}>{activePetName}</span><br />
              Mode: <span style={{ color: "#ff2db7", fontWeight: 900 }}>{renderMode.toUpperCase()}</span>
            </div>
          </div>
        </div>

        {/* Bottom Legal Row */}
        <div
          className="font-grotesk"
          style={{
            maxWidth: "1400px",
            margin: "24px auto 0 auto",
            display: "flex",
            flexWrap: "wrap",
            justifyContent: "space-between",
            alignItems: "center",
            fontSize: "12px",
            fontWeight: 700,
            color: "rgba(255, 255, 255, 0.4)",
          }}
        >
          <div>© 2026 AgentPet · Autonomous Coding Companion Studio</div>
          <div>Local-First · Tamper-Evident · Privacy-Preserving</div>
        </div>
      </footer>

      {/* Command Deck Popover Modal */}
      <CommandDeck
        isOpen={isCommandDeckOpen}
        onClose={() => setIsCommandDeckOpen(false)}
        providers={providers}
        activePetName={activePetName}
        activeState={snapshot.petState}
        isSimulated={isSimulated}
        onToggleSimulated={handleToggleSimulated}
        onSelectPet={handleSelectPet}
        installedPacks={installedPacks}
        onVerifyLedger={handleVerifyLedger}
        selectedPetId={snapshot.selectedPetId}
        ledgerVerification={ledgerVerification}
      />
    </div>
  );
}
