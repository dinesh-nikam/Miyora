import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  DEFAULT_COMPANION_PREFERENCES,
  currentCompanionPreferences,
  loadCompanionPreferences,
  resetCompanionPreferencesForTests,
  saveCompanionPreferences,
} from "../companionPreferences";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => undefined) }));

const invokeMock = vi.mocked(invoke);
const stored = { ...DEFAULT_COMPANION_PREFERENCES, renderMode: "2d" as const, nickname: "Pip" };

describe("companion preferences", () => {
  beforeEach(() => {
    resetCompanionPreferencesForTests();
    localStorage.clear();
    invokeMock.mockReset();
    vi.spyOn(console, "error").mockImplementation(() => undefined);
  });

  it("imports legacy keys once, drops bond XP, and removes keys after Rust commits", async () => {
    localStorage.setItem("agentpet_render_mode", "2d");
    localStorage.setItem("agentpet_custom_nickname", "Pip");
    localStorage.setItem("agentpet_companion_bond_xp", "9000");
    invokeMock.mockImplementation(async (command) =>
      command === "import_legacy_companion_settings" ? { imported: true, preferences: stored } : stored,
    );

    await loadCompanionPreferences();

    expect(invokeMock).toHaveBeenCalledWith("import_legacy_companion_settings", {
      legacy: { renderMode: "2d", nickname: "Pip" },
    });
    expect(localStorage.length).toBe(0);
    expect(currentCompanionPreferences()).toEqual(stored);
  });

  it("keeps legacy keys when the import fails so it retries next launch", async () => {
    localStorage.setItem("agentpet_sound_muted", "true");
    invokeMock.mockImplementation(async (command) => {
      if (command === "import_legacy_companion_settings") throw new Error("COMPANION_LEGACY_IMPORT_FAILED");
      return DEFAULT_COMPANION_PREFERENCES;
    });

    await loadCompanionPreferences();

    expect(localStorage.getItem("agentpet_sound_muted")).toBe("true");
  });

  it("skips the import command when no legacy keys exist", async () => {
    invokeMock.mockResolvedValue(stored);

    await loadCompanionPreferences();

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("get_companion_preferences", undefined);
  });

  it("reverts an optimistic change that Rust rejects", async () => {
    invokeMock.mockRejectedValue(new Error("COMPANION_PREFERENCES_SAVE_FAILED"));

    const result = await saveCompanionPreferences({ affection: 101 });

    expect(result.ok).toBe(false);
    expect(currentCompanionPreferences()).toEqual(DEFAULT_COMPANION_PREFERENCES);
  });
});
