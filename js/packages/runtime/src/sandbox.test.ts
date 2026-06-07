// @vitest-environment jsdom
import { describe, expect, it } from "vitest";

import { buildSrcdoc, createSandboxedFrame } from "./sandbox";

describe("sandbox", () => {
  it("never grants same-origin", () => {
    const frame = createSandboxedFrame({ html: "<p>x</p>", allowScripts: true });
    const sandbox = frame.getAttribute("sandbox") ?? "";
    expect(sandbox).toBe("allow-scripts");
    expect(sandbox).not.toContain("allow-same-origin");
  });

  it("uses an empty sandbox when scripts are not needed", () => {
    const frame = createSandboxedFrame({ html: "<p>x</p>" });
    expect(frame.getAttribute("sandbox")).toBe("");
  });

  it("blocks network and forbids scripts by default in the inner CSP", () => {
    const doc = buildSrcdoc("<p>x</p>", false);
    expect(doc).toContain("default-src 'none'");
    expect(doc).toContain("connect-src 'none'");
    expect(doc).toContain("script-src 'none'");
  });

  it("allows inline scripts only when requested", () => {
    expect(buildSrcdoc("<p>x</p>", true)).toContain("script-src 'unsafe-inline'");
  });
});
