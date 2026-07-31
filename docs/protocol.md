# Luna compiler/tooling protocol

[English](protocol.md) | [简体中文](protocol.zh-CN.md)

> Status: implemented experimental in the companion Luna compiler worktree
> Protocol: `luna.diagnostic` version 1

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

## Deferred analysis protocol

`luna.analysis` remains version `0` and unsupported. Hover, definition,
references, completion, and rename must wait for compiler-owned symbol/type
records. They must not be inferred from MoonIR names or rendered diagnostics.
