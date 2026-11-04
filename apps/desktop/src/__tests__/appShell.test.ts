import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// Source-level guards for the v1 shell: the Pet Library must stay a lazy
// chunk and the generic Studio must stay unreachable from navigation.
const app = readFileSync(join(__dirname, "..", "App.tsx"), "utf-8");

describe("v1 app shell", () => {
  it("loads the Pet Library through a dynamic import only", () => {
    expect(app).toMatch(/lazy\(\(\) => import\("\.\/avatar\/CharacterStudio"\)/);
    expect(app).not.toMatch(/^import .*["']\.\/avatar\/CharacterStudio["']/m);
  });

  it("has no route, tab, or import for the generic Studio", () => {
    expect(app).not.toMatch(/StudioView/);
    expect(app).not.toMatch(/["']\.\/studio\//);
    expect(app).not.toMatch(/id: "studio"/);
  });

  it("uses the typed tab model instead of casts", () => {
    expect(app).not.toMatch(/as any/);
  });
});
