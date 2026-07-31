# Luna 编译器/工具协议

[English](protocol.md) | [简体中文](protocol.zh-CN.md)

> 状态：配套 Luna 编译器工作区中已实现，当前为实验性
> 协议：`luna.diagnostic` version 1

## 传输

首版传输是换行分隔 JSON。`luna check` 选择
`--message-format=json` 后，一次调用依次输出：

1. 严格一条 `hello`；
2. 零条或多条 `diagnostic`；
3. 严格一条 `summary`。

协议流不得混入人类可读输出。退出码 `0` 表示无错误，`1` 表示已经报告源码或编译
诊断，`2` 表示命令或协议用法错误；进程崩溃不属于协议内错误。

## 必需身份

`hello` 必须携带语言版本、编译器源码 commit、构建 target、诊断协议名称/版本和
可选 capability。客户端必须在解释诊断前拒绝不支持的主版本。由于 Luna 长期保持
`0.2.0-alpha`，compiler commit 仍是必要身份。

## 诊断记录

```json
{
  "protocol": "luna.diagnostic",
  "version": 1,
  "kind": "diagnostic",
  "severity": "error",
  "phase": "semantic",
  "code": "SEM0001",
  "message": "undefined name 'value'",
  "primary": {
    "path": "/workspace/main.luna",
    "start": { "byte": 42, "line": 3, "column": 9 },
    "end": { "byte": 47, "line": 3, "column": 14 }
  },
  "labels": [],
  "notes": [],
  "fixes": []
}
```

`primary` 存在时，byte offset 为必填，按 UTF-8 byte 计数，end 为 exclusive；没有
磁盘位置的诊断使用 `"primary": null`。line/column 是从 1 开始的人类展示辅助值；
LSP 客户端根据自己的文档快照把 byte offset 转为 UTF-16 位置。磁盘输入使用规范化
绝对路径；后续 overlay 使用显式 document URI，不复用 path 字段。

fix 是带 applicability（`machine-applicable`、`maybe-incorrect` 或 `manual`）的结构化
编辑。渲染后的 help 文本属于 note，不自动成为 fix。
`luna-protocol` 负责 Serde wire type，并拒绝错误协议身份、不支持的版本、无效记录
顺序和不一致的 summary。

## 延后的分析协议

`luna.analysis` 当前为 version `0`，表示尚不支持。hover、definition、reference、
completion 和 rename 必须等待编译器拥有的符号/类型记录，不能从 MoonIR 名字或渲染
诊断中推断。
