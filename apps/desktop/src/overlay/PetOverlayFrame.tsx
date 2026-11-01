import { useEffect, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invokeSafe } from "../utils/ipc";

const OVERLAY_MODE_EVENT = "overlay-mode-changed";
const ACTIVITY_REPORT_INTERVAL_MS = 1_000;

/**
 * Transparent frame around the desktop pet. Normally the window is
 * click-through (set by Rust), so nothing here receives input. In reposition
 * mode the pet can be dragged; Escape ends the mode early.
 */
export function PetOverlayFrame({ children }: { children: ReactNode }) {
  const [repositioning, setRepositioning] = useState(false);
  const frameRef = useRef<HTMLElement | null>(null);
  const lastActivityReport = useRef(0);

  useEffect(() => {
    let stop: (() => void) | undefined;
    let disposed = false;
    listen<{ repositioning: boolean }>(OVERLAY_MODE_EVENT, (event) => setRepositioning(event.payload.repositioning))
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch((error: unknown) => console.error("[Overlay] mode listener unavailable:", error));
    return () => {
      disposed = true;
      stop?.();
    };
  }, []);

  useEffect(() => {
    if (repositioning) frameRef.current?.focus();
  }, [repositioning]);

  const reportActivity = () => {
    const now = Date.now();
    if (now - lastActivityReport.current < ACTIVITY_REPORT_INTERVAL_MS) return;
    lastActivityReport.current = now;
    void invokeSafe("report_overlay_activity");
  };

  return (
    <main
      ref={frameRef}
      className="agentpet-overlay-window"
      aria-label={repositioning ? "Miyora pet, reposition mode. Drag to move, press Escape when done." : "Miyora pet"}
      tabIndex={repositioning ? 0 : -1}
      onMouseDown={
        repositioning
          ? () => {
              void getCurrentWindow()
                .startDragging()
                .catch((error: unknown) => console.error("[Overlay] drag failed:", error));
            }
          : undefined
      }
      onPointerMove={repositioning ? reportActivity : undefined}
      onKeyDown={
        repositioning
          ? (event) => {
              if (event.key === "Escape") void invokeSafe("exit_overlay_reposition");
              else reportActivity();
            }
          : undefined
      }
      style={{
        width: "100vw",
        height: "100vh",
        backgroundColor: "transparent",
        display: "flex",
        alignItems: "flex-end",
        justifyContent: "center",
        overflow: "hidden",
        cursor: repositioning ? "grab" : "default",
        outline: "none",
      }}
    >
      {children}
    </main>
  );
}
