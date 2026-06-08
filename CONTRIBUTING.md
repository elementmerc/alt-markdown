# Contributing to alt-markdown

Thanks for your interest. This guide covers how to build the project, how to run
the checks, and the standards a change is held to before it lands.

## What you need

- A recent stable Rust toolchain. The pinned version and the `wasm32-unknown-unknown`
  target are declared in `rust-toolchain.toml`, so `rustup` installs them for you.
- Node 20 or newer, for the browser runtime and the demo.
- `wasm-bindgen-cli`, only if you are rebuilding the WebAssembly bindings.

## Repository layout

The project is two workspaces side by side.

- `crates/` is a Rust workspace. `altmd-ast` holds the syntax tree and the
  parser and serializer traits; `altmd-parser` is the comrak-backed parser, the
  component grammar, and the serializer; `altmd-sanitize` is the raw-HTML
  allowlist sanitiser; `altmd-core` is the public facade; `altmd-wasm` is the
  WebAssembly bindings; `altmd-cli` is the `altmd` binary.
- `js/` is an npm workspace. `packages/runtime` is the browser runtime that
  upgrades components; `demo` is the live gallery.

## Building and testing

Rust:

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

The browser runtime:

```
cd js
npm install
npm run typecheck
npm run lint
npm test            # unit tests (vitest)
npm run build       # the runtime bundle
npm run wasm        # rebuild the WebAssembly bindings
npx playwright test # end-to-end tests in a real browser
```

A change is ready when both workspaces are green: formatting, lints with no
warnings, and all tests passing, including the CommonMark conformance suite and
the round-trip property tests.

## Standards

- **Safety is not optional.** The renderer is safe by construction. Any change
  that touches rendering keeps the guarantee that untrusted input cannot produce
  a live script, an event handler, or a dangerous URL, and ships a test that
  proves it. Anything that runs third-party or generated code (diagrams, embeds,
  the escape hatch) renders inside the sandboxed iframe, never in the host page.
- **The CommonMark superset is sacred.** Plain markdown must keep parsing
  exactly as it did. The conformance suite is the backstop; do not regress it.
- **Every component has a static fallback.** A new component renders readable,
  semantic HTML with no runtime, and only then enhances. A document must always
  degrade gracefully.
- **Resource caps on the parser.** The parser bounds nesting depth and never
  panics on hostile input. New recursion carries the same bound.
- **Test the change.** New code is covered, error paths included. Adversarial
  input gets an adversarial test.

## Code style

- Rust follows `rustfmt` and `clippy` with warnings denied. Prefer error
  propagation over panics outside tests.
- TypeScript is strict, linted with ESLint, formatted consistently.
- Prose, comments, and messages use British English and plain language: lead
  with what a thing does, then name it.
- Public documentation states what the current code does. It does not promise
  unreleased features.

## Commits

- One atomic change per commit. The message explains why, not what.
- No tool-attribution or co-author trailers.
- Do not commit secrets, private paths, or infrastructure identifiers.

## Dependencies

The dependency graph is attack surface. Before adding a dependency, weigh who
maintains it, how large it is, and what it pulls in. Lockfiles are committed.
New versions wait out a short cooldown before adoption, and installs disable
package scripts where the workflow allows it.

## Adding a component

A component is a contract. It declares its syntax (a colon directive for
structure, a fenced block for a data payload), it renders a mandatory static
fallback in the core renderer, and it upgrades in the runtime without ever
trusting its own input. Open an issue describing the component and its fallback
before building it, so the vocabulary stays small and coherent.

## Reporting a security issue

Please do not open a public issue for a vulnerability. Report it privately
through the repository's security advisory page so it can be fixed before it is
disclosed.
