import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {withBrowser} from './browser-harness.mjs';
await withBrowser(async page=>{
 for(const name of ['branch','sequence','class','state','er','gantt','nested','parallel']) {
  const source=await readFile(`tests/fixtures/${name}.mmd`,'utf8');
  const expected=await readFile(`tests/snapshots/${name}.svg`,'utf8');
  const actual=await page.evaluate(async source=>{
   const api=await import('/crates/diagram-wasm/pkg-latin/index.js');
   api.check(source);return api.render(source);
  },source);
  assert.equal(actual,expected,`Latin-only WASM parity: ${name}`);
 }
 const source=await readFile('tests/fixtures/japanese.mmd','utf8');
 const result=await page.evaluate(async source=>{
  const api=await import('/crates/diagram-wasm/pkg-latin/index.js');
  const font=await fetch('/assets/fonts/NotoSansJP.ttf').then(r=>r.arrayBuffer());
  let error='';try {api.render(source);}catch(e){error=String(e);}
  const svg=api.renderWithFont(source,new Uint8Array(font));
  const xml=new DOMParser().parseFromString(svg,'image/svg+xml');
  return {error,svg,valid:!xml.querySelector('parsererror')};
 },source);
 assert.match(result.error,/font does not cover/);
 assert.ok(result.valid);assert.match(result.svg,/<svg/);assert.match(result.svg,/受付/);
});
console.log('Latin-only WASM: six-family parity, nested/parallel edges, missing-glyph error and custom Japanese font passed');
