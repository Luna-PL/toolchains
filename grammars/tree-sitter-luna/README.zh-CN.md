# tree-sitter-luna

[English](README.md) | [简体中文](README.zh-CN.md)

该 package 是 LunaToolchain 的无损结构语法边界。M3 grammar 精确识别编译器 header
和声明起始，保留注释与每个 token，并要求 block、圆括号和方括号成对。表达式和类型
节点暂时保持粗粒度，直到编译器发布独立于私有 C++ AST 的语法契约。

```sh
npm ci
npm run generate
npm test
```

当 `LUNA_SOURCE_DIR` 指向编译器 checkout 时，`npm run test:compiler` 还会解析选定的
Luna 编译器 fixture。
