interface RuntimeDiagnosticBannerProps {
  error: string;
  onDismiss: () => void;
}

export function RuntimeDiagnosticBanner({ error, onDismiss }: RuntimeDiagnosticBannerProps) {
  return (
    <div
      style={{
        backgroundColor: "#ff2db7",
        color: "#ffffff",
        padding: "8px 24px",
        fontSize: "12px",
        fontWeight: 800,
        fontFamily: "'Space Grotesk', sans-serif",
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        borderBottom: "4px solid #0a0a0a",
      }}
    >
      <span>🚨 RUNTIME DIAGNOSTIC: {error}</span>
      <button
        onClick={onDismiss}
        style={{
          background: "#0a0a0a",
          color: "#ffffff",
          border: "2px solid #ffffff",
          padding: "2px 8px",
          borderRadius: "4px",
          fontWeight: 900,
          cursor: "pointer",
        }}
      >
        ✕
      </button>
    </div>
  );
}
