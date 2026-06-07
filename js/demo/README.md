# alt-markdown browser demo

Zero build: the page loads the WASM core and the runtime as ES modules, no
bundler involved.

## Build the inputs

From `js/`:

1. `npm install`
2. `npm run build` (builds the runtime to `packages/runtime/dist`)
3. `npm run wasm` (generates `js/wasm/web`; needs `wasm-bindgen-cli` installed,
   matching the `wasm-bindgen` version in `Cargo.lock`)

## Serve

From the repo root, serve the `js/` directory with any static file server, then
open `/demo/`. For example:

```
python3 -m http.server --directory js
```
