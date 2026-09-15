# LunaToolchain delivery roadmap

[English](roadmap.md) | [简体中文](roadmap.zh-CN.md)

## M0: repository and contracts

- Independent Git repository and Rust workspace.
- English-default documentation with Chinese companions.
- Diagnostic protocol v1 proposal and compatibility policy.
- Declarative VS Code language registration.

Exit: repository tests run without downloading the Luna compiler.

## M1: compiler diagnostic bridge

Status: implemented locally; cross-platform CI evidence remains before release.

- Luna compiler implements structured diagnostics and JSONL output.
- `luna-protocol` owns validated protocol types and golden fixtures.
- Compiler discovery uses explicit configuration, `PATH`, then a local
  development fallback; every selected binary is version-probed.

Exit: saved-file diagnostics are stable on Linux, macOS, and Windows.

## M2: minimal language server

Status: implemented and covered by three-platform process-level CI; editor-host
evidence remains before release.

- LSP stdio lifecycle and incremental document synchronization.
- Debounced saved-file checks, diagnostic publication, document symbols, and
  folding ranges.
- Package/workspace root discovery without reimplementing package semantics.

Exit: VS Code can start/stop the server repeatedly without leaked processes.

## M3: lossless syntax and formatter

Status: implemented and passing three-platform CI; broader corpus and
editor-host evidence remain before release.

- Tree-sitter Luna grammar with compiler conformance fixtures.
- Comment-preserving full-document formatter and `--check` mode.
- Idempotence and malformed-input tests; range formatting follows later.

Exit: formatting twice is byte-identical to formatting once.

## M4: semantic editor features

Status: in progress; declaration protocol v1 types, validation, compiler
producer, client, saved-file document symbols, and direct-function,
trait-method, type-syntax, trait, struct-field, and enum-variant definition are
implemented. Single- and multi-document dirty-buffer overlays are implemented
with version-vector cache validation.
Package-scoped `textDocument/references` and a deliberately limited 0.2.x
package rename are implemented; local rename and persistent multi-package
indexing remain pending for the post-0.3 syntax baseline.

- Compiler-owned analysis protocol and document overlays.
- Hover and completion on the post-0.3 syntax baseline; revisit rename there.
- Feature gates follow protocol capabilities, not guessed compiler versions.

Exit: cross-file package examples pass semantic LSP integration tests.

## M5: build, package, and distribution

Status: three-platform binary archives, bundled platform VSIX packages,
checksums, and a compatibility manifest are implemented. Developer task and
cache commands remain planned.

- Check/build/run/test tasks, test selection, workspace status, and local cache
  reporting.
- Prebuilt `luna-lsp` artifacts, VSIX packaging, checksums, and a compatibility
  matrix pinned to exact validated compiler commits and protocol versions.

Remote registries, debugger support, and language-surface expansion are not in
the 0.2 plan.
