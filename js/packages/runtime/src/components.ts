// The standard-library component custom elements.
//
// Each component upgrades its static fallback in place, so removing the runtime
// leaves a readable document. Tabs become an interactive tablist; diagrams and
// the escape hatch render inside the sandboxed iframe (sandbox.ts), never in the
// host DOM. The remaining components are enhanced markers in this phase; chart,
// math, and rich diagrams gain their libraries (lazy-loaded) as a follow-up.

import { renderChart, renderDiagram, renderMath } from "./graphics";
import { createSandboxedFrame } from "./sandbox";

/** The component names that get an `alt-<name>` custom element. */
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

// Sub-component and escape-hatch names also need defining so they upgrade.
const ALL_COMPONENTS = [...V1_COMPONENTS, "tab", "column", "sandbox"] as const;

/** Base class: marks the element upgraded and runs its enhancement, if any. */
export class AltElement extends HTMLElement {
  connectedCallback(): void {
    this.setAttribute("data-altmd-upgraded", "");
    this.enhance();
  }

  // Overridden by components that enhance their fallback.
  protected enhance(): void {}
}

/** Tabs: turn the headed sections into an interactive tablist. */
class TabsElement extends AltElement {
  protected override enhance(): void {
    const tabs = this.ownTabs();
    if (tabs.length === 0) {
      return;
    }
    const tablist = document.createElement("div");
    tablist.className = "alt-tablist";
    tablist.setAttribute("role", "tablist");
    tabs.forEach((tab, index) => {
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("role", "tab");
      button.textContent = tab.getAttribute("data-title") ?? `Tab ${String(index + 1)}`;
      button.addEventListener("click", () => {
        this.select(index);
      });
      tablist.appendChild(button);
    });
    this.prepend(tablist);
    this.select(0);
  }

  private ownTabs(): HTMLElement[] {
    return Array.from(this.querySelectorAll("alt-tab")).filter(
      (tab): tab is HTMLElement => tab.closest("alt-tabs") === this,
    );
  }

  private select(index: number): void {
    this.ownTabs().forEach((tab, position) => {
      tab.hidden = position !== index;
    });
    const buttons = Array.from(this.querySelectorAll(".alt-tablist > button"));
    buttons.forEach((button, position) => {
      button.setAttribute("aria-selected", position === index ? "true" : "false");
    });
  }
}

/** Chart: render an interactive uPlot chart from the fallback data table. */
class ChartElement extends AltElement {
  protected override enhance(): void {
    // Lazy-loaded; a failure leaves the accessible fallback table in place.
    void renderChart(this).catch(() => {});
  }
}

/** Maths: typeset the fallback expression with KaTeX. */
class MathElement extends AltElement {
  protected override enhance(): void {
    void renderMath(this).catch(() => {});
  }
}

/** Diagram: render Mermaid in the host, display its SVG in a locked sandbox. */
class DiagramElement extends AltElement {
  protected override enhance(): void {
    void renderDiagram(this).catch(() => {});
  }
}

/** Escape hatch: render arbitrary fallback content inside a sandbox. */
class SandboxedElement extends AltElement {
  protected override enhance(): void {
    const frame = createSandboxedFrame({
      html: this.innerHTML,
      allowScripts: true,
      title: this.localName,
    });
    this.replaceChildren(frame);
  }
}

function baseFor(name: string): typeof AltElement {
  if (name === "tabs") {
    return TabsElement;
  }
  if (name === "chart") {
    return ChartElement;
  }
  if (name === "math") {
    return MathElement;
  }
  if (name === "diagram") {
    return DiagramElement;
  }
  if (name === "sandbox") {
    return SandboxedElement;
  }
  return AltElement;
}

/**
 * Make task-list checkboxes interactive. The renderer emits them `disabled` so a
 * no-JS reader sees a faithful read-only checklist; here we enable them and, on
 * toggle, dispatch a bubbling `altmd:taskchange` event carrying the task's index
 * and new state. The runtime does not persist the change (it has no source to
 * write to); an editing host listens for the event and rewrites the document.
 */
export function enhanceTaskLists(root: ParentNode): void {
  const boxes = root.querySelectorAll<HTMLInputElement>(
    "li.task-list-item > input[type='checkbox']",
  );
  boxes.forEach((box, index) => {
    box.disabled = false;
    box.addEventListener("change", () => {
      box.dispatchEvent(
        new CustomEvent("altmd:taskchange", {
          bubbles: true,
          detail: { index, checked: box.checked },
        }),
      );
    });
  });
}

/**
 * Define the component custom elements. Idempotent: re-registering is a no-op, so
 * it is safe to call on every mount.
 */
export function registerComponents(): void {
  for (const name of ALL_COMPONENTS) {
    const tag = `alt-${name}`;
    if (customElements.get(tag) === undefined) {
      const Base = baseFor(name);
      // A fresh subclass per tag: a constructor may back only one element name.
      customElements.define(tag, class extends Base {});
    }
  }
}
