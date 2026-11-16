interface RawJsonModalProps {
  json: string | null;
  onClose: () => void;
}

export function RawJsonModal({ json, onClose }: RawJsonModalProps) {
  if (!json) return null;

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "rgba(10, 10, 10, 0.75)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9999,
        backdropFilter: "blur(2px)",
      }}
      onClick={onClose}
    >
      <div
        className="edge-thick shadow-pop-lg"
        style={{
          background: "#ffffff",
          width: "85%",
          maxWidth: "680px",
          maxHeight: "80vh",
          padding: "24px",
          display: "flex",
          flexDirection: "column",
          gap: "16px",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "3px solid #0a0a0a", paddingBottom: "12px" }}>
          <div className="display" style={{ fontSize: "20px", color: "#0a0a0a" }}>
            RAW EVENT PAYLOAD JSON
          </div>
          <button
            onClick={onClose}
            className="shadow-pop-sm shadow-pop-hover"
            style={{
              background: "#ff2db7",
              color: "#ffffff",
              border: "2px solid #0a0a0a",
              width: "32px",
              height: "32px",
              borderRadius: "50%",
              fontWeight: 900,
              cursor: "pointer",
            }}
          >
            ✕
          </button>
        </div>
        <pre
          style={{
            background: "#0a0a0a",
            color: "#00d4ff",
            padding: "16px",
            borderRadius: "0px",
            border: "3px solid #0a0a0a",
            overflow: "auto",
            fontSize: "12px",
            fontFamily: "monospace",
            lineHeight: 1.4,
          }}
        >
          {(() => {
            try {
              return JSON.stringify(JSON.parse(json), null, 2);
            } catch {
              return json;
            }
          })()}
        </pre>
      </div>
    </div>
  );
}
