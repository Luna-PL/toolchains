# LunaToolchain architecture

[English](architecture.md) | [简体中文](architecture.zh-CN.md)

> Status: M2 minimal language server implemented; formatter remains planned
> Applies to: LunaToolchain 0.1 development

## Dependency direction

```text
VS Code / other editors
          |
          v
       luna-lsp ------> luna-fmt
          |                 |
          v                 v
    luna-compiler     lossless syntax tree
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

The current compiler lexer drops comments, so it cannot support a preserving
formatter. A future lossless Tree-sitter grammar owns incremental syntax,
comments, folding, and formatting input. Compiler conformance fixtures must
prevent that grammar from silently accepting a different language.

## Compatibility

The language version, compiler build identity, diagnostic protocol, analysis
protocol, LSP release, and editor-extension release are separate identities.
Luna remaining at `0.2.0-alpha` must not hide protocol incompatibility; clients
gate features on explicit protocol versions and compiler commit metadata.
