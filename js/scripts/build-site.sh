#!/usr/bin/env bash
# Assemble a self-contained static site for the playground and gallery, ready to
# deploy to any static host (GitHub Pages by default).
#
# It mirrors the parts of js/ the pages reference (wasm, runtime, the three
# graphics libraries) so the relative paths the playground already uses resolve
# without rewriting. Everything is bundled locally: the site loads no CDN.
#
# Prerequisites: the wasm bindings (bash js/scripts/build-wasm.sh), the runtime
# build (cd js && npm run build), and the graphics libraries (npm ci in js/).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
JS="$ROOT/js"
SITE="$ROOT/site"

# Pre-flight: fail loud if an input the site needs is missing.
need() { [ -e "$1" ] || { echo "missing build input: $1 (run the prerequisites)" >&2; exit 1; }; }
need "$JS/wasm/web/altmd_wasm.js"
need "$JS/packages/runtime/dist/index.js"
need "$JS/node_modules/uplot/dist/uPlot.esm.js"
need "$JS/node_modules/katex/dist/katex.mjs"
need "$JS/node_modules/mermaid/dist/mermaid.esm.min.mjs"

rm -rf "$SITE"
mkdir -p "$SITE/demo" "$SITE/wasm" "$SITE/packages/runtime/dist" "$SITE/node_modules"

# The playground, the gallery, and their assets.
cp "$JS/demo/"*.html "$JS/demo/"*.js "$JS/demo/"*.css "$SITE/demo/"
cp -r "$JS/demo/articles" "$SITE/demo/"

# The wasm core and the runtime, referenced by the pages as ../wasm/web and
# ../packages/runtime/dist.
cp -r "$JS/wasm/web" "$SITE/wasm/"
cp "$JS/packages/runtime/dist/index.js" "$SITE/packages/runtime/dist/"

# The three graphics libraries, whole dist trees (mermaid loads lazy chunks and
# katex loads font files, so copying the full dist is the safe, simple choice).
for lib in uplot katex mermaid; do
  mkdir -p "$SITE/node_modules/$lib"
  cp -r "$JS/node_modules/$lib/dist" "$SITE/node_modules/$lib/"
done

# Root entry: send visitors straight to the playground.
cat > "$SITE/index.html" <<'HTML'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta http-equiv="refresh" content="0; url=./demo/playground.html" />
    <title>alt-markdown</title>
  </head>
  <body>
    <a href="./demo/playground.html">Open the alt-markdown playground</a>
  </body>
</html>
HTML

# Serve files verbatim (no Jekyll processing).
touch "$SITE/.nojekyll"

echo "Built static site at $SITE"
