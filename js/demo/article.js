// Article viewer: fetch the .alt document named by the ?doc= query parameter,
// render it through the WASM core, and mount it so the runtime upgrades the
// components. This is the whole stack working as a reader would see it.
import init, { render } from "../wasm/web/altmd_wasm.js";
import { mount } from "../packages/runtime/dist/index.js";

const root = document.getElementById("doc");

// Allowlist the document names so the ?doc parameter cannot be used to fetch
// arbitrary paths.
const ARTICLES = {
  cybersecurity: "The Anatomy of a Cross-Site Scripting Attack",
  "cs-lewis": "C. S. Lewis on the Reading of Old Books",
  "paradox-of-genius": "The Paradox of Genius",
};

function fail(message) {
  if (root) {
    root.innerHTML = `<p class="doc-error">${message}</p><p><a href="./index.html">Return to the gallery</a></p>`;
  }
}

async function main() {
  const name = new URLSearchParams(location.search).get("doc");
  if (!name || !(name in ARTICLES)) {
    fail("Unknown article. Pick one from the gallery.");
    return;
  }
  document.title = `${ARTICLES[name]} — alt-markdown`;

  const response = await fetch(`./articles/${name}.alt`);
  if (!response.ok) {
    fail(`Could not load the article (HTTP ${String(response.status)}).`);
    return;
  }
  const source = await response.text();

  await init();
  let html;
  try {
    html = render(source);
  } catch (error) {
    fail(`The document could not be parsed: ${String(error)}`);
    return;
  }
  if (root) {
    mount(root, html);
  }
}

main().catch((error) => {
  fail(`Unexpected error: ${String(error)}`);
});
