// Defence-in-depth HTML sanitiser for the runtime.
//
// The Rust core already sanitises rendered HTML with ammonia. This is the second
// layer: any HTML the runtime itself generates and injects into the host DOM
// (component output, fallbacks) passes through DOMPurify first, so a bug in one
// layer is caught by the other. Untrusted interactive content does not come here
// at all; it goes into the sandboxed iframe (see sandbox.ts).

import DOMPurify from "dompurify";

/** Sanitise an HTML fragment to a safe subset before injecting it into the host DOM. */
export function sanitizeHtml(html: string): string {
  return DOMPurify.sanitize(html);
}
