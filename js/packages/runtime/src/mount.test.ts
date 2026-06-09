// @vitest-environment jsdom
import { describe, expect, it } from "vitest";

import { registerComponents, revealOnScroll } from "./components";
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

  it("makes task-list checkboxes interactive and fires an event on toggle", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    mount(
      root,
      '<ul><li class="task-list-item"><input type="checkbox" disabled /> a</li></ul>',
    );
    const box = root.querySelector<HTMLInputElement>("input[type='checkbox']");
    expect(box).not.toBeNull();
    // No longer read-only.
    expect(box?.disabled).toBe(false);

    let fired: { index: number; checked: boolean } | null = null;
    root.addEventListener("altmd:taskchange", (e) => {
      fired = (e as CustomEvent<{ index: number; checked: boolean }>).detail;
    });
    if (box) {
      box.checked = true;
      box.dispatchEvent(new Event("change", { bubbles: true }));
    }
    expect(fired).toEqual({ index: 0, checked: true });
  });
});

describe("revealOnScroll", () => {
  it("marks heavy blocks for reveal and reveals them on intersection", () => {
    const callbacks: IntersectionObserverCallback[] = [];
    const observed: Element[] = [];
    class FakeObserver {
      constructor(cb: IntersectionObserverCallback) {
        callbacks.push(cb);
      }
      observe(el: Element): void {
        observed.push(el);
      }
      unobserve(): void {}
      disconnect(): void {}
    }
    const original = globalThis.IntersectionObserver;
    globalThis.IntersectionObserver = FakeObserver as unknown as typeof IntersectionObserver;

    const root = document.createElement("div");
    root.innerHTML = "<alt-chart></alt-chart><table></table><p>plain</p>";
    revealOnScroll(root);

    const chart = root.querySelector("alt-chart");
    expect(chart?.classList.contains("altmd-reveal")).toBe(true);
    expect(observed).toContain(chart);
    // A plain paragraph is not a reveal target.
    expect(root.querySelector("p")?.classList.contains("altmd-reveal")).toBe(false);

    // Simulate the chart scrolling into view.
    const fire = callbacks[0];
    if (!fire) throw new Error("the observer callback was never registered");
    fire(
      [{ isIntersecting: true, target: chart } as unknown as IntersectionObserverEntry],
      { unobserve() {} } as unknown as IntersectionObserver,
    );
    expect(chart?.classList.contains("altmd-revealed")).toBe(true);

    globalThis.IntersectionObserver = original;
  });
});
