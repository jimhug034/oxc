# Oxc Allocator 学习示例

这个目录包含了学习 `oxc_allocator` 模块的完整示例程序。每个示例都专注于特定的概念和用法。

## 🚀 快速开始

```bash
# 进入示例目录
cd learn_docs/examples

# 运行所有示例（按顺序）
./run_all_examples.sh

# 或者单独运行某个示例
cargo run --bin 01_allocator_basics
```

## 📚 示例列表

### 🏗️ Oxc Allocator 实践示例

### 01. 基础使用 (`01_allocator_basics.rs`)
**学习目标**: 理解 Arena 分配器的基本概念和使用方法

**内容包括**:
- 创建和使用 Allocator
- 分配基本数据类型
- 观察内存地址和连续性
- 字符串分配

**运行命令**:
```bash
cargo run --bin 01_allocator_basics
```

**预期输出**: 展示基本分配操作和内存地址信息

---

### 02. 性能对比 (`02_performance_comparison.rs`)
**学习目标**: 理解 Arena 分配器相比传统分配方式的性能优势

**内容包括**:
- 不同规模的性能测试
- 不同数据类型的性能对比
- 内存使用效率分析
- 缓存友好性演示

**运行命令**:
```bash
# 建议使用 release 模式以获得更明显的性能差异
cargo run --bin 02_performance_comparison --release
```

**预期输出**: 详细的性能对比数据和分析

---

### 03. Arena 数据结构 (`03_arena_data_structures.rs`)
**学习目标**: 掌握 Arena 版本的数据结构使用

**内容包括**:
- ArenaBox 使用
- ArenaVec 操作
- ArenaHashMap 应用
- 嵌套和复杂数据结构

**运行命令**:
```bash
cargo run --bin 03_arena_data_structures
```

**预期输出**: 各种 Arena 数据结构的使用示例

---

### 04. 内存管理 (`04_memory_management.rs`)
**学习目标**: 理解内存管理和生命周期

**内容包括**:
- Allocator reset 功能
- 生命周期演示
- 内存增长和管理
- 批处理场景

**运行命令**:
```bash
cargo run --bin 04_memory_management
```

**预期输出**: 内存管理策略和生命周期的演示

---

### 05. AST 模拟 (`05_ast_simulation.rs`)
**学习目标**: 了解 Arena 分配器在 AST 构建中的实际应用

**内容包括**:
- 简单和复杂 AST 构建
- AST 遍历算法
- JavaScript 解析模拟
- 性能对比分析

**运行命令**:
```bash
cargo run --bin 05_ast_simulation
```

**预期输出**: AST 构建和遍历的完整演示

---

### 06. 高级特性 (`06_advanced_features.rs`)
**学习目标**: 掌握高级特性和最佳实践

**内容包括**:
- 内存对齐演示
- 大对象分配策略
- 自定义分配器模式
- 错误处理和最佳实践

**运行命令**:
```bash
cargo run --bin 06_advanced_features
```

**预期输出**: 高级特性和实用技巧的展示

---

### 🦀 Rust 概念学习示例

### R01. 生命周期 (`rust_concepts_01_lifetimes.rs`)
**学习目标**: 深入理解 Rust 生命周期系统

**内容包括**:
- 基础生命周期概念
- Arena 分配器中的生命周期绑定
- 生命周期省略规则
- 多个生命周期参数
- 静态生命周期

**运行命令**:
```bash
cargo run --bin rust_concepts_01_lifetimes
```

### R02. 不安全 Rust (`rust_concepts_02_unsafe.rs`)
**学习目标**: 理解 unsafe Rust 的正确使用

**内容包括**:
- 为什么需要 unsafe
- Arena 分配器中的 unsafe 使用
- 编译时 vs 运行时检查
- 内存安全保证
- Unsafe 最佳实践

**运行命令**:
```bash
cargo run --bin rust_concepts_02_unsafe
```

### R03-R07. 更多 Rust 概念
- `rust_concepts_03_generics_traits.rs` - 泛型和 Trait
- `rust_concepts_04_smart_pointers.rs` - 智能指针
- `rust_concepts_05_advanced_traits.rs` - 高级 Trait 应用
- `rust_concepts_06_macros.rs` - 宏编程
- `rust_concepts_07_performance.rs` - 性能优化

## 🎯 学习路径建议

### 第一天：基础概念
1. 运行 `01_allocator_basics` - 理解基本概念
2. 运行 `02_performance_comparison` - 感受性能优势
3. 阅读输出，理解 Arena 分配器的工作原理

### 第二天：数据结构和内存管理
1. 运行 `03_arena_data_structures` - 学习 Arena 数据结构
2. 运行 `04_memory_management` - 理解内存管理
3. 实践：修改示例代码，添加自己的测试

### 第三天：实际应用
1. 运行 `05_ast_simulation` - 了解实际应用场景
2. 运行 `06_advanced_features` - 掌握高级特性
3. 总结：整理学习笔记，准备进入下一个模块

### 🦀 Rust 概念学习路径

### 第一周：Rust 基础概念
- **第1-2天**: 生命周期系统 (`rust_concepts_01_lifetimes`)
- **第3-4天**: 不安全 Rust (`rust_concepts_02_unsafe`)
- **第5-7天**: 泛型和 Trait (`rust_concepts_03_generics_traits`)

### 第二周：高级 Rust 概念
- **第8-10天**: 智能指针 (`rust_concepts_04_smart_pointers`)
- **第11-12天**: 高级 Trait (`rust_concepts_05_advanced_traits`)
- **第13-14天**: 宏编程 (`rust_concepts_06_macros`)

### 第三周：性能优化
- **第15-17天**: 性能优化技巧 (`rust_concepts_07_performance`)
- **第18-21天**: 综合实践和项目应用

## 🔧 环境要求

- Rust 1.87.0 或更高版本
- 已克隆的 Oxc 仓库

## 📊 性能测试建议

为了获得最佳的性能测试结果：

1. **使用 release 模式**:
   ```bash
   cargo run --bin 02_performance_comparison --release
   ```

2. **关闭其他程序**: 减少系统负载影响

3. **多次运行**: 取平均值以获得更准确的结果

4. **观察趋势**: 重点关注相对性能提升，而不是绝对数值

## 🐛 常见问题

### Q: 编译错误怎么办？
A: 确保你在正确的目录中，并且 Oxc 项目已经成功构建：
```bash
cd /path/to/oxc/learn_docs/examples
cargo check
```

### Q: 性能差异不明显？
A: 尝试使用 `--release` 模式，并确保测试规模足够大。

### Q: 想要修改示例代码？
A: 完全可以！这些示例就是为了让你实验和学习。

## 📝 练习建议

1. **修改参数**: 改变分配数量、对象大小等参数，观察性能变化

2. **添加新测试**: 基于现有示例，添加你感兴趣的测试场景

3. **性能分析**: 使用 `cargo flamegraph` 等工具进行更深入的性能分析

4. **内存分析**: 使用 `valgrind` 或其他内存分析工具

## 🎓 进阶学习

完成这些示例后，你可以：

1. 阅读 `oxc_parser` 源码，看看实际的 AST 构建
2. 学习 `oxc_semantic` 模块，了解语义分析中的内存使用
3. 研究其他使用 Arena 分配器的项目

## 🤝 贡献

如果你发现示例中的问题或有改进建议，欢迎提交 PR 或 Issue！

---

**记住**: Arena 分配器是 Oxc 高性能的核心秘诀。通过这些示例，你将深入理解现代编译器的内存管理策略！
