# LunaToolchain architecture

[English](architecture.md) | [简体中文](architecture.zh-CN.md)

> Status: M3 syntax and full-document formatter implemented
> Applies to: LunaToolchain 0.1 development

## Dependency direction

```text
             VS Code / other editors
                 /             \
                v               v
           luna-lsp          luna-fmt
               |                |
               v                v
         luna-compiler    tree-sitter-luna
               |
               v
         luna-protocol
               |
               v
       Luna compiler process
```

`luna-workspace` discovers manifests and compiler roots but does not implement
package semantics. The compiler remains authoritative for package identity,
exports, types, ownership, selectors, and diagnostics.

## Process boundary

`luna-compiler` discovers candidates in the order explicit configuration,
`PATH`, then local development fallback. It accepts a candidate only after
probing `--version` and `luna.diagnostic` JSONL identity, then independently
probing optional `luna.analysis` identity and capabilities. The first
language server consumes this API and must not parse human-rendered stderr as a
stable API. Saved-file diagnostics remain separate. Compilers advertising
`single-document-overlay` accept the current document through stdin while
loading the rest of its package normally; no temporary package layout is
created. A future multi-document or daemon transport is required before one
snapshot can combine several dirty files.

## Syntax boundary

The compiler lexer drops comments, so it cannot support a preserving formatter.
The generated `tree-sitter-luna` grammar therefore owns lossless structural
syntax, comments, and formatting input. Its expression and type subtrees remain
deliberately coarse and are not semantic authority. Compiler conformance
fixtures prevent the grammar from silently accepting a different language.

The editor invokes `luna-fmt` directly with the current document over stdin;
the language server does not depend on the formatter. Folding still uses the
language server's conservative scanner until Tree-sitter integration has its
own measured benefit and compatibility tests.

## Compatibility

The language version, compiler build identity, diagnostic protocol, analysis
protocol, LSP release, and editor-extension release are separate identities.
Luna remaining at `0.2.0-alpha` must not hide protocol incompatibility; clients
gate features on explicit protocol versions and compiler commit metadata.

The three-platform development CI follows the Luna default branch for source
fixture conformance, exposing language drift immediately. Manual runs can test
an explicit branch, tag, or commit. Release artifacts instead pin a compatible
Luna release tag so published packages remain reproducible.
