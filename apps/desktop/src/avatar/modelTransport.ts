/** Kept equal to native VRM validation's DEFAULT_MAX_VRM_FILE_BYTES. */
const MAX_MODEL_BYTES = 50 * 1024 * 1024;

export function modelResponseBytes(response: unknown, maxBytes: number = MAX_MODEL_BYTES): ArrayBuffer {
  if (response instanceof ArrayBuffer && response.byteLength > 0 && response.byteLength <= maxBytes) {
    return response;
  }
  // Tauri serializes raw responses as JSON bytes when custom-protocol IPC is blocked.
  if (Array.isArray(response) && response.length > 0 && response.length <= maxBytes) {
    const values: unknown[] = response;
    for (const value of values) {
      if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > 255) {
        throw new Error("AVATAR_MODEL_READ_FAILED: invalid model byte");
      }
    }
    // All values, including sparse-array entries, were checked before allocation.
    return Uint8Array.from(values as number[]).buffer;
  }
  throw new Error("AVATAR_MODEL_READ_FAILED: model response was not binary");
}
