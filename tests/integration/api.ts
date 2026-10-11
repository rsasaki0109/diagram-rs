import { render, check, renderWithFont } from '../../crates/diagram-wasm/pkg/index.js';
const source = 'sequenceDiagram; A->>B: message';
const svg: string = render(source);
const checked: void = check(source);
const custom: string = renderWithFont(source, new Uint8Array());
void [svg, checked, custom];
// @ts-expect-error source must be text, not a number
render(42);
// @ts-expect-error font bytes must be a Uint8Array
renderWithFont(source, 'font.ttf');
