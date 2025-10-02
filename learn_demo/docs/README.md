# 📚 Oxc 学习文档

> 从实践到原理，循序渐进学习 Oxc

## 🚀 快速开始

**第一次来？** 👉 从这里开始：[START_HERE.md](./START_HERE.md)

## 📂 文档结构

```
docs/
├── README.md                    # 本文件 - 文档导航
├── START_HERE.md                # 🌟 开始指南
├── 学习路径.md                   # 完整学习路径规划
├── QUICK_REFERENCE.md           # 常用命令速查
│
├── 第一周/                       # 工具链使用
│   ├── Day1-2_Parser基础.md
│   ├── Day3-4_Linter基础.md
│   └── Day5-7_其他工具.md       (待创建)
│
├── 第二周/                       # 核心概念
│   ├── 访问者模式.md            (待创建)
│   ├── AST操作.md               (待创建)
│   └── Semantic分析.md          (待创建)
│
├── 第三周/                       # 深入方向
│   ├── 方向A_贡献Linter规则.md  (待创建)
│   ├── 方向B_理解Parser实现.md  (待创建)
│   ├── 方向C_学习代码转换.md    (待创建)
│   └── 方向D_深入内存管理.md    (待创建)
│
└── allocator_深入学习/          # Allocator 专题（可选）
    ├── allocator_学习总结.md
    ├── allocator_设计分析.md
    └── ...
```

## 📖 学习路径

### 第一周：工具链使用 (5-7 天)

从**实际使用**开始，快速上手 Oxc 的各个工具。

| 天数 | 主题 | 文档 | 预计时间 |
|-----|------|------|---------|
| Day 1-2 | Parser 基础 | [📄](./第一周/Day1-2_Parser基础.md) | 2-3 小时 |
| Day 3-4 | Linter 基础 | [📄](./第一周/Day3-4_Linter基础.md) | 2-3 小时 |
| Day 5-7 | 其他工具 | 待创建 | 3-4 小时 |

**学习目标**:
- ✅ 能够运行 Parser 解析代码并理解 AST
- ✅ 能够阅读和编写简单的 Linter 规则
- ✅ 了解 Formatter、Transformer、Minifier 的作用

---

### 第二周：核心概念 (7-10 天)

深入理解 AST 遍历和分析机制。

| 主题 | 文档 | 预计时间 |
|-----|------|---------|
| 访问者模式 | 待创建 | 2-3 小时 |
| AST 操作 | 待创建 | 2-3 小时 |
| Semantic 分析 | 待创建 | 3-4 小时 |

**学习目标**:
- ✅ 掌握 Visitor 模式进行 AST 遍历
- ✅ 能够查询和分析 AST 节点
- ✅ 理解作用域、符号和引用

---

### 第三周：深入方向 (按兴趣选择)

根据个人兴趣和目标，选择一个方向深入学习。

| 方向 | 适合人群 | 文档 |
|-----|---------|------|
| A: 贡献 Linter 规则 | 想为项目做贡献 | 待创建 |
| B: 理解 Parser 实现 | 对编译原理感兴趣 | 待创建 |
| C: 学习代码转换 | 对 Babel 感兴趣 | 待创建 |
| D: 深入内存管理 | 对 Rust 和性能感兴趣 | 待创建 |

---

## 🎯 如何使用这些文档

### 线性学习（推荐）

按照第一周 → 第二周 → 第三周的顺序学习。

```bash
# 1. 阅读开始指南
open docs/START_HERE.md

# 2. 第一周 Day 1-2
open docs/第一周/Day1-2_Parser基础.md

# 3. 第一周 Day 3-4
open docs/第一周/Day3-4_Linter基础.md

# ... 依次进行
```

### 按需学习

根据你的需求，跳到相应的文档。

```bash
# 只想了解 Parser
open docs/第一周/Day1-2_Parser基础.md

# 只想写 Linter 规则
open docs/第一周/Day3-4_Linter基础.md
open docs/第三周/方向A_贡献Linter规则.md

# 只想深入 Allocator
open docs/allocator_深入学习/
```

### 配合代码示例

每个文档都有对应的代码示例：

```bash
cd learn_demo

# Parser 示例
cargo run --bin 01_parser_basics

# Linter 示例
cargo run --bin 02_linter_basics

# Allocator 示例
cargo run --bin allocator_01_basics
```

## 📚 特殊主题

### Allocator 深入学习

如果你对**内存管理和性能优化**特别感兴趣，可以深入学习 Allocator：

📁 [allocator_深入学习/](./allocator_深入学习/)

这部分内容较为高级，建议：
- **时机**: 完成第一周和第二周后
- **方式**: 作为第三周方向 D 的学习内容
- **前置**: 需要对 Rust 有一定了解

## 🔗 相关资源

### 项目文档
- [项目 README](../README.md) - 项目总览
- [快速参考](../QUICK_REFERENCE.md) - 常用命令

### Oxc 官方
- [官方网站](https://oxc.rs/)
- [GitHub 仓库](https://github.com/oxc-project/oxc)
- [AGENTS.md](../../AGENTS.md) - AI 助手指南
- [CONTRIBUTING.md](../../CONTRIBUTING.md) - 贡献指南

### 在线工具
- [AST Explorer](https://astexplorer.net/) - 查看 AST 结构
- [Rust Playground](https://play.rust-lang.org/) - 在线 Rust

## 💡 学习建议

### 每天的学习节奏

- **阅读文档** 30 分钟
- **运行示例** 20 分钟
- **修改实验** 40 分钟
- **总结记录** 10 分钟

**总计**: 约 1.5-2 小时/天

### 学习检查点

- [ ] 第一周结束：能运行所有工具，理解基本概念
- [ ] 第二周结束：能独立编写简单的 Linter 规则
- [ ] 第三周结束：在选定方向上有深入理解

### 遇到问题

1. 查看文档中的"常见问题"部分
2. 查看 [QUICK_REFERENCE.md](../QUICK_REFERENCE.md)
3. 搜索相关源码
4. 提交 Issue 或在 Discord 讨论

## 📝 贡献文档

发现文档中的问题或有改进建议？

- 提交 Issue
- 发起 Pull Request
- 完善现有文档
- 创建新的学习材料

---

## 🎓 准备好了吗？

👉 **从这里开始**: [START_HERE.md](./START_HERE.md)

祝学习愉快！🚀

