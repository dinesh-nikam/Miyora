interface SimulatedBannerProps {
  onExit: () => void;
}

export function SimulatedBanner({ onExit }: SimulatedBannerProps) {
  return (
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
        onClick={onExit}
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
  );
}
