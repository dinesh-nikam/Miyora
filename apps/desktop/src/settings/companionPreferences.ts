import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invokeSafe, type IpcResult } from "../utils/ipc";

export type RenderMode = "2d" | "3d";
export type CompanionVibe = "sweet" | "playful" | "loyal" | "calm";

/** Mirrors `agentpet_core::CompanionPreferences`; Rust validates and owns it. */
export interface CompanionPreferences {
  renderMode: RenderMode;
  soundMuted: boolean;
  affection: number;
  nickname: string;
  accessory: string;
  aura: string;
  vibe: CompanionVibe;
  reducedMotion: boolean;
}

interface LegacyImportOutcome {
  imported: boolean;
  preferences: CompanionPreferences;
}

export const DEFAULT_COMPANION_PREFERENCES: CompanionPreferences = {
  renderMode: "3d",
  soundMuted: false,
  affection: 75,
  nickname: "",
  accessory: "none",
  aura: "",
  vibe: "sweet",
  reducedMotion: false,
};

const PREFERENCES_EVENT = "companion-preferences-changed";

const LEGACY_KEYS = {
  renderMode: "agentpet_render_mode",
  soundMuted: "agentpet_sound_muted",
  affection: "agentpet_affection",
  nickname: "agentpet_custom_nickname",
  accessory: "agentpet_custom_accessory",
  aura: "agentpet_custom_aura",
  vibe: "agentpet_custom_vibe",
} as const;

// Frontend-generated XP must never reach the ledger: removed, never imported.
const DISCARDED_LEGACY_KEYS = ["agentpet_companion_bond_xp"];

type LegacySettings = Partial<Record<keyof typeof LEGACY_KEYS, string>>;

let current: CompanionPreferences = DEFAULT_COMPANION_PREFERENCES;
let loading: Promise<void> | null = null;
const listeners = new Set<(preferences: CompanionPreferences) => void>();

function publish(next: CompanionPreferences) {
  current = next;
  listeners.forEach((listener) => listener(next));
}

export function currentCompanionPreferences(): CompanionPreferences {
  return current;
}

function readLegacySettings(): LegacySettings | null {
  try {
    const legacy: LegacySettings = {};
    for (const [field, key] of Object.entries(LEGACY_KEYS) as [keyof typeof LEGACY_KEYS, string][]) {
      const value = localStorage.getItem(key);
      if (value !== null) legacy[field] = value;
    }
    return legacy;
  } catch {
    return null; // storage unavailable: nothing to import
  }
}

function removeLegacyKeys() {
  try {
    [...Object.values(LEGACY_KEYS), ...DISCARDED_LEGACY_KEYS].forEach((key) => localStorage.removeItem(key));
  } catch {
    // storage unavailable: nothing left to remove
  }
}

/** Keys are removed only after Rust commits the import, so a failure retries next launch. */
async function importLegacySettings(): Promise<void> {
  const legacy = readLegacySettings();
  if (legacy && Object.keys(legacy).length > 0) {
    const result = await invokeSafe<LegacyImportOutcome>("import_legacy_companion_settings", { legacy });
    if (!result.ok) return;
  }
  removeLegacyKeys();
}

export function loadCompanionPreferences(): Promise<void> {
  loading ??= (async () => {
    await importLegacySettings();
    try {
      await listen<CompanionPreferences>(PREFERENCES_EVENT, (event) => publish(event.payload));
    } catch {
      // Outside Tauri (tests, plain browser preview) there is no event bus.
    }
    const result = await invokeSafe<CompanionPreferences>("get_companion_preferences");
    if (result.ok) publish(result.data);
  })();
  return loading;
}

/** Applies the change immediately and reverts it if Rust rejects or fails to save it. */
export async function saveCompanionPreferences(
  patch: Partial<CompanionPreferences>,
): Promise<IpcResult<CompanionPreferences>> {
  const previous = current;
  publish({ ...current, ...patch });
  const result = await invokeSafe<CompanionPreferences>("save_companion_preferences", { preferences: current });
  publish(result.ok ? result.data : previous);
  return result;
}

export function useCompanionPreferences(): CompanionPreferences {
  const [preferences, setPreferences] = useState(current);
  useEffect(() => {
    listeners.add(setPreferences);
    setPreferences(current);
    void loadCompanionPreferences();
    return () => {
      listeners.delete(setPreferences);
    };
  }, []);
  return preferences;
}

/** Test-only: forget cached state so each test starts from defaults. */
export function resetCompanionPreferencesForTests() {
  current = DEFAULT_COMPANION_PREFERENCES;
  loading = null;
  listeners.clear();
}
