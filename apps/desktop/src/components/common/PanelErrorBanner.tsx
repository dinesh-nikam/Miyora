interface PanelErrorBannerProps {
  error: string | null;
  onRetry: () => void;
}

export function PanelErrorBanner({ error, onRetry }: PanelErrorBannerProps) {
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
        onClick={onRetry}
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
}
