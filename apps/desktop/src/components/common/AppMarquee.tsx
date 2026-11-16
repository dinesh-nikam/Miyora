export function AppMarquee() {
  return (
    <>
      {/* Slogan Marquee (Forward) */}
      <div
        className="marquee-mask"
        style={{
          backgroundColor: "#0a0a0a",
          color: "#ffffff",
          padding: "12px 0",
          borderBottom: "4px solid #0a0a0a",
        }}
      >
        <div className="marquee-strip">
          {[1, 2].map((idx) => (
            <div
              key={idx}
              aria-hidden={idx === 2}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "28px",
                paddingRight: "28px",
              }}
            >
              <span className="display" style={{ fontSize: "22px" }}>
                ZERO FABRICATED METRICS
              </span>
              <span style={{ color: "#ff2db7", fontSize: "24px", fontWeight: 900 }}>✦</span>
              <span className="display" style={{ fontSize: "22px", color: "#00d4ff" }}>
                TAMPER-EVIDENT LEDGER
              </span>
              <span style={{ color: "#ffd000", fontSize: "24px", fontWeight: 900 }}>✦</span>
              <span className="display" style={{ fontSize: "22px" }}>
                3D PROCEDURAL RIG
              </span>
              <span style={{ color: "#ff2db7", fontSize: "24px", fontWeight: 900 }}>✦</span>
              <span className="display" style={{ fontSize: "22px", color: "#ffd000" }}>
                TAURI 2 & RUST DOMAIN
              </span>
              <span style={{ color: "#00d4ff", fontSize: "24px", fontWeight: 900 }}>✦</span>
              <span className="display" style={{ fontSize: "22px" }}>
                LOCAL-FIRST ARCHITECTURE
              </span>
              <span style={{ color: "#ff2db7", fontSize: "24px", fontWeight: 900 }}>✦</span>
            </div>
          ))}
        </div>
      </div>

      {/* Secondary Client / Agent Marquee (Reverse) */}
      <div
        className="marquee-mask"
        style={{
          backgroundColor: "#ffd000",
          color: "#0a0a0a",
          padding: "8px 0",
          borderBottom: "4px solid #0a0a0a",
        }}
      >
        <div className="marquee-rev-strip">
          {[1, 2].map((idx) => (
            <div
              key={idx}
              aria-hidden={idx === 2}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "24px",
                paddingRight: "24px",
              }}
            >
              {[
                "CLAUDE CODE",
                "•",
                "COPILOT",
                "•",
                "CURSOR",
                "•",
                "CODEX",
                "•",
                "OPENAI",
                "•",
                "AOI",
                "•",
                "MIRA",
                "•",
                "ROWAN",
                "•",
                "HANA",
                "•",
                "IVO",
                "•",
                "TETSU",
                "•",
                "SUMIRE",
                "•",
                "MOMIJI",
              ].map((token, i) => (
                <span
                  key={i}
                  className="font-grotesk"
                  style={{
                    fontSize: "14px",
                    fontWeight: 900,
                    textTransform: "uppercase",
                    letterSpacing: "0.15em",
                    color: token === "•" ? "#ff2db7" : "#0a0a0a",
                  }}
                >
                  {token}
                </span>
              ))}
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
