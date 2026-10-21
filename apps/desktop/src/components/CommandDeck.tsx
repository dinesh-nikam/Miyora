import { useEffect } from "react";

export interface ProviderReport {
  providerId: string;
  displayName: string;
  detected: boolean;
  hookStatus: string;
  quotaStatus?: {
    fiveHour?: { usedPercentage: number; resetsAt?: string };
    sevenDay?: { usedPercentage: number; resetsAt?: string };
    contextWindowUsedPercentage?: number;
    updatedAt: string;
    provenance: string;
  } | null;
  quotaProvenance: string;
  quotaUpdatedSecondsAgo?: number | null;
  costUsd?: number | null;
  costProvenance: string;
  inputTokens?: number | null;
  outputTokens?: number | null;
  tokensProvenance: string;
  configPath?: string | null;
}

export interface CommandDeckProps {
  isOpen: boolean;
  onClose: () => void;
  providers: ProviderReport[];
  activePetName: string;
  activeState: string;
  isSimulated: boolean;
  onToggleSimulated: () => void;
  onSelectPet: (id: string) => void;
  installedPacks: { id: string; selectionId: string; name: string }[];
  onVerifyLedger: () => void;
  latencyMs?: number | null;
}

export function CommandDeck({
  isOpen,
  onClose,
  providers,
  activePetName,
  activeState,
  isSimulated,
  onToggleSimulated,
  onSelectPet,
  installedPacks,
  onVerifyLedger,
  latencyMs,
}: CommandDeckProps) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      className="command-deck-overlay"
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "rgba(0, 0, 0, 0.75)",
        backdropFilter: "blur(10px)",
        zIndex: 9999,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        animation: "commandDeckFadeIn 0.12s cubic-bezier(0, 0, 0.2, 1)",
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="command-deck-dialog"
        style={{
          width: "90%",
          maxWidth: "780px",
          maxHeight: "88vh",
          backgroundColor: "#0d131f",
          border: "1px solid rgba(82, 168, 255, 0.3)",
          borderRadius: "12px",
          boxShadow: "0 16px 40px rgba(0, 0, 0, 0.8), 0 0 20px rgba(82, 168, 255, 0.15)",
          color: "#e2e8f0",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "14px 20px",
            borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
            background: "linear-gradient(90deg, #101a2e 0%, #0d131f 100%)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <span style={{ fontSize: "16px" }}>⚡</span>
            <div>
              <div style={{ fontWeight: 700, fontSize: "14px", letterSpacing: "0.5px" }}>
                AGENTPET COMMAND DECK
              </div>
              <div style={{ fontSize: "10px", color: "#64748b" }}>
                Low-latency HUD · Active Companion:{" "}
                <span style={{ color: "#38bdf8", fontWeight: 600 }}>{activePetName}</span> (
                {activeState})
              </div>
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            {isSimulated && (
              <span
                style={{
                  background: "#f59e0b",
                  color: "#000",
                  fontWeight: 800,
                  fontSize: "10px",
                  padding: "2px 8px",
                  borderRadius: "4px",
                  letterSpacing: "1px",
                }}
              >
                SIMULATED
              </span>
            )}
            <button
              onClick={onClose}
              style={{
                background: "rgba(255, 255, 255, 0.06)",
                border: "1px solid rgba(255, 255, 255, 0.1)",
                color: "#94a3b8",
                borderRadius: "6px",
                padding: "4px 10px",
                cursor: "pointer",
                fontSize: "13px",
              }}
            >
              ✕ Esc
            </button>
          </div>
        </div>

        {/* Content Body */}
        <div style={{ padding: "18px 20px", overflowY: "auto", display: "flex", flexDirection: "column", gap: "16px" }}>
          {/* Provider Telemetry Matrix */}
          <div>
            <div style={{ fontSize: "11px", fontWeight: 600, color: "#94a3b8", marginBottom: "8px", textTransform: "uppercase", letterSpacing: "1px" }}>
              AI Provider Telemetry
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: "10px" }}>
              {providers.map((p) => {
                const fiveHourUsed = p.quotaStatus?.fiveHour?.usedPercentage;
                const sevenDayUsed = p.quotaStatus?.sevenDay?.usedPercentage;
                return (
                  <div
                    key={p.providerId}
                    style={{
                      background: "rgba(15, 23, 42, 0.6)",
                      border: p.detected ? "1px solid rgba(56, 189, 248, 0.25)" : "1px solid rgba(255, 255, 255, 0.05)",
                      borderRadius: "8px",
                      padding: "12px",
                      display: "flex",
                      flexDirection: "column",
                      gap: "8px",
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                      <span style={{ fontWeight: 600, fontSize: "12px", color: p.detected ? "#f8fafc" : "#64748b" }}>
                        {p.displayName}
                      </span>
                      <span
                        style={{
                          fontSize: "9px",
                          fontWeight: 700,
                          padding: "1px 5px",
                          borderRadius: "3px",
                          background: p.detected ? "rgba(34, 197, 94, 0.15)" : "rgba(148, 163, 184, 0.1)",
                          color: p.detected ? "#4ade80" : "#64748b",
                          border: `1px solid ${p.detected ? "rgba(34, 197, 94, 0.3)" : "rgba(148, 163, 184, 0.2)"}`,
                        }}
                      >
                        {p.detected ? (p.hookStatus === "installed" ? "HOOK ACTIVE" : "DETECTED") : "NOT FOUND"}
                      </span>
                    </div>

                    {/* Rate limit gauges */}
                    <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
                      <div>
                        <div style={{ fontSize: "9px", color: "#64748b" }}>5h Quota:</div>
                        <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                          <span style={{ fontSize: "13px", fontWeight: 700, color: fiveHourUsed !== undefined ? "#38bdf8" : "#94a3b8" }}>
                            {fiveHourUsed !== undefined ? `${fiveHourUsed.toFixed(1)}%` : "N/A"}
                          </span>
                          <span
                            style={{
                              fontSize: "8px",
                              padding: "1px 4px",
                              borderRadius: "2px",
                              background: p.quotaProvenance === "LIVE" ? "#0284c7" : "#334155",
                              color: "#fff",
                            }}
                          >
                            {p.quotaProvenance}
                          </span>
                        </div>
                      </div>

                      {sevenDayUsed !== undefined && (
                        <div>
                          <div style={{ fontSize: "9px", color: "#64748b" }}>7d Quota:</div>
                          <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                            <span style={{ fontSize: "13px", fontWeight: 700, color: "#a855f7" }}>
                              {sevenDayUsed.toFixed(1)}%
                            </span>
                          </div>
                        </div>
                      )}

                      {p.costUsd !== undefined && p.costUsd !== null && (
                        <div>
                          <div style={{ fontSize: "9px", color: "#64748b" }}>Cost USD:</div>
                          <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                            <span style={{ fontSize: "13px", fontWeight: 700, color: "#4ade80" }}>
                              ${p.costUsd.toFixed(2)}
                            </span>
                            <span style={{ fontSize: "8px", padding: "1px 4px", borderRadius: "2px", background: "#15803d", color: "#fff" }}>
                              {p.costProvenance}
                            </span>
                          </div>
                        </div>
                      )}
                    </div>

                    {/* Token usage */}
                    {p.inputTokens !== undefined && p.inputTokens !== null && (
                      <div style={{ fontSize: "10px", color: "#94a3b8", display: "flex", gap: "8px" }}>
                        <span>In: {(p.inputTokens || 0).toLocaleString()}</span>
                        <span>Out: {(p.outputTokens || 0).toLocaleString()}</span>
                      </div>
                    )}

                    {p.quotaUpdatedSecondsAgo !== undefined && p.quotaUpdatedSecondsAgo !== null && (
                      <div style={{ fontSize: "9px", color: "#64748b" }}>
                        Updated {p.quotaUpdatedSecondsAgo}s ago
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Quick Companion Switcher */}
          <div>
            <div style={{ fontSize: "11px", fontWeight: 600, color: "#94a3b8", marginBottom: "8px", textTransform: "uppercase", letterSpacing: "1px" }}>
              Companion Switcher
            </div>
            <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
              {installedPacks.map((pack) => {
                const isSelected = activePetName.toLowerCase().includes(pack.name.toLowerCase());
                return (
                  <button
                    key={pack.selectionId}
                    onClick={() => onSelectPet(pack.selectionId)}
                    style={{
                      padding: "8px 14px",
                      background: isSelected ? "rgba(56, 189, 248, 0.2)" : "rgba(30, 41, 59, 0.6)",
                      border: isSelected ? "1px solid #38bdf8" : "1px solid rgba(255, 255, 255, 0.1)",
                      borderRadius: "6px",
                      color: isSelected ? "#38bdf8" : "#cbd5e1",
                      cursor: "pointer",
                      fontSize: "12px",
                      fontWeight: isSelected ? 700 : 500,
                      display: "flex",
                      alignItems: "center",
                      gap: "6px",
                    }}
                  >
                    <span>{pack.name.includes("Arjuna") ? "🏹" : pack.name.includes("Wukong") ? "👑" : pack.name.includes("Ninja") ? "🐱" : "⚡"}</span>
                    <span>{pack.name}</span>
                    {isSelected && <span style={{ fontSize: "9px", background: "#38bdf8", color: "#000", padding: "1px 4px", borderRadius: "2px" }}>ACTIVE</span>}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Quick Commands & Cryptographic Verification */}
          <div style={{ display: "flex", gap: "10px", flexWrap: "wrap", alignItems: "center", marginTop: "4px" }}>
            <button
              onClick={onVerifyLedger}
              style={{
                padding: "8px 14px",
                background: "rgba(16, 185, 129, 0.15)",
                border: "1px solid rgba(16, 185, 129, 0.4)",
                color: "#34d399",
                borderRadius: "6px",
                cursor: "pointer",
                fontSize: "11px",
                fontWeight: 600,
                display: "flex",
                alignItems: "center",
                gap: "6px",
              }}
            >
              <span>🔒</span>
              <span>Verify SHA-256 XP Ledger</span>
            </button>

            <button
              onClick={onToggleSimulated}
              style={{
                padding: "8px 14px",
                background: isSimulated ? "rgba(245, 158, 11, 0.2)" : "rgba(51, 65, 85, 0.5)",
                border: isSimulated ? "1px solid #f59e0b" : "1px solid rgba(255, 255, 255, 0.1)",
                color: isSimulated ? "#f59e0b" : "#94a3b8",
                borderRadius: "6px",
                cursor: "pointer",
                fontSize: "11px",
                fontWeight: 600,
                display: "flex",
                alignItems: "center",
                gap: "6px",
              }}
            >
              <span>🧪</span>
              <span>{isSimulated ? "Exit Simulated Mode (Return to Live)" : "Toggle Simulated Demo Mode"}</span>
            </button>

            {latencyMs !== undefined && latencyMs !== null && (
              <span style={{ fontSize: "10px", color: "#64748b", marginLeft: "auto", fontFamily: "monospace" }}>
                Render Latency: {latencyMs}ms
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
