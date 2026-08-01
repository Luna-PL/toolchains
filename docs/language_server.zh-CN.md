# Luna language server 支持范围

[English](language_server.md) | [简体中文](language_server.zh-CN.md)

> 状态：LunaToolchain 0.1 中已实现，当前为实验性
> 语义权威：Luna 编译器 `luna.diagnostic` 与 `luna.analysis` version 1

## 生命周期与同步

`luna-lsp` 使用带 `Content-Length` framing 的 stdio LSP。它实现
`initialize`、`initialized`、`shutdown` 和 `exit`，拒绝初始化前或 shutdown 后的
请求，并且不会留下持久编译器子进程。

打开的文档使用增量 UTF-16 同步。修改会更新带版本的内存 snapshot、使旧语义结果
失效并安排分析；150ms 内的事件会合并。编译器报告 `multi-document-overlay` 时，同一
package 中所有打开的脏文档会组成一次带版本的 stdin 请求。对旧编译器，只有一个脏文件时使用
`single-document-overlay`；否则脏文档继续使用词法 fallback。

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

仅当编译器报告 `call-references`、`method-references`、`type-references`、
`trait-references`、`field-references` 或 `enum-variant-references` capability 时才声明
`textDocument/definition`。它在完整的保存文件或版本匹配的 overlay 快照中，
通过不透明 Symbol ID 解析直接函数、用户 trait method、类型语法名称、impl/bound
trait 名称、struct 字段以及 enum 构造/match variant。

`textDocument/prepareRename` 和 `textDocument/rename` 提供有意收紧的 Luna 0.2.x
package rename。只有同时具备 `package-references` 和该符号对应引用 capability 时才启用，
并且只编辑当前完整快照中的声明及相同 Symbol ID 引用。支持 function、method、struct、
enum、trait、field 和 enum variant。局部变量、参数、metadata、constraint、kernel、fragment、
匿名 record 字段、文件改名、冲突预测和持久化跨 package 索引不在这个兼容实现内。
hover 和 completion 仍保持禁用。

仅当编译器报告 `package-references` 时才声明 `textDocument/references`。请求可以从声明或
任意已输出引用位置发起，并返回当前完整 package 快照中的所有匹配位置；它遵守
`context.includeDeclaration`。结果仅覆盖编译器广告的引用类别，持久化多 package workspace 索引仍待实现。

多文档分析会原子替换所有打开的根 package 脏文档，并把快照绑定到每个参与文档的
LSP 版本。其他 package 源码和依赖仍使用磁盘版本。脏 buffer 诊断仍要等待结构化
diagnostic-overlay capability。

## VS Code 配置

- `luna.server.path`：`luna-lsp` 可执行文件，默认从 `PATH` 查找；
- `luna.compiler.path`：可选的精确编译器路径；
- `Luna: Restart Language Server`：修改配置后停止并重新创建 client。

扩展要求 VS Code 1.91 或更高版本，并使用 `vscode-languageclient` 10.1.0。M2 在发布
前仍需 editor host 以及 Linux/macOS/Windows CI 证据。
