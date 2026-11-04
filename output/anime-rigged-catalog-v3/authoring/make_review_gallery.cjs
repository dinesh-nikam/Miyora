const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('C:/Users/DineshNikam/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');

const escapeHtml = value => String(value).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

async function main() {
  const [directory, address = 'http://127.0.0.1:1425/'] = process.argv.slice(2);
  if (!directory) throw new Error('Usage: make_review_gallery.cjs MODEL_DIRECTORY [LOOPBACK_VIEWER_URL]');
  const root = path.resolve(directory);
  const url = new URL(address);
  if (url.protocol !== 'http:' || !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) throw new Error('Only a loopback model viewer is permitted');
  const entries = JSON.parse(fs.readFileSync(path.join(root, 'collection.json'), 'utf8')).characters;
  const report = JSON.parse(fs.readFileSync(path.join(root, 'runtime-validation.json'), 'utf8'));
  if (report.status !== 'passed' || report.checked !== entries.length) throw new Error('Review gallery requires a complete runtime validation');
  const review = path.join(root, 'review');
  fs.mkdirSync(review, { recursive: true });
  const sheets = [];
  for (let offset = 0; offset < entries.length; offset += 24) {
    const number = sheets.length + 1;
    const cards = entries.slice(offset, offset + 24).map(entry => {
      if (!/^[a-z0-9-]+\/model\.vrm$/.test(entry.model)) throw new Error('Unsafe character path');
      const slug = entry.model.split('/')[0];
      if (!fs.existsSync(path.join(root, slug, 'review.png'))) throw new Error('Missing model review image: ' + entry.id);
      return `<a class="card" href="../index.html?character=${encodeURIComponent(entry.id)}"><img src="../${slug}/review.png" alt="${escapeHtml(entry.name)}"><strong>${escapeHtml(entry.name)}</strong><span>${escapeHtml(entry.body)} · Open 3D model</span></a>`;
    }).join('');
    const html = `<!doctype html><html lang="en"><meta charset="utf-8"><title>Miyora model review ${number}</title><style>*{box-sizing:border-box}body{margin:0;padding:20px;background:#16121e;color:#eee;font:15px system-ui}header{display:flex;justify-content:space-between;align-items:center;margin-bottom:16px}h1{font-size:24px;margin:0}a{color:#f0c1de}main{display:grid;grid-template-columns:repeat(6,1fr);gap:12px}.card{display:flex;flex-direction:column;align-items:center;min-height:245px;text-decoration:none;border:1px solid #4a3a55;border-radius:8px;padding:7px;background:#211b2b}img{width:100%;height:198px;object-fit:contain;background:#14121d}strong{margin-top:4px}span{font-size:11px;color:#b9a7ca}</style><header><h1>Miyora · Refined model review ${number}</h1><a href="index.html">All review sheets</a></header><main>${cards}</main></html>`;
    const name = `sheet-${number}.html`;
    fs.writeFileSync(path.join(review, name), html);
    sheets.push(name);
  }
  fs.writeFileSync(path.join(review, 'index.html'), `<!doctype html><html lang="en"><meta charset="utf-8"><title>Miyora refined collection review</title><style>body{background:#191321;color:#eee;font:18px system-ui;padding:40px}a{color:#f4bbdb}li{margin:14px}</style><h1>${entries.length} refined rigged models</h1><p>Each sheet links directly to its real 3D models, editable sources, animation clips, and facial expressions.</p><ul>${sheets.map((name, i) => `<li><a href="${name}">Review sheet ${i + 1}</a></li>`).join('')}</ul><p><a href="../index.html">Interactive 3D collection</a></p></html>`);
  const browser = await chromium.launch({ executablePath: 'C:/Users/DineshNikam/AppData/Local/Google/Chrome/Application/chrome.exe', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1800, height: 1200 } });
    for (const name of sheets) {
      await page.goto(new URL('review/' + name, address).href, { waitUntil: 'networkidle' });
      await page.evaluate(async () => {
        for (const image of document.images) {
          if (image.complete && image.naturalWidth > 0) continue;
          try { await image.decode(); }
          catch {
            const retry = new URL(image.src);
            retry.searchParams.set('review-retry', '1');
            image.src = retry.href;
            await image.decode();
          }
          if (!image.naturalWidth) throw new Error('A model review image did not load');
        }
      });
      await page.screenshot({ path: path.join(review, name.replace('.html', '.png')), fullPage: true });
    }
  } finally {
    await browser.close();
  }
  console.log(JSON.stringify({ event: 'review-gallery-created', models: entries.length, sheets: sheets.length }));
}

main().catch(error => { console.error(error.message); process.exitCode = 1; });
