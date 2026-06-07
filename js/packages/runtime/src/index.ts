// @altmd/runtime - the zero-build alt-markdown CDN runtime (scaffold).
//
// Phase 4 loads the WASM core, walks the render plan, and upgrades component
// nodes into Web Components (static fallback first, then progressive
// enhancement). Phase 0 exports the version and a placeholder bootstrap so the
// build and tests are real.

export const VERSION = "0.1.0";

export {
  createSandboxedFrame,
  buildSrcdoc,
  type SandboxOptions,
} from "./sandbox";
export { sanitizeHtml } from "./sanitize";
export { AltElement, registerComponents, V1_COMPONENTS } from "./components";
export { mount, renderInto, type RenderFn } from "./mount";

import { registerComponents } from "./components";

/**
 * Bootstrap the runtime: define the component custom elements so any `alt-<name>`
 * elements already in the document upgrade in place. Call once on page load.
 */
export function bootstrap(): void {
  registerComponents();
}
