# Test layout

[English](README.md) | [简体中文](README.zh-CN.md)

- `protocol/`: compiler JSONL golden records consumed by `luna-protocol` tests.
- `fixtures/`: minimal syntax/workspace inputs licensed with this repository.
- `integration/`: tests requiring an explicit `LUNA_BIN` compiler path.

Unit tests must run without a compiler checkout or network access.
When `LUNA_BIN` is set, `luna-compiler` also probes that real binary during the
workspace test run.
`luna-lsp` has an stdio integration test covering lifecycle, symbols, folding,
and—when `LUNA_BIN` is set—real compiler diagnostics.
`luna-fmt` tests comment preservation, idempotence, `--check`, `--write`, stdin,
malformed-input immutability, and an optional compiler source corpus. When both
`LUNA_SOURCE_DIR` and `LUNA_BIN` are set, the real compiler also checks every
formatted corpus file.
