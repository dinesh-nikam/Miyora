// Fails the build when a JavaScript chunk exceeds its raw-size budget.
import { readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const KB = 1000;
const DEFAULT_BUDGET = 500 * KB;
// shortcut: three.js core ships as one ~660 kB module that cannot be split;
// tighten this when three offers a smaller tree-shakeable build.
const BUDGETS = [{ prefix: "three-", limit: 700 * KB }];

const assets = join(dirname(fileURLToPath(import.meta.url)), "..", "dist", "assets");
const failures = [];
for (const file of readdirSync(assets).filter((name) => name.endsWith(".js"))) {
  const size = statSync(join(assets, file)).size;
  const limit = BUDGETS.find(({ prefix }) => file.startsWith(prefix))?.limit ?? DEFAULT_BUDGET;
  const line = `${file}: ${(size / KB).toFixed(1)} kB (budget ${(limit / KB).toFixed(0)} kB)`;
  if (size > limit) failures.push(line);
  else console.log(`ok   ${line}`);
}
if (failures.length > 0) {
  for (const line of failures) console.error(`OVER ${line}`);
  process.exit(1);
}
