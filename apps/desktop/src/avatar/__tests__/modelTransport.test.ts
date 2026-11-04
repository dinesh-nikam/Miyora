// @vitest-environment node
import { describe, expect, it } from "vitest";
import { modelResponseBytes } from "../modelTransport";

describe("modelResponseBytes", () => {
  it("accepts the native binary transport without copying", () => {
    const buffer = new Uint8Array([103, 108, 84, 70]).buffer;
    expect(modelResponseBytes(buffer)).toBe(buffer);
  });

  it("decodes Tauri's JSON fallback without changing byte values", () => {
    expect(new Uint8Array(modelResponseBytes([0, 127, 128, 255]))).toEqual(
      new Uint8Array([0, 127, 128, 255]),
    );
  });

  it.each([null, {}, "model", [], [256], [-1], [1.5], [NaN], ["1"], [null], new Array(1), new ArrayBuffer(0)].map(response => ({ response })))(
    "rejects malformed transport data: $response",
    ({ response }) => expect(() => modelResponseBytes(response)).toThrow("AVATAR_MODEL_READ_FAILED"),
  );

  it("bounds fallback arrays before allocating model memory", () => {
    const oversized = new Array(50 * 1024 * 1024 + 1);
    expect(() => modelResponseBytes(oversized)).toThrow("AVATAR_MODEL_READ_FAILED");
  });
});
