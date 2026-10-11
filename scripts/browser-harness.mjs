import { createServer } from 'node:http';
import { readFile, realpath } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
import { chromium } from 'playwright';
export async function withBrowser(run) {
  const root = await realpath(new URL('..', import.meta.url).pathname);
  const server = createServer(async (req, res) => {
    try {
      const path = resolve(root, '.' + decodeURIComponent(new URL(req.url, 'http://localhost').pathname));
      if (!path.startsWith(root + sep)) { res.writeHead(403).end(); return; }
      const file = await realpath(path);
      if (!file.startsWith(root + sep)) { res.writeHead(403).end(); return; }
      const mime = { '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.html': 'text/html' };
      res.writeHead(200, { 'Content-Type': mime[extname(file)] || 'text/plain' });
      res.end(await readFile(file));
    } catch { res.writeHead(404).end(); }
  });
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  let browser;
  try {
    const options = process.env.DIAGRAM_CHROMIUM ? { executablePath: process.env.DIAGRAM_CHROMIUM } : {};
    browser = await chromium.launch({ headless: true, ...options });
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${server.address().port}/examples/browser.html`);
    return await run(page);
  } finally {
    if (browser) await browser.close();
    await new Promise(r => server.close(r));
  }
}
