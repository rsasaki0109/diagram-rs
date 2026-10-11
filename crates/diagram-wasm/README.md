# diagram-rs (WebAssembly)

Build locally with `scripts/build-wasm.sh`; the npm package is not published.
Browser ESM exports initialize WebAssembly using top-level await:

```ts
import { render, check, renderWithFont } from 'diagram-rs';
const svg: string = render('flowchart LR; A[Input] --> B[SVG]');
document.querySelector('#diagram')!.innerHTML = svg;
check('flowchart TB; A-->B');
```

Serve the generated `.wasm` alongside the JavaScript module using HTTP(S), with
`application/wasm` MIME type. Direct file URLs do not work. Bundlers must preserve
the WASM asset URL and support ESM/top-level await; the verified integration is
browser ESM and an esbuild browser bundle with an explicitly copied WASM asset.
Run `npm run test:bundle` for the tested TypeScript and asset-copy workflow. Errors throw JavaScript
exceptions with source positions. The synchronous render API is available after
the import resolves. `renderWithFont(source, Uint8Array)` accepts a custom font.
Default builds include DejaVu Sans and Japanese Noto Sans JP fallback. Both
licenses ship in the package (`FONT-LICENSE.txt`, `NOTO-FONT-LICENSE.txt`).
The six supported diagram families use the same API; a custom font replaces
the bundled collection.
See the repository README for supported syntax and limits.
