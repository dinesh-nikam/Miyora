import { invoke } from "@tauri-apps/api/core";

export type IpcResult<T> =
  | { ok: true; data: T; error?: never }
  | { ok: false; error: string; data?: never };

/**
 * Safely calls a Tauri IPC command, returning a typed Result object
 * rather than throwing or silently discarding errors.
 */
export async function invokeSafe<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<IpcResult<T>> {
  try {
    const data = await invoke<T>(cmd, args);
    return { ok: true, data };
  } catch (err: unknown) {
    const message =
      err instanceof Error
        ? err.message
        : typeof err === "string"
        ? err
        : typeof err === "object" && err !== null && "message" in err
        ? String((err as { message: unknown }).message)
        : JSON.stringify(err);
    console.error(`[IPC Error] ${cmd}:`, message);
    return { ok: false, error: message };
  }
}
