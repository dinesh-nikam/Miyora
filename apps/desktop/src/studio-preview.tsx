// Temporary design harness: mocks Tauri IPC with the bundled catalog. Not shipped.
import catalogJson from "../../../crates/avatar/catalog/characters.json";
import type { CharacterSpec, ResolvedAvatar } from "./avatar/types";

const characters = (catalogJson as { characters: CharacterSpec[] }).characters;
const resolve = (id: string): ResolvedAvatar => {
  const spec = characters.find((c) => c.id === id) ?? characters[0];
  return { source: "catalog", avatarId: spec.id, kind: "procedural", name: spec.name, spec, customized: false, modelSha256: null, metadata: null };
};
(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  invoke: async (cmd: string, args: Record<string, unknown>) => {
    if (cmd === "list_avatar_catalog") return characters.map((spec) => ({ spec, customized: spec.id === "miyora.zippa" }));
    if (cmd === "list_custom_avatars") return [];
    if (cmd === "select_avatar" || cmd === "get_avatar") return resolve(String(args?.avatarId ?? "miyora.aoi"));
    throw new Error(`mock: ${cmd}`);
  },
  transformCallback: () => 0,
};
const [{ createRoot }, { CharacterStudio }] = await Promise.all([import("react-dom/client"), import("./avatar/CharacterStudio")]);
await import("./styles.css");
createRoot(document.getElementById("root") as HTMLElement).render(
  <CharacterStudio liveState="idle" hud={{ level: 4, totalXp: 820, xpIntoLevel: 120, xpToNextLevel: 300 }} activeAvatar={resolve("miyora.aoi")} onAvatarChanged={() => undefined} />,
);
