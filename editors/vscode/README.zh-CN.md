# Luna VS Code 扩展

[English](README.md) | [简体中文](README.zh-CN.md)

Alpha 扩展注册 `.luna`、提供词法高亮，并启动 `luna-lsp`，支持已保存文件诊断、
document symbol 和 folding；同时以 `luna-fmt` 提供全文格式化。它目前不宣称支持
hover、completion、definition、reference、rename、range formatting 或未保存
buffer 的语义诊断。

当 `luna-lsp` 不在 `PATH` 中时设置 `luna.server.path`。如需固定编译器，设置
`luna.compiler.path`；否则按 `LUNA_BIN`、`PATH`、本地开发 fallback 的顺序发现。
修改路径后可执行 **Luna: Restart Language Server**。

当 `luna-fmt` 不在 `PATH` 中时设置 `luna.formatter.path`。格式化通过 stdin 传递
当前 editor snapshot，不会对该未保存内容运行编译器诊断。

平台 Release VSIX 内置 `luna-lsp` 与 `luna-fmt`。server/formatter path 留空时优先
使用这些 binary，否则 fallback 到 `PATH`；Luna compiler 仍需单独安装。
