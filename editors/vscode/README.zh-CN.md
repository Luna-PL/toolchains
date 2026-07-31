# Luna VS Code 扩展

[English](README.md) | [简体中文](README.zh-CN.md)

Alpha 扩展注册 `.luna`、提供词法高亮，并启动 `luna-lsp`，支持已保存文件诊断、
document symbol 和 folding。它目前不宣称支持 hover、completion、definition、
reference、rename、formatting 或未保存 buffer 的语义诊断。

当 `luna-lsp` 不在 `PATH` 中时设置 `luna.server.path`。如需固定编译器，设置
`luna.compiler.path`；否则按 `LUNA_BIN`、`PATH`、本地开发 fallback 的顺序发现。
修改路径后可执行 **Luna: Restart Language Server**。
