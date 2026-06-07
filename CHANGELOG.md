# Changelog

All notable changes to alt-markdown are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0/).

## [Unreleased]

### Added

- Workspace scaffold for the Rust core: the `altmd-ast`, `altmd-parser`,
  `altmd-sanitize`, `altmd-core`, `altmd-wasm`, and `altmd-cli` crates.
- The JavaScript runtime workspace (`@altmd/runtime`, `@altmd/components`).
- A working `altmd render` command that converts CommonMark to HTML.
