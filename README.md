# LunaToolchain

[English](README.md) | [简体中文](README.zh-CN.md)

LunaToolchain is the independent editor and developer-tooling workspace for
Luna `0.2.0-alpha`. It lives beside the compiler during local development but
has its own Git history, releases, compatibility policy, and version number.

Current status: M1 diagnostic bridge. `luna-protocol` parses and validates the
compiler's JSONL record sequence against golden fixtures; `luna-compiler`
discovers and probes compatible compiler executables. The LSP and formatter
binaries remain placeholders. The declarative VS Code package provides only
basic language registration and lexical syntax highlighting.

## Repository boundaries

- The Luna compiler owns language semantics, stable diagnostics, source spans,
  package resolution, and machine-readable analysis protocols.
- LunaToolchain owns editor integration, protocol clients, formatting,
  workspace presentation, and developer commands.
- Tooling must not infer semantic truth from rendered compiler messages or
  link against private compiler C++ classes.

See [architecture](docs/architecture.md), [protocol](docs/protocol.md), and the
[delivery roadmap](docs/roadmap.md).

## Local validation

```sh
cargo test --workspace --offline
node -e 'JSON.parse(require("fs").readFileSync("editors/vscode/package.json"))'
cargo run --offline -p luna-tools -- compiler --luna /path/to/luna
```

Compiler discovery checks an explicit `--luna`/`LUNA_BIN`, then `PATH`, then
`../build/luna` when that local development candidate exists. Selection always
runs both `--version` and a diagnostic-protocol probe.
