# Luna formatter support

[English](formatter.md) | [简体中文](formatter.zh-CN.md)

> Status: implemented experimental in LunaToolchain 0.1
> Grammar: generated with Tree-sitter 0.26.11

`luna-fmt` performs deterministic full-document formatting from the generated
`tree-sitter-luna` parser. The grammar preserves comments and all lexical
tokens, recognizes compiler headers and declaration starts, and rejects
unbalanced or malformed structure before any file is written.

```sh
luna-fmt source.luna
luna-fmt --write source.luna package/src/main.luna
luna-fmt --check source.luna
printf 'fn main()->i32{return 0;}' | luna-fmt -
```

Print mode writes one formatted document to stdout. `--write` updates files
only after a complete parse and format succeeds. `--check` never writes and
returns `1` when any file would change. Invocation, I/O, or syntax errors return
`2`; malformed input remains byte-identical.

The current style uses four-space block indentation, one statement per line,
stable operator spacing, a final newline, and preserved line-comment text.
Formatting is idempotent. Range formatting is not supported yet.

The structural grammar intentionally keeps expression and type subtrees coarse.
It is not an independent semantic parser and must not be used for hover,
completion, ownership, or type decisions. Compiler fixture conformance prevents
the grammar from silently drifting away from accepted Luna source.
