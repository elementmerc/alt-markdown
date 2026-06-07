// @altmd/runtime - the zero-build alt-markdown CDN runtime (scaffold).
//
// Phase 4 loads the WASM core, walks the render plan, and upgrades component
// nodes into Web Components (static fallback first, then progressive
// enhancement). Phase 0 exports the version and a placeholder bootstrap so the
// build and tests are real.

export const VERSION = "0.1.0";

/**
 * Bootstrap the runtime against a root node. Phase 4 wires the WASM core and the
 * component registry; this is intentionally minimal in Phase 0.
 */
export function bootstrap(_root: ParentNode): void {
  // Phase 4: load WASM, scan for component nodes, enhance.
}
