// Initialize once before exposing the synchronous rendering API.
import init, { render, renderWithFont, check } from './diagram_rs.js';
await init();
export { render, renderWithFont, check };
