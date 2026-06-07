// The standard-library component custom elements.
//
// In Phase 4 each component is a minimal upgrade that marks itself enhanced, so
// the mount-and-upgrade pipeline is real and testable. Phase 5 fills in the rich
// behaviour and the per-component static fallbacks, and lazy-loads the heavy ones
// (chart, diagram) via dynamic import. Diagrams and embeds render through the
// sandboxed iframe (see sandbox.ts), never directly in the host DOM.

/** The v1 component names (the "richer nine"); each maps to an `alt-<name>` tag. */
export const V1_COMPONENTS = [
  "callout",
  "tabs",
  "accordion",
  "columns",
  "chart",
  "math",
  "table",
  "diagram",
  "embed",
] as const;

/**
 * Base class for alt-markdown components. The static fallback is already in the
 * light DOM (rendered by the core); upgrading enhances it in place, so removing
 * the runtime leaves a readable document.
 */
export class AltElement extends HTMLElement {
  connectedCallback(): void {
    this.setAttribute("data-altmd-upgraded", "");
  }
}

/**
 * Define the component custom elements. Idempotent: re-registering is a no-op, so
 * it is safe to call on every mount.
 */
export function registerComponents(): void {
  for (const name of V1_COMPONENTS) {
    const tag = `alt-${name}`;
    if (customElements.get(tag) === undefined) {
      // A fresh subclass per tag: a constructor may back only one element name.
      customElements.define(tag, class extends AltElement {});
    }
  }
}
