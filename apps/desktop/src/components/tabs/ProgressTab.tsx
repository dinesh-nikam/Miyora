import type { AchievementSummary, LedgerVerificationResult, XpTransaction } from "../../types/app";
import { PanelErrorBanner } from "../common/PanelErrorBanner";

interface ProgressTabProps {
  error: string | null;
  onRetry: () => void;
  ledgerVerification: LedgerVerificationResult | null;
  ledgerVerifying: boolean;
  onVerifyLedger: () => void;
  xpHistory: XpTransaction[];
  achievements: AchievementSummary[];
}

export function ProgressTab({
  error,
  onRetry,
  ledgerVerification,
  ledgerVerifying,
  onVerifyLedger,
  xpHistory,
  achievements,
}: ProgressTabProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "28px" }}>
      <PanelErrorBanner error={error} onRetry={onRetry} />

      {/* Tamper-Evident Ledger Proof Banner (Cyan Block) */}
      <div
        className="edge-thick shadow-pop-lg"
        style={{
          backgroundColor: ledgerVerification
            ? ledgerVerification.isValid
              ? "#00d4ff"
              : "#ff4444"
            : "#00d4ff",
          color: "#0a0a0a",
          padding: "28px",
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "20px",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "18px", maxWidth: "700px" }}>
          <span style={{ fontSize: "42px", filter: "drop-shadow(3px 3px 0 #0a0a0a)" }}>
            {ledgerVerification && !ledgerVerification.isValid ? "⚠️" : "🛡️"}
          </span>
          <div>
            <div className="display" style={{ fontSize: "26px", color: "#0a0a0a" }}>
              {ledgerVerification
                ? ledgerVerification.isValid
                  ? "TAMPER-EVIDENT APPEND-ONLY LEDGER · VERIFIED"
                  : "LEDGER VERIFICATION FAILED · TAMPERING DETECTED"
                : "TAMPER-EVIDENT APPEND-ONLY LEDGER"}
            </div>
            <div className="font-grotesk" style={{ fontSize: "13px", fontWeight: 700, color: "#0a0a0a", marginTop: "4px" }}>
              Every XP transaction is cryptographically linked with SHA-256 hash chaining and protected by SQLite triggers.
            </div>
            {ledgerVerification && (
              <div style={{ fontSize: "11px", fontWeight: 800, marginTop: "6px", fontFamily: "monospace", color: "#0a0a0a" }}>
                {ledgerVerification.isValid ? (
                  <>TOTAL ENTRIES: {ledgerVerification.totalEntries} · HEAD: {ledgerVerification.headHash.slice(0, 18)}...</>
                ) : (
                  <>BROKEN AT INDEX: {ledgerVerification.brokenAtIndex ?? "Unknown"} · ERROR: {ledgerVerification.errorMessage ?? "Hash mismatch"}</>
                )}
              </div>
            )}
          </div>
        </div>
        <button
          onClick={onVerifyLedger}
          disabled={ledgerVerifying}
          className="font-grotesk shadow-pop shadow-pop-hover"
          style={{
            backgroundColor: "#ffd000",
            color: "#0a0a0a",
            fontWeight: 900,
            fontSize: "13px",
            textTransform: "uppercase",
            letterSpacing: "0.08em",
            padding: "12px 24px",
            border: "3px solid #0a0a0a",
            borderRadius: "999px",
            cursor: "pointer",
          }}
        >
          {ledgerVerifying ? "Verifying Hash Chain..." : "Verify Hash Chain 🔒"}
        </button>
      </div>

      {/* XP Transactions Table */}
      <div>
        <h3 className="display" style={{ fontSize: "28px", color: "#0a0a0a", marginBottom: "12px" }}>
          RECENT IMMUTABLE TRANSACTIONS
        </h3>
        {xpHistory.length === 0 ? (
          <div className="font-grotesk" style={{ color: "rgba(10,10,10,0.6)", fontSize: "13px", fontWeight: 700 }}>
            No XP transactions recorded yet. Complete coding sessions to earn XP.
          </div>
        ) : (
          <div className="edge-thick shadow-pop" style={{ backgroundColor: "#ffffff", overflow: "hidden" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "12px", textAlign: "left" }}>
              <thead>
                <tr
                  className="font-grotesk"
                  style={{
                    backgroundColor: "#0a0a0a",
                    color: "#ffffff",
                    borderBottom: "4px solid #0a0a0a",
                    fontWeight: 900,
                    textTransform: "uppercase",
                    letterSpacing: "0.08em",
                  }}
                >
                  <th style={{ padding: "12px 16px" }}>TRANSACTION ID</th>
                  <th style={{ padding: "12px 16px" }}>SOURCE</th>
                  <th style={{ padding: "12px 16px" }}>AMOUNT</th>
                  <th style={{ padding: "12px 16px" }}>CREATED AT</th>
                </tr>
              </thead>
              <tbody>
                {xpHistory.map((tx) => (
                  <tr key={tx.transactionId} style={{ borderBottom: "2px solid #0a0a0a" }}>
                    <td style={{ padding: "12px 16px", fontFamily: "monospace", fontWeight: 700, color: "#0a0a0a" }}>
                      {tx.transactionId.slice(0, 14)}...
                    </td>
                    <td style={{ padding: "12px 16px", fontWeight: 700, color: "#0a0a0a" }}>{tx.source}</td>
                    <td style={{ padding: "12px 16px", color: "#ff2db7", fontWeight: 900, fontSize: "13px" }}>
                      +{tx.amount} XP
                    </td>
                    <td style={{ padding: "12px 16px", fontWeight: 700, color: "rgba(10,10,10,0.7)" }}>
                      {new Date(tx.createdAt).toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Achievements */}
      <div>
        <h3 className="display" style={{ fontSize: "28px", color: "#0a0a0a", marginBottom: "12px" }}>
          ACHIEVEMENTS
        </h3>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "16px" }}>
          {achievements.map((ach) => (
            <div
              key={ach.id}
              className="edge-thick shadow-pop shadow-pop-hover"
              style={{
                backgroundColor: ach.unlockedAt ? "#ffd000" : "#ffffff",
                padding: "18px",
                display: "flex",
                flexDirection: "column",
                justifyContent: "space-between",
                gap: "8px",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span className="display" style={{ fontSize: "16px", color: "#0a0a0a" }}>
                  {ach.title}
                </span>
                <span
                  className="font-grotesk shadow-pop-sm"
                  style={{
                    fontSize: "11px",
                    color: "#ffffff",
                    backgroundColor: "#0a0a0a",
                    padding: "2px 8px",
                    fontWeight: 900,
                  }}
                >
                  +{ach.rewardXp} XP
                </span>
              </div>
              <div className="font-grotesk" style={{ fontSize: "12px", fontWeight: 600, color: "rgba(10,10,10,0.8)" }}>
                {ach.description}
              </div>
              <div
                className="font-grotesk"
                style={{
                  fontSize: "11px",
                  fontWeight: 900,
                  color: "#0a0a0a",
                  marginTop: "4px",
                  textTransform: "uppercase",
                }}
              >
                {ach.unlockedAt
                  ? `★ UNLOCKED: ${new Date(ach.unlockedAt).toLocaleDateString()}`
                  : `PROGRESS: ${ach.progress} / ${ach.threshold}`}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
