import { useEffect, useRef } from "react";

export interface ProviderReport {
  providerId: string;
  displayName: string;
  detected: boolean;
  hookStatus: string;
  quotaStatus?: {
    fiveHour?: { usedPercentage: number; resetsAt?: string };
    sevenDay?: { usedPercentage: number; resetsAt?: string };
    contextWindowUsedPercentage?: number;
    updatedAt: string;
    provenance: string;
  } | null;
  quotaProvenance: string;
  quotaUpdatedSecondsAgo?: number | null;
  costUsd?: number | null;
  costProvenance: string;
  inputTokens?: number | null;
  outputTokens?: number | null;
  tokensProvenance: string;
  configPath?: string | null;
}

export interface CommandDeckProps {
  isOpen: boolean;
  onClose: () => void;
  providers: ProviderReport[];
  activePetName: string;
  activeState: string;
  isSimulated: boolean;
  onToggleSimulated: () => void;
  onSelectPet: (id: string) => void;
  installedPacks: { id: string; selectionId: string; name: string }[];
  onVerifyLedger: () => void;
  latencyMs?: number | null;
}

/** Quota share at which a meter switches to the warning colour. */
const QUOTA_WARNING_PERCENT = 80;

const PROVENANCE_LABELS: Record<string, string> = {
  LIVE: "live",
  LOCAL: "from local logs",
};

function provenanceLabel(provenance: string): string | null {
  return PROVENANCE_LABELS[provenance] ?? null;
}

function formatResetTime(resetsAt: string | undefined): string | null {
  if (!resetsAt) return null;
  const date = new Date(resetsAt);
  if (Number.isNaN(date.getTime())) return null;
  return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

function formatAge(seconds: number): string {
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  return `${Math.floor(minutes / 60)}h ago`;
}

function hookLabel(p: ProviderReport): string {
  return p.hookStatus === "installed" ? "Hook on" : "Detected";
}

function QuotaMeter({ label, used, resetsAt }: { label: string; used: number; resetsAt?: string }) {
  const clamped = Math.min(100, Math.max(0, used));
  const reset = formatResetTime(resetsAt);
  return (
    <div className="deck-meter">
      <div className="deck-meter-text">
        <span className="deck-meter-value">{clamped.toFixed(0)}%</span>
        <span>of {label} used</span>
        {reset && <span className="deck-quiet">resets {reset}</span>}
      </div>
      <div
        className="deck-meter-track"
        role="meter"
        aria-label={`${label} quota used`}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(clamped)}
      >
        <div
          className={clamped >= QUOTA_WARNING_PERCENT ? "deck-meter-fill is-high" : "deck-meter-fill"}
          style={{ width: `${clamped}%` }}
        />
      </div>
    </div>
  );
}

function AgentRow({ p }: { p: ProviderReport }) {
  const fiveHour = p.quotaStatus?.fiveHour;
  const sevenDay = p.quotaStatus?.sevenDay;
  const quotaSource = provenanceLabel(p.quotaProvenance);
  const costSource = provenanceLabel(p.costProvenance);
  const hasTokens = p.inputTokens !== undefined && p.inputTokens !== null;
  const hasCost = p.costUsd !== undefined && p.costUsd !== null;

  return (
    <li className="deck-agent">
      <div className="deck-agent-head">
        <span className="deck-agent-name">{p.displayName}</span>
        <span className="deck-agent-status">{hookLabel(p)}</span>
      </div>

      {fiveHour ? (
        <QuotaMeter label="5-hour limit" used={fiveHour.usedPercentage} resetsAt={fiveHour.resetsAt} />
      ) : (
        <p className="deck-quiet">No quota reading yet.</p>
      )}

      {(sevenDay || hasCost || hasTokens) && (
      <div className="deck-agent-facts">
        {sevenDay && (
          <span>
            <strong>{sevenDay.usedPercentage.toFixed(0)}%</strong> of weekly limit
          </span>
        )}
        {hasCost && (
          <span>
            <strong>${p.costUsd?.toFixed(2)}</strong> estimated cost
          </span>
        )}
        {hasTokens && (
          <span>
            <strong>{(p.inputTokens ?? 0).toLocaleString()}</strong> in,{" "}
            <strong>{(p.outputTokens ?? 0).toLocaleString()}</strong> out
          </span>
        )}
      </div>
      )}

      {(quotaSource || costSource || p.quotaUpdatedSecondsAgo != null) && (
        <p className="deck-quiet deck-agent-source">
          {quotaSource && <span>Quota {quotaSource}</span>}
          {costSource && <span>Cost {costSource}</span>}
          {p.quotaUpdatedSecondsAgo != null && <span>Updated {formatAge(p.quotaUpdatedSecondsAgo)}</span>}
        </p>
      )}
    </li>
  );
}

export function CommandDeck({
  isOpen,
  onClose,
  providers,
  activePetName,
  activeState,
  isSimulated,
  onToggleSimulated,
  onSelectPet,
  installedPacks,
  onVerifyLedger,
  latencyMs,
}: CommandDeckProps) {
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  useEffect(() => {
    if (isOpen) closeRef.current?.focus();
  }, [isOpen]);

  if (!isOpen) return null;

  const detected = providers.filter((p) => p.detected);
  const missing = providers.filter((p) => !p.detected);

  return (
    <div
      className="command-deck-overlay"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <section className="command-deck-dialog" role="dialog" aria-modal="true" aria-labelledby="deck-title">
        {isSimulated && (
          <div className="deck-simulated" role="status">
            [SIMULATED - DEV ONLY] Activity below is not from a real agent.
          </div>
        )}

        <header className="deck-header">
          <div>
            <h1 id="deck-title" className="deck-title">
              {activePetName} is {activeState}.
            </h1>
            <p className="deck-subtitle">
              Watching {detected.length} of {providers.length} coding agents
            </p>
          </div>
          <button ref={closeRef} className="deck-close" onClick={onClose} aria-label="Close command deck">
            Close <kbd>Esc</kbd>
          </button>
        </header>

        <div className="deck-body">
          <section aria-labelledby="deck-agents">
            <h2 id="deck-agents" className="deck-heading">Agents</h2>
            {detected.length > 0 ? (
              <ul className="deck-agent-list">
                {detected.map((p) => (
                  <AgentRow key={p.providerId} p={p} />
                ))}
              </ul>
            ) : (
              <p className="deck-empty">
                No coding agents found on this machine. Install Claude Code, Codex CLI or Copilot CLI, then reopen this deck.
              </p>
            )}
            {missing.length > 0 && (
              <p className="deck-quiet deck-missing">Not installed: {missing.map((p) => p.displayName).join(", ")}</p>
            )}
          </section>

          {installedPacks.length > 0 && (
            <section aria-labelledby="deck-companion">
              <h2 id="deck-companion" className="deck-heading">Companion</h2>
              <div className="deck-pets" role="group" aria-labelledby="deck-companion">
                {installedPacks.map((pack) => {
                  const isSelected = activePetName.toLowerCase().includes(pack.name.toLowerCase());
                  return (
                    <button
                      key={pack.selectionId}
                      className="deck-pet"
                      aria-pressed={isSelected}
                      onClick={() => onSelectPet(pack.selectionId)}
                    >
                      {pack.name}
                    </button>
                  );
                })}
              </div>
            </section>
          )}
        </div>

        <footer className="deck-footer">
          <button className="deck-action" onClick={onVerifyLedger}>
            Verify XP ledger
          </button>
          <button className="deck-action is-quiet" onClick={onToggleSimulated}>
            {isSimulated ? "Return to live data" : "Start simulated demo"}
          </button>
          {latencyMs !== undefined && latencyMs !== null && (
            <span className="deck-quiet deck-latency">Rendered in {latencyMs} ms</span>
          )}
        </footer>
      </section>
    </div>
  );
}
