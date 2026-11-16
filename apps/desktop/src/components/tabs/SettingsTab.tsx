import type { ProviderReport } from "../CommandDeck";
import { quotaSummary } from "../CommandDeck";
import { CompanionControlsPanel } from "../../settings/CompanionControlsPanel";
import { PanelErrorBanner } from "../common/PanelErrorBanner";

interface SettingsTabProps {
  error: string | null;
  onRetry: () => void;
  isSimulated: boolean;
  onToggleSimulated: () => void;
  onSimulateEvent: (eventType: string) => void;
  providers: ProviderReport[];
}

export function SettingsTab({
  error,
  onRetry,
  isSimulated,
  onToggleSimulated,
  onSimulateEvent,
  providers,
}: SettingsTabProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
      <PanelErrorBanner error={error} onRetry={onRetry} />

      <CompanionControlsPanel />

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
              onClick={onToggleSimulated}
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
                  onClick={() => onSimulateEvent(btn.id)}
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
                  Usage: {quotaSummary(p.quota)}
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
  );
}
