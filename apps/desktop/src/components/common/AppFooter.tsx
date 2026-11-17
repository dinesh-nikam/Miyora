import { APP_TABS, type AppTab } from "../../types/app";

interface AppFooterProps {
  activePetName: string;
  renderMode: string;
  onTabChange: (tab: AppTab) => void;
  onOpenCommandDeck: () => void;
  onOpenDeskOverlay: () => void;
}

export function AppFooter({
  activePetName,
  renderMode,
  onTabChange,
  onOpenCommandDeck,
  onOpenDeskOverlay,
}: AppFooterProps) {
  return (
    <>
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
              onClick={onOpenCommandDeck}
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
              onClick={onOpenDeskOverlay}
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
                MIYORA
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
              {APP_TABS.map(({ id, label }) => (
                <button
                  key={id}
                  onClick={() => onTabChange(id)}
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
                  {label}
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
          <div>© 2026 Miyora · Autonomous Coding Companion Studio</div>
          <div>Local-First · Tamper-Evident · Privacy-Preserving</div>
        </div>
      </footer>
    </>
  );
}
