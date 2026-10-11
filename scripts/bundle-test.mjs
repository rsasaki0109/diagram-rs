import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {mkdir,copyFile,readFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {withBrowser} from './browser-harness.mjs';
execFileSync(process.execPath,['node_modules/typescript/bin/tsc','--noEmit','--strict','--target','ES2022','--module','ESNext','--moduleResolution','Bundler','--lib','ES2022,DOM','tests/integration/api.ts'],{stdio:'inherit'});
const out='benchmarks/results/bundle';await mkdir(out,{recursive:true});
await build({entryPoints:['crates/diagram-wasm/pkg/index.js'],outfile:`${out}/index.js`,bundle:true,format:'esm',target:'es2022',platform:'browser'});
await copyFile('crates/diagram-wasm/pkg/diagram_rs_bg.wasm',`${out}/diagram_rs_bg.wasm`);
await withBrowser(async page=>{for(const name of ['branch','sequence','class','state','er','gantt','japanese','nested','parallel']) {
 const source=await readFile(`tests/fixtures/${name}.mmd`,'utf8');const expected=await readFile(`tests/snapshots/${name}.svg`,'utf8');
 const actual=await page.evaluate(async source=>{const api=await import('/benchmarks/results/bundle/index.js');api.check(source);return api.render(source);},source);
 assert.equal(actual,expected,`bundled API parity: ${name}`);
}});
console.log('TypeScript API contracts and esbuild browser bundle passed for all six families, Japanese and nested/parallel graphs');
