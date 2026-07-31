# Luna formatter 支持范围

[English](formatter.md) | [简体中文](formatter.zh-CN.md)

> 状态：LunaToolchain 0.1 中已实现，当前为实验性
> Grammar：使用 Tree-sitter 0.26.11 生成

`luna-fmt` 使用生成的 `tree-sitter-luna` parser 执行确定性的全文格式化。grammar
保留注释和所有词法 token，识别编译器 header 与声明起始，并在写入任何文件前拒绝
不配对或 malformed 的结构。

```sh
luna-fmt source.luna
luna-fmt --write source.luna package/src/main.luna
luna-fmt --check source.luna
printf 'fn main()->i32{return 0;}' | luna-fmt -
```

print 模式把一个格式化文档写到 stdout。`--write` 仅在完整解析和格式化成功后更新
文件。`--check` 不写入；任何文件需要修改时返回 `1`。命令用法、I/O 或语法错误
返回 `2`，malformed 输入保持逐 byte 不变。

当前风格使用四空格 block 缩进、每行一条 statement、稳定 operator spacing、末尾
换行，并保留 line comment 文本。格式化具有幂等性；暂不支持 range formatting。

结构 grammar 有意让表达式和类型子树保持粗粒度。它不是独立语义 parser，不得用于
hover、completion、所有权或类型判断。编译器 fixture 一致性测试用于防止 grammar
静默偏离编译器接受的 Luna 源码。
