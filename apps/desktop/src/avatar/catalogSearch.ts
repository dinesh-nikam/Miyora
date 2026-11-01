import type { CharacterCategory, CharacterSpec } from "./types";

export const CATEGORY_LABELS: Record<Exclude<CharacterCategory, "legacy">, string> = {
  character: "Characters",
  animal: "Animals",
  creature: "Creatures",
  robot: "Robots",
  deity_inspired: "Deity-inspired",
};

/** NFKD, combining marks removed, case-folded, whitespace collapsed. */
export function normalizeSearchText(text: string): string {
  return text.normalize("NFKD").replace(/\p{M}/gu, "").toLowerCase().replace(/\s+/g, " ").trim();
}

function tokens(text: string): string[] {
  const normalized = normalizeSearchText(text);
  return normalized ? normalized.split(" ") : [];
}

/**
 * Filters and ranks 3D-eligible characters. Rank: exact name, then name
 * prefix, then every query token prefixing a token of the title, species,
 * tags, or personality. Ties keep catalog order.
 */
export function searchCatalog<T extends { spec: CharacterSpec }>(
  entries: readonly T[],
  query: string,
  category: CharacterCategory | "all" = "all",
): T[] {
  const needle = normalizeSearchText(query);
  const queryTokens = tokens(query);
  const ranked: { entry: T; rank: number; order: number }[] = [];
  entries.forEach((entry, order) => {
    const { spec } = entry;
    if (!spec.threeDEligible) return;
    if (category !== "all" && spec.category !== category) return;
    if (!needle) {
      ranked.push({ entry, rank: 0, order });
      return;
    }
    const name = normalizeSearchText(spec.name);
    if (name === needle) {
      ranked.push({ entry, rank: 0, order });
    } else if (name.startsWith(needle)) {
      ranked.push({ entry, rank: 1, order });
    } else {
      const haystack = [spec.name, spec.title, spec.body, spec.personality ?? "", ...spec.tags].flatMap(tokens);
      if (queryTokens.every((token) => haystack.some((word) => word.startsWith(token)))) {
        ranked.push({ entry, rank: 2, order });
      }
    }
  });
  return ranked.sort((a, b) => a.rank - b.rank || a.order - b.order).map(({ entry }) => entry);
}
