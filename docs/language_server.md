# Luna language-server support

[English](language_server.md) | [简体中文](language_server.zh-CN.md)

> Status: implemented experimental in LunaToolchain 0.1
> Semantic authority: Luna compiler `luna.diagnostic` version 1

## Supported lifecycle and synchronization

`luna-lsp` uses LSP over stdio with `Content-Length` framing. It implements
`initialize`, `initialized`, `shutdown`, and `exit`, rejects requests before
initialization or after shutdown, and leaves no persistent compiler child.

Open documents use incremental UTF-16 synchronization. Changes update only the
server's memory snapshot and cancel pending checks. A check is scheduled after
open only when that snapshot exactly matches disk, and after `didSave`; saves
within 150 ms are coalesced.

## Diagnostics

The server discovers a compatible compiler during initialization and invokes
`luna check --message-format=json` on the nearest package, workspace, or
standalone source boundary. It never copies an unsaved buffer into a temporary
package. Compiler UTF-8 byte spans are converted against the saved file to LSP
UTF-16 ranges. Diagnostics are grouped by source URI, and stale diagnostics are
cleared after a successful recheck or close.

## Structural editor features

The current server provides `textDocument/documentSymbol` and
`textDocument/foldingRange`. These are lexical structural aids, not semantic
analysis: symbols recognize declaration headers, while folding tracks balanced
braces outside comments and strings. Hover, completion, definition, references,
and rename remain disabled until `luna.analysis` is implemented by the compiler.

## VS Code configuration

- `luna.server.path`: `luna-lsp` executable, defaulting to `PATH` lookup.
- `luna.compiler.path`: optional exact compiler path.
- `Luna: Restart Language Server`: stop and recreate the client after changes.

The extension requires VS Code 1.91 or later and uses
`vscode-languageclient` 10.1.0. M2 still requires editor-host and
Linux/macOS/Windows CI evidence before release status.
