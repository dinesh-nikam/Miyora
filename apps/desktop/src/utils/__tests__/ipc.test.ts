import { describe, it, expect, vi, beforeEach } from "vitest";
import { invokeSafe } from "../ipc";
import * as tauriCore from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("invokeSafe", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns ok: true with resolved data when IPC succeeds", async () => {
    const mockData = { level: 5, totalXp: 1200 };
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(mockData);

    const result = await invokeSafe<{ level: number; totalXp: number }>("get_progression");

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.data).toEqual(mockData);
      expect(result.error).toBeUndefined();
    }
    expect(tauriCore.invoke).toHaveBeenCalledWith("get_progression", undefined);
  });

  it("returns ok: false with error string when IPC rejects with string", async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce("DATABASE_LOCKED");

    const result = await invokeSafe("get_progression");

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toBe("DATABASE_LOCKED");
      expect(result.data).toBeUndefined();
    }
  });

  it("returns ok: false with Error message when IPC rejects with Error object", async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce(new Error("Connection refused"));

    const result = await invokeSafe("get_progression");

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toBe("Connection refused");
    }
  });

  it("returns ok: false with object message field when IPC rejects with object containing message", async () => {
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce({ message: "Custom object error" });

    const result = await invokeSafe("get_progression");

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toBe("Custom object error");
    }
  });

  it("handles circular rejected values gracefully without throwing", async () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    vi.mocked(tauriCore.invoke).mockRejectedValueOnce(circular);

    const result = await invokeSafe("get_progression");

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(typeof result.error).toBe("string");
      expect(result.error).toBe("[object Object]");
    }
  });

  it("passes arguments correctly to tauri invoke", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValueOnce(true);

    const result = await invokeSafe("select_pet", { packId: "starter-aoi" });

    expect(result.ok).toBe(true);
    expect(tauriCore.invoke).toHaveBeenCalledWith("select_pet", { packId: "starter-aoi" });
  });
});
