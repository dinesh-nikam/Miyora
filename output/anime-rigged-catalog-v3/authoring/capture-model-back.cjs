const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('C:/Users/DineshNikam/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');

async function main() {
  const [directory, identity = 'miyora.nezuko', address = 'http://127.0.0.1:1425/'] = process.argv.slice(2);
  if (!directory || !/^miyora\.[a-z0-9-]+$/.test(identity)) throw new Error('Pass a collection directory and a valid character identity');
  const url = new URL(address);
  if (url.protocol !== 'http:' || !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) throw new Error('Use a loopback viewer');
  const root = path.resolve(directory);
  const entry = JSON.parse(fs.readFileSync(path.join(root, 'collection.json'), 'utf8')).characters.find(e => e.id === identity);
  if (!entry || !/^[a-z0-9-]+\/model\.vrm$/.test(entry.model)) throw new Error('Character is not in the verified collection');
  url.searchParams.set('character', identity);
  const browser = await chromium.launch({ executablePath: 'C:/Users/DineshNikam/AppData/Local/Google/Chrome/Application/chrome.exe', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 1050 } });
    await page.goto(url.href);
    await page.waitForFunction(id => window.MODEL_QA?.id === id && document.querySelector('#status').dataset.loaded === 'true', identity);
    await page.locator('#back').click();
    await page.waitForTimeout(250);
    await page.locator('canvas').screenshot({ path: path.join(root, entry.model.split('/')[0], 'back-review.png') });
  } finally {
    await browser.close();
  }
  console.log('Back-view model inspection captured: ' + identity);
}

main().catch(error => { console.error(error.message); process.exitCode = 1; });
