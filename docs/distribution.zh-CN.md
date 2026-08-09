# LunaToolchain 分发

[English](distribution.md) | [简体中文](distribution.zh-CN.md)

> 状态：三平台 archive 与 VSIX 打包已实现

push 已存在的版本 tag（例如 `v0.1.2`）会运行 release workflow；手动运行时也必须
输入已存在 tag。tag 必须与 Cargo workspace、VS Code extension、Tree-sitter grammar
和兼容性 manifest 中的版本一致。

打包开始前，release workflow 会对 `compatibility/luna.json` 声明的精确 Luna release
tag 复用完整 Toolchain CI。三个目标平台 job 与强制真实编译器集成 job 必须全部通过；
发布不能与该兼容门禁并行执行，也不能绕过它。

每个 Release 包含：

- `luna-toolchain-<version>-linux-x86_64.tar.gz`
- `luna-toolchain-<version>-macos-arm64.tar.gz`
- `luna-toolchain-<version>-windows-x86_64.zip`
- 各平台的 `luna-language-*.vsix`
- `SHA256SUMS`

archive 包含 `luna-lsp`、`luna-fmt`、`luna-tools`、中英文文档、许可证和
`compatibility/luna.json`。把其中的 `bin` 目录加入 `PATH` 即可使用命令行工具。

例如在 Linux 或 macOS 上：

```sh
tar -xzf luna-toolchain-<version>-<platform>.tar.gz
export PATH="$PWD/luna-toolchain-<version>-<platform>/bin:$PATH"
luna-tools
```

Windows 上解压 ZIP 后，将其 `bin` 目录加入用户或当前进程的 `PATH`。
平台 VSIX 无需重新构建，可直接安装：

```sh
code --install-extension luna-language-<tag>-<vscode-platform>.vsix
```

平台 VSIX 内置 `luna-lsp` 与 `luna-fmt`。server/formatter path 留空时优先使用内置
binary，再 fallback 到 `PATH`。编译器诊断仍要求单独安装兼容的 Luna compiler，
或通过 `luna.compiler.path`/`LUNA_BIN` 配置。

Release 兼容性由 `compatibility/luna.json` 声明，固定发布 package 支持的 Luna
release tag、语言版本和诊断协议。开发 CI 则独立跟踪 Luna `main` 以发现漂移。

发布后，`published-release.yml` 会在干净的 Linux、macOS 与 Windows runner 上下载不可变
GitHub Release。每个 job 都会验证完整 `SHA256SUMS` manifest、本平台 archive 和 VSIX，
验证其 GitHub/Sigstore attestation 来自仓库 release workflow，解包后启动
`luna-tools`、`luna-fmt` 与 `luna-lsp`，并检查随包分发的兼容性 manifest。
