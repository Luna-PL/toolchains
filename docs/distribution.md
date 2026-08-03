# LunaToolchain distribution

[English](distribution.md) | [简体中文](distribution.zh-CN.md)

> Status: three-platform archive and VSIX packaging implemented

Pushing an existing version tag such as `v0.1.1` runs the release workflow.
A manual run accepts an existing tag as input. The tag must match the Cargo
workspace, VS Code extension, Tree-sitter grammar, and compatibility-manifest
versions.

Each release contains:

- `luna-toolchain-<version>-linux-x86_64.tar.gz`
- `luna-toolchain-<version>-macos-arm64.tar.gz`
- `luna-toolchain-<version>-windows-x86_64.zip`
- platform-specific `luna-language-*.vsix` packages
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

Release compatibility is declarative: `compatibility/luna.json` pins the Luna
release tag, language version, and diagnostic protocol supported by published
packages. Development CI follows Luna `main` independently to detect drift.
