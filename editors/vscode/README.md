# Luna VS Code extension

[English](README.md) | [简体中文](README.zh-CN.md)

The Alpha extension registers `.luna`, provides lexical highlighting, and
starts `luna-lsp` for saved-file diagnostics, document symbols, and folding.
It also runs `luna-fmt` as a full-document formatting provider. It intentionally
does not claim hover, completion, definition, references, rename, range
formatting, or unsaved-buffer semantic diagnostics.

Set `luna.server.path` when `luna-lsp` is not on `PATH`. Set
`luna.compiler.path` to pin one compiler; otherwise discovery uses `LUNA_BIN`,
`PATH`, then the local development fallback. Use **Luna: Restart Language
Server** after changing either path.

Set `luna.formatter.path` when `luna-fmt` is not on `PATH`. Formatting sends the
current editor snapshot over stdin and does not run compiler diagnostics on it.

Platform release VSIX packages bundle `luna-lsp` and `luna-fmt`. Empty server
and formatter path settings prefer those binaries and otherwise fall back to
`PATH`. The Luna compiler remains a separate installation.
