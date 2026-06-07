// @vitest-environment jsdom
import { describe, expect, it } from "vitest";

import { registerComponents } from "./components";
import { mount, renderInto } from "./mount";

describe("mount", () => {
  it("upgrades component custom elements in place", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    mount(root, '<alt-callout data-type="note">hi</alt-callout>');
    const element = root.querySelector("alt-callout");
    expect(element).not.toBeNull();
    expect(element?.hasAttribute("data-altmd-upgraded")).toBe(true);
    expect(element?.textContent).toBe("hi");
  });

  it("registerComponents is idempotent", () => {
    registerComponents();
    expect(() => {
      registerComponents();
    }).not.toThrow();
  });

  it("renderInto uses the injected render function", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    renderInto(root, "source", () => "<alt-tabs>x</alt-tabs>");
    expect(root.querySelector("alt-tabs")?.hasAttribute("data-altmd-upgraded")).toBe(true);
  });
});
