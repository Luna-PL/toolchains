# LunaToolchain 架构

[English](architecture.md) | [简体中文](architecture.zh-CN.md)

> 状态：M1 诊断桥已实现；LSP 与 formatter 仍为规划状态
> 适用：LunaToolchain 0.1 开发期

## 依赖方向

```text
VS Code / 其他编辑器
          |
          v
       luna-lsp ------> luna-fmt
          |                 |
          v                 v
    luna-compiler       无损语法树
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
候选同时通过 `--version` 和 `luna.diagnostic` JSONL 身份探测后才接受。首版
language server 消费该 API；人类可读 stderr 不是稳定 API。首个支持模式是已保存
文件诊断；未保存 buffer 必须等待编译器 overlay 或 daemon 协议，不能通过伪造临时
package 布局实现。

## 语法边界

当前编译器 lexer 会丢弃注释，因此不能直接支持保留式 formatter。后续无损
Tree-sitter grammar 负责增量语法、注释、folding 和 formatter 输入；它必须使用
编译器一致性 fixture，防止静默接受另一套语言。

## 兼容性

语言版本、编译器构建身份、诊断协议、分析协议、LSP 版本和编辑器扩展版本是不同
身份。Luna 长期保持 `0.2.0-alpha` 时仍须显式暴露协议不兼容；客户端按协议版本和
compiler commit 决定是否启用功能。
