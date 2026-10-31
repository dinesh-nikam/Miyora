const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('C:/Users/DineshNikam/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');

async function main() {
  const [directory, address = 'http://127.0.0.1:1425/'] = process.argv.slice(2);
  if (!directory) throw new Error('Usage: verify-refined.cjs MODEL_DIRECTORY [LOOPBACK_VIEWER_URL]');
  const url = new URL(address);
  if (url.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname)) throw new Error('Only a local model viewer is permitted');
  const root = path.resolve(directory);
  const entries = JSON.parse(fs.readFileSync(path.join(root, 'collection.json'), 'utf8')).characters;
  if (!Array.isArray(entries) || entries.length < 1 || entries.length > 512) throw new Error('Invalid model collection size');
  const samples = new Set(['nezuko', 'tanjiro', 'luffy', 'gojo', 'naruto', 'goku', 'aoi', 'mira', 'radha', 'ganesha', 'ember', 'bao', 'pikachu', 'lumibug', 'puffaloo', 'magnebit']);
  const errors = [], results = [];
  const browser = await chromium.launch({ executablePath: 'C:/Users/DineshNikam/AppData/Local/Google/Chrome/Application/chrome.exe', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 1050 } });
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => {
      if (message.type() === 'error' && !message.location().url?.endsWith('/favicon.ico')) errors.push(message.text());
    });
    await page.goto(address, { waitUntil: 'load' });
    for (const entry of entries) {
      if (!/^[a-z0-9-]+\/model\.vrm$/.test(entry.model)) throw new Error('Unsafe collection model path');
      const slug = entry.model.split('/')[0];
      await page.locator('#character').selectOption(entry.id);
      await page.waitForFunction(id => {
        const s = document.querySelector('#status');
        return s.dataset.character === id && ['true', 'false'].includes(s.dataset.loaded);
      }, entry.id, { timeout: 60000 });
      await page.waitForTimeout(200);
      const qa = await page.evaluate(() => window.MODEL_QA);
      if (!qa?.vrm || qa.id !== entry.id || qa.sha256 !== entry.sha256 || qa.skinned < 1 || qa.clips.length !== 15) throw new Error('Rig or integrity check failed: ' + entry.id);
      if (!['blink', 'happy', 'sad', 'aa', 'surprised'].every(name => qa.expressions[name])) throw new Error('Facial morph did not drive mesh weights: ' + entry.id);
      if (qa.poseChecks?.length !== 15 || !qa.poseChecks.every(p => p.finite && p.samples > 0)) throw new Error('Pose deformation check failed: ' + entry.id);
      if (qa.toonMaterials < 1 || !qa.springFinite || (qa.springJoints > 0 && qa.springMaxRotation <= .00001)) throw new Error('Material or secondary motion check failed: ' + entry.id);
      await page.locator('#front').click();
      await page.waitForTimeout(60);
      await page.locator('canvas').screenshot({ path: path.join(root, slug, 'review.png') });
      if (samples.has(slug)) {
        for (const state of ['working', 'success']) {
          await page.locator('#animation').selectOption(state);
          await page.waitForTimeout(120);
          await page.locator('canvas').screenshot({ path: path.join(root, slug, 'runtime-' + state + '.png') });
        }
        await page.locator('#animation').selectOption('idle');
        await page.locator('#portrait').click();
        await page.locator('#expression').selectOption('happy');
        await page.waitForTimeout(100);
        await page.locator('canvas').screenshot({ path: path.join(root, slug, 'portrait-happy.png') });
        await page.locator('#front').click();
      }
      results.push({ ...qa, status: 'passed' });
      if (results.length % 10 === 0 || results.length === entries.length) {
        fs.writeFileSync(path.join(root, 'refinement-progress.json'), JSON.stringify({ schemaVersion: 1, requested: entries.length, phase: 'runtime-checks', verified: results.length, failed: errors.length }, null, 2));
        console.log(JSON.stringify({ event: 'runtime-checked', verified: results.length, requested: entries.length }));
      }
    }
    if (errors.length) throw new Error(errors.join('\n'));
    const report = { schemaVersion: 1, engine: 'Chrome WebGL + three-vrm', status: 'passed', checked: results.length, poseChecks: results.length * 15, expressionChecks: results.length * 5, errors, characters: results };
    fs.writeFileSync(path.join(root, 'runtime-validation.json'), JSON.stringify(report, null, 2));
    const statusPath = path.join(root, 'production-status.json');
    const status = JSON.parse(fs.readFileSync(statusPath, 'utf8'));
    status.runtimeValidated = results.length;
    status.refinementRevision = JSON.parse(fs.readFileSync(path.join(root, entries[0].model.split('/')[0], 'production.json'), 'utf8')).revision;
    status.poseChecks = report.poseChecks;
    status.expressionChecks = report.expressionChecks;
    status.artReview = 'refined-assets-with-per-model-review-images';
    fs.writeFileSync(statusPath, JSON.stringify(status, null, 2));
    console.log(JSON.stringify({ event: 'refinement-validation-complete', checked: results.length, poses: report.poseChecks, expressions: report.expressionChecks }));
  } finally {
    await browser.close();
  }
}

main().catch(error => { console.error(error.message); process.exitCode = 1; });
