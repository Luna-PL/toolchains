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
probing both `--version` and `luna.diagnostic` JSONL identity. The first
language server consumes this API and must not parse human-rendered stderr as a
stable API. Saved-file diagnostics are the first supported mode. Unsaved
buffers wait for a compiler overlay or daemon protocol instead of being copied
into a fake package layout.

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

The three-platform CI matrix pins the Luna source-fixture commit used for
grammar conformance. Advancing that pin is an explicit compatibility review,
not an implicit dependency on the compiler's moving default branch.
