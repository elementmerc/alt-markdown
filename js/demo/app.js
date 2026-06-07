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

\`\`\`chart kind=bar
month,sales
jan,10
feb,18
mar,25
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
`;

await init();
const root = document.getElementById("doc");
if (root) {
  mount(root, render(SAMPLE));
}
