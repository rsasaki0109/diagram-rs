import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { withBrowser } from './browser-harness.mjs';
await withBrowser(async page => {
  await page.waitForSelector('#diagram svg');
  assert.equal(await page.locator('#diagram svg [id^="node-"]').count(), 4);
  for (const name of ['branch', 'cycle', 'groups', 'styles', 'escaping', 'chain', 'hero', 'extended', 'sequence', 'class', 'state', 'er', 'gantt', 'japanese', 'nested', 'parallel']) {
    const path = name === 'hero' ? 'examples/hero.mmd' : `tests/fixtures/${name}.mmd`;
    const source = await readFile(path, 'utf8');
    const expected = await readFile(`tests/snapshots/${name}.svg`, 'utf8');
    const actual = await page.evaluate(async source => {
      const { render, check } = await import('/crates/diagram-wasm/pkg/index.js');
      check(source); return render(source);
    }, source);
    assert.equal(actual, expected, `WASM/native SVG parity: ${name}`);
  }
  const checks = await page.evaluate(async () => {
    const api = await import('/crates/diagram-wasm/pkg/index.js');
    let badSyntax = false, badFont = false;
    try { api.render('flowchart TB\nA~~B'); } catch (e) { badSyntax = String(e).includes('2:2:'); }
    try { api.renderWithFont('flowchart TB; A', new Uint8Array([1, 2, 3])); } catch (e) { badFont = String(e).includes('invalid'); }
    const bytes = new Uint8Array(await (await fetch('/assets/fonts/DejaVuSans.ttf')).arrayBuffer());
    const custom = api.renderWithFont('flowchart TB; A', bytes) === api.render('flowchart TB; A');
    return { badSyntax, badFont, custom };
  });
  assert.deepEqual(checks, { badSyntax: true, badFont: true, custom: true });
  await page.locator('#source').fill('flowchart TB\nA~~B');
  await page.locator('#render').click();
  assert.match(await page.locator('#error').textContent(), /2:2:/);
  console.log('Browser WASM: 16 fixture parity checks, custom font, diagnostics, and UI passed');
});
