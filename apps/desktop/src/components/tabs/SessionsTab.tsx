import type { AgentSessionSummary, StoredAgentEvent } from "../../types/app";
import { PanelErrorBanner } from "../common/PanelErrorBanner";

interface SessionsTabProps {
  error: string | null;
  onRetry: () => void;
  realEvents: StoredAgentEvent[];
  sessions: AgentSessionSummary[];
  onSelectEventJson: (json: string | null) => void;
}

export function SessionsTab({
  error,
  onRetry,
  realEvents,
  sessions,
  onSelectEventJson,
}: SessionsTabProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
      <PanelErrorBanner error={error} onRetry={onRetry} />

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
                  onClick={() => onSelectEventJson(evt.payloadJson)}
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
  );
}
