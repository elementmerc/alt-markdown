// Sandboxed-iframe host for untrusted or arbitrary interactive content:
// diagrams, embeds, and the :::sandbox escape hatch.
//
// The iframe is given `allow-scripts` but deliberately NOT `allow-same-origin`,
// so its content runs in an opaque origin with no access to the host page, its
// cookies, or its storage. An inner Content-Security-Policy blocks network egress
// as defence in depth. Together this contains the Mermaid-class stored-XSS that
// the prior-art research flagged (finding A8): even hostile content cannot reach
// the host or exfiltrate anything.

/** Options for building a sandboxed frame. */
export interface SandboxOptions {
  /** The HTML to render inside the sandbox. */
  html: string;
  /** Whether the content may run scripts (for example an interactive diagram). */
  allowScripts?: boolean;
  /** Optional fixed height in pixels. */
  height?: number;
  /** A title for assistive technology. */
  title?: string;
}

/**
 * Build the `srcdoc` document for a sandbox. Exposed for testing. The inner CSP
 * forbids everything by default, allows inline styles and data: images, allows
 * inline scripts only when `allowScripts` is set, and always blocks network
 * connections so nothing can be exfiltrated from inside the frame.
 */
export function buildSrcdoc(bodyHtml: string, allowScripts: boolean): string {
  const directives = [
    "default-src 'none'",
    "img-src data:",
    "style-src 'unsafe-inline'",
    allowScripts ? "script-src 'unsafe-inline'" : "script-src 'none'",
    "connect-src 'none'",
  ];
  const csp = directives.join("; ");
  return [
    "<!doctype html>",
    "<html><head>",
    `<meta http-equiv="Content-Security-Policy" content="${csp}">`,
    "</head><body>",
    bodyHtml,
    "</body></html>",
  ].join("");
}

/**
 * Create a sandboxed iframe rendering `options.html`. The caller inserts the
 * returned element into the document. The frame cannot reach the host page: no
 * same-origin, no top navigation, no forms, no popups, no network.
 */
export function createSandboxedFrame(options: SandboxOptions): HTMLIFrameElement {
  const frame = document.createElement("iframe");
  // allow-scripts only when needed; allow-same-origin is never granted.
  frame.setAttribute("sandbox", options.allowScripts ? "allow-scripts" : "");
  frame.setAttribute("referrerpolicy", "no-referrer");
  if (options.title !== undefined) {
    frame.setAttribute("title", options.title);
  }
  frame.style.border = "0";
  frame.style.width = "100%";
  if (options.height !== undefined) {
    frame.style.height = `${options.height}px`;
  }
  frame.srcdoc = buildSrcdoc(options.html, options.allowScripts ?? false);
  return frame;
}
