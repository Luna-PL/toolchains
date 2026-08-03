# LunaToolchain 架构

[English](architecture.md) | [简体中文](architecture.zh-CN.md)

> 状态：M3 语法和全文 formatter 已实现
> 适用：LunaToolchain 0.1 开发期

## 依赖方向

```text
             VS Code / 其他编辑器
                 /             \
                v               v
           luna-lsp          luna-fmt
               |                |
               v                v
         luna-compiler    tree-sitter-luna
               |
               v
         luna-protocol
               |
               v
           Luna 编译器进程
```

`luna-workspace` 只发现 manifest 和编译器根目录，不重新实现 package 语义。package
身份、export、类型、所有权、selector 和诊断仍以编译器为权威。

## 进程边界

`luna-compiler` 按显式配置、`PATH`、本地开发 fallback 的顺序发现候选，并且仅在
候选同时通过 `--version` 和 `luna.diagnostic` JSONL 身份探测后才接受，随后独立探测
可选的 `luna.analysis` 身份与 capability。首版
language server 消费该 API；人类可读 stderr 不是稳定 API，保存文件诊断仍保持独立。
报告 `multi-document-overlay` 的编译器通过带版本的 JSON stdin envelope 接收根 package 中
所有脏文档。language server 把结果绑定到完整的规范路径/版本表；任何编辑、保存或关闭都会
使其失效。旧编译器在只有一个脏文件时保留原有单文档 fallback，全程不创建临时 package 布局。

## 语法边界

编译器 lexer 会丢弃注释，因此不能直接支持保留式 formatter。生成的
`tree-sitter-luna` grammar 负责无损结构语法、注释和 formatter 输入。它的表达式与
类型子树有意保持粗粒度，不是语义权威；编译器一致性 fixture 用于防止 grammar
静默接受另一套语言。

编辑器把当前文档经 stdin 直接交给 `luna-fmt`；language server 不依赖 formatter。
folding 暂时仍使用 language server 的保守 scanner，后续仅在有明确收益测量和兼容
测试时再集成 Tree-sitter。

## 兼容性

语言版本、编译器构建身份、诊断协议、分析协议、LSP 版本和编辑器扩展版本是不同
身份。Luna 长期保持 `0.2.1` maintenance line 时仍须显式暴露协议不兼容；客户端按协议版本和
compiler commit 决定是否启用功能。

三平台开发 CI 跟踪 Luna 默认分支的源码 fixture，以立即暴露语言漂移；手动运行可
指定 branch、tag 或 commit。Release 产物则固定兼容的 Luna release tag，保证已发布
package 可复现。
