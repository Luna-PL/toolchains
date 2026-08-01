# LunaToolchain

[English](README.md) | [简体中文](README.zh-CN.md)

LunaToolchain 是 Luna `0.2.0-alpha` 的独立编辑器与开发者工具工作区。本地开发时
它位于编译器目录内，但具有独立的 Git 历史、发布、兼容策略和版本号。

当前状态为 M4 语义工具进行中。`luna-protocol` 校验诊断和分析 JSONL，
`luna-compiler` 按 capability 探测兼容编译器，`luna-lsp` 通过 stdio 提供已保存文件
诊断、编译器驱动的 document symbol、直接函数、trait method、类型、trait、
struct 字段和 enum variant definition，
package reference、原子多文档脏 buffer 分析，以及 folding。生成的
Tree-sitter grammar 与 `luna-fmt` 提供无损、保留注释的全文格式化。

## 仓库边界

- Luna 编译器负责语言语义、稳定诊断、源码 span、package 解析和机器分析协议。
- LunaToolchain 负责编辑器集成、协议客户端、格式化、workspace 展示和开发命令。
- 工具不得从渲染后的诊断文本推断语义，也不得链接编译器私有 C++ 类。

详细设计见[架构](docs/architecture.zh-CN.md)、[协议](docs/protocol.zh-CN.md)、
[language server 支持范围](docs/language_server.zh-CN.md)、
[formatter 支持范围](docs/formatter.zh-CN.md)、[分发](docs/distribution.zh-CN.md)和
[交付路线图](docs/roadmap.zh-CN.md)。

## 本地验证

```sh
cargo test --workspace --offline
npm --prefix grammars/tree-sitter-luna ci
npm --prefix grammars/tree-sitter-luna test
npm --prefix editors/vscode run check
cargo run --offline -p luna-tools -- compiler --luna /path/to/luna
```

编译器发现依次检查显式 `--luna`/`LUNA_BIN`、`PATH`，最后在本地开发候选
`../build/luna` 存在时检查它。任何候选都必须同时通过 `--version` 和诊断协议探测，
随后独立探测可选的 analysis capability。

## 持续集成

`.github/workflows/ci.yml` 在 Ubuntu 24.04、macOS 15 和 Windows Server 2022 上执行
相同的 Rust、Tree-sitter 与 VS Code client 检查。普通 push 和 pull request 的
grammar conformance 跟踪 `Luna-PL/Luna:main`，以尽早发现语言漂移；手动运行时可以
指定任意 Luna branch、tag 或 commit。Release 兼容性将另行固定到 Luna release
tag。在 CI 接入兼容的预编译 compiler package 前，启动真实 Luna 编译器的测试仍为
可选项。

## Release

版本 tag 会生成 Linux x86_64、macOS arm64 和 Windows x86_64 命令行 archive、
内置 `luna-lsp`/`luna-fmt` 的平台 VSIX，以及 SHA-256 checksum。内容、安装方式及
独立 Luna compiler 要求见[分发文档](docs/distribution.zh-CN.md)。
