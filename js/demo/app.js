// Zero-build demo: load the WASM core and the runtime as ES modules, render a
// sample document, and mount it. Build the inputs first (see README.md):
//   npm run build   (runtime dist)   and   npm run wasm   (js/wasm/web)
import init, { render } from "../wasm/web/altmd_wasm.js";
import { mount } from "../packages/runtime/dist/index.js";

const SAMPLE = `# alt-markdown

A strict superset of CommonMark. This renders rich, and still reads fine with no
JavaScript at all.

:::callout{type=warning}
Heads up: this is a callout. Without the runtime it degrades to a plain aside.
:::

## A chart

The runtime lazy-loads uPlot and draws an interactive chart; with no JavaScript
the same data reads as a plain table.

\`\`\`chart kind=bar
month,sales,returns
jan,10,2
feb,18,3
mar,25,4
apr,22,5
\`\`\`

## Some maths

Inline prose, then a typeset expression rendered by KaTeX:

\`\`\`math
\\int_0^\\infty e^{-x^2}\\,dx = \\frac{\\sqrt{\\pi}}{2}
\`\`\`

## Tabs

::::tabs
:::tab{title=Overview}
First tab content.
:::
:::tab{title=Details}
Second tab content, with **bold** text.
:::
::::

## A diagram

\`\`\`diagram
graph TD
  A[Source] --> B[Render]
  B --> C[Sandboxed iframe]
\`\`\`
`;

await init();
const root = document.getElementById("doc");
if (root) {
  mount(root, render(SAMPLE));
}
