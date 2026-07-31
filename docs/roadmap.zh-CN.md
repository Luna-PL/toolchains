# LunaToolchain 交付路线图

[English](roadmap.md) | [简体中文](roadmap.zh-CN.md)

## M0：仓库与契约

- 独立 Git 仓库和 Rust workspace。
- 英文默认文档及中文对应版本。
- 诊断协议 v1 提案和兼容策略。
- 声明式 VS Code 语言注册。

退出条件：不下载 Luna 编译器即可运行仓库自身测试。

## M1：编译器诊断桥

状态：本地实现完成；发布前仍需补齐跨平台 CI 证据。

- Luna 编译器实现结构化诊断和 JSONL 输出。
- `luna-protocol` 提供经过验证的协议类型和 golden fixture。
- 编译器发现顺序为显式配置、`PATH`、本地开发 fallback；任何候选都必须先探测版本。

退出条件：Linux、macOS 和 Windows 的已保存文件诊断保持稳定。

## M2：最小 language server

- LSP stdio 生命周期和增量文档同步。
- debounce 后执行已保存文件检查，发布诊断、document symbol 和 folding range。
- 发现 package/workspace 根，但不重新实现 package 语义。

退出条件：VS Code 可反复启动/停止 server，且不遗留进程。

## M3：无损语法和 formatter

- Tree-sitter Luna grammar 及编译器一致性 fixture。
- 保留注释的全文 formatter 和 `--check`。
- 幂等及错误输入测试；range formatting 后续再做。

退出条件：格式化两次与格式化一次逐 byte 相同。

## M4：语义编辑能力

- 编译器拥有的分析协议和文档 overlay。
- 依次实现 hover、definition、completion、reference 和 rename。
- 功能按协议 capability 开关，不猜测编译器版本。

退出条件：跨文件 package 示例通过语义 LSP 集成测试。

## M5：构建、package 与分发

- check/build/run/test task、测试选择、workspace 状态和本地缓存报告。
- `luna-lsp` 预编译产物、VSIX、checksum，以及绑定 compiler commit 与协议版本的
  compatibility matrix。

远程 registry、调试器和语言表面扩张不属于 0.1 计划。
