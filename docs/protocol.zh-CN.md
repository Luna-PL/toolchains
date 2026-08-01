# Luna 编译器/工具协议

[English](protocol.md) | [简体中文](protocol.zh-CN.md)

> 状态：诊断生产端已实现；analysis v1 消费端契约已实现
> 协议：`luna.diagnostic` version 1、`luna.analysis` version 1

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

## Analysis protocol v1

`luna.analysis` v1 是换行分隔的语义快照。一次调用严格输出一条 `hello`、零条或多条
`symbol`/`reference`，以及严格一条 `summary`。`declarations` capability 传递由编译器
拥有的符号身份和类型；`call-references` 增加已解析的直接调用目标，但不宣称覆盖全部
引用类别或未保存文档 overlay。

```json
{
  "protocol": "luna.analysis",
  "version": 1,
  "kind": "symbol",
  "id": "luna.symbol.v1:4:main:0::8:function:9:main::add",
  "name": "add",
  "qualified_name": "main::add",
  "package_id": "main",
  "module_path": "",
  "linkage_name": "main::add",
  "symbol_kind": "function",
  "signature": "fn add(left: i32, right: i32) -> i32",
  "selection": {
    "path": "/workspace/main.luna",
    "start": { "byte": 3, "line": 1, "column": 4 },
    "end": { "byte": 7, "line": 1, "column": 8 }
  },
  "exported": true,
  "external": false
}
```

Symbol ID 是不透明的稳定身份；客户端可以比较和保存它，但不能解析当前的长度分隔
表示。`selection` 与诊断 span 使用相同的 UTF-8 byte 及从 1 开始的展示位置规则。
summary 给出 symbol/reference 记录数和 `complete`。解析恢复或语义分析失败后，
编译器可以输出仍有用途的部分声明快照，并设置 `complete: false`。

`reference` 包含源码 span 和不透明 `target_id`，该 ID 必须指向同一快照中的 symbol。
客户端只能为 capability 明确声明的引用类别实现 definition；v1 的
`call-references` 当前只保证直接函数调用。

Rust wire type 和序列校验已经在 `luna-protocol` 中实现；配套编译器生产端及按
capability 开关的 `luna-compiler` 客户端也已实现。完整保存文件快照具备
`call-references` 时已经启用 definition。hover、workspace
reference、completion 和 rename 必须使用后续显式 capability，不能从 MoonIR 名字或
渲染诊断中推断。
