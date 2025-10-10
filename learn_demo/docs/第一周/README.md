# 第一周学习指南：工具链使用与 AST 深入

> 从基础使用到深入理解，全面掌握 Oxc AST

## 📋 本周概览

第一周分为三个阶段，循序渐进地学习：

```
Day 1-2: Parser 基础
   ↓
Day 3-4: Linter 基础
   ↓
Day 5-7: oxc_ast 深入 + Rust 进阶
```

**学习目标**：
- ✅ 理解 AST 的概念和结构
- ✅ 掌握 Parser 和 Linter 的基本使用
- ✅ 深入理解 AST 节点定义
- ✅ 学会使用 Visitor 模式
- ✅ 理解 Arena 分配器
- ✅ 掌握必要的 Rust 知识

**预计时间**: 每天 1-2 小时，共 7 天

---

## 📚 学习路径

### Day 1-2: Parser 基础 (入门)

**文档**: [Day1-2_Parser基础.md](./Day1-2_Parser基础.md)

**学习内容**:
- 什么是 AST（抽象语法树）
- Parser 的工作流程
- 常见 AST 节点类型
- 运行和观察 Parser 输出

**实践**:
```bash
# 运行官方示例
cargo run -p oxc_parser --example parser -- test.js

# 运行学习示例
cargo run --bin 01_parser_basics
```

**关键概念**:
- `Program` - AST 根节点
- `Statement` - 语句节点
- `Expression` - 表达式节点
- `Declaration` - 声明节点

**检查点**:
- [ ] 能够运行 Parser 解析 JS/TS 文件
- [ ] 能够识别 5 种以上 AST 节点类型
- [ ] 理解 Statement 和 Expression 的区别

---

### Day 3-4: Linter 基础 (应用)

**文档**: [Day3-4_Linter基础.md](./Day3-4_Linter基础.md)

**学习内容**:
- Linter 的工作原理
- 访问者模式 (Visitor Pattern)
- 如何阅读现有规则
- 如何创建自定义规则

**实践**:
```bash
# 运行 Linter
cargo run -p oxc_linter --example linter -- test.js

# 创建新规则
just new-rule my-rule-name

# 测试规则
cargo test -p oxc_linter my_rule_name
```

**关键概念**:
- `Visit` trait
- `walk_*` 函数
- `run` 和 `run_once` 方法
- Diagnostic 诊断信息

**检查点**:
- [ ] 理解 Visitor 模式
- [ ] 能够阅读简单的 Lint 规则
- [ ] 成功实现一个自定义规则

---

### Day 5-7: oxc_ast 深入 + Rust 进阶 (深入)

**文档**: [Day5-7_深入oxc_ast与Rust进阶.md](./Day5-7_深入oxc_ast与Rust进阶.md)

**学习内容**:

#### Day 5: AST 节点定义
- `crates/oxc_ast/src/ast/js.rs` 详解
- 节点继承关系 (`@inherit`)
- Rust 生命周期 `'a`
- Rust 枚举和模式匹配

#### Day 6: Visitor 模式深入
- 自定义 Visitor 实现
- Pre-order 和条件遍历
- 实用分析器编写

#### Day 7: Arena 分配器
- 为什么需要 Arena？
- `Box<'a, T>` vs `Box<T>`
- `AstBuilder` 使用
- 手动构建 AST 节点

**实践**:
```bash
# 运行深入示例
cargo run --bin 05_ast_deep_dive

# 查看 AST 定义
code crates/oxc_ast/src/ast/js.rs
```

**关键 Rust 知识**:
- 生命周期 `'a`
- 智能指针 `Box<'a, T>`
- Arena 分配 `Vec<'a, T>`
- 内部可变性 `Cell<T>`
- Trait 系统

**检查点**:
- [ ] 理解生命周期的作用
- [ ] 能够找到任意 AST 节点的定义
- [ ] 能够实现复杂的 Visitor
- [ ] 能够手动构建 AST 节点
- [ ] 理解 Arena 分配器的优势

---

## 📖 配套资源

### 文档

1. **[Rust知识点速查表.md](./Rust知识点速查表.md)** ⭐
   - 生命周期、所有权、借用
   - 智能指针、枚举、Trait
   - 宏、属性、内部可变性
   - 随时查阅的参考卡片

2. **核心代码位置**
   ```
   crates/oxc_ast/src/
   ├── ast/
   │   ├── js.rs          # JavaScript AST 节点 ⭐
   │   ├── ts.rs          # TypeScript 节点
   │   ├── jsx.rs         # JSX 节点
   │   └── literal.rs     # 字面量节点
   ├── ast_builder.rs     # AST 构建工具
   └── visit.rs           # Visitor trait
   ```

### 代码示例

1. **01_parser_basics.rs**
   - Parser 基础使用
   - AST 遍历示例

2. **05_ast_deep_dive.rs** ⭐ (新增)
   - Day 5: AST 节点结构分析
   - Day 6: 多种 Visitor 实现
   - Day 7: 手动构建 AST

### 在线工具

- [AST Explorer](https://astexplorer.net/) - 可视化 AST 结构
- [Rust Playground](https://play.rust-lang.org/) - 在线运行 Rust
- [Rust Book](https://doc.rust-lang.org/book/) - Rust 官方教程

---

## 🎯 学习建议

### 学习节奏

| 时间 | 内容 | 重点 |
|------|------|------|
| Day 1 | Parser 基础 - 理论 | 理解 AST 概念 |
| Day 2 | Parser 基础 - 实践 | 运行示例，观察输出 |
| Day 3 | Linter 基础 - Visitor | 理解访问者模式 |
| Day 4 | Linter 基础 - 规则 | 实现自定义规则 |
| Day 5 | AST 节点定义 | 理解节点结构 |
| Day 6 | Visitor 深入 | 实现复杂分析器 |
| Day 7 | Arena 分配器 | 理解内存管理 |

### 学习方法

#### 1. 边学边做 👷
不要只看文档，一定要：
- ✅ 运行示例代码
- ✅ 修改示例代码
- ✅ 完成练习题
- ✅ 实现自己的想法

#### 2. 对照学习 📖
同时打开多个窗口：
- 左侧：文档
- 中间：代码编辑器
- 右侧：AST Explorer

#### 3. 循序渐进 🪜
遇到困难时：
- 第一遍：快速浏览，了解大概
- 第二遍：仔细阅读，做笔记
- 第三遍：动手实践，解决问题

#### 4. 记录笔记 📝
每个文档都有笔记模板：
- 今天学到了什么
- 印象最深的概念
- 遇到的问题和解决方案
- 明天的计划

---

## 🔬 实践任务清单

### 必做任务 ✅

- [ ] Day 1-2: 运行 Parser，理解 3 种以上节点类型
- [ ] Day 3-4: 实现 `no-magic-numbers` 规则
- [ ] Day 5: 找到 5 种节点定义，理解其结构
- [ ] Day 6: 实现变量声明分析器
- [ ] Day 7: 手动构建 `1 + 2` 表达式

### 可选任务 ⭐

- [ ] 实现 `no-var` 规则
- [ ] 实现函数复杂度分析器
- [ ] 手动构建完整的函数声明
- [ ] 阅读 3 个以上 ESLint 规则源码

### 挑战任务 🔥

- [ ] 实现 `require-await` 规则
- [ ] 实现依赖分析器（分析 import）
- [ ] 构建一个小型 AST 转换工具

---

## 📊 学习进度追踪

| 日期 | 内容 | 耗时 | 完成度 | 笔记 |
|------|------|------|--------|------|
| __/__ | Day 1: Parser 理论 | __ h | __% | |
| __/__ | Day 2: Parser 实践 | __ h | __% | |
| __/__ | Day 3: Visitor 模式 | __ h | __% | |
| __/__ | Day 4: 自定义规则 | __ h | __% | |
| __/__ | Day 5: AST 节点 | __ h | __% | |
| __/__ | Day 6: Visitor 深入 | __ h | __% | |
| __/__ | Day 7: Arena 分配器 | __ h | __% | |

---

## 💡 常见问题

### Q1: Rust 基础不好，能学吗？

**A**: 可以！我们从使用开始，边学边掌握 Rust。遇到不懂的概念：
1. 查看 [Rust知识点速查表.md](./Rust知识点速查表.md)
2. 阅读 [Rust Book](https://doc.rust-lang.org/book/) 相关章节
3. 在 [Rust Playground](https://play.rust-lang.org/) 实验

### Q2: 某个概念看不懂怎么办？

**A**: 很正常！建议：
1. 先跳过，继续往下学
2. 多看几遍，重复加深理解
3. 动手实践，在做中学
4. 提问讨论，寻求帮助

### Q3: 一天学多少内容合适？

**A**: 建议：
- **最低目标**: 每天 30 分钟，阅读文档
- **推荐目标**: 每天 1-2 小时，包含实践
- **不要求**: 完全理解所有细节

重要的是**持续学习**，而不是一次学完！

### Q4: 如何检验学习效果？

**A**: 通过实践检验：
- 能否运行示例代码？
- 能否完成练习题？
- 能否解释给别人听？
- 能否应用到新场景？

### Q5: Day 5-7 太难了怎么办？

**A**: Day 5-7 确实有挑战性：
1. 可以先快速浏览，了解有这些内容
2. 重点放在 Day 1-4，打好基础
3. 后面需要时再回来深入学习
4. 难点可以标记，后续讨论

---

## 🎓 完成第一周后

### 你将掌握

✅ AST 的基本概念和结构
✅ Parser 和 Linter 的使用
✅ Visitor 模式的应用
✅ 基本的 Rust 语法
✅ oxc_ast 的节点定义
✅ Arena 分配器的原理

### 下一步方向

完成第一周学习后，你可以：

1. **继续第二周**: 深入核心概念
   - Semantic Analysis
   - 作用域和符号表
   - 类型系统

2. **选择专项深入**:
   - 方向 A: Linter 规则开发
   - 方向 B: Parser 实现原理
   - 方向 C: AST 转换和优化
   - 方向 D: 性能优化和内存管理

3. **实战项目**:
   - 开发自己的 Lint 规则包
   - 构建代码分析工具
   - 为 Oxc 贡献代码

---

## 📞 获取帮助

### 文档资源
- [START_HERE.md](../START_HERE.md) - 入门指南
- [学习路径.md](../学习路径.md) - 整体规划
- [AGENTS.md](../../../AGENTS.md) - Oxc 架构

### 社区支持
- GitHub Issues
- Discord 社区
- 讨论区

### 提问技巧
好的提问应该包含：
1. 你想做什么
2. 你尝试了什么
3. 遇到了什么问题
4. 相关的代码和错误信息

---

## ✨ 开始学习

准备好了吗？让我们开始吧！

👉 **从这里开始**: [Day 1-2: Parser 基础](./Day1-2_Parser基础.md)

---

**记住**:
- 学习是一个过程，不要急于求成
- 遇到困难很正常，坚持就是胜利
- 多动手实践，在做中学
- 保持好奇心和探索精神

Good luck! 🚀

---

**学习开始日期**: ___________
**预计完成日期**: ___________
**实际完成日期**: ___________

**本周最大收获**:


**遇到的最大挑战**:


**对下周的期待**:

