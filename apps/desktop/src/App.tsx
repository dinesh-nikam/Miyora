import { lazy, Suspense, useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { invokeSafe } from "./utils/ipc";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { CompanionRenderer, SelectedPetAnimation } from "./components/CompanionRenderer";
import { CommandDeck, measuredWindow, type ProviderReport } from "./components/CommandDeck";
import type { AvatarHud, ResolvedAvatar } from "./avatar/types";
import { saveCompanionPreferences, useCompanionPreferences, type RenderMode } from "./settings/companionPreferences";
import { PetOverlayFrame } from "./overlay/PetOverlayFrame";

import type {
  AchievementSummary,
  AgentSessionSummary,
  AppTab,
  LedgerVerificationResult,
  PetPackSummary,
  ProgressionSnapshot,
  RuntimeSnapshot,
  StoredAgentEvent,
  XpTransaction,
} from "./types/app";

import { SimulatedBanner } from "./components/common/SimulatedBanner";
import { RuntimeDiagnosticBanner } from "./components/common/RuntimeDiagnosticBanner";
import { AppHeader } from "./components/common/AppHeader";
import { AppMarquee } from "./components/common/AppMarquee";
import { AppFooter } from "./components/common/AppFooter";
import { RawJsonModal } from "./components/common/RawJsonModal";

import { DenTab } from "./components/tabs/DenTab";
import { SessionsTab } from "./components/tabs/SessionsTab";
import { CompanionsTab } from "./components/tabs/CompanionsTab";
import { ProgressTab } from "./components/tabs/ProgressTab";
import { SettingsTab } from "./components/tabs/SettingsTab";

const SNAPSHOT_POLL_INTERVAL_MS = 1_500;

// The Pet Library loads on demand so the overlay and Den start without it.
const PetLibrary = lazy(() => import("./avatar/CharacterStudio").then((module) => ({ default: module.CharacterStudio })));

export default function App() {
  const isOverlayWindow = new URLSearchParams(window.location.search).get("overlay") === "1";

  // Navigation State
  const [activeTab, setActiveTab] = useState<AppTab>("den");
  const [isCommandDeckOpen, setIsCommandDeckOpen] = useState(false);

  // Core Data
  const companionPreferences = useCompanionPreferences();
  const renderMode = companionPreferences.renderMode;
  const setRenderMode = (mode: RenderMode) => {
    void saveCompanionPreferences({ renderMode: mode });
  };
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

      const avatarRes = await invokeSafe<ResolvedAvatar | null>("get_selected_avatar");
      if (avatarRes.ok) {
        setAvatar(avatarRes.data);
        setPanelErrors((prev) => ({ ...prev, avatar: null }));
      } else {
        setPanelErrors((prev) => ({ ...prev, avatar: `Failed to load 3D avatar: ${avatarRes.error}` }));
      }

      const hudRes = await invokeSafe<AvatarHud | null>("get_avatar_hud");
      if (hudRes.ok) {
        setAvatarHud(hudRes.data);
      }

      if (!isOverlayWindow) {
        const [progRes, eventsRes, provsRes] = await Promise.all([
          invokeSafe<ProgressionSnapshot>("get_progression"),
          invokeSafe<StoredAgentEvent[]>("get_real_events", { limit: 15 }),
          invokeSafe<ProviderReport[]>("get_provider_reports"),
        ]);

        if (progRes.ok) {
          setProgression(progRes.data);
          setPanelErrors((prev) => ({ ...prev, progress: null }));
        } else {
          setPanelErrors((prev) => ({ ...prev, progress: `Failed to load progression: ${progRes.error}` }));
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

  // The tray's "Open Command Deck" shows this window, then asks it to open the deck.
  useEffect(() => {
    if (isOverlayWindow) return;
    let stop: (() => void) | undefined;
    let disposed = false;
    listen("open-command-deck", () => setIsCommandDeckOpen(true))
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((error: unknown) => console.error("[App] command deck listener unavailable:", error));
    return () => {
      disposed = true;
      stop?.();
    };
  }, [isOverlayWindow]);

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
  const fiveHour = claudeProvider ? measuredWindow(claudeProvider.quota, "five_hour") : null;
  const quotaWindow = fiveHour ? { usedPercentage: 100 - fiveHour.remainingPercent, resetsAt: fiveHour.resetAt ?? undefined } : null;
  const quotaProvenance = claudeProvider?.quota.kind === "measured" ? claudeProvider.quota.provenance : "N/A";

  // Real today metrics computed from real local evidence
  const totalCostToday = claudeProvider?.costUsd ?? null;
  const totalInTokensToday = claudeProvider?.inputTokens ?? null;
  const totalOutTokensToday = claudeProvider?.outputTokens ?? null;
  const totalTokensToday =
    totalInTokensToday !== null || totalOutTokensToday !== null
      ? (totalInTokensToday ?? 0) + (totalOutTokensToday ?? 0)
      : null;

  const soundMuted = companionPreferences.soundMuted;

  const toggleSound = () => {
    void saveCompanionPreferences({ soundMuted: !soundMuted });
  };

  // -------------------------------------------------------------
  // RENDER: Overlay Window
  // -------------------------------------------------------------
  // Pet only: controls live in the tray, global shortcuts, and Settings.
  if (isOverlayWindow) {
    return (
      <PetOverlayFrame>
        <CompanionRenderer
          state={snapshot.petState}
          animation={selectedAnimation}
          avatar={avatar}
          hud={avatarHud}
          quotaWindow={quotaWindow}
          quotaProvenance={quotaProvenance}
          size={180}
          mode={renderMode}
          presentation={snapshot.presentation}
          interactive={false}
          bare
          onError={setRuntimeError}
        />
      </PetOverlayFrame>
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
      {import.meta.env.DEV && isSimulated && (
        <SimulatedBanner onExit={handleToggleSimulated} />
      )}

      {/* Runtime Diagnostics Banner */}
      {runtimeError && (
        <RuntimeDiagnosticBanner error={runtimeError} onDismiss={() => setRuntimeError(null)} />
      )}

      {/* Sticky White Nav */}
      <AppHeader
        activeTab={activeTab}
        onTabChange={setActiveTab}
        activePetName={activePetName}
        soundMuted={soundMuted}
        onToggleSound={toggleSound}
        onOpenCommandDeck={() => setIsCommandDeckOpen(true)}
        onOpenDeskOverlay={() => void invoke("show_pet_overlay").catch((err) => setRuntimeError(`Failed to show companion window: ${err}`))}
      />

      {/* Forward and Reverse Marquees */}
      <AppMarquee />

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
        {activeTab === "den" && (
          <DenTab
            error={panelErrors.den ?? null}
            onRetry={() => retryPanel("den")}
            onOpenCommandDeck={() => setIsCommandDeckOpen(true)}
            onOpenDeskOverlay={() => void invoke("show_pet_overlay").catch((err) => setRuntimeError(`Failed to show companion window: ${err}`))}
            renderMode={renderMode}
            onSetRenderMode={setRenderMode}
            snapshot={snapshot}
            selectedAnimation={selectedAnimation}
            avatar={avatar}
            avatarHud={avatarHud}
            quotaWindow={quotaWindow}
            quotaProvenance={quotaProvenance}
            onError={setRuntimeError}
            activePetName={activePetName}
            totalTokensToday={totalTokensToday}
            totalCostToday={totalCostToday}
            claudeProvider={claudeProvider}
            progression={progression}
          />
        )}

        {activeTab === "sessions" && (
          <SessionsTab
            error={panelErrors.sessions ?? null}
            onRetry={() => retryPanel("sessions")}
            realEvents={realEvents}
            sessions={sessions}
            onSelectEventJson={setSelectedEventJson}
          />
        )}

        {activeTab === "companions" && (
          <CompanionsTab
            avatarError={panelErrors.avatar ?? null}
            companionsError={panelErrors.companions ?? null}
            onRetryAvatar={() => retryPanel("avatar")}
            onRetryCompanions={() => retryPanel("companions")}
            installedPacks={installedPacks}
            selectedPetId={snapshot.selectedPetId}
            onSelectPet={handleSelectPet}
            onImportPack={handleImportPack}
          >
            <Suspense fallback={<p style={{ color: "#cbd5e1" }}>Loading the Pet Library…</p>}>
              <PetLibrary liveState={snapshot.petState} hud={avatarHud} activeAvatar={avatar} onAvatarChanged={setAvatar} />
            </Suspense>
          </CompanionsTab>
        )}

        {activeTab === "progress" && (
          <ProgressTab
            error={panelErrors.progress ?? null}
            onRetry={() => retryPanel("progress")}
            ledgerVerification={ledgerVerification}
            ledgerVerifying={ledgerVerifying}
            onVerifyLedger={handleVerifyLedger}
            xpHistory={xpHistory}
            achievements={achievements}
          />
        )}

        {activeTab === "settings" && (
          <SettingsTab
            error={panelErrors.settings ?? null}
            onRetry={() => retryPanel("settings")}
            isSimulated={isSimulated}
            onToggleSimulated={handleToggleSimulated}
            onSimulateEvent={handleSimulateEvent}
            providers={providers}
          />
        )}
      </main>

      {/* Raw Event JSON Inspector Modal */}
      <RawJsonModal json={selectedEventJson} onClose={() => setSelectedEventJson(null)} />

      {/* Closing CTA and 4-Column Neo-Brutalist Footer */}
      <AppFooter
        activePetName={activePetName}
        renderMode={renderMode}
        onTabChange={setActiveTab}
        onOpenCommandDeck={() => setIsCommandDeckOpen(true)}
        onOpenDeskOverlay={() => void invoke("show_pet_overlay").catch((err) => setRuntimeError(`Failed to show companion window: ${err}`))}
      />

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
