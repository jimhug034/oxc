# Rust 零成本抽象模式演示

本目录包含了学习 Rust 高级特性的演示代码，重点展示 **枚举 + 模式匹配** 实现的零成本抽象。

## 📁 文件说明

### 1. `simple_demo.rs` - 最简演示
**推荐先看这个！**

最简单的例子，展示核心概念：
- 传统 OOP 方式（动态分发）
- Oxc 方式（静态分发）
- 性能对比

**运行方式：**
```bash
rustc simple_demo.rs && ./simple_demo
# 或者
cargo run --bin simple_demo
```

### 2. `rust_patterns_demo.rs` - 完整演示
更详细的例子，包含：
- 动物系统示例
- Lint 规则系统示例
- 手动模拟宏的工作流程

**运行方式：**
```bash
rustc rust_patterns_demo.rs && ./rust_patterns_demo
```

### 3. `macro_example/` - Proc Macro 示例
展示了如何创建一个 proc macro 来自动生成代码。

这是一个简化的版本，演示了：
- 如何解析宏输入
- 如何生成枚举和方法
- 如何使用 `syn` 和 `quote` 库

## 🎯 核心概念

### 问题：传统 OOP 的性能问题

```rust
// 传统方式：使用 trait 对象
fn process(handler: &dyn Handler) {
    handler.run();  // ❌ 运行时查找 vtable，有性能开销
}
```

**问题：**
- 需要虚函数表（vtable）
- 运行时查找和间接调用
- 编译器难以优化
- Rust 的 trait 对象有很多限制

### 解决方案：枚举 + 模式匹配

```rust
// Oxc 方式：使用枚举
enum HandlerEnum {
    TypeA(TypeA),
    TypeB(TypeB),
}

impl HandlerEnum {
    fn run(&self) {
        match self {
            Self::TypeA(h) => h.run(),  // ✅ 编译期直接内联
            Self::TypeB(h) => h.run(),  // ✅ 零开销
        }
    }
}
```

**优势：**
- ✅ 无运行时开销
- ✅ 编译器可以完全优化
- ✅ 类型安全，编译期保证
- ✅ 绕过对象安全性限制

## 🔍 关键原理

### 1. 静态分发 vs 动态分发

| 特性 | 动态分发（trait 对象） | 静态分发（枚举） |
|------|---------------------|----------------|
| 调用方式 | 通过 vtable 间接调用 | 直接函数调用 |
| 编译器优化 | 受限 | 完全优化 |
| 运行时开销 | 有 | 无 |
| 类型信息 | 丢失 | 完整保留 |

### 2. 编译期代码生成

```
输入（宏调用）:
declare_rules! {
    animal::dog,
    animal::cat,
}

      ↓ [proc macro 处理]

输出（生成的代码）:
pub enum RuleEnum {
    AnimalDog(Dog),
    AnimalCat(Cat),
}

impl RuleEnum {
    fn run(&self) {
        match self {
            Self::AnimalDog(d) => d.run(),
            Self::AnimalCat(c) => c.run(),
        }
    }
}
```

### 3. 匹配性能

Rust 编译器会将 match 优化为：
- 跳转表（jump table）
- 直接比较
- 完全内联（如果可以）

## 💡 使用场景

### ✅ 适合使用枚举方式的场景

1. **已知的、有限数量的类型**
   - 如：规则系统、插件系统
   - Oxc 有 600+ 条规则，但数量是固定的

2. **需要最高性能**
   - 如：编译器和 linter
   - 频繁调用，性能敏感

3. **需要绕过对象安全性限制**
   - trait 对象无法表达某些特性
   - 枚举可以包含任意类型

### ❌ 不适合使用枚举方式的场景

1. **类型数量不确定或太多**
   - 如：UI 组件系统
   - 动态加载的插件

2. **需要动态添加新类型**
   - 运行时才知道有哪些类型

## 🚀 进阶学习

### 相关 Rust 特性

1. **Trait 对象**
   - `&dyn Trait`
   - 动态分发机制

2. **模式匹配**
   - `match` 表达式
   - Exhaustiveness 检查

3. **过程宏**
   - `#[proc_macro]`
   - `syn` 和 `quote` 库

4. **泛型**
   - 零成本抽象的核心

### 推荐阅读

- [Rust Book: Traits](https://doc.rust-lang.org/book/ch10-02-traits.html)
- [Rust Book: Pattern Matching](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Rust Book: Macros](https://doc.rust-lang.org/book/ch19-06-macros.html)
- [The Little Book of Rust Macros](https://veykril.github.io/tlborm/)

## 📊 性能测试

运行这些示例时，可以观察：

1. **编译时的优化**
   - 编译器会内联 match 分支
   - 直接函数调用

2. **运行时效率**
   - 无间接调用
   - 无 vtable 查找
   - 缓存友好

## 🤔 思考题

1. 为什么 Rust 编译器可以优化 match 表达式？
2. trait 对象和枚举各有什么限制？
3. 什么时候应该用 trait 对象，什么时候用枚举？
4. proc macro 是如何工作的？

## 📝 实践建议

1. **运行示例**
   ```bash
   cd learn_demo
   rustc simple_demo.rs && ./simple_demo
   ```

2. **修改代码**
   - 添加新的处理器类型
   - 修改 match 表达式
   - 观察编译输出

3. **性能测试**
   - 对比两种方式的性能
   - 使用 `cargo bench` 或 `criterion`

4. **理解 Oxc 源码**
   - 查看 `crates/oxc_macros/src/declare_all_lint_rules.rs`
   - 跟踪代码生成流程

## 🎓 总结

这种模式的核心思想是：
> **在编译期做尽可能多的工作，在运行时做尽可能少的工作**

通过枚举 + 模式匹配 + proc macro，我们实现了：
- 🚀 零成本抽象
- 🛡️ 类型安全
- ⚡ 极致性能
- 🔧 易于扩展

这正是 Rust "零成本抽象"哲学的完美体现！

