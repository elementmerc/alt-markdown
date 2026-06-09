// Mounting: inject rendered HTML into the page and upgrade its components.

import { enhanceTaskLists, registerComponents } from "./components";

/**
 * A function that renders alt-markdown source to component-aware HTML. In the
 * browser this is backed by the WASM core (`render`); tests can inject a stub.
 */
export type RenderFn = (source: string) => string;

/**
 * Mount already-rendered, safe HTML into `root` and upgrade its components.
 *
 * The HTML must come from the core renderer (`core::render`), which is safe by
 * construction. Component custom elements upgrade automatically once defined, so
 * a plain document (no runtime) still shows the static fallbacks.
 */
export function mount(root: Element, safeHtml: string): void {
  registerComponents();
  root.innerHTML = safeHtml;
  enhanceTaskLists(root);
  // Scroll-reveal is opt-in: a host can call revealOnScroll(root) after mount.
  // It is off by default because the fade can feel out of place on a document.
}

/**
 * Render `source` with `render` and mount the result into `root`. The browser
 * entry point wires `render` to the WASM core; this keeps the runtime decoupled
 * from the core's loading mechanism.
 */
export function renderInto(root: Element, source: string, render: RenderFn): void {
  mount(root, render(source));
}
