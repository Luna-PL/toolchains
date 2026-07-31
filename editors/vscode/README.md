# Luna VS Code extension

[English](README.md) | [简体中文](README.zh-CN.md)

The Alpha extension registers `.luna`, provides lexical highlighting, and
starts `luna-lsp` for saved-file diagnostics, document symbols, and folding.
It intentionally does not claim hover, completion, definition, references,
rename, formatting, or unsaved-buffer semantic diagnostics.

Set `luna.server.path` when `luna-lsp` is not on `PATH`. Set
`luna.compiler.path` to pin one compiler; otherwise discovery uses `LUNA_BIN`,
`PATH`, then the local development fallback. Use **Luna: Restart Language
Server** after changing either path.
