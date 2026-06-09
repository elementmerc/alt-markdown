// The playground: edit alt-markdown on the left, see it render live on the right.
// It runs the real WASM core and the real runtime, the same stack a reader gets.
import init, { render } from "../wasm/web/altmd_wasm.js";
import { mount } from "../packages/runtime/dist/index.js";

const source = document.getElementById("src");
const output = document.getElementById("out");
const status = document.getElementById("status");
const preset = document.getElementById("preset");

// The example articles the preset menu can load, by file name. An allowlist so
// the menu cannot be turned into a fetch of an arbitrary path.
const PRESETS = new Set([
  "cybersecurity",
  "cs-lewis",
  "paradox-of-genius",
  "languages",
]);

const STARTER = `# Welcome to alt-markdown

Type on the left; it renders live on the right. This is ordinary markdown, plus a
small set of **components** that light up with no build step and no server.

:::ai-policy{model=any default=editable}
- Welcome to alt-markdown: read-only
- A figure you can cross-reference: read-only
:::

:::callout{type=tip}
It is all one safe text file. Switch the runtime off and it still reads as markdown.
:::

## A figure you can cross-reference

:::figure{#fig:cost caption="Token cost of the same page, three ways"}
\`\`\`chart kind=bar
format,tokens
Markdown,606
alt-markdown,637
HTML,3247
\`\`\`
:::

As [#fig:cost] shows, alt-markdown costs about as much as plain markdown but renders
what markdown cannot.

## Citations

The effectiveness of plain HTML has been overstated [@thariq2026].

\`\`\`bib
thariq2026: Shihipar, T. The Unreasonable Effectiveness of HTML. 2026.
\`\`\`

:::references
:::

## How AI agents read this

The \`:::ai-policy\` block near the top is an \`[alt.ai]\` policy. It renders to
nothing; it is metadata telling an AI collaborator which sections it may rewrite.
Here the title and the figure are read-only, everything else is open. The document
carries its own collaboration rules.
`;

// UTF-8 safe base64, so a document with any script survives the round trip into
// and out of a shareable URL.
function encodeSource(text) {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

function decodeSource(encoded) {
  const binary = atob(encoded);
  const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

let ready = false;

function renderNow() {
  if (!ready) {
    return;
  }
  try {
    const html = render(source.value);
    mount(output, html);
    status.textContent = "";
    status.classList.remove("pg-status-error");
  } catch (error) {
    // Keep the last good render on screen; just report the problem. While
    // someone is mid-keystroke the document is often briefly invalid.
    status.textContent = String(error instanceof Error ? error.message : error);
    status.classList.add("pg-status-error");
  }
}

let debounce;
function scheduleRender() {
  clearTimeout(debounce);
  debounce = setTimeout(renderNow, 150);
}

async function loadPreset(name) {
  if (!PRESETS.has(name)) {
    return;
  }
  // Bypass the HTTP cache so an edited example always loads fresh.
  const response = await fetch(`./articles/${name}.alt`, { cache: "no-cache" });
  if (!response.ok) {
    throw new Error(`HTTP ${String(response.status)}`);
  }
  source.value = await response.text();
  renderNow();
}

function initialSource() {
  const hash = new URLSearchParams(location.hash.slice(1));
  const shared = hash.get("src");
  if (shared) {
    try {
      return decodeSource(shared);
    } catch {
      // Fall through to the starter on a corrupt link.
    }
  }
  return STARTER;
}

function wireControls() {
  source.addEventListener("input", scheduleRender);

  preset.addEventListener("change", () => {
    const choice = preset.value;
    if (!choice) {
      source.value = STARTER;
      renderNow();
      return;
    }
    loadPreset(choice).catch((error) => {
      status.textContent = `Could not load "${choice}": ${String(error.message || error)}`;
      status.classList.add("pg-status-error");
    });
  });

  document.getElementById("copy-link").addEventListener("click", async () => {
    location.hash = `src=${encodeSource(source.value)}`;
    try {
      await navigator.clipboard.writeText(location.href);
      flash("Link copied");
    } catch {
      flash("Link is in the address bar");
    }
  });

  document.getElementById("theme").addEventListener("click", () => {
    const root = document.documentElement;
    const next = root.getAttribute("data-theme") === "dark" ? "light" : "dark";
    root.setAttribute("data-theme", next);
  });
}

function flash(message) {
  status.textContent = message;
  status.classList.remove("pg-status-error");
  setTimeout(() => {
    if (status.textContent === message) {
      status.textContent = "";
    }
  }, 1500);
}

async function main() {
  source.value = initialSource();
  wireControls();
  await init();
  ready = true;
  renderNow();
}

main().catch((error) => {
  status.textContent = `Could not start the playground: ${String(error)}`;
  status.classList.add("pg-status-error");
});
