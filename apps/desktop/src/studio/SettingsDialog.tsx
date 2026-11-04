import { useState } from "react";
import { studioApi } from "./api";
import { ErrorCallout } from "./ErrorCallout";
import type { AccountBalance, ErrorReport, PollPolicy, ProviderInfo } from "./types";

interface SettingsDialogProps {
  provider: ProviderInfo;
  pollPolicy: PollPolicy;
  onClose: () => void;
  onChanged: () => void;
}

type Outcome = { ok: true } | { ok: false; error: ErrorReport };

export function SettingsDialog({ provider, pollPolicy, onClose, onChanged }: SettingsDialogProps) {
  const [apiKey, setApiKey] = useState("");
  const [policy, setPolicy] = useState(pollPolicy);
  const [balance, setBalance] = useState<AccountBalance | null>(null);
  const [error, setError] = useState<ErrorReport | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const run = async (action: () => Promise<Outcome>, success: string) => {
    setError(null);
    setNotice(null);
    const result = await action();
    if (result.ok) {
      setNotice(success);
      onChanged();
    } else {
      setError(result.error);
    }
  };

  const checkBalance = async () => {
    setError(null);
    setNotice(null);
    const result = await studioApi.accountBalance(provider.id);
    if (result.ok) setBalance(result.data);
    else setError(result.error);
  };

  const saveKey = async (): Promise<Outcome> => {
    const result = await studioApi.saveApiKey(provider.id, apiKey);
    if (result.ok) setApiKey("");
    return result;
  };

  const numberField = (label: string, key: keyof PollPolicy, min: number, max: number, scale = 1) => (
    <label className="studio-field">
      <span>{label}</span>
      <input
        type="number"
        min={min}
        max={max}
        value={Math.round(policy[key] / scale)}
        onChange={(event) => setPolicy({ ...policy, [key]: Number(event.target.value) * scale })}
      />
    </label>
  );

  return (
    <div className="studio-dialog-backdrop" role="presentation" onClick={onClose}>
      <div className="studio-dialog" role="dialog" aria-modal="true" aria-label="Studio settings" onClick={(event) => event.stopPropagation()}>
        <h3>Studio settings</h3>

        <h4>{provider.label}</h4>
        {provider.acceptsApiKey ? (
          <>
            <p className="studio-muted">
              Keys are stored in the operating-system credential manager, never in the app database or logs. Status:{" "}
              {provider.hasApiKey ? "key stored" : provider.requiresApiKey ? "no key (required)" : "no key (optional)"}.
            </p>
            <label className="studio-field">
              <span>{provider.requiresApiKey ? "API key" : "Access token (optional)"}</span>
              <input
                type="password"
                autoComplete="off"
                value={apiKey}
                placeholder={provider.hasApiKey ? "Enter a new key to replace the stored one" : "Paste key"}
                onChange={(event) => setApiKey(event.target.value)}
              />
            </label>
            <div className="studio-row-actions">
              <button className="studio-primary" disabled={!apiKey.trim()} onClick={() => void run(saveKey, "Key saved.")}>
                Save key
              </button>
              {provider.requiresApiKey && (
                <button disabled={!provider.hasApiKey} onClick={() => void checkBalance()}>
                  Check balance
                </button>
              )}
              <button disabled={!provider.hasApiKey} onClick={() => void run(() => studioApi.deleteApiKey(provider.id), "Key removed.")}>
                Remove key
              </button>
            </div>
          </>
        ) : provider.unavailableReason ? (
          <p className="studio-warning">{provider.unavailableReason}</p>
        ) : (
          <p className="studio-notice">Installed. Runs on this computer; no account or internet needed.</p>
        )}
        {balance && (
          <p>
            Available credits: <strong>{balance.available.toFixed(2)}</strong> · frozen for running tasks: {balance.frozen.toFixed(2)}{" "}
            <small className="studio-muted">(reported by {provider.label})</small>
          </p>
        )}

        <label className="studio-field">
          <span>Default model</span>
          <select
            defaultValue={provider.defaultModel}
            onChange={(event) => void run(() => studioApi.setDefaultModel(provider.id, event.target.value), "Default model saved.")}
          >
            {provider.models.map((model) => (
              <option key={model.id} value={model.id}>
                {model.label}
              </option>
            ))}
          </select>
        </label>
        <p className="studio-muted">Model list comes from the provider documentation; the provider has no model-list endpoint.</p>

        <h4>Task tracking</h4>
        <div className="studio-grid-2">
          {numberField("Polling interval (s)", "intervalSecs", 2, 60)}
          {numberField("Max backoff (s)", "maxIntervalSecs", 2, 600)}
          {numberField("Timeout (min)", "timeoutSecs", 1, 180, 60)}
          {numberField("Network errors before stop", "maxConsecutiveErrors", 1, 50)}
        </div>
        <button onClick={() => void run(() => studioApi.setPollPolicy(policy), "Tracking settings saved for newly tracked tasks.")}>
          Save tracking settings
        </button>

        {notice && <p className="studio-notice">{notice}</p>}
        {error && <ErrorCallout error={error} />}
        <div className="studio-row-actions">
          <button onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
