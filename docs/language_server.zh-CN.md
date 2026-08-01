# Luna language server 支持范围

[English](language_server.md) | [简体中文](language_server.zh-CN.md)

> 状态：LunaToolchain 0.1 中已实现，当前为实验性
> 语义权威：Luna 编译器 `luna.diagnostic` 与 `luna.analysis` version 1

## 生命周期与同步

`luna-lsp` 使用带 `Content-Length` framing 的 stdio LSP。它实现
`initialize`、`initialized`、`shutdown` 和 `exit`，拒绝初始化前或 shutdown 后的
请求，并且不会留下持久编译器子进程。

打开的文档使用增量 UTF-16 同步。修改会更新带版本的内存 snapshot、使旧语义结果
失效并安排分析；150ms 内的事件会合并。编译器报告 `single-document-overlay` 时，脏
文本通过 stdin 传入；否则脏文档继续使用词法 fallback。

## 诊断

server 在初始化期间发现兼容编译器，并对最近的 package、workspace 或独立源码边界
执行 `luna check --message-format=json`。它不会把未保存 buffer 复制到临时 package。
编译器 UTF-8 byte span 会根据已保存文件转换成 LSP UTF-16 range。诊断按源码 URI
分组；成功重新检查或关闭文档后会清除旧诊断。

## 结构化编辑能力

当前提供 `textDocument/documentSymbol` 和 `textDocument/foldingRange`。保存文件成功
分析后，document symbol 使用编译器拥有的声明 ID、kind、签名
和 selection span。完整且版本匹配的 overlay 为脏文档提供相同能力；结果返回前或
编译器较旧时继续使用词法声明 fallback。
folding 仍跟踪注释和字符串之外的配对大括号。

仅当编译器报告 `call-references`、`method-references`、`type-references` 或
`trait-references` capability 时才声明 `textDocument/definition`。它在完整的保存文件或版本匹配的
overlay 快照中通过不透明 Symbol ID 解析直接函数、用户 trait method、类型语法名称和
impl/bound trait 名称；构造器和字段引用尚未覆盖。
hover、completion、workspace reference 和 rename 仍需编译器协议
提供各自所需的更多语义记录，目前保持禁用。

当前 overlay capability 只替换根 package 中一个文档。同一 package 有多个脏文件时，
每次分析只看到请求文档和其他文件的磁盘版本；多文档 overlay 是独立的后续协议里程碑。
脏 buffer 诊断也要等待结构化 diagnostic-overlay capability。

## VS Code 配置

- `luna.server.path`：`luna-lsp` 可执行文件，默认从 `PATH` 查找；
- `luna.compiler.path`：可选的精确编译器路径；
- `Luna: Restart Language Server`：修改配置后停止并重新创建 client。

扩展要求 VS Code 1.91 或更高版本，并使用 `vscode-languageclient` 10.1.0。M2 在发布
前仍需 editor host 以及 Linux/macOS/Windows CI 证据。
