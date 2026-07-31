# 测试布局

[English](README.md) | [简体中文](README.zh-CN.md)

- `protocol/`：由 `luna-protocol` 测试读取的编译器 JSONL golden record。
- `fixtures/`：随本仓库授权的最小语法与 workspace 输入。
- `integration/`：需要显式提供 `LUNA_BIN` 编译器路径的测试。

单元测试必须能在没有编译器 checkout 和网络访问的情况下运行。设置 `LUNA_BIN` 后，
workspace 测试还会让 `luna-compiler` 探测真实编译器。`luna-lsp` 的 stdio 集成测试
覆盖生命周期、symbol、folding，并在设置 `LUNA_BIN` 时覆盖真实编译器诊断。

`luna-fmt` 测试覆盖注释保留、幂等性、`--check`、`--write`、stdin、malformed 输入
不变性和可选编译器源码 corpus。同时设置 `LUNA_SOURCE_DIR` 与 `LUNA_BIN` 时，真实
编译器还会检查每个格式化后的 corpus 文件。
