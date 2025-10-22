# 第一周 Day 5-7: 深入 oxc_ast 与 Rust 进阶

> 理解 AST 节点定义、Rust 高级特性，掌握 AST 操作的核心技能

## 📖 学习目标

- [ ] 理解 oxc_ast 的整体架构
- [ ] 掌握 AST 节点的 Rust 定义方式
- [ ] 学习 Rust 生命周期和智能指针
- [ ] 理解 Arena 分配器的作用
- [ ] 能够读懂和操作 AST 节点
- [ ] 掌握 Rust trait 系统的应用

## 🎯 为什么要深入学习 oxc_ast？

### oxc_ast 在 Oxc 中的核心地位

```
源代码 → Parser → AST ← Linter
                  ↓
              Formatter
                  ↓
            Transformer
                  ↓
              Minifier
                  ↓
              Codegen
```

所有工具都依赖 AST，理解它是理解整个 Oxc 的关键！

## 🚀 快速开始

### 1. 查看 AST 结构

```bash
# 打开核心 AST 定义文件
code crates/oxc_ast/src/ast/js.rs

# 查看 AST 模块结构
ls -la crates/oxc_ast/src/ast/
```

### 2. 运行 AST 示例

```bash
# 运行我们的 AST 学习示例
cargo run --bin 05_ast_deep_dive

# 查看不同节点的结构
cargo run -p oxc_parser --example parser -- test.js
```

---

## 📚 Day 5: AST 节点定义与 Rust 基础

### 核心文件导览

```
crates/oxc_ast/src/
├── ast/
│   ├── js.rs          # JavaScript AST 节点 ⭐ 最重要
│   ├── ts.rs          # TypeScript 扩展节点
│   ├── jsx.rs         # JSX 节点
│   ├── literal.rs     # 字面量节点
│   └── macros.rs      # AST 宏定义
├── ast_builder.rs     # AST 构建工具
├── visit.rs           # 访问者模式
└── span.rs            # 位置信息
```

### 理解 AST 节点定义

打开 `crates/oxc_ast/src/ast/js.rs`，让我们逐步理解：

#### 1. Program 节点 - AST 的根

```rust
#[ast(visit)]
#[scope(
    flags = ScopeFlags::Top,
    strict_if = self.source_type.is_strict() || self.has_use_strict_directive(),
)]
#[derive(Debug)]
#[generate_derive(CloneIn, Dummy, TakeIn, GetSpan, GetSpanMut, ContentEq, ESTree)]
pub struct Program<'a> {
    pub span: Span,                       // 位置信息
    pub source_type: SourceType,          // 文件类型
    pub source_text: &'a str,             // 源代码 (生命周期标记)
    pub comments: Vec<'a, Comment>,       // 注释
    pub hashbang: Option<Hashbang<'a>>,   // shebang
    pub directives: Vec<'a, Directive<'a>>, // 指令
    pub body: Vec<'a, Statement<'a>>,     // 主体语句
    pub scope_id: Cell<Option<ScopeId>>,  // 作用域 ID
}
```

**🔑 关键 Rust 知识点**：

##### 生命周期 `'a`

```rust
// 为什么需要生命周期？
pub struct Program<'a> {
    pub source_text: &'a str,  // 借用源代码，不拥有
}

// 这表示：Program 的生命周期不能超过 source_text 的生命周期
// 避免了内存拷贝，提升性能！
```

##### 自定义类型

```rust
// Vec<'a, T> 不是标准库的 Vec！
// 这是 oxc_allocator 提供的 Arena 版本
use oxc_allocator::Vec;

// Span 存储位置信息
pub struct Span {
    pub start: u32,  // 起始位置
    pub end: u32,    // 结束位置
}

// Cell 允许内部可变性
use std::cell::Cell;
pub scope_id: Cell<Option<ScopeId>>,
```

##### 属性宏

```rust
#[ast(visit)]                    // 标记为可访问的 AST 节点
#[scope(...)]                    // 定义作用域规则
#[derive(Debug)]                 // 自动实现 Debug trait
#[generate_derive(CloneIn, ...)] // 自定义派生宏
```

---

#### 2. Expression 节点 - 表达式的核心

在 `js.rs` 的 54-153 行：

```rust
#[ast(visit)]
#[derive(Debug)]
#[generate_derive(CloneIn, Dummy, TakeIn, GetSpan, GetSpanMut, GetAddress, ContentEq, ESTree)]
pub enum Expression<'a> {
    // 字面量
    BooleanLiteral(Box<'a, BooleanLiteral>) = 0,
    NullLiteral(Box<'a, NullLiteral>) = 1,
    NumericLiteral(Box<'a, NumericLiteral<'a>>) = 2,
    StringLiteral(Box<'a, StringLiteral<'a>>) = 5,

    // 标识符
    Identifier(Box<'a, IdentifierReference<'a>>) = 7,

    // 复杂表达式
    ArrayExpression(Box<'a, ArrayExpression<'a>>) = 10,
    BinaryExpression(Box<'a, BinaryExpression<'a>>) = 14,
    CallExpression(Box<'a, CallExpression<'a>>) = 15,

    // ... 更多变体

    // 继承 MemberExpression 的变体
    @inherit MemberExpression
}
```

**🔑 关键 Rust 知识点**：

##### Enum（枚举）

```rust
// Rust 的 enum 非常强大，可以携带数据
pub enum Expression<'a> {
    NumericLiteral(Box<'a, NumericLiteral<'a>>),  // 携带数据
    BinaryExpression(Box<'a, BinaryExpression<'a>>),
}

// 使用模式匹配访问
match expr {
    Expression::NumericLiteral(lit) => {
        println!("数字: {}", lit.value);
    }
    Expression::BinaryExpression(bin) => {
        println!("操作符: {:?}", bin.operator);
    }
    _ => {}
}
```

##### Box<'a, T> - Arena 分配的智能指针

```rust
// 不是标准库的 Box！
use oxc_allocator::Box;

// 在 Arena 上分配，生命周期为 'a
// 所有 AST 节点共享同一个 allocator
// 可以一次性释放所有节点，非常高效！
```

##### Variant 编号

```rust
pub enum Expression<'a> {
    BooleanLiteral(...) = 0,  // 显式指定编号
    NullLiteral(...) = 1,
    // 编号用于序列化和稳定性
}
```

---

#### 3. Statement 节点 - 语句

在 `js.rs` 的 1066-1100 行：

```rust
pub enum Statement<'a> {
    BlockStatement(Box<'a, BlockStatement<'a>>) = 0,
    BreakStatement(Box<'a, BreakStatement<'a>>) = 1,
    ContinueStatement(Box<'a, ContinueStatement<'a>>) = 2,
    // ...

    // 继承 Declaration 的变体
    @inherit Declaration
    // 继承 ModuleDeclaration 的变体
    @inherit ModuleDeclaration
}
```

**注意 `@inherit` 宏**：这是 Oxc 的特殊语法糖，用于共享变体。

---

#### 4. 具体节点示例

##### BinaryExpression - 二元表达式

```rust
// 在 js.rs 第 694-705 行
#[ast(visit)]
#[derive(Debug)]
#[generate_derive(CloneIn, Dummy, TakeIn, GetSpan, GetSpanMut, ContentEq, ESTree)]
pub struct BinaryExpression<'a> {
    pub span: Span,
    pub left: Expression<'a>,           // 左操作数
    pub operator: BinaryOperator,       // 操作符
    pub right: Expression<'a>,          // 右操作数
}
```

对应 JS 代码：

```javascript
1 + 2
// ↓ 解析为
BinaryExpression {
    left: NumericLiteral(1),
    operator: Add,
    right: NumericLiteral(2),
}
```

##### Function - 函数

```rust
// 在 js.rs 第 1686-1808 行
pub struct Function<'a> {
    pub span: Span,
    pub r#type: FunctionType,                              // function vs expression
    pub id: Option<BindingIdentifier<'a>>,                 // 函数名
    pub generator: bool,                                    // 是否生成器
    pub r#async: bool,                                      // 是否异步
    pub params: Box<'a, FormalParameters<'a>>,            // 参数
    pub body: Option<Box<'a, FunctionBody<'a>>>,          // 函数体
    pub scope_id: Cell<Option<ScopeId>>,                   // 作用域
    // TypeScript 特性
    pub type_parameters: Option<Box<'a, TSTypeParameterDeclaration<'a>>>,
    pub return_type: Option<Box<'a, TSTypeAnnotation<'a>>>,
}
```

**🔑 关键 Rust 知识点**：

```rust
// r#async, r#type - 原始标识符
// async 和 type 是 Rust 关键字，加 r# 前缀可以作为标识符

pub r#async: bool,  // 字段名是 async
pub r#type: FunctionType,  // 字段名是 type
```

---

### 🔬 实践练习 1: 查找节点定义

在 `js.rs` 中找到以下节点的定义，理解它们的结构：

1. **ArrayExpression** (数组表达式)
   - 位置: 第 311-320 行
   - 思考: 为什么 `elements` 是 `Vec<'a, ArrayExpressionElement<'a>>`？

2. **CallExpression** (函数调用)
   - 位置: 第 566-596 行
   - 思考: `callee` 和 `arguments` 分别是什么类型？

3. **IfStatement** (if 语句)
   - 位置: 第 1239-1248 行
   - 思考: `alternate` 为什么是 `Option<Statement<'a>>`？

4. **VariableDeclaration** (变量声明)
   - 位置: 第 1174-1186 行
   - 思考: `kind` 字段有哪些可能的值？

### 📝 学习笔记 Day 5

#### 今天理解的核心概念：

-
-
-

#### Rust 新知识点：

- 生命周期:
- 智能指针:
- 枚举类型:

#### 遇到的困惑：

---

## 📚 Day 6: AST 操作与 Visitor 模式

### 理解 Visitor 模式在 AST 中的应用

#### Visitor Trait 定义

```rust
// crates/oxc_ast/src/visit.rs (简化版)
pub trait Visit<'a> {
    fn visit_program(&mut self, program: &Program<'a>) {
        walk_program(self, program);
    }

    fn visit_statement(&mut self, stmt: &Statement<'a>) {
        walk_statement(self, stmt);
    }

    fn visit_expression(&mut self, expr: &Expression<'a>) {
        walk_expression(self, expr);
    }

    // 为每种节点类型提供 visit 方法
    fn visit_binary_expression(&mut self, expr: &BinaryExpression<'a>) {
        walk_binary_expression(self, expr);
    }

    fn visit_function(&mut self, func: &Function<'a>) {
        walk_function(self, func);
    }
}
```

#### 实现自己的 Visitor

**示例 1: 统计表达式数量**

```rust
use oxc_ast::ast::*;
use oxc_ast::visit::{Visit, walk_program};

struct ExpressionCounter {
    count: usize,
}

impl<'a> Visit<'a> for ExpressionCounter {
    fn visit_expression(&mut self, _expr: &Expression<'a>) {
        self.count += 1;
        // 注意：不调用 walk_expression，避免重复计数
    }
}

// 使用
let mut counter = ExpressionCounter { count: 0 };
counter.visit_program(&program);
println!("表达式总数: {}", counter.count);
```

**示例 2: 收集所有函数名**

```rust
struct FunctionCollector<'a> {
    functions: Vec<String>,
}

impl<'a> Visit<'a> for FunctionCollector<'a> {
    fn visit_function(&mut self, func: &Function<'a>) {
        // 收集函数名
        if let Some(id) = &func.id {
            self.functions.push(id.name.to_string());
        }

        // 继续遍历子节点
        walk_function(self, func);
    }
}
```

**示例 3: 查找所有 console.log 调用**

```rust
struct ConsoleLogFinder<'a> {
    locations: Vec<Span>,
}

impl<'a> Visit<'a> for ConsoleLogFinder<'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        // 检查是否是 console.log
        if let Expression::StaticMemberExpression(member) = &call.callee {
            if let Expression::Identifier(obj) = &member.object {
                if obj.name == "console" && member.property.name == "log" {
                    self.locations.push(call.span);
                }
            }
        }

        walk_call_expression(self, call);
    }
}
```

---

### 理解 AST 遍历的两种模式

#### 模式 1: Pre-order（先序遍历）

```rust
impl<'a> Visit<'a> for MyVisitor {
    fn visit_function(&mut self, func: &Function<'a>) {
        // 1. 先处理当前节点
        println!("进入函数: {:?}", func.id);

        // 2. 然后遍历子节点
        walk_function(self, func);

        // 3. 最后是离开节点时的处理
        println!("离开函数: {:?}", func.id);
    }
}
```

#### 模式 2: 条件遍历

```rust
impl<'a> Visit<'a> for MyVisitor {
    fn visit_function(&mut self, func: &Function<'a>) {
        // 只处理异步函数
        if func.r#async {
            println!("找到异步函数");
            walk_function(self, func);
        }
        // 不调用 walk，跳过同步函数的遍历
    }
}
```

---

### 🔬 实践练习 2: 实现自定义 Visitor

#### 练习 2.1: 变量声明分析器

实现一个 Visitor，统计代码中：

- `const` 声明的数量
- `let` 声明的数量
- `var` 声明的数量

<details>
<summary>点击查看实现提示</summary>

```rust
struct VariableAnalyzer {
    const_count: usize,
    let_count: usize,
    var_count: usize,
}

impl<'a> Visit<'a> for VariableAnalyzer {
    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration<'a>) {
        match decl.kind {
            VariableDeclarationKind::Const => self.const_count += 1,
            VariableDeclarationKind::Let => self.let_count += 1,
            VariableDeclarationKind::Var => self.var_count += 1,
            _ => {}
        }
        walk_variable_declaration(self, decl);
    }
}
```

</details>

#### 练习 2.2: 函数复杂度分析器

计算函数的循环复杂度（Cyclomatic Complexity）：

- 基础复杂度: 1
- 每个 if/for/while: +1
- 每个 case: +1

<details>
<summary>点击查看实现框架</summary>

```rust
struct ComplexityAnalyzer {
    current_function: Option<String>,
    complexity_map: HashMap<String, usize>,
    current_complexity: usize,
}

impl<'a> Visit<'a> for ComplexityAnalyzer {
    fn visit_function(&mut self, func: &Function<'a>) {
        // 1. 保存当前状态
        // 2. 重置复杂度为 1
        // 3. 遍历函数体
        // 4. 保存结果
        // 5. 恢复状态
    }

    fn visit_if_statement(&mut self, _: &IfStatement<'a>) {
        self.current_complexity += 1;
        // walk...
    }

    fn visit_for_statement(&mut self, _: &ForStatement<'a>) {
        self.current_complexity += 1;
        // walk...
    }
}
```

</details>

#### 练习 2.3: 依赖分析器

找出所有的 `import` 语句，提取导入的模块名：

<details>
<summary>点击查看实现提示</summary>

```rust
struct ImportAnalyzer<'a> {
    imports: Vec<(String, Span)>,  // (模块名, 位置)
}

impl<'a> Visit<'a> for ImportAnalyzer<'a> {
    fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
        let module_name = import.source.value.to_string();
        self.imports.push((module_name, import.span));
        walk_import_declaration(self, import);
    }
}
```

</details>

---

### 📝 学习笔记 Day 6

#### Visitor 模式的关键点：

-
-

#### 实现的练习：

- [ ] 练习 2.1: 变量声明分析器
- [ ] 练习 2.2: 函数复杂度分析器
- [ ] 练习 2.3: 依赖分析器

#### 遇到的挑战：

---

## 📚 Day 7: Arena 分配器与内存管理

### 为什么需要 Arena Allocator？

#### 传统堆分配的问题

```rust
// 标准 Box/Vec 的问题
let mut nodes = Vec::new();
for i in 0..10000 {
    nodes.push(Box::new(AstNode { /* ... */ }));
}
// 每个 Box 都是独立分配，释放时需要逐个 drop
// 大量 AST 节点会导致内存碎片和性能问题
```

#### Arena 分配器的优势

```rust
// Oxc 的方式
let allocator = Allocator::default();
let mut nodes = Vec::new_in(&allocator);
for i in 0..10000 {
    nodes.push(allocator.alloc(AstNode { /* ... */ }));
}
// 所有节点在同一个 Arena 中分配
// 释放 allocator 时，一次性释放所有内存！
```

**优势**：

1. **快速分配**: 几乎零开销的分配
2. **缓存友好**: 节点内存连续，提升 CPU 缓存命中率
3. **简单释放**: 一次性释放所有节点

---

### oxc_allocator 的使用

#### 基础使用

```rust
use oxc_allocator::{Allocator, Box, Vec};

// 创建 allocator
let allocator = Allocator::default();

// 分配单个对象
let node = allocator.alloc(MyStruct { x: 10 });
//         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//         返回 &mut MyStruct，生命周期与 allocator 绑定

// 分配 Box
let boxed = Box::new_in(MyStruct { x: 20 }, &allocator);

// 分配 Vec
let mut vec = Vec::new_in(&allocator);
vec.push(item1);
vec.push(item2);
```

#### 生命周期约束

```rust
fn parse<'a>(allocator: &'a Allocator) -> Program<'a> {
    // Program 的生命周期 'a 与 allocator 绑定
    // 确保 Program 不会超过 allocator 的生命周期
    let body = Vec::new_in(allocator);
    Program {
        body,
        // ...
    }
}
```

**关键理解**：

- `'a` 是生命周期参数
- 所有 AST 节点共享同一个生命周期
- 节点不能比 allocator 活得更久

---

### AST 构建器 (ASTBuilder)

Oxc 提供了 `ASTBuilder` 工具，简化 AST 节点的创建：

```rust
use oxc_ast::AstBuilder;
use oxc_allocator::Allocator;

let allocator = Allocator::default();
let ast = AstBuilder::new(&allocator);

// 创建字面量
let num = ast.numeric_literal(SPAN, 42.0, "42", NumberBase::Decimal);
let str = ast.string_literal(SPAN, "hello");

// 创建标识符
let ident = ast.identifier_reference(SPAN, "foo");

// 创建二元表达式
let expr = ast.binary_expression(
    SPAN,
    left,
    BinaryOperator::Addition,
    right,
);

// 创建变量声明
let decl = ast.variable_declaration(
    SPAN,
    VariableDeclarationKind::Const,
    declarations,
    false,  // declare
);
```

---

### 🔬 实践练习 3: 手动构建 AST

#### 练习 3.1: 构建简单表达式

目标：手动构建 `1 + 2`

```rust
use oxc_allocator::Allocator;
use oxc_ast::AstBuilder;
use oxc_span::SPAN;

fn build_one_plus_two() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);

    // 1. 创建左操作数: 1
    let left = ast.expression_numeric_literal(SPAN, 1.0, "1", NumberBase::Decimal);

    // 2. 创建右操作数: 2
    let right = ast.expression_numeric_literal(SPAN, 2.0, "2", NumberBase::Decimal);

    // 3. 创建二元表达式
    let expr = ast.expression_binary(
        SPAN,
        left,
        BinaryOperator::Addition,
        right,
    );

    // 现在 expr 就是 1 + 2 的 AST 表示
    println!("{:?}", expr);
}
```

#### 练习 3.2: 构建变量声明

目标：手动构建 `const x = 10;`

<details>
<summary>点击查看实现框架</summary>

```rust
fn build_const_x() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);

    // 1. 创建标识符 "x"
    let id = ast.binding_identifier(SPAN, "x");

    // 2. 创建绑定模式
    let pattern = ast.binding_pattern(
        ast.binding_pattern_kind_binding_identifier(SPAN, id),
        None,  // type_annotation
        false, // optional
    );

    // 3. 创建初始值: 10
    let init = ast.expression_numeric_literal(SPAN, 10.0, "10", NumberBase::Decimal);

    // 4. 创建声明器
    let declarator = ast.variable_declarator(
        SPAN,
        VariableDeclarationKind::Const,
        pattern,
        Some(init),
        false, // definite
    );

    // 5. 创建声明语句
    let decl = ast.variable_declaration(
        SPAN,
        VariableDeclarationKind::Const,
        ast.vec1(declarator),
        false, // declare
    );

    println!("{:?}", decl);
}
```

</details>

#### 练习 3.3: 构建函数声明

目标：手动构建 `function greet(name) { return "Hello"; }`

<details>
<summary>点击查看实现框架</summary>

```rust
fn build_function() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);

    // 1. 函数名
    let id = ast.binding_identifier(SPAN, "greet");

    // 2. 参数
    let param_id = ast.binding_identifier(SPAN, "name");
    let param_pattern = ast.binding_pattern(
        ast.binding_pattern_kind_binding_identifier(SPAN, param_id),
        None, false,
    );
    let param = ast.formal_parameter(SPAN, ast.vec(), param_pattern, None, false, false);
    let params = ast.alloc_formal_parameters(
        SPAN,
        FormalParameterKind::FormalParameter,
        ast.vec1(param),
        None,
    );

    // 3. 函数体
    let return_value = ast.expression_string_literal(SPAN, "Hello");
    let return_stmt = ast.statement_return(SPAN, Some(return_value));
    let body = ast.alloc_function_body(SPAN, ast.vec(), ast.vec1(return_stmt));

    // 4. 创建函数
    let func = ast.alloc_function(
        FunctionType::FunctionDeclaration,
        SPAN,
        Some(id),
        false, // generator
        false, // async
        false, // declare
        None,  // type_parameters
        None,  // this_param
        params,
        None,  // return_type
        Some(body),
    );

    let decl = ast.statement_declaration(
        ast.declaration_function(func)
    );

    println!("{:?}", decl);
}
```

</details>

---

### 理解内存布局

```
┌─────────────────────────────────────────┐
│         Allocator (Arena)                │
├─────────────────────────────────────────┤
│  Program                                 │
│  ├── body: Vec<Statement>               │
│  │   ├── Statement 1                    │
│  │   │   └── Expression                 │
│  │   ├── Statement 2                    │
│  │   │   ├── If condition               │
│  │   │   └── Consequent                 │
│  │   └── Statement 3                    │
│  ├── directives: Vec<Directive>         │
│  └── comments: Vec<Comment>             │
│                                          │
│  [所有节点在连续内存中]                  │
└─────────────────────────────────────────┘

当 Allocator drop 时，所有内存一次性释放
```

---

### 📝 学习笔记 Day 7

#### Arena 分配器的核心优势：

-
-

#### 实现的练习：

- [ ] 练习 3.1: 简单表达式
- [ ] 练习 3.2: 变量声明
- [ ] 练习 3.3: 函数声明

#### 对生命周期的理解：

---

## 🎯 第一周总结：检查点

完成以下任务，检验学习效果：

### AST 理解

- [ ] 能够找到任意 AST 节点的定义
- [ ] 理解节点之间的继承关系 (`@inherit`)
- [ ] 知道 Statement、Expression、Declaration 的区别
- [ ] 能够画出简单代码的 AST 结构图

### Rust 知识

- [ ] 理解生命周期标记 `'a` 的含义
- [ ] 知道 `Box<'a, T>` 和标准 `Box<T>` 的区别
- [ ] 理解 `Cell<T>` 的作用（内部可变性）
- [ ] 能够使用 `match` 模式匹配枚举

### Visitor 模式

- [ ] 能够实现自定义 Visitor
- [ ] 理解 `visit_*` 和 `walk_*` 的区别
- [ ] 会使用 Visitor 统计和收集信息

### 内存管理

- [ ] 理解 Arena 分配器的优势
- [ ] 能够使用 `AstBuilder` 创建节点
- [ ] 理解为什么所有节点共享生命周期

---

## 🔗 相关资源

### 代码位置

- **AST 定义**: `crates/oxc_ast/src/ast/js.rs`
- **Visitor**: `crates/oxc_ast/src/visit.rs`
- **AST Builder**: `crates/oxc_ast/src/ast_builder.rs`
- **Allocator**: `crates/oxc_allocator/src/`

### 文档

- [Rust 生命周期](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)
- [Rust 智能指针](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html)
- [Visitor 模式](https://rust-unofficial.github.io/patterns/patterns/behavioural/visitor.html)

### 在线工具

- [Rust Playground](https://play.rust-lang.org/) - 在线运行 Rust
- [AST Explorer](https://astexplorer.net/) - 查看 AST 结构

---

## 💡 学习技巧

### 1. 对照阅读

同时打开三个文件：

- JS/TS 代码
- AST Explorer 的可视化
- Oxc 的 `js.rs` 定义

对照理解节点结构。

### 2. 画图理解

手绘 AST 树状图，加深理解。

### 3. 动手实践

不要只看代码，一定要：

- 运行示例
- 修改代码
- 实现练习

### 4. 循序渐进

如果某个概念不理解，先跳过，后面会慢慢清晰。

---

## 🎓 进阶方向

完成第一周学习后，你可以选择：

### 方向 1: 深入 Linter 开发

- 学习 Semantic Analysis
- 实现复杂的 Lint 规则
- 理解作用域和符号

### 方向 2: 理解 Parser 实现

- 学习词法分析
- 理解语法分析算法
- 研究错误恢复

### 方向 3: AST 转换

- 学习 Transformer
- 实现代码转换插件
- 理解 Babel 插件

### 方向 4: 内存优化

- 深入 Arena 分配器
- 研究零拷贝设计
- 性能分析和优化

---

## ➡️ 下一步

完成第一周的学习后，继续：

- **第二周**: 核心概念深入（Semantic Analysis、作用域、符号表）
- **第三周**: 选择方向深入学习

---

**学习日期**: ___________
**完成情况**: ⬜ 未开始 / ⬜ 进行中 / ⬜ 已完成

**本周最大收获**:

**下周学习计划**:

---

Good luck! 🚀

记住：学习是一个迭代的过程，第一遍不理解很正常。
多看几遍，多动手实践，知识会慢慢沉淀下来！
