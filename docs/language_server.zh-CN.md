# Luna language server 支持范围

[English](language_server.md) | [简体中文](language_server.zh-CN.md)

> 状态：LunaToolchain 0.1 中已实现，当前为实验性
> 语义权威：Luna 编译器 `luna.diagnostic` version 1

## 生命周期与同步

`luna-lsp` 使用带 `Content-Length` framing 的 stdio LSP。它实现
`initialize`、`initialized`、`shutdown` 和 `exit`，拒绝初始化前或 shutdown 后的
请求，并且不会留下持久编译器子进程。

打开的文档使用增量 UTF-16 同步。修改只更新 server 的内存 snapshot，并取消待执行
检查。仅当打开时 snapshot 与磁盘完全一致，或收到 `didSave` 后才安排检查；150ms
内的连续保存会合并。

## 诊断

server 在初始化期间发现兼容编译器，并对最近的 package、workspace 或独立源码边界
执行 `luna check --message-format=json`。它不会把未保存 buffer 复制到临时 package。
编译器 UTF-8 byte span 会根据已保存文件转换成 LSP UTF-16 range。诊断按源码 URI
分组；成功重新检查或关闭文档后会清除旧诊断。

## 结构化编辑能力

当前提供 `textDocument/documentSymbol` 和 `textDocument/foldingRange`。它们只是词法
结构辅助，不是语义分析：symbol 识别声明头，folding 跟踪注释和字符串之外的配对
大括号。hover、completion、definition、reference 和 rename 必须等待编译器实现
`luna.analysis`，目前保持禁用。

## VS Code 配置

- `luna.server.path`：`luna-lsp` 可执行文件，默认从 `PATH` 查找；
- `luna.compiler.path`：可选的精确编译器路径；
- `Luna: Restart Language Server`：修改配置后停止并重新创建 client。

扩展要求 VS Code 1.91 或更高版本，并使用 `vscode-languageclient` 10.1.0。M2 在发布
前仍需 editor host 以及 Linux/macOS/Windows CI 证据。
