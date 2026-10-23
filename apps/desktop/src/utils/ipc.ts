import { invoke } from "@tauri-apps/api/core";

export type IpcResult<T> =
  | { ok: true; data: T; error?: never }
  | { ok: false; error: string; data?: never };

function toErrorMessage(err: unknown): string {
  if (err instanceof Error) {
    return err.message;
  }
  if (typeof err === "string") {
    return err;
  }
  if (typeof err === "object" && err !== null && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  try {
    const serialized = JSON.stringify(err);
    if (serialized !== undefined) {
      return serialized;
    }
  } catch {
    // JSON serialization can fail on circular structures or unsupported types
  }
  return String(err);
}

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
    const message = toErrorMessage(err);
    console.error(`[IPC Error] ${cmd}:`, message);
    return { ok: false, error: message };
  }
}
