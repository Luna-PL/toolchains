# Luna language-server support

[English](language_server.md) | [简体中文](language_server.zh-CN.md)

> Status: implemented experimental in LunaToolchain 0.1
> Semantic authority: Luna compiler `luna.diagnostic` and `luna.analysis` version 1

## Supported lifecycle and synchronization

`luna-lsp` uses LSP over stdio with `Content-Length` framing. It implements
`initialize`, `initialized`, `shutdown`, and `exit`, rejects requests before
initialization or after shutdown, and leaves no persistent compiler child.

Open documents use incremental UTF-16 synchronization. Changes update the
server's versioned memory snapshot, invalidate older semantic results, and
schedule analysis. Events within 150 ms are coalesced. A compiler advertising
`multi-document-overlay` receives every open dirty document in the same package
as one versioned stdin request. With an older compiler, exactly one dirty file
uses `single-document-overlay`; otherwise dirty documents retain the lexical
fallback.

## Diagnostics

The server discovers a compatible compiler during initialization and invokes
`luna check --message-format=json` on the nearest package, workspace, or
standalone source boundary. It never copies an unsaved buffer into a temporary
package. Compiler UTF-8 byte spans are converted against the saved file to LSP
UTF-16 ranges. Diagnostics are grouped by source URI, and stale diagnostics are
cleared after a successful recheck or close.

## Structural editor features

The current server provides `textDocument/documentSymbol` and
`textDocument/foldingRange`. After a successful saved-file analysis,
document symbols use compiler-owned declaration IDs, kinds, signatures, and
selection spans. A complete, version-matched overlay provides the same symbols
for a dirty document. Until that result arrives, or with an older compiler, the
server retains the lexical declaration fallback. Folding remains a structural
aid that tracks balanced braces outside comments and strings.

`textDocument/definition` is advertised only when the compiler reports the
`call-references`, `method-references`, `type-references`,
`trait-references`, `field-references`, or `enum-variant-references`
capability. It resolves direct functions, user trait methods, type-syntax
names, impl/bound trait names, struct fields, and enum construction/match
variants by opaque Symbol ID in a complete saved-file or version-matched
overlay snapshot.

`textDocument/prepareRename` and `textDocument/rename` provide a deliberately
small Luna 0.2.x package rename. They are enabled only with
`package-references` and the symbol's explicit reference capability, and edit
the declaration plus all matching Symbol ID references in the current complete
snapshot. Function, method, struct, enum, trait, field, and enum-variant symbols
are supported. Locals, parameters, metadata, constraints, kernels, fragments,
anonymous-record fields, file renames, collision prediction, and persistent
cross-package indexing are outside this compatibility implementation. Hover
and completion remain disabled.

`textDocument/references` is advertised only with `package-references`. A
request may start on a declaration or any emitted reference and returns all
matching locations in the current complete package snapshot. It honors
`context.includeDeclaration`. Results cover only the reference classes listed
by the compiler; persistent multi-package workspace indexing remains pending.

Multi-document analysis replaces all open dirty root-package documents
atomically and caches the snapshot against every participating LSP version.
Other package sources and dependencies remain disk-backed. Dirty-buffer
diagnostics remain pending a structured diagnostic-overlay capability.

## VS Code configuration

- `luna.server.path`: `luna-lsp` executable, defaulting to `PATH` lookup.
- `luna.compiler.path`: optional exact compiler path.
- `Luna: Restart Language Server`: stop and recreate the client after changes.

The extension requires VS Code 1.91 or later and uses
`vscode-languageclient` 10.1.0. Process-level Linux/macOS/Windows CI is in
place; editor-host evidence remains before release status.
