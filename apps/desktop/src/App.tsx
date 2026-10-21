import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { CompanionRenderer, PetState, SelectedPetAnimation } from "./components/CompanionRenderer";
import { CommandDeck, ProviderReport } from "./components/CommandDeck";

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
  const [isSimulated, setIsSimulated] = useState(false);

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
  const [progression, setProgression] = useState<ProgressionSnapshot | null>(null);
  const [xpHistory, setXpHistory] = useState<XpTransaction[]>([]);
  const [sessions, setSessions] = useState<AgentSessionSummary[]>([]);
  const [achievements, setAchievements] = useState<AchievementSummary[]>([]);
  const [installedPacks, setInstalledPacks] = useState<PetPackSummary[]>([]);
  const [selectedAnimation, setSelectedAnimation] = useState<SelectedPetAnimation | null>(null);
  const [realEvents, setRealEvents] = useState<StoredAgentEvent[]>([]);
  const [providers, setProviders] = useState<ProviderReport[]>([]);
  const [ledgerVerification, setLedgerVerification] = useState<LedgerVerificationResult | null>(null);
  const [selectedEventJson, setSelectedEventJson] = useState<string | null>(null);

  // UI state
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [ledgerVerifying, setLedgerVerifying] = useState(false);

  // Poll runtime data
  const refreshCoreData = useCallback(async () => {
    try {
      const snap = await invoke<RuntimeSnapshot>("get_runtime_snapshot");
      setSnapshot(snap);

      const anim = await invoke<SelectedPetAnimation | null>("get_selected_pet_animation");
      setSelectedAnimation(anim);

      // Only query heavy items in main window
      if (!isOverlayWindow) {
        const [prog, packs, events, provs] = await Promise.all([
          invoke<ProgressionSnapshot>("get_progression").catch(() => null),
          invoke<PetPackSummary[]>("list_pet_pack_cards").catch(() => []),
          invoke<StoredAgentEvent[]>("get_real_events", { limit: 25 }).catch(() => []),
          invoke<ProviderReport[]>("get_providers_status").catch(() => []),
        ]);
        if (prog) setProgression(prog);
        setInstalledPacks(packs);
        setRealEvents(events);
        setProviders(provs);
      }
    } catch (err: unknown) {
      setRuntimeError(String(err));
    }
  }, [isOverlayWindow]);

  useEffect(() => {
    void refreshCoreData();
    const interval = setInterval(refreshCoreData, SNAPSHOT_POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [refreshCoreData]);

  // Load tab-specific data
  useEffect(() => {
    if (activeTab === "sessions") {
      void invoke<AgentSessionSummary[]>("get_session_history", { limit: 20, offset: 0 })
        .then(setSessions)
        .catch(() => {});
    } else if (activeTab === "progress") {
      void Promise.all([
        invoke<XpTransaction[]>("get_xp_history").catch(() => []),
        invoke<AchievementSummary[]>("get_achievements").catch(() => []),
        invoke<LedgerVerificationResult>("verify_ledger").catch(() => null),
      ]).then(([xp, ach, ver]) => {
        setXpHistory(xp);
        setAchievements(ach);
        if (ver) setLedgerVerification(ver);
      });
    }
  }, [activeTab]);

  // Actions
  const handleVerifyLedger = useCallback(async () => {
    setLedgerVerifying(true);
    try {
      const result = await invoke<LedgerVerificationResult>("verify_ledger");
      setLedgerVerification(result);
    } catch (err: unknown) {
      setRuntimeError(String(err));
    } finally {
      setLedgerVerifying(false);
    }
  }, []);

  const handleToggleSimulated = useCallback(async () => {
    try {
      const next = await invoke<boolean>("toggle_demo_mode");
      setIsSimulated(next);
      await refreshCoreData();
    } catch (err: unknown) {
      setRuntimeError(String(err));
    }
  }, [refreshCoreData]);

  const handleSimulateEvent = useCallback(
    async (eventType: string) => {
      try {
        await invoke("simulate_agent_event", { eventType });
        setIsSimulated(true);
        await refreshCoreData();
      } catch (err: unknown) {
        setRuntimeError(String(err));
      }
    },
    [refreshCoreData]
  );

  const handleSelectPet = useCallback(
    async (selectionId: string) => {
      try {
        await invoke("select_pet", { packId: selectionId });
        await refreshCoreData();
      } catch (err: unknown) {
        setRuntimeError(String(err));
      }
    },
    [refreshCoreData]
  );

  const handleTogglePause = useCallback(async () => {
    setBusy(true);
    try {
      const updated = await invoke<RuntimeSnapshot>("toggle_pet_pause");
      setSnapshot(updated);
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
        await invoke("install_pet_pack", { sourcePath: selected });
        await refreshCoreData();
      }
    } catch (err: unknown) {
      setRuntimeError(String(err));
    }
  }, [refreshCoreData]);

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
    try {
      await invoke("simulate_agent_event", { eventType: "working" });
      await refreshCoreData();
    } catch {
      // fallback
    }
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
          style={{
            width: "100%",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            background: "rgba(10, 15, 25, 0.9)",
            border: "1px solid rgba(82, 168, 255, 0.35)",
            borderRadius: "8px",
            padding: "4px 8px",
            cursor: "grab",
            fontSize: "10px",
            color: "#94a3b8",
            backdropFilter: "blur(8px)",
            boxSizing: "border-box",
          }}
          onMouseDown={() => {
            void getCurrentWindow()
              .startDragging()
              .catch(() => {});
          }}
        >
          <span style={{ fontWeight: 800, color: "#38bdf8", letterSpacing: "0.5px" }}>
            AGENTPET {isSimulated ? "· SIMULATED" : ""}
          </span>
          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <button
              onClick={toggleSound}
              style={{
                background: "transparent",
                border: "none",
                color: soundMuted ? "#64748b" : "#38bdf8",
                fontSize: "11px",
                padding: 0,
                cursor: "pointer",
              }}
              title={soundMuted ? "Unmute Sounds" : "Mute Sounds"}
            >
              {soundMuted ? "🔇" : "🔊"}
            </button>
            <button
              onClick={() => setIsCommandDeckOpen(true)}
              style={{
                background: "transparent",
                border: "none",
                color: "#ffd700",
                fontSize: "11px",
                padding: 0,
                cursor: "pointer",
              }}
              title="Open Command Deck"
            >
              ⚡
            </button>
            <button
              onClick={handleTogglePause}
              disabled={busy}
              style={{
                background: "transparent",
                border: "none",
                color: "#cbd5e1",
                fontSize: "10px",
                padding: 0,
                cursor: "pointer",
              }}
              title={snapshot.petState === "paused" ? "Resume" : "Pause"}
            >
              {snapshot.petState === "paused" ? "▶" : "Ⅱ"}
            </button>
            <button
              onClick={() => void getCurrentWindow().hide()}
              style={{
                background: "transparent",
                border: "none",
                color: "#94a3b8",
                fontSize: "11px",
                padding: 0,
                cursor: "pointer",
              }}
              title="Hide Overlay"
            >
              ×
            </button>
          </div>
        </div>

        {/* Center Companion Stage with Unified Status & Controls */}
        <CompanionRenderer
          state={snapshot.petState}
          animation={selectedAnimation}
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
        />
      </main>
    );
  }

  // -------------------------------------------------------------
  // RENDER: Full Desktop Application Window
  // -------------------------------------------------------------
  return (
    <div
      style={{
        backgroundColor: "#070b12",
        color: "#f8fafc",
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        fontFamily: "'Inter', sans-serif",
      }}
    >
      {/* SIMULATED MODE BANNER (Mandatory Ground Rule 1) */}
      {isSimulated && (
        <div
          style={{
            backgroundColor: "#d97706",
            color: "#000000",
            fontWeight: 800,
            fontSize: "11px",
            letterSpacing: "1px",
            padding: "5px 16px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span>⚠️ SIMULATED DEMO MODE ACTIVE · SEPARATE DB (`agentpet_simulated.sqlite3`) · ZERO REAL XP AWARDED</span>
          <button
            onClick={handleToggleSimulated}
            style={{
              background: "#000000",
              color: "#ffffff",
              padding: "2px 8px",
              borderRadius: "4px",
              fontSize: "10px",
              fontWeight: 700,
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
            backgroundColor: "rgba(220, 38, 38, 0.9)",
            color: "#ffffff",
            padding: "6px 16px",
            fontSize: "11px",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span>RUNTIME DIAGNOSTIC: {runtimeError}</span>
          <button onClick={() => setRuntimeError(null)} style={{ color: "#fff", fontWeight: 700, cursor: "pointer" }}>✕</button>
        </div>
      )}

      {/* Main Top Header */}
      <header
        style={{
          height: "64px",
          borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
          background: "linear-gradient(180deg, #0e1626 0%, #080d17 100%)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0 28px",
          position: "sticky",
          top: 0,
          zIndex: 100,
        }}
      >
        {/* Brand */}
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <div
            style={{
              width: "32px",
              height: "32px",
              borderRadius: "8px",
              background: "linear-gradient(135deg, #0284c7 0%, #06b6d4 100%)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              boxShadow: "0 0 12px rgba(2, 132, 199, 0.4)",
            }}
          >
            <span style={{ fontSize: "16px" }}>⚡</span>
          </div>
          <div>
            <div style={{ fontWeight: 800, fontSize: "15px", letterSpacing: "1px" }}>AGENTPET</div>
            <div style={{ fontSize: "9px", color: "#64748b", fontWeight: 600, letterSpacing: "0.5px" }}>
              ACTIVE COMPANION: <span style={{ color: "#38bdf8" }}>{activePetName.toUpperCase()}</span>
            </div>
          </div>
        </div>

        {/* Tab Navigation */}
        <nav style={{ display: "flex", gap: "6px" }}>
          {(
            [
              { id: "den", label: "Den", icon: "🏛️" },
              { id: "sessions", label: "Sessions", icon: "📊" },
              { id: "companions", label: "Companions", icon: "✨" },
              { id: "progress", label: "Progress", icon: "🔒" },
              { id: "settings", label: "Settings", icon: "⚙️" },
            ] as const
          ).map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "6px",
                padding: "8px 16px",
                borderRadius: "6px",
                fontSize: "12px",
                fontWeight: activeTab === tab.id ? 700 : 500,
                color: activeTab === tab.id ? "#38bdf8" : "#94a3b8",
                background: activeTab === tab.id ? "rgba(56, 189, 248, 0.12)" : "transparent",
                border: activeTab === tab.id ? "1px solid rgba(56, 189, 248, 0.3)" : "1px solid transparent",
                cursor: "pointer",
                transition: "all 0.15s ease",
              }}
            >
              <span>{tab.icon}</span>
              <span>{tab.label}</span>
            </button>
          ))}
        </nav>

        {/* Header Right Actions */}
        <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
          <button
            onClick={() => setIsCommandDeckOpen(true)}
            style={{
              padding: "6px 12px",
              background: "rgba(255, 215, 0, 0.12)",
              border: "1px solid rgba(255, 215, 0, 0.35)",
              color: "#ffd700",
              borderRadius: "6px",
              fontSize: "11px",
              fontWeight: 700,
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: "5px",
            }}
          >
            <span>⚡</span>
            <span>COMMAND DECK</span>
          </button>

          <button
            onClick={() => void invoke("show_pet_overlay")}
            style={{
              padding: "6px 12px",
              background: "rgba(56, 189, 248, 0.15)",
              border: "1px solid rgba(56, 189, 248, 0.4)",
              color: "#38bdf8",
              borderRadius: "6px",
              fontSize: "11px",
              fontWeight: 600,
              cursor: "pointer",
            }}
          >
            DESK OVERLAY
          </button>
        </div>
      </header>

      {/* Main Tab Content */}
      <main style={{ flex: 1, padding: "28px", maxWidth: "1280px", width: "100%", margin: "0 auto", boxSizing: "border-box" }}>
        {/* =========================================================
            TAB 1: DEN
           ========================================================= */}
        {activeTab === "den" && (
          <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
            {/* Stage Showcase Card */}
            <div
              style={{
                background: "radial-gradient(ellipse at top, #111e38 0%, #090f1d 70%)",
                border: "1px solid rgba(82, 168, 255, 0.2)",
                borderRadius: "16px",
                padding: "36px 24px",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                position: "relative",
                boxShadow: "0 20px 50px rgba(0, 0, 0, 0.5)",
              }}
            >
              {/* Top-Right Stage Mode Toggle */}
              <div style={{ position: "absolute", top: "16px", right: "16px", zIndex: 10 }}>
                <button
                  type="button"
                  onClick={() => {
                    const next = renderMode === "3d" ? "2d" : "3d";
                    setRenderMode(next);
                    try {
                      localStorage.setItem("agentpet_render_mode", next);
                    } catch {}
                  }}
                  title={`Switch to ${renderMode === "3d" ? "2D Pixel Sprite" : "3D Living Avatar"} mode`}
                  style={{
                    background: renderMode === "3d" ? "rgba(56, 189, 248, 0.15)" : "rgba(255, 255, 255, 0.06)",
                    border: `1px solid ${renderMode === "3d" ? "#38bdf8" : "rgba(255, 255, 255, 0.15)"}`,
                    color: renderMode === "3d" ? "#38bdf8" : "#94a3b8",
                    padding: "4px 12px",
                    borderRadius: "14px",
                    fontSize: "11px",
                    fontWeight: 700,
                    letterSpacing: "0.5px",
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    gap: "5px",
                  }}
                >
                  <span>{renderMode === "3d" ? "🎮 3D AVATAR ACTIVE" : "🖼️ 2D SPRITE ACTIVE"}</span>
                  <span style={{ fontSize: "9px", opacity: 0.7 }}>⇄ Toggle</span>
                </button>
              </div>

              {/* Pet Renderer Stage */}
              <CompanionRenderer
                state={snapshot.petState}
                animation={selectedAnimation}
                quotaWindow={quotaWindow}
                quotaProvenance={quotaProvenance}
                size={270}
                mode={renderMode}
                onModeToggle={setRenderMode}
                interactive={true}
                onDoubleClick={() => setIsCommandDeckOpen(true)}
                onError={setRuntimeError}
              />

              {/* Status Ribbon */}
              <div style={{ marginTop: "16px", display: "flex", flexDirection: "column", alignItems: "center", gap: "6px" }}>
                <div style={{ fontSize: "18px", fontWeight: 800, letterSpacing: "1px", color: "#f8fafc" }}>
                  {activePetName}
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <span
                    style={{
                      background: "rgba(56, 189, 248, 0.15)",
                      color: "#38bdf8",
                      border: "1px solid rgba(56, 189, 248, 0.3)",
                      borderRadius: "12px",
                      padding: "2px 10px",
                      fontSize: "11px",
                      fontWeight: 700,
                      letterSpacing: "0.5px",
                    }}
                  >
                    STATE: {snapshot.petState.toUpperCase().replaceAll("_", " ")}
                  </span>
                  <span style={{ fontSize: "11px", color: "#64748b" }}>
                    Double-click companion to trigger Command Deck
                  </span>
                </div>
              </div>
            </div>

            {/* Today Summary Strip (Mandatory Ground Rule 2 Provenance Chips) */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "14px" }}>
              {/* 1. Real Tokens Today */}
              <div
                style={{
                  background: "#0c1322",
                  border: "1px solid rgba(255, 255, 255, 0.08)",
                  borderRadius: "10px",
                  padding: "16px",
                  display: "flex",
                  flexDirection: "column",
                  gap: "6px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontSize: "11px", color: "#64748b", fontWeight: 600 }}>TOKENS PROCESSED</span>
                  <span style={{ fontSize: "9px", padding: "1px 5px", background: "#0284c7", color: "#fff", borderRadius: "3px", fontWeight: 700 }}>
                    LOCAL
                  </span>
                </div>
                <div style={{ fontSize: "20px", fontWeight: 800, color: totalTokensToday !== null ? "#38bdf8" : "#94a3b8" }}>
                  {totalTokensToday !== null ? totalTokensToday.toLocaleString() : "N/A"}
                </div>
                <div style={{ fontSize: "10px", color: "#64748b" }}>Computed from local Claude session logs</div>
              </div>

              {/* 2. Real Cost Today */}
              <div
                style={{
                  background: "#0c1322",
                  border: "1px solid rgba(255, 255, 255, 0.08)",
                  borderRadius: "10px",
                  padding: "16px",
                  display: "flex",
                  flexDirection: "column",
                  gap: "6px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontSize: "11px", color: "#64748b", fontWeight: 600 }}>SESSION COST (USD)</span>
                  <span style={{ fontSize: "9px", padding: "1px 5px", background: "#15803d", color: "#fff", borderRadius: "3px", fontWeight: 700 }}>
                    LOCAL
                  </span>
                </div>
                <div style={{ fontSize: "20px", fontWeight: 800, color: totalCostToday !== null ? "#4ade80" : "#94a3b8" }}>
                  {totalCostToday !== null ? `$${totalCostToday.toFixed(2)}` : "N/A"}
                </div>
                <div style={{ fontSize: "10px", color: "#64748b" }}>Exact sum from `.claude/metrics/costs.jsonl`</div>
              </div>

              {/* 3. Real Level & Total XP */}
              <div
                style={{
                  background: "#0c1322",
                  border: "1px solid rgba(255, 255, 255, 0.08)",
                  borderRadius: "10px",
                  padding: "16px",
                  display: "flex",
                  flexDirection: "column",
                  gap: "6px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontSize: "11px", color: "#64748b", fontWeight: 600 }}>PROGRESSION XP</span>
                  <span style={{ fontSize: "9px", padding: "1px 5px", background: "#7c3aed", color: "#fff", borderRadius: "3px", fontWeight: 700 }}>
                    LOCAL
                  </span>
                </div>
                <div style={{ fontSize: "20px", fontWeight: 800, color: "#a855f7" }}>
                  LVL {progression?.level ?? 1} · {(progression?.totalXp ?? 0).toLocaleString()} XP
                </div>
                <div style={{ fontSize: "10px", color: "#64748b" }}>Cryptographically verified append-only ledger</div>
              </div>

              {/* 4. 5-Hour Rate Limit Quota */}
              <div
                style={{
                  background: "#0c1322",
                  border: "1px solid rgba(255, 255, 255, 0.08)",
                  borderRadius: "10px",
                  padding: "16px",
                  display: "flex",
                  flexDirection: "column",
                  gap: "6px",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ fontSize: "11px", color: "#64748b", fontWeight: 600 }}>5H RATE LIMIT</span>
                  <span
                    style={{
                      fontSize: "9px",
                      padding: "1px 5px",
                      background: quotaProvenance === "LIVE" ? "#0284c7" : "#334155",
                      color: "#fff",
                      borderRadius: "3px",
                      fontWeight: 700,
                    }}
                  >
                    {quotaProvenance}
                  </span>
                </div>
                <div style={{ fontSize: "20px", fontWeight: 800, color: quotaWindow?.usedPercentage !== undefined ? "#38bdf8" : "#94a3b8" }}>
                  {quotaWindow?.usedPercentage !== undefined ? `${(100 - quotaWindow.usedPercentage).toFixed(1)}% rem` : "N/A"}
                </div>
                <div style={{ fontSize: "10px", color: "#64748b" }}>
                  {claudeProvider?.quotaUpdatedSecondsAgo !== undefined && claudeProvider?.quotaUpdatedSecondsAgo !== null
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
          <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div>
                <h2 style={{ fontSize: "18px", fontWeight: 700, margin: 0 }}>Normalized Agent Events</h2>
                <p style={{ fontSize: "12px", color: "#64748b", margin: "4px 0 0 0" }}>
                  Live event stream delivered to AgentPet with measured event-to-render latency. Click any row to inspect raw payload JSON.
                </p>
              </div>
              <span style={{ fontSize: "11px", color: "#94a3b8", fontFamily: "monospace" }}>
                Total Captured: {realEvents.length}
              </span>
            </div>

            {realEvents.length === 0 ? (
              <div
                style={{
                  background: "#0c1322",
                  border: "1px dashed rgba(255, 255, 255, 0.15)",
                  borderRadius: "12px",
                  padding: "48px 24px",
                  textAlign: "center",
                  color: "#64748b",
                }}
              >
                <div style={{ fontSize: "28px", marginBottom: "8px" }}>📡</div>
                <div style={{ fontSize: "14px", fontWeight: 600, color: "#94a3b8" }}>No agent events captured yet</div>
                <p style={{ fontSize: "11px", maxWidth: "420px", margin: "8px auto 0 auto" }}>
                  Attach native hooks in Settings or run your agents via <code>agentpet run -- claude</code> to start streaming real events.
                </p>
              </div>
            ) : (
              <div
                style={{
                  background: "#0c1322",
                  border: "1px solid rgba(255, 255, 255, 0.08)",
                  borderRadius: "10px",
                  overflow: "hidden",
                }}
              >
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "11px", textAlign: "left" }}>
                  <thead>
                    <tr style={{ background: "rgba(255, 255, 255, 0.03)", borderBottom: "1px solid rgba(255, 255, 255, 0.08)", color: "#64748b" }}>
                      <th style={{ padding: "10px 14px" }}>EVENT ID</th>
                      <th style={{ padding: "10px 14px" }}>TYPE</th>
                      <th style={{ padding: "10px 14px" }}>PROVIDER</th>
                      <th style={{ padding: "10px 14px" }}>SOURCE</th>
                      <th style={{ padding: "10px 14px" }}>TIMESTAMP</th>
                      <th style={{ padding: "10px 14px" }}>LATENCY</th>
                      <th style={{ padding: "10px 14px" }}>ACTION</th>
                    </tr>
                  </thead>
                  <tbody>
                    {realEvents.map((evt) => (
                      <tr
                        key={evt.id}
                        style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.04)", cursor: "pointer" }}
                        onClick={() => setSelectedEventJson(evt.payloadJson)}
                      >
                        <td style={{ padding: "10px 14px", fontFamily: "monospace", color: "#94a3b8" }}>
                          {evt.id.slice(0, 10)}...
                        </td>
                        <td style={{ padding: "10px 14px" }}>
                          <span
                            style={{
                              padding: "2px 6px",
                              borderRadius: "4px",
                              fontSize: "10px",
                              fontWeight: 700,
                              background:
                                evt.eventType.includes("completed")
                                  ? "rgba(34, 197, 94, 0.15)"
                                  : evt.eventType.includes("failed")
                                  ? "rgba(239, 68, 68, 0.15)"
                                  : "rgba(56, 189, 248, 0.15)",
                              color:
                                evt.eventType.includes("completed")
                                  ? "#4ade80"
                                  : evt.eventType.includes("failed")
                                  ? "#f87171"
                                  : "#38bdf8",
                            }}
                          >
                            {evt.eventType.toUpperCase()}
                          </span>
                        </td>
                        <td style={{ padding: "10px 14px", color: "#e2e8f0" }}>{evt.provider}</td>
                        <td style={{ padding: "10px 14px", color: "#64748b" }}>{evt.source}</td>
                        <td style={{ padding: "10px 14px", color: "#94a3b8" }}>
                          {new Date(evt.occurredAt).toLocaleTimeString()}
                        </td>
                        <td style={{ padding: "10px 14px", fontFamily: "monospace", color: "#4ade80" }}>
                          {evt.latencyMs !== null ? `${evt.latencyMs}ms` : "—"}
                        </td>
                        <td style={{ padding: "10px 14px" }}>
                          <button
                            style={{
                              background: "rgba(255, 255, 255, 0.06)",
                              border: "1px solid rgba(255, 255, 255, 0.1)",
                              color: "#cbd5e1",
                              padding: "2px 8px",
                              borderRadius: "4px",
                              fontSize: "10px",
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
                  backgroundColor: "rgba(0, 0, 0, 0.8)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  zIndex: 9999,
                }}
                onClick={() => setSelectedEventJson(null)}
              >
                <div
                  style={{
                    background: "#0f172a",
                    border: "1px solid rgba(255, 255, 255, 0.15)",
                    borderRadius: "8px",
                    width: "80%",
                    maxWidth: "640px",
                    maxHeight: "80vh",
                    padding: "20px",
                    display: "flex",
                    flexDirection: "column",
                    gap: "12px",
                  }}
                  onClick={(e) => e.stopPropagation()}
                >
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <strong style={{ fontSize: "13px" }}>Raw Event Payload JSON</strong>
                    <button onClick={() => setSelectedEventJson(null)} style={{ color: "#94a3b8", cursor: "pointer" }}>
                      ✕
                    </button>
                  </div>
                  <pre
                    style={{
                      background: "#070b12",
                      padding: "12px",
                      borderRadius: "6px",
                      overflow: "auto",
                      fontSize: "11px",
                      color: "#38bdf8",
                      fontFamily: "monospace",
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
              <h3 style={{ fontSize: "14px", fontWeight: 700, marginBottom: "8px" }}>Completed Agent Sessions</h3>
              {sessions.length === 0 ? (
                <div style={{ fontSize: "11px", color: "#64748b" }}>No completed sessions recorded yet.</div>
              ) : (
                <div style={{ background: "#0c1322", border: "1px solid rgba(255, 255, 255, 0.08)", borderRadius: "8px", overflow: "hidden" }}>
                  <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "11px", textAlign: "left" }}>
                    <thead>
                      <tr style={{ background: "rgba(255, 255, 255, 0.03)", borderBottom: "1px solid rgba(255, 255, 255, 0.08)", color: "#64748b" }}>
                        <th style={{ padding: "8px 12px" }}>SESSION ID</th>
                        <th style={{ padding: "8px 12px" }}>AGENT</th>
                        <th style={{ padding: "8px 12px" }}>STATUS</th>
                        <th style={{ padding: "8px 12px" }}>DURATION</th>
                        <th style={{ padding: "8px 12px" }}>TOKENS</th>
                        <th style={{ padding: "8px 12px" }}>EXIT CODE</th>
                      </tr>
                    </thead>
                    <tbody>
                      {sessions.map((s) => (
                        <tr key={s.sessionId} style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.04)" }}>
                          <td style={{ padding: "8px 12px", fontFamily: "monospace", color: "#94a3b8" }}>
                            {s.sessionId.slice(0, 12)}...
                          </td>
                          <td style={{ padding: "8px 12px", color: "#e2e8f0" }}>{s.agentId}</td>
                          <td style={{ padding: "8px 12px" }}>
                            <span
                              style={{
                                padding: "2px 6px",
                                borderRadius: "3px",
                                fontSize: "9px",
                                fontWeight: 700,
                                background: s.status === "completed" ? "rgba(34, 197, 94, 0.15)" : "rgba(239, 68, 68, 0.15)",
                                color: s.status === "completed" ? "#4ade80" : "#f87171",
                              }}
                            >
                              {s.status.toUpperCase()}
                            </span>
                          </td>
                          <td style={{ padding: "8px 12px", color: "#94a3b8" }}>
                            {s.durationMs !== null ? `${(s.durationMs / 1000).toFixed(1)}s` : "—"}
                          </td>
                          <td style={{ padding: "8px 12px", color: "#38bdf8" }}>
                            {((s.inputTokens || 0) + (s.outputTokens || 0)).toLocaleString()}
                          </td>
                          <td style={{ padding: "8px 12px", fontFamily: "monospace", color: s.exitCode === 0 ? "#4ade80" : "#f87171" }}>
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
          <div style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div>
                <h2 style={{ fontSize: "18px", fontWeight: 700, margin: 0 }}>Companion Library</h2>
                <p style={{ fontSize: "12px", color: "#64748b", margin: "4px 0 0 0" }}>
                  Verified PetPacks with pure data-driven spritesheets and vector rigs. Click to select companion.
                </p>
              </div>
              <button
                onClick={handleImportPack}
                style={{
                  background: "rgba(56, 189, 248, 0.15)",
                  border: "1px solid rgba(56, 189, 248, 0.4)",
                  color: "#38bdf8",
                  padding: "6px 14px",
                  borderRadius: "6px",
                  fontSize: "12px",
                  fontWeight: 600,
                  cursor: "pointer",
                }}
              >
                + Import PetPack Archive
              </button>
            </div>

            {/* Pack Cards Grid */}
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "16px" }}>
              {installedPacks.map((pack) => {
                const isSelected = activePetName.toLowerCase().includes(pack.name.toLowerCase());
                return (
                  <div
                    key={pack.selectionId}
                    style={{
                      background: "#0c1322",
                      border: isSelected ? "2px solid #38bdf8" : "1px solid rgba(255, 255, 255, 0.08)",
                      borderRadius: "12px",
                      padding: "16px",
                      display: "flex",
                      flexDirection: "column",
                      gap: "12px",
                      position: "relative",
                    }}
                  >
                    {isSelected && (
                      <span
                        style={{
                          position: "absolute",
                          top: "12px",
                          right: "12px",
                          background: "#38bdf8",
                          color: "#000",
                          fontSize: "9px",
                          fontWeight: 800,
                          padding: "2px 8px",
                          borderRadius: "4px",
                          letterSpacing: "0.5px",
                        }}
                      >
                        ACTIVE
                      </span>
                    )}

                    {/* Preview Thumbnail */}
                    <div
                      style={{
                        width: "100%",
                        height: "160px",
                        borderRadius: "8px",
                        background: "#080d17",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        overflow: "hidden",
                      }}
                    >
                      {pack.previewDataUrl ? (
                        <img
                          src={pack.previewDataUrl}
                          alt={pack.name}
                          style={{ maxHeight: "100%", maxWidth: "100%", objectFit: "contain", imageRendering: "pixelated" }}
                        />
                      ) : (
                        <span style={{ fontSize: "42px" }}>
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

                    {/* Metadata */}
                    <div>
                      <div style={{ fontSize: "15px", fontWeight: 700, color: "#f8fafc" }}>{pack.name}</div>
                      <div style={{ fontSize: "11px", color: "#64748b", marginTop: "2px" }}>
                        v{pack.version} · {pack.creator} · {pack.license}
                      </div>
                    </div>

                    {/* Select Button */}
                    <button
                      onClick={() => handleSelectPet(pack.selectionId)}
                      disabled={isSelected}
                      style={{
                        padding: "8px 12px",
                        background: isSelected ? "rgba(56, 189, 248, 0.2)" : "rgba(255, 255, 255, 0.06)",
                        border: isSelected ? "1px solid #38bdf8" : "1px solid rgba(255, 255, 255, 0.15)",
                        color: isSelected ? "#38bdf8" : "#f8fafc",
                        borderRadius: "6px",
                        fontSize: "12px",
                        fontWeight: 600,
                        cursor: isSelected ? "default" : "pointer",
                      }}
                    >
                      {isSelected ? "Currently Active" : "Select Companion"}
                    </button>
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
          <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
            {/* Cryptographic Ledger Proof Banner */}
            <div
              style={{
                background: "linear-gradient(135deg, rgba(16, 185, 129, 0.15) 0%, rgba(6, 78, 59, 0.15) 100%)",
                border: "1px solid rgba(16, 185, 129, 0.4)",
                borderRadius: "12px",
                padding: "20px 24px",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "16px" }}>
                <span style={{ fontSize: "32px" }}>🛡️</span>
                <div>
                  <div style={{ fontSize: "15px", fontWeight: 700, color: "#34d399" }}>
                    CRYPTOGRAPHIC APPEND-ONLY LEDGER
                  </div>
                  <div style={{ fontSize: "11px", color: "#94a3b8", marginTop: "4px" }}>
                    Every XP transaction is cryptographically linked with SHA-256 hash chaining and protected by SQLite triggers.
                  </div>
                  {ledgerVerification && (
                    <div style={{ fontSize: "10px", color: "#cbd5e1", marginTop: "6px", fontFamily: "monospace" }}>
                      Total Verified Entries: {ledgerVerification.totalEntries} · Head:{" "}
                      {ledgerVerification.headHash.slice(0, 16)}...
                    </div>
                  )}
                </div>
              </div>
              <button
                onClick={handleVerifyLedger}
                disabled={ledgerVerifying}
                style={{
                  background: "#10b981",
                  color: "#000000",
                  fontWeight: 700,
                  fontSize: "12px",
                  padding: "8px 16px",
                  borderRadius: "6px",
                  cursor: "pointer",
                }}
              >
                {ledgerVerifying ? "Verifying Hash Chain..." : "Verify Hash Chain"}
              </button>
            </div>

            {/* XP Transactions Table */}
            <div>
              <h3 style={{ fontSize: "14px", fontWeight: 700, marginBottom: "10px" }}>Recent Immutable Transactions</h3>
              {xpHistory.length === 0 ? (
                <div style={{ color: "#64748b", fontSize: "12px" }}>No XP transactions recorded yet. Complete sessions to earn XP.</div>
              ) : (
                <div style={{ background: "#0c1322", border: "1px solid rgba(255, 255, 255, 0.08)", borderRadius: "8px", overflow: "hidden" }}>
                  <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "11px", textAlign: "left" }}>
                    <thead>
                      <tr style={{ background: "rgba(255, 255, 255, 0.03)", borderBottom: "1px solid rgba(255, 255, 255, 0.08)", color: "#64748b" }}>
                        <th style={{ padding: "8px 12px" }}>TRANSACTION ID</th>
                        <th style={{ padding: "8px 12px" }}>SOURCE</th>
                        <th style={{ padding: "8px 12px" }}>AMOUNT</th>
                        <th style={{ padding: "8px 12px" }}>CREATED AT</th>
                      </tr>
                    </thead>
                    <tbody>
                      {xpHistory.map((tx) => (
                        <tr key={tx.transactionId} style={{ borderBottom: "1px solid rgba(255, 255, 255, 0.04)" }}>
                          <td style={{ padding: "8px 12px", fontFamily: "monospace", color: "#94a3b8" }}>
                            {tx.transactionId.slice(0, 12)}...
                          </td>
                          <td style={{ padding: "8px 12px", color: "#e2e8f0" }}>{tx.source}</td>
                          <td style={{ padding: "8px 12px", color: "#4ade80", fontWeight: 700 }}>+{tx.amount} XP</td>
                          <td style={{ padding: "8px 12px", color: "#64748b" }}>
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
              <h3 style={{ fontSize: "14px", fontWeight: 700, marginBottom: "10px" }}>Achievements</h3>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: "12px" }}>
                {achievements.map((ach) => (
                  <div
                    key={ach.id}
                    style={{
                      background: "#0c1322",
                      border: ach.unlockedAt ? "1px solid rgba(255, 215, 0, 0.4)" : "1px solid rgba(255, 255, 255, 0.08)",
                      borderRadius: "8px",
                      padding: "12px",
                      display: "flex",
                      flexDirection: "column",
                      gap: "6px",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                      <span style={{ fontWeight: 700, fontSize: "12px", color: ach.unlockedAt ? "#ffd700" : "#94a3b8" }}>
                        {ach.title}
                      </span>
                      <span style={{ fontSize: "10px", color: "#4ade80", fontWeight: 600 }}>+{ach.rewardXp} XP</span>
                    </div>
                    <div style={{ fontSize: "11px", color: "#64748b" }}>{ach.description}</div>
                    <div style={{ fontSize: "10px", color: ach.unlockedAt ? "#ffd700" : "#64748b", marginTop: "4px" }}>
                      {ach.unlockedAt ? `Unlocked: ${new Date(ach.unlockedAt).toLocaleDateString()}` : `Progress: ${ach.progress} / ${ach.threshold}`}
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
          <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
            {/* Simulation Controls */}
            <div
              style={{
                background: "#0c1322",
                border: "1px solid rgba(255, 255, 255, 0.08)",
                borderRadius: "12px",
                padding: "20px",
                display: "flex",
                flexDirection: "column",
                gap: "12px",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontSize: "14px", fontWeight: 700 }}>Simulation & Test Mode</div>
                  <div style={{ fontSize: "11px", color: "#64748b", marginTop: "2px" }}>
                    Runs in an isolated database (<code>agentpet_simulated.sqlite3</code>). Grants 0 XP to the live ledger.
                  </div>
                </div>
                <button
                  onClick={handleToggleSimulated}
                  style={{
                    padding: "8px 16px",
                    background: isSimulated ? "#f59e0b" : "rgba(255, 255, 255, 0.08)",
                    border: isSimulated ? "1px solid #d97706" : "1px solid rgba(255, 255, 255, 0.15)",
                    color: isSimulated ? "#000" : "#fff",
                    borderRadius: "6px",
                    fontWeight: 700,
                    fontSize: "12px",
                    cursor: "pointer",
                  }}
                >
                  {isSimulated ? "Simulated Mode Active" : "Enable Simulated Mode"}
                </button>
              </div>

              {/* Event Triggers */}
              {isSimulated && (
                <div style={{ display: "flex", gap: "8px", flexWrap: "wrap", marginTop: "8px" }}>
                  {(
                    [
                      { id: "started", label: "Agent Started" },
                      { id: "working", label: "Agent Working (Draw Bow / Staff)" },
                      { id: "thinking", label: "Agent Thinking" },
                      { id: "needs_input", label: "Input Needed (Sound Conch)" },
                      { id: "completed", label: "Agent Completed (Triumph)" },
                      { id: "failed", label: "Agent Failed (Defend / Error)" },
                    ] as const
                  ).map((btn) => (
                    <button
                      key={btn.id}
                      onClick={() => handleSimulateEvent(btn.id)}
                      style={{
                        padding: "6px 12px",
                        background: "rgba(245, 158, 11, 0.15)",
                        border: "1px solid rgba(245, 158, 11, 0.4)",
                        color: "#f59e0b",
                        borderRadius: "4px",
                        fontSize: "11px",
                        cursor: "pointer",
                      }}
                    >
                      {btn.label}
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* Provider Hook Manager */}
            <div
              style={{
                background: "#0c1322",
                border: "1px solid rgba(255, 255, 255, 0.08)",
                borderRadius: "12px",
                padding: "20px",
                display: "flex",
                flexDirection: "column",
                gap: "12px",
              }}
            >
              <div style={{ fontSize: "14px", fontWeight: 700 }}>Coding Agent Provider Adapters</div>
              <div style={{ fontSize: "11px", color: "#64748b" }}>
                Inspect real machine paths and hook configurations. Documented in <code>docs/PROVIDERS.md</code>.
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "10px", marginTop: "6px" }}>
                {providers.map((p) => (
                  <div
                    key={p.providerId}
                    style={{
                      background: "rgba(255, 255, 255, 0.02)",
                      border: "1px solid rgba(255, 255, 255, 0.06)",
                      borderRadius: "6px",
                      padding: "10px 14px",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                    }}
                  >
                    <div>
                      <div style={{ fontWeight: 600, fontSize: "12px", color: "#f8fafc" }}>{p.displayName}</div>
                      <div style={{ fontSize: "10px", color: "#64748b", marginTop: "2px" }}>
                        Config: {p.configPath ?? "Default directory"}
                      </div>
                    </div>
                    <span
                      style={{
                        fontSize: "9px",
                        fontWeight: 700,
                        padding: "2px 6px",
                        borderRadius: "3px",
                        background: p.detected ? "rgba(34, 197, 94, 0.15)" : "rgba(148, 163, 184, 0.1)",
                        color: p.detected ? "#4ade80" : "#64748b",
                        border: `1px solid ${p.detected ? "rgba(34, 197, 94, 0.3)" : "rgba(148, 163, 184, 0.2)"}`,
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
      />
    </div>
  );
}
