# LunaToolchain distribution

[English](distribution.md) | [简体中文](distribution.zh-CN.md)

> Status: three-platform archive and VSIX packaging implemented

Pushing an existing version tag such as `v0.2.0` runs the release workflow.
A manual run accepts an existing tag as input. The tag must match the Cargo
workspace, VS Code extension, Tree-sitter grammar, and compatibility-manifest
versions.

Before packaging starts, the release workflow reuses the complete Toolchain CI against the
exact Luna source commit declared by `compatibility/luna.json`. All three platform jobs and the
mandatory real-compiler integration job must pass; publication cannot run in parallel with or
bypass that compatibility gate.

Each release contains:

- `luna-toolchain-<version>-linux-x86_64.tar.gz`
- `luna-toolchain-<version>-macos-arm64.tar.gz`
- `luna-toolchain-<version>-windows-x86_64.zip`
- platform-specific `luna-language-*.vsix` packages
- `LUNA-SOURCE-COMMIT`
- `SHA256SUMS`

The archive contains `luna-lsp`, `luna-fmt`, `luna-tools`, English and Chinese
documentation, licenses, and `compatibility/luna.json`. Add its `bin` directory
to `PATH` to use the command-line tools.

For example, on Linux or macOS:

```sh
tar -xzf luna-toolchain-<version>-<platform>.tar.gz
export PATH="$PWD/luna-toolchain-<version>-<platform>/bin:$PATH"
luna-tools
```

On Windows, expand the ZIP and add its `bin` directory to the user or process
`PATH`. A platform VSIX can be installed without rebuilding it:

```sh
code --install-extension luna-language-<tag>-<vscode-platform>.vsix
```

The platform VSIX bundles `luna-lsp` and `luna-fmt`. Empty server and formatter
path settings use those bundled binaries first, then fall back to `PATH`.
Compiler-backed diagnostics still require a compatible Luna compiler installed
separately or configured with `luna.compiler.path`/`LUNA_BIN`.

Release compatibility is declarative: `compatibility/luna.json` pins the exact
Luna source commit and records its intended release tag, language version, and
protocols. Development CI follows Luna `main` independently to detect drift.

After publication, `published-release.yml` downloads the immutable GitHub Release on clean
Linux, macOS, and Windows runners. Each job verifies the complete `SHA256SUMS` manifest,
the Luna source-commit asset, and its platform archive and VSIX; verifies their GitHub/Sigstore attestations came from the
repository release workflow, extracts the archive, starts `luna-tools`, `luna-fmt`, and
`luna-lsp`, and checks the packaged compatibility manifest.
