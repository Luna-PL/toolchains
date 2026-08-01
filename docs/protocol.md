# Luna compiler/tooling protocol

[English](protocol.md) | [简体中文](protocol.zh-CN.md)

> Status: diagnostic producer implemented; analysis v1 consumer contract implemented
> Protocols: `luna.diagnostic` version 1, `luna.analysis` version 1

## Transport

The initial transport is newline-delimited JSON written by `luna check` when
`--message-format=json` is selected. One invocation emits:

1. exactly one `hello` record;
2. zero or more `diagnostic` records;
3. exactly one `summary` record.

Human output and JSONL must never be mixed on the protocol stream. Exit status
`0` means no errors, `1` means source/compiler diagnostics were reported, and
`2` means invocation or protocol misuse. Crashes remain outside the protocol.

## Required identity

The `hello` record carries:

- language version (`0.2.0-alpha`);
- compiler source commit and build target;
- diagnostic protocol name/version;
- supported optional capabilities.

Clients reject an unsupported major protocol version before interpreting
diagnostics. The compiler commit remains necessary because the language
version intentionally changes infrequently.

## Diagnostic record

```json
{
  "protocol": "luna.diagnostic",
  "version": 1,
  "kind": "diagnostic",
  "severity": "error",
  "phase": "semantic",
  "code": "SEM0001",
  "message": "undefined name 'value'",
  "primary": {
    "path": "/workspace/main.luna",
    "start": { "byte": 42, "line": 3, "column": 9 },
    "end": { "byte": 47, "line": 3, "column": 14 }
  },
  "labels": [],
  "notes": [],
  "fixes": []
}
```

When `primary` is present, byte offsets are required, count UTF-8 bytes, and use
an exclusive end. Diagnostics without a disk-backed location use
`"primary": null`. Human line/column fields are one-based display aids. LSP
clients convert byte offsets against their document snapshot to UTF-16
positions. Paths are normalized absolute paths for disk-backed input; future
overlays add explicit document URIs rather than overloading paths.

Fixes are structured edits with applicability (`machine-applicable`,
`maybe-incorrect`, or `manual`). A rendered help string is a note, not a fix.
`luna-protocol` owns the Serde wire types and rejects an incorrect protocol
identity, unsupported version, invalid ordering, or inconsistent summary.

## Analysis protocol v1

`luna.analysis` v1 is a newline-delimited semantic snapshot. One invocation
emits exactly one `hello`, zero or more `symbol` and `reference` records, and
exactly one `summary`. The `declarations` capability carries compiler-owned
symbol identity and types. `call-references` adds resolved direct-call targets.
`method-references` adds resolved user trait-method targets.
`type-references` adds user struct, enum, and metadata-schema names appearing
in type syntax. `trait-references` adds resolved trait names in impls and
bounds. `package-references` declares that clients may reverse-query Symbol IDs
across every emitted reference record in one complete package snapshot; its
coverage is the union of the separately advertised reference classes.
`field-references` covers fields of named struct declarations; anonymous
record fields have no declaration Symbol ID. `enum-variant-references` covers
variant construction and match patterns, including their explicit enum owner.
`single-document-overlay` accepts one real source path through `--overlay` and
reads its replacement UTF-8 text from stdin while resolving the selected
package normally. `multi-document-overlay` accepts `--overlays-from-stdin` and
a versioned `luna.overlay` JSON object containing a non-empty array of
`{"path", "text"}` replacements. The compiler validates the whole set before
package analysis, so one snapshot never mixes only a subset of the requested
documents.

```json
{
  "protocol": "luna.analysis",
  "version": 1,
  "kind": "symbol",
  "id": "luna.symbol.v1:4:main:0::8:function:9:main::add",
  "name": "add",
  "qualified_name": "main::add",
  "package_id": "main",
  "module_path": "",
  "linkage_name": "main::add",
  "symbol_kind": "function",
  "signature": "fn add(left: i32, right: i32) -> i32",
  "selection": {
    "path": "/workspace/main.luna",
    "start": { "byte": 3, "line": 1, "column": 4 },
    "end": { "byte": 7, "line": 1, "column": 8 }
  },
  "exported": true,
  "external": false
}
```

Symbol IDs are opaque, stable identities; clients compare and store them but
must not parse their current length-delimited representation. `selection` uses
the same UTF-8 byte and one-based display-position rules as diagnostic spans.
The summary reports the numbers of symbol and reference records plus
`complete`. A compiler may emit a useful partial declaration snapshot with
`complete: false` after parser recovery or semantic failure.

A `reference` contains a source span and an opaque `target_id` that must name a
symbol in the same snapshot. Clients may implement definition only for the
reference classes explicitly named by capabilities; v1 currently guarantees
direct function calls through `call-references` and resolved user trait-method
calls through `method-references`. `type-references` covers resolved user type
names in type syntax; `trait-references` covers impl and bound trait names.
`field-references` and `enum-variant-references` use child Symbol IDs whose
identity is derived from the compiler-resolved parent declaration.

The Rust wire types and sequence validator are implemented in
`luna-protocol`. The companion compiler producer and capability-gated
`luna-compiler` client are implemented. Definition is enabled for complete
saved-file snapshots with
any advertised reference capability; a complete version-matched overlay
snapshot has the same semantics. Package-scoped `textDocument/references` is
enabled only with `package-references`. Hover, completion, rename, and broader
workspace indexing require future explicit capabilities and must not be
inferred from MoonIR names or rendered diagnostics.
