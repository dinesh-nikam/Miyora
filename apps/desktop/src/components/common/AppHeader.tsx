import { APP_TABS, type AppTab } from "../../types/app";

interface AppHeaderProps {
  activeTab: AppTab;
  onTabChange: (tab: AppTab) => void;
  activePetName: string;
  soundMuted: boolean;
  onToggleSound: () => void;
  onOpenCommandDeck: () => void;
  onOpenDeskOverlay: () => void;
}

export function AppHeader({
  activeTab,
  onTabChange,
  activePetName,
  soundMuted,
  onToggleSound,
  onOpenCommandDeck,
  onOpenDeskOverlay,
}: AppHeaderProps) {
  return (
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
              MIYORA
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
          {APP_TABS.map((tab) => {
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => onTabChange(tab.id)}
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
            onClick={onToggleSound}
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
            onClick={onOpenCommandDeck}
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
            onClick={onOpenDeskOverlay}
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
  );
}
