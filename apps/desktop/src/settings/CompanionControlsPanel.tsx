import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invokeSafe } from "../utils/ipc";

type ShortcutAction = "show_hide" | "pause" | "reposition";

type ShortcutStatus =
  | { state: "registered"; accelerator: string }
  | { state: "unbound" }
  | { state: "conflict"; accelerator: string; message: string };

interface ShortcutStatusEntry {
  action: ShortcutAction;
  status: ShortcutStatus;
}

const SHORTCUT_STATUS_EVENT = "shortcut-status-changed";

const ACTION_LABELS: Record<ShortcutAction, string> = {
  show_hide: "Show/Hide pet",
  pause: "Pause/Resume",
  reposition: "Reposition mode",
};

function describeStatus(status: ShortcutStatus): string {
  switch (status.state) {
    case "registered":
      return `Active: ${status.accelerator}`;
    case "unbound":
      return "No shortcut. Use the tray icon or the button above.";
    case "conflict":
      return `${status.message}. This action is unbound; choose another shortcut. It stays available from the tray and the button above.`;
  }
}

const buttonStyle = {
  padding: "8px 14px",
  backgroundColor: "#ffffff",
  color: "#0a0a0a",
  border: "2px solid #0a0a0a",
  borderRadius: "4px",
  fontSize: "12px",
  fontWeight: 800,
  cursor: "pointer",
} as const;

/**
 * Keyboard-complete replacement for the controls removed from the desktop
 * overlay: run each companion action and configure its global shortcut.
 */
export function CompanionControlsPanel() {
  const [entries, setEntries] = useState<ShortcutStatusEntry[]>([]);
  const [drafts, setDrafts] = useState<Partial<Record<ShortcutAction, string>>>({});
  const [message, setMessage] = useState<string | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    let stop: (() => void) | undefined;
    let disposed = false;
    void invokeSafe<ShortcutStatusEntry[]>("get_companion_shortcuts").then((result) => {
      if (disposed || !mountedRef.current) return;
      if (result.ok) setEntries(result.data);
      else setMessage(`Could not load shortcuts: ${result.error}`);
    });
    listen<ShortcutStatusEntry[]>(SHORTCUT_STATUS_EVENT, (event) => {
      if (!disposed && mountedRef.current) setEntries(event.payload);
    })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((error: unknown) => console.error("[Shortcuts] status listener unavailable:", error));
    return () => {
      disposed = true;
      stop?.();
    };
  }, []);

  const runAction = async (action: ShortcutAction) => {
    const result = await invokeSafe("run_companion_action", { action });
    if (!mountedRef.current) return;
    setMessage(result.ok ? null : `${ACTION_LABELS[action]} failed: ${result.error}`);
  };

  const saveShortcut = async (action: ShortcutAction, accelerator: string | null) => {
    const result = await invokeSafe<ShortcutStatusEntry[]>("set_companion_shortcut", { action, accelerator });
    if (!mountedRef.current) return;
    if (result.ok) {
      setEntries(result.data);
      setDrafts((previous) => ({ ...previous, [action]: undefined }));
      setMessage(`${ACTION_LABELS[action]} shortcut ${accelerator ? "saved" : "removed"}.`);
    } else {
      setMessage(`${ACTION_LABELS[action]} shortcut not changed: ${result.error}`);
    }
  };

  return (
    <section
      aria-labelledby="companion-controls-heading"
      className="edge-thick"
      style={{ padding: "24px", display: "flex", flexDirection: "column", gap: "16px", backgroundColor: "#ffffff" }}
    >
      <div>
        <h2 id="companion-controls-heading" className="display" style={{ fontSize: "22px", margin: 0 }}>
          COMPANION CONTROLS
        </h2>
        <p className="font-grotesk" style={{ fontSize: "12px", fontWeight: 700, margin: "4px 0 0" }}>
          The desktop pet has no on-screen buttons and lets clicks pass through to your apps. Control it here, from
          the Miyora tray icon, or with these shortcuts.
        </p>
      </div>

      <div style={{ display: "flex", gap: "10px", flexWrap: "wrap" }}>
        {(Object.keys(ACTION_LABELS) as ShortcutAction[]).map((action) => (
          <button key={action} type="button" style={buttonStyle} onClick={() => void runAction(action)}>
            {ACTION_LABELS[action]}
          </button>
        ))}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
        {entries.map(({ action, status }) => {
          const inputId = `shortcut-${action}`;
          const current = status.state === "unbound" ? "" : status.accelerator;
          const draft = drafts[action] ?? current;
          return (
            <div key={action} style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
              <div style={{ display: "flex", gap: "8px", alignItems: "center", flexWrap: "wrap" }}>
                <label htmlFor={inputId} className="font-grotesk" style={{ fontWeight: 800, minWidth: "140px" }}>
                  {ACTION_LABELS[action]}
                </label>
                <input
                  id={inputId}
                  value={draft}
                  placeholder="Ctrl+Alt+P"
                  maxLength={64}
                  aria-describedby={`${inputId}-status`}
                  onChange={(event) => setDrafts((previous) => ({ ...previous, [action]: event.target.value }))}
                  style={{ padding: "6px 10px", border: "2px solid #0a0a0a", borderRadius: "4px", fontSize: "12px" }}
                />
                <button
                  type="button"
                  style={buttonStyle}
                  disabled={draft.trim() === "" || (draft === current && status.state === "registered")}
                  onClick={() => void saveShortcut(action, draft.trim())}
                >
                  Save
                </button>
                <button
                  type="button"
                  style={buttonStyle}
                  disabled={status.state === "unbound"}
                  onClick={() => void saveShortcut(action, null)}
                >
                  Remove
                </button>
              </div>
              <div
                id={`${inputId}-status`}
                className="font-grotesk"
                style={{ fontSize: "12px", color: status.state === "conflict" ? "#b91c1c" : "#334155" }}
              >
                {describeStatus(status)}
              </div>
            </div>
          );
        })}
      </div>

      <div role="status" aria-live="polite" className="font-grotesk" style={{ fontSize: "12px", fontWeight: 700 }}>
        {message}
      </div>
    </section>
  );
}
