import { describe, expect, it } from "vitest";
import catalogJson from "../../../../../crates/avatar/catalog/characters.json";
import { normalizeSearchText, searchCatalog } from "../catalogSearch";
import type { CharacterSpec } from "../types";

const catalog = (catalogJson as { characters: CharacterSpec[] }).characters.map((spec) => ({ spec }));

function ids(entries: { spec: CharacterSpec }[]): string[] {
  return entries.map(({ spec }) => spec.id);
}

describe("catalog search", () => {
  it("normalizes accents, case, and whitespace", () => {
    expect(normalizeSearchText("  Pokémon   CAFÉ ")).toBe("pokemon cafe");
  });

  it("never returns legacy franchise rows, even by exact name", () => {
    expect(ids(searchCatalog(catalog, "Nezuko"))).toEqual([]);
    expect(searchCatalog(catalog, "").length).toBe(catalog.filter(({ spec }) => spec.threeDEligible).length);
    expect(searchCatalog(catalog, "").every(({ spec }) => spec.category !== "legacy")).toBe(true);
  });

  it("ranks exact name before name prefix before token matches", () => {
    expect(ids(searchCatalog(catalog, "aoi"))[0]).toBe("miyora.aoi");
    const ranked = searchCatalog(catalog, "fox");
    const firstTokenOnly = ranked.findIndex(({ spec }) => !normalizeSearchText(spec.name).startsWith("fox"));
    const lastPrefix = ranked.map(({ spec }) => normalizeSearchText(spec.name).startsWith("fox")).lastIndexOf(true);
    if (firstTokenOnly !== -1 && lastPrefix !== -1) expect(lastPrefix).toBeLessThan(firstTokenOnly);
  });

  it("keeps catalog order for ties", () => {
    const order = searchCatalog(catalog, "", "animal").map(({ spec }) => catalog.findIndex((entry) => entry.spec.id === spec.id));
    expect(order).toEqual([...order].sort((a, b) => a - b));
  });

  it("filters by category and matches species tokens", () => {
    const robots = searchCatalog(catalog, "", "robot");
    expect(robots.length).toBeGreaterThan(0);
    expect(robots.every(({ spec }) => spec.category === "robot")).toBe(true);
    expect(searchCatalog(catalog, "slime").some(({ spec }) => spec.body === "slime")).toBe(true);
    expect(ids(searchCatalog(catalog, "ganesha", "deity_inspired"))).toContain("miyora.ganesha");
  });

  it("every bundled row carries attribution and a consistent category", () => {
    for (const { spec } of catalog) {
      expect(spec.attribution?.license, spec.id).toBeTruthy();
      expect(spec.threeDEligible, spec.id).toBe(spec.category !== "legacy");
    }
  });
});
