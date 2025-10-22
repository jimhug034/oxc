# Oxc 学习第一周：从 oxc_allocator 开始

## 为什么从 oxc_allocator 开始？

作为前端工程师，你可能习惯了 JavaScript 的垃圾回收机制。但 Oxc 的高性能秘诀就在于其独特的内存管理策略 - Arena 分配器。

### 🎯 学习目标

- 理解 Arena 分配器的工作原理
- 掌握 Oxc 内存管理的核心概念
- 体验性能优势的根本原因
- 为后续模块学习打下基础

## Day 1: 理论基础与环境准备

### 上午：概念理解

1. **传统内存分配 vs Arena 分配**
   ```rust
   // 传统方式 - 每次都要向系统申请内存
   let node1 = Box::new(AstNode::new());
   let node2 = Box::new(AstNode::new());
   // ... 成千上万个节点，每个都是独立的堆分配

   // Arena 方式 - 一次申请大块内存，然后快速分配
   let allocator = Allocator::default();
   let node1 = allocator.alloc(AstNode::new());
   let node2 = allocator.alloc(AstNode::new());
   // ... 所有节点都在同一个内存区域，分配极快
   ```

2. **为什么 AST 适合 Arena 分配？**
   - AST 节点生命周期相同（解析完成后一起释放）
   - 大量小对象分配
   - 很少修改，主要是读取和遍历

### 下午：代码实践

1. **创建第一个 allocator 程序**
   ```bash
   cd /Users/makeblock/Developer/my-git/oxc
   cargo new --bin learn_allocator
   cd learn_allocator
   ```

2. **添加依赖到 Cargo.toml**
   ```toml
   [dependencies]
   oxc_allocator = { path = "../crates/oxc_allocator" }
   ```

3. **编写基础示例**
   ```rust
   use oxc_allocator::Allocator;
   use std::time::Instant;

   fn main() {
       // 基础使用
       let allocator = Allocator::default();

       // 分配简单数据
       let number = allocator.alloc(42);
       let text = allocator.alloc_str("Hello Oxc!");

       println!("分配的数字: {}", number);
       println!("分配的文本: {}", text);

       // 性能对比测试
       benchmark_allocation();
   }

   fn benchmark_allocation() {
       const COUNT: usize = 100_000;

       // Arena 分配测试
       let start = Instant::now();
       let allocator = Allocator::default();
       for i in 0..COUNT {
           let _data = allocator.alloc(i);
       }
       let arena_time = start.elapsed();

       // 标准分配测试
       let start = Instant::now();
       let mut vec = Vec::new();
       for i in 0..COUNT {
           vec.push(Box::new(i));
       }
       let std_time = start.elapsed();

       println!("Arena 分配耗时: {:?}", arena_time);
       println!("标准分配耗时: {:?}", std_time);
       println!("性能提升: {:.2}x", std_time.as_nanos() as f64 / arena_time.as_nanos() as f64);
   }
   ```

## Day 2: 深入 Arena 机制

### 上午：源码阅读

1. **阅读核心文件**
   - `crates/oxc_allocator/src/allocator.rs` - 主要实现
   - `crates/oxc_allocator/src/alloc.rs` - 分配接口
   - `crates/oxc_allocator/README.md` - 架构说明

2. **理解关键概念**
   - Bump 指针分配
   - 内存块（Chunk）管理
   - 生命周期绑定

### 下午：高级特性探索

1. **AllocatorPool 的使用**
   ```rust
   use oxc_allocator::{Allocator, AllocatorPool};
   use std::sync::Arc;
   use std::thread;

   fn main() {
       // 创建分配器池
       let pool = Arc::new(AllocatorPool::new());

       // 多线程使用
       let handles: Vec<_> = (0..4).map(|thread_id| {
           let pool = Arc::clone(&pool);
           thread::spawn(move || {
               // 从池中获取分配器
               let allocator = pool.get();

               // 使用分配器
               for i in 0..1000 {
                   let data = allocator.alloc(format!("Thread {} - Item {}", thread_id, i));
                   println!("{}", data);
               }

               // 分配器自动归还到池中
           })
       }).collect();

       for handle in handles {
           handle.join().unwrap();
       }
   }
   ```

2. **自定义数据结构**
   ```rust
   use oxc_allocator::{Allocator, Vec as ArenaVec, Box as ArenaBox};

   #[derive(Debug)]
   struct AstNode<'a> {
       name: &'a str,
       children: ArenaVec<'a, ArenaBox<'a, AstNode<'a>>>,
   }

   impl<'a> AstNode<'a> {
       fn new_in(allocator: &'a Allocator, name: &'a str) -> ArenaBox<'a, Self> {
           allocator.alloc(AstNode {
               name,
               children: ArenaVec::new_in(allocator),
           })
       }

       fn add_child(&mut self, child: ArenaBox<'a, AstNode<'a>>) {
           self.children.push(child);
       }
   }

   fn main() {
       let allocator = Allocator::default();

       // 创建 AST 树
       let mut root = AstNode::new_in(&allocator, "root");
       let child1 = AstNode::new_in(&allocator, "child1");
       let child2 = AstNode::new_in(&allocator, "child2");

       root.add_child(child1);
       root.add_child(child2);

       println!("AST 树: {:#?}", root);
   }
   ```

## Day 3-4: 与其他模块的集成

### 理解 allocator 在整个 Oxc 生态中的作用

1. **查看其他模块如何使用 allocator**
   ```bash
   # 搜索 allocator 的使用
   rg "Allocator" crates/oxc_parser/src/ -A 3 -B 3
   rg "alloc\(" crates/oxc_ast/src/ -A 2
   ```

2. **实践：模拟 Parser 的内存使用模式**
   ```rust
   use oxc_allocator::Allocator;

   // 模拟 Token 结构
   #[derive(Debug)]
   struct Token<'a> {
       kind: TokenKind,
       value: &'a str,
       span: (usize, usize),
   }

   #[derive(Debug)]
   enum TokenKind {
       Identifier,
       Number,
       String,
       Keyword,
   }

   // 模拟解析过程
   fn simulate_parsing<'a>(allocator: &'a Allocator, source: &'a str) -> Vec<&'a Token<'a>> {
       let mut tokens = Vec::new();

       // 模拟词法分析
       let words: Vec<&str> = source.split_whitespace().collect();
       for (i, word) in words.iter().enumerate() {
           let token = allocator.alloc(Token {
               kind: if word.chars().all(|c| c.is_numeric()) {
                   TokenKind::Number
               } else {
                   TokenKind::Identifier
               },
               value: word,
               span: (i * 10, i * 10 + word.len()),
           });
           tokens.push(token);
       }

       tokens
   }

   fn main() {
       let allocator = Allocator::default();
       let source = "function add x y return x + y end";

       let tokens = simulate_parsing(&allocator, source);

       for token in tokens {
           println!("{:?}", token);
       }
   }
   ```

## Day 5-7: 性能分析与优化

### 深入性能分析

1. **内存使用分析**
   ```rust
   use oxc_allocator::Allocator;
   use std::mem;

   fn analyze_memory_usage() {
       let allocator = Allocator::default();

       println!("Allocator 大小: {} bytes", mem::size_of::<Allocator>());

       // 分配不同大小的数据
       let small_data = allocator.alloc([0u8; 16]);
       let medium_data = allocator.alloc([0u8; 1024]);
       let large_data = allocator.alloc([0u8; 65536]);

       println!("小数据地址: {:p}", small_data);
       println!("中数据地址: {:p}", medium_data);
       println!("大数据地址: {:p}", large_data);

       // 观察内存布局
       let addr_diff = medium_data.as_ptr() as usize - small_data.as_ptr() as usize;
       println!("地址差: {} bytes", addr_diff);
   }
   ```

2. **与真实 Parser 的性能对比**
   ```bash
   # 运行 oxc parser 示例
   cargo run -p oxc_parser --example parser -- test.js

   # 查看内存使用
   cargo run --example memory_analysis
   ```

## 实践作业

### 作业 1: 实现一个简单的表达式树

```rust
// 使用 Arena 分配器实现一个数学表达式解析器
// 支持 +, -, *, / 和括号
// 例如: "2 + 3 * (4 - 1)" -> AST
```

### 作业 2: 性能基准测试

```rust
// 对比不同分配策略的性能
// 1. 标准 Box/Vec
// 2. Arena 分配器
// 3. 预分配容量的 Vec
// 测试场景：创建 10万个 AST 节点
```

### 作业 3: 内存池优化

```rust
// 实现一个自定义的内存池
// 支持不同大小的对象分配
// 比较与 AllocatorPool 的性能差异
```

## 检查点

完成第一周学习后，你应该能够：

- [ ] 解释 Arena 分配器的工作原理
- [ ] 理解为什么 Oxc 比其他工具快
- [ ] 使用 Allocator 创建自定义数据结构
- [ ] 分析内存使用模式和性能特征
- [ ] 为学习其他模块打下坚实基础

## 下周预告

第二周我们将学习 `oxc_ast` 和 `oxc_parser`，你会看到 allocator 如何在实际的 AST 构建中发挥作用。

---

**记住**：allocator 是 Oxc 性能优势的核心，理解它就理解了 Oxc 为什么能够比 JavaScript 工具快几十倍！
