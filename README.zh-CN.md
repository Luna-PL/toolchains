# LunaToolchain

[English](README.md) | [简体中文](README.zh-CN.md)

LunaToolchain 是 Luna `0.2.0-alpha` 的独立编辑器与开发者工具工作区。本地开发时
它位于编译器目录内，但具有独立的 Git 历史、发布、兼容策略和版本号。

当前状态为 M1 诊断桥。`luna-protocol` 已能解析 JSONL、校验记录顺序并以 golden
fixture 固定协议；`luna-compiler` 负责发现并探测兼容编译器。LSP 和 formatter
二进制仍为占位实现；声明式 VS Code package 目前只提供语言注册和基础词法高亮。

## 仓库边界

- Luna 编译器负责语言语义、稳定诊断、源码 span、package 解析和机器分析协议。
- LunaToolchain 负责编辑器集成、协议客户端、格式化、workspace 展示和开发命令。
- 工具不得从渲染后的诊断文本推断语义，也不得链接编译器私有 C++ 类。

详细设计见[架构](docs/architecture.zh-CN.md)、[协议](docs/protocol.zh-CN.md)和
[交付路线图](docs/roadmap.zh-CN.md)。

## 本地验证

```sh
cargo test --workspace --offline
node -e 'JSON.parse(require("fs").readFileSync("editors/vscode/package.json"))'
cargo run --offline -p luna-tools -- compiler --luna /path/to/luna
```

编译器发现依次检查显式 `--luna`/`LUNA_BIN`、`PATH`，最后在本地开发候选
`../build/luna` 存在时检查它。任何候选都必须同时通过 `--version` 和诊断协议探测。
