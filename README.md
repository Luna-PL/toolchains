# LunaToolchain

[English](README.md) | [简体中文](README.zh-CN.md)

LunaToolchain is the independent editor and developer-tooling workspace for
Luna `0.2.1`. It lives beside the compiler during local development but
has its own Git history, releases, compatibility policy, and version number.

Current status: M4 semantic tooling in progress. `luna-protocol` validates
diagnostic and analysis JSONL, `luna-compiler` capability-probes compatible
compilers, and `luna-lsp` provides saved-file diagnostics, compiler-backed
document symbols, direct-function, trait-method, type, trait, struct-field, and
enum-variant definition, package references, capability-gated package rename,
atomic multi-document dirty-buffer analysis, and folding
over stdio. The generated
Tree-sitter grammar and `luna-fmt` provide lossless, comment-preserving
full-document formatting.

## Repository boundaries

- The Luna compiler owns language semantics, stable diagnostics, source spans,
  package resolution, and machine-readable analysis protocols.
- LunaToolchain owns editor integration, protocol clients, formatting,
  workspace presentation, and developer commands.
- Tooling must not infer semantic truth from rendered compiler messages or
  link against private compiler C++ classes.

See [architecture](docs/architecture.md), [protocol](docs/protocol.md),
[language-server support](docs/language_server.md),
[formatter support](docs/formatter.md), [distribution](docs/distribution.md),
and the [delivery roadmap](docs/roadmap.md).

## Local validation

```sh
cargo test --workspace --offline
npm --prefix grammars/tree-sitter-luna ci
npm --prefix grammars/tree-sitter-luna test
npm --prefix editors/vscode run check
cargo run --offline -p luna-tools -- compiler --luna /path/to/luna
```

Compiler discovery checks an explicit `--luna`/`LUNA_BIN`, then `PATH`, then
`../build/luna` when that local development candidate exists. Selection always
runs `--version` and a diagnostic-protocol probe, then probes optional analysis
capabilities independently.

## Continuous integration

`.github/workflows/ci.yml` runs the same Rust, Tree-sitter, and VS Code client
checks on Ubuntu 24.04, macOS 15, and Windows Server 2022. Grammar conformance
tracks `Luna-PL/Luna:main` during normal push and pull-request runs so language
drift is detected early. A manual run can override this with any Luna branch,
tag, or commit. A dedicated Linux job builds that exact ref and makes the
compiler-backed protocol, formatter-corpus, definition, reference, rename, and
multi-document overlay tests mandatory. Published compatibility remains pinned
separately to a Luna release tag.

## Releases

Version tags produce Linux x86_64, macOS arm64, and Windows x86_64 command-line
archives, platform VSIX packages with bundled `luna-lsp`/`luna-fmt`, and SHA-256
checksums. See [distribution](docs/distribution.md) for contents, installation,
and the separate Luna compiler requirement.
