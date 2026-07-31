# tree-sitter-luna

[English](README.md) | [简体中文](README.zh-CN.md)

This package owns LunaToolchain's lossless structural syntax boundary. The M3
grammar recognizes compiler headers and declaration starts exactly, preserves
comments and every token, and requires balanced blocks, parentheses, and
brackets. Expression and type nodes remain deliberately coarse until the
compiler publishes a syntax contract independent from private C++ AST classes.

```sh
npm ci
npm run generate
npm test
```

`npm run test:compiler` additionally parses selected Luna compiler fixtures
when `LUNA_SOURCE_DIR` points at a compiler checkout.
