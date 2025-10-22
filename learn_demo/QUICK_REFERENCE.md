# 🎯 快速参考卡片

> 常用命令和快捷方式，方便随时查阅

## 🚀 运行示例

```bash
# 进入目录
cd learn_demo

# 第一周示例
cargo run --bin 01_parser_basics      # Parser 基础
cargo run --bin 02_linter_basics      # Linter 基础（待创建）
cargo run --bin 03_formatter_basics   # Formatter（待创建）

# Allocator 示例（第三周方向 D）
cargo run --bin allocator_01_basics   # Allocator 基础
cargo run --bin allocator_02_performance  # 性能对比

# 编译检查
cargo check

# 运行测试
cargo test --bin 01_parser_basics
```

## 📁 快速导航

```bash
# 查看文档
open docs/START_HERE.md              # 开始指南
open docs/00_学习路径_实用优先.md     # 学习路径
open docs/第一周_Day1-2_Parser基础.md # 第一天

# 查看源码
code src/skeleton_01_parser_basics.rs # 示例代码

# 查看 Oxc 源码
code ../crates/oxc_parser/           # Parser 实现
code ../crates/oxc_linter/           # Linter 实现
code ../crates/oxc_ast/              # AST 定义
```

## 🔧 Oxc 官方示例

```bash
# Parser
cargo run -p oxc_parser --example parser -- test.js

# Linter
cargo run -p oxc_linter --example linter -- src/

# Formatter
cargo run -p oxc_formatter --example formatter -- input.js

# Transformer
cargo run -p oxc_transformer --example transformer -- input.js

# Minifier
cargo run -p oxc_minifier --example minifier -- input.js

# 完整编译器
cargo run -p oxc --example compiler --features="full" -- test.js
```

## 📚 常用目录

| 目录                    | 说明           |
| ----------------------- | -------------- |
| `learn_demo/docs/`      | 学习文档       |
| `learn_demo/src/`       | 示例代码       |
| `crates/oxc_parser/`    | Parser 源码    |
| `crates/oxc_linter/`    | Linter 源码    |
| `crates/oxc_ast/`       | AST 定义       |
| `crates/oxc_semantic/`  | Semantic 分析  |
| `crates/oxc_allocator/` | Allocator 实现 |

## 🔍 代码搜索

```bash
# 搜索某个符号的定义
grep -r "struct VariableDeclaration" crates/oxc_ast/

# 搜索函数使用
grep -r "visit_variable_declaration" crates/

# 查找示例
find . -name "*.rs" -path "*/examples/*"

# 查找测试
find . -name "*.rs" -path "*/tests/*"
```

## 📝 常用 Git 命令

```bash
# 查看状态
git status

# 创建分支
git checkout -b learn_oxc

# 提交修改
git add .
git commit -m "学习笔记和实验"

# 放弃修改
git restore <file>

# 查看差异
git diff
```

## 🧪 测试相关

```bash
# 运行所有测试
just test

# 运行特定 crate 的测试
cargo test -p oxc_parser
cargo test -p oxc_linter

# 运行 conformance 测试
just conformance
cargo coverage

# 更新快照
cargo insta review

# 格式化代码
just fmt
```

## 🌐 在线工具

| 工具            | 链接                               | 说明          |
| --------------- | ---------------------------------- | ------------- |
| AST Explorer    | https://astexplorer.net/           | 查看 AST 结构 |
| Rust Playground | https://play.rust-lang.org/        | 在线 Rust     |
| Oxc 官网        | https://oxc.rs/                    | 官方文档      |
| GitHub          | https://github.com/oxc-project/oxc | 源码仓库      |

## 💡 调试技巧

```bash
# 详细日志
RUST_LOG=debug cargo run --bin 01_parser_basics

# 打印类型信息（在代码中）
dbg!(&variable);

# 打印到 stderr
eprintln!("Debug: {:?}", value);

# 条件编译
#[cfg(debug_assertions)]
println!("This only prints in debug mode");
```

## 🎯 学习检查点

### 第一周

- [ ] Parser: 能解析 JS/TS 代码
- [ ] Linter: 能创建简单规则
- [ ] 其他: 了解各工具作用

### 第二周

- [ ] Visitor: 理解遍历模式
- [ ] AST: 能查询和分析
- [ ] Semantic: 理解作用域

### 第三周

- [ ] 选择方向并深入

## 📞 获取帮助

| 方式      | 链接                                      |
| --------- | ----------------------------------------- |
| 文档      | `docs/` 目录                              |
| Issues    | https://github.com/oxc-project/oxc/issues |
| Discord   | https://discord.gg/9uXCAwqQZW             |
| AGENTS.md | 根目录                                    |

## ⌨️ VS Code 快捷键

```bash
# Rust Analyzer
Cmd+. (Mac) / Ctrl+. (Win)  # 快速修复
F12                          # 跳转到定义
Shift+F12                   # 查找所有引用
Cmd+P                       # 快速打开文件

# 终端
Ctrl+`                      # 打开/关闭终端
Cmd+K Cmd+S                 # 快捷键列表
```

## 📊 学习时间建议

| 活动     | 时间            |
| -------- | --------------- |
| 阅读文档 | 30 分钟         |
| 运行示例 | 20 分钟         |
| 修改实验 | 40 分钟         |
| 总结笔记 | 10 分钟         |
| **合计** | **100 分钟/天** |

## 🎨 Markdown 语法

````markdown
# 一级标题

## 二级标题

### 三级标题

**粗体** _斜体_ `代码`

- 列表项
  - 子项

1. 有序列表
2. 第二项

[链接](url)

\```rust
// 代码块
fn main() {}
\```

> 引用

---

分隔线
````

---

**打印这份文档，放在手边随时查阅！** 📄
