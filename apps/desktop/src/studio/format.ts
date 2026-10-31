import type { ErrorReport, TaskView } from "./types";

/** Studio commands return a JSON-encoded ErrorReport; anything else is wrapped. */
export function parseStudioError(raw: string): ErrorReport {
  try {
    const value: unknown = JSON.parse(raw);
    if (
      typeof value === "object" &&
      value !== null &&
      "kind" in value &&
      "message" in value &&
      "suggestion" in value &&
      typeof value.kind === "string" &&
      typeof value.message === "string" &&
      typeof value.suggestion === "string"
    ) {
      const record = value as Record<string, unknown>;
      return {
        kind: value.kind,
        message: value.message,
        suggestion: value.suggestion,
        requestId: typeof record.requestId === "string" ? record.requestId : null,
        providerCode: typeof record.providerCode === "number" ? record.providerCode : null,
      };
    }
  } catch {
    // Not JSON: an IPC-level failure such as a missing command.
  }
  return { kind: "unknown", message: raw, suggestion: "Retry; restart the app if it persists.", requestId: null, providerCode: null };
}

export function taskStatusLabel(task: Pick<TaskView, "state" | "stage">): string {
  switch (task.state) {
    case "QUEUED":
      return "Queued";
    case "RUNNING":
      if (task.stage === "downloading") return "Downloading…";
      if (task.stage === "retrying") return "Reconnecting…";
      return "Generating…";
    case "SUCCEEDED":
      return "Completed";
    case "FAILED":
      return "Failed";
    case "CANCELLED":
      return "Cancelled";
    case "EXPIRED":
      return "Expired";
  }
}

export function isActive(task: Pick<TaskView, "state">): boolean {
  return task.state === "QUEUED" || task.state === "RUNNING";
}

export function formatElapsed(startIso: string, endIso: string | null, nowMs: number): string {
  const start = Date.parse(startIso);
  const end = endIso ? Date.parse(endIso) : nowMs;
  if (Number.isNaN(start) || Number.isNaN(end)) return "—";
  const totalSeconds = Math.max(0, Math.round((end - start) / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const pad = (value: number) => value.toString().padStart(2, "0");
  if (hours > 0) return `${hours}h ${pad(minutes)}m`;
  if (minutes > 0) return `${minutes}m ${pad(seconds)}s`;
  return `${seconds}s`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${units[unit]}`;
}

export function formatCount(value: number | null | undefined): string {
  return value === null || value === undefined ? "—" : value.toLocaleString("en-US");
}
