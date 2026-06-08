// Lazy-loaded rich renderers for the graphics components.
//
// The heavy libraries (uPlot for charts, KaTeX for maths) are dynamic-imported,
// so they only download when a document actually contains that component; the
// core runtime stays small. Each renderer reads its data from the component's
// own static fallback (the chart's data table, the maths code span), so this is
// pure progressive enhancement: with no runtime the fallback is what shows, and
// with the runtime the fallback becomes the rich, accessible source of truth.

import { createSandboxedFrame } from "./sandbox";

interface ChartData {
  labels: string[];
  series: { name: string; values: number[] }[];
}

/** Read a chart's data back out of its static fallback table. */
function readTableData(host: HTMLElement): ChartData | null {
  const headers = Array.from(host.querySelectorAll("thead th")).map(
    (th) => th.textContent ?? "",
  );
  const rows = Array.from(host.querySelectorAll("tbody tr")).map((tr) =>
    Array.from(tr.querySelectorAll("td")).map((td) => td.textContent ?? ""),
  );
  if (headers.length < 2 || rows.length === 0) {
    return null;
  }
  const labels = rows.map((r) => r[0] ?? "");
  const series = headers.slice(1).map((name, column) => ({
    name,
    values: rows.map((r) => Number(r[column + 1] ?? "")),
  }));
  return { labels, series };
}

const PALETTE = ["#2563eb", "#dc2626", "#16a34a", "#d97706", "#7c3aed"];

/** Render a chart into `host` from its fallback table, via uPlot (lazy-loaded). */
export async function renderChart(host: HTMLElement): Promise<void> {
  const data = readTableData(host);
  if (!data) {
    return;
  }
  const { default: uPlot } = await import("uplot");
  const kind = host.getAttribute("data-kind") ?? "line";
  const xs = data.labels.map((_, i) => i);
  const aligned = [xs, ...data.series.map((s) => s.values)] as unknown as ArrayLike<
    number | null
  >[];

  const opts: import("uplot").Options = {
    width: host.clientWidth || 640,
    height: 320,
    scales: { x: { time: false } },
    axes: [
      { values: (_self, splits) => splits.map((i) => data.labels[i] ?? "") },
      {},
    ],
    series: [
      {},
      ...data.series.map((s, i) => {
        const colour = PALETTE[i % PALETTE.length];
        return kind === "bar"
          ? { label: s.name, stroke: colour, fill: colour, paths: uPlot.paths.bars?.({ size: [0.6] }) }
          : { label: s.name, stroke: colour, width: 2 };
      }),
    ],
  };

  // Capture the fallback table before uPlot adds its own legend table, so we
  // hide the right one.
  const fallbackTable = host.querySelector("table");
  const canvas = document.createElement("div");
  canvas.className = "alt-chart-canvas";
  host.prepend(canvas);
  new uPlot(opts, aligned as never, canvas);
  // Keep the data table in the DOM for accessibility, but hide it visually now
  // that the chart is the primary representation.
  fallbackTable?.classList.add("alt-visually-hidden");
}

/** Render a maths expression into `host` from its fallback, via KaTeX (lazy). */
export async function renderMath(host: HTMLElement): Promise<void> {
  const code = host.querySelector("code");
  const tex = (code?.textContent ?? host.textContent ?? "").trim();
  if (!tex) {
    return;
  }
  const { default: katex } = await import("katex");
  const rendered = document.createElement("div");
  rendered.className = "alt-math-rendered";
  // throwOnError keeps a malformed expression from breaking the page: KaTeX
  // renders the error inline instead, and the fallback source stays available.
  katex.render(tex, rendered, { throwOnError: false, displayMode: true });
  host.prepend(rendered);
  code?.classList.add("alt-visually-hidden");
}

let diagramSeq = 0;
let mermaidPromise: Promise<(typeof import("mermaid"))["default"]> | null = null;

/** Load and initialise Mermaid once, in strict mode (defence in depth). */
function loadMermaid(): Promise<(typeof import("mermaid"))["default"]> {
  mermaidPromise ??= import("mermaid").then(({ default: mermaid }) => {
    mermaid.initialize({ startOnLoad: false, securityLevel: "strict" });
    return mermaid;
  });
  return mermaidPromise;
}

/** Derive a sensible iframe height from the rendered SVG's viewBox. */
function svgHeight(svg: string): number {
  const match = svg.match(/viewBox="[\d.-]+ [\d.-]+ [\d.-]+ ([\d.]+)"/);
  if (match) {
    return Math.min(Math.max(Math.round(Number(match[1])) + 24, 80), 800);
  }
  return 320;
}

/**
 * Render a Mermaid diagram from its fallback source. Mermaid runs in the host
 * (where its code-splitting resolves), but its SVG output, which is the
 * Mermaid-class XSS vector, is displayed inside a script-disabled,
 * origin-isolated iframe: the diagram shows, yet nothing in it can execute or
 * reach the host page.
 */
export async function renderDiagram(host: HTMLElement): Promise<void> {
  const source = (host.querySelector("pre")?.textContent ?? host.textContent ?? "").trim();
  if (!source) {
    return;
  }
  const mermaid = await loadMermaid();
  diagramSeq += 1;
  let svg: string;
  try {
    ({ svg } = await mermaid.render(`altmd-diagram-${String(diagramSeq)}`, source));
  } catch {
    return; // malformed diagram: keep the readable fallback source in place
  }
  const frame = createSandboxedFrame({
    html: svg,
    allowScripts: false,
    height: svgHeight(svg),
    title: "diagram",
  });
  host.replaceChildren(frame);
}
