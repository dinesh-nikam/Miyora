import { CompanionRenderer, type SelectedPetAnimation } from "../CompanionRenderer";
import type { AvatarHud, ResolvedAvatar } from "../../avatar/types";
import type { RenderMode } from "../../settings/companionPreferences";
import type { ProgressionSnapshot, RuntimeSnapshot } from "../../types/app";
import type { ProviderReport } from "../CommandDeck";
import { quotaSummary } from "../CommandDeck";
import { PanelErrorBanner } from "../common/PanelErrorBanner";

interface DenTabProps {
  error: string | null;
  onRetry: () => void;
  onOpenCommandDeck: () => void;
  onOpenDeskOverlay: () => void;
  renderMode: RenderMode;
  onSetRenderMode: (mode: RenderMode) => void;
  snapshot: RuntimeSnapshot;
  selectedAnimation: SelectedPetAnimation | null;
  avatar: ResolvedAvatar | null;
  avatarHud: AvatarHud | null;
  quotaWindow: { usedPercentage: number; resetsAt?: string } | null;
  quotaProvenance: string;
  onError: (msg: string) => void;
  activePetName: string;
  totalTokensToday: number | null;
  totalCostToday: number | null;
  claudeProvider: ProviderReport | undefined;
  progression: ProgressionSnapshot | null;
}

export function DenTab({
  error,
  onRetry,
  onOpenCommandDeck,
  onOpenDeskOverlay,
  renderMode,
  onSetRenderMode,
  snapshot,
  selectedAnimation,
  avatar,
  avatarHud,
  quotaWindow,
  quotaProvenance,
  onError,
  activePetName,
  totalTokensToday,
  totalCostToday,
  claudeProvider,
  progression,
}: DenTabProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
      <PanelErrorBanner error={error} onRetry={onRetry} />

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
              onClick={onOpenCommandDeck}
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
              onClick={onOpenDeskOverlay}
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

      {/* Living Companion Sanctuary Pod */}
      <div
        className="companion-sanctuary-pod"
        style={{
          background: "radial-gradient(ellipse at 50% 25%, rgba(255, 182, 193, 0.1) 0%, rgba(255, 255, 255, 0.98) 55%, rgba(241, 245, 249, 0.95) 100%)",
          borderRadius: "36px",
          padding: "44px 28px",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          position: "relative",
          boxShadow: "0 24px 65px -15px rgba(0, 0, 0, 0.1), 0 0 0 1px rgba(0, 0, 0, 0.04)",
          overflow: "hidden",
          border: "1px solid rgba(255, 255, 255, 0.9)",
        }}
      >
        {/* Top-Right Stage Mode Toggle */}
        <div style={{ position: "absolute", top: "20px", right: "20px", zIndex: 10 }}>
          <button
            type="button"
            onClick={() => onSetRenderMode(renderMode === "3d" ? "2d" : "3d")}
            style={{
              backgroundColor: renderMode === "3d" ? "rgba(250, 204, 21, 0.22)" : "rgba(15, 23, 42, 0.06)",
              color: renderMode === "3d" ? "#854d0e" : "#475569",
              border: renderMode === "3d" ? "1.5px solid #facc15" : "1.5px solid rgba(148, 163, 184, 0.3)",
              borderRadius: "999px",
              padding: "8px 18px",
              fontSize: "12px",
              fontWeight: 800,
              letterSpacing: "0.04em",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: "8px",
              backdropFilter: "blur(10px)",
              transition: "all 0.15s ease",
            }}
          >
            <span>{renderMode === "3d" ? "🎮 3D LIVING PET" : "🖼️ 2D SPRITE PET"}</span>
            <span style={{ opacity: 0.6 }}>⇄ Switch</span>
          </button>
        </div>

        {/* Pet Renderer Stage */}
        <CompanionRenderer
          state={snapshot.petState}
          presentation={snapshot.presentation}
          animation={selectedAnimation}
          avatar={avatar}
          hud={avatarHud}
          quotaWindow={quotaWindow}
          quotaProvenance={quotaProvenance}
          size={290}
          mode={renderMode}
          onModeToggle={onSetRenderMode}
          interactive={true}
          onDoubleClick={onOpenCommandDeck}
          onError={onError}
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
              style={{
                backgroundColor: "rgba(56, 189, 248, 0.18)",
                color: "#0369a1",
                border: "1.5px solid rgba(56, 189, 248, 0.5)",
                borderRadius: "999px",
                padding: "4px 18px",
                fontSize: "12px",
                fontWeight: 800,
                letterSpacing: "0.08em",
                textTransform: "uppercase",
                boxShadow: "0 4px 12px rgba(56, 189, 248, 0.15)",
              }}
            >
              STATE: {snapshot.petState.toUpperCase().replaceAll("_", " ")}
            </span>
            <span style={{ fontSize: "12px", color: "#475569", fontWeight: 700 }}>
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
            {claudeProvider ? quotaSummary(claudeProvider.quota) : "Claude Code not detected"}
          </div>
        </div>
      </div>
    </div>
  );
}
