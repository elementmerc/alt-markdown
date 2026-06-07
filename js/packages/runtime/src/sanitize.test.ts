// @vitest-environment jsdom
import { describe, expect, it } from "vitest";

import { sanitizeHtml } from "./sanitize";

describe("sanitizeHtml", () => {
  it("strips script tags but keeps safe content", () => {
    const out = sanitizeHtml("<p>ok</p><script>alert(1)</script>");
    expect(out).not.toContain("<script");
    expect(out).toContain("ok");
  });

  it("strips event handlers", () => {
    const out = sanitizeHtml('<img src="x" onerror="alert(1)">');
    expect(out.toLowerCase()).not.toContain("onerror");
  });

  it("strips javascript: urls", () => {
    const out = sanitizeHtml('<a href="javascript:alert(1)">x</a>');
    expect(out).not.toContain("javascript:");
  });
});
