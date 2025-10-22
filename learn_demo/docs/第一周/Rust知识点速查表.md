# Rust 知识点速查表

> 学习 oxc_ast 过程中需要的 Rust 核心概念

## 📚 目录

- [生命周期](#生命周期)
- [所有权与借用](#所有权与借用)
- [智能指针](#智能指针)
- [枚举与模式匹配](#枚举与模式匹配)
- [Trait 系统](#trait-系统)
- [宏与属性](#宏与属性)
- [内部可变性](#内部可变性)

---

## 生命周期

### 基础概念

```rust
// 生命周期 'a 表示一个作用域
// 它确保引用不会超过被引用数据的生存时间

fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
// 返回值的生命周期与输入参数中较短的那个相同
```

### 在 oxc_ast 中的应用

```rust
pub struct Program<'a> {
    //               ^^^^ 生命周期参数
    pub source_text: &'a str,        // 借用源代码
    pub body: Vec<'a, Statement<'a>>, // Arena 分配的 Vec
}

// 所有 AST 节点共享同一个生命周期 'a
// 表示它们都依赖于同一个 Allocator
```

### 关键规则

1. **生命周期省略规则**
   ```rust
   // 可以省略
   fn foo(s: &str) -> &str { s }

   // 完整写法
   fn foo<'a>(s: &'a str) -> &'a str { s }
   ```

2. **多个生命周期**
   ```rust
   fn foo<'a, 'b>(x: &'a str, y: &'b str) -> &'a str {
       x  // 返回值的生命周期与 x 绑定
   }
   ```

3. **结构体生命周期**
   ```rust
   struct Container<'a> {
       data: &'a str,
   }
   // Container 不能比 data 活得更久
   ```

---

## 所有权与借用

### 核心原则

```rust
// 1. 每个值都有一个所有者
let s1 = String::from("hello");  // s1 拥有这个 String

// 2. 值只能有一个所有者
let s2 = s1;  // 所有权转移，s1 不再有效

// 3. 当所有者离开作用域，值被 drop
{
    let s = String::from("hello");
} // s 被 drop
```

### 借用规则

```rust
// 1. 可变借用（独占）
let mut s = String::from("hello");
let r = &mut s;      // OK: 一个可变借用
// let r2 = &mut s;  // 错误: 不能有多个可变借用

// 2. 不可变借用（共享）
let s = String::from("hello");
let r1 = &s;  // OK
let r2 = &s;  // OK: 可以有多个不可变借用

// 3. 不能同时存在可变和不可变借用
let mut s = String::from("hello");
let r1 = &s;
// let r2 = &mut s;  // 错误
```

### 在 AST 中的应用

```rust
// Parser 返回拥有的 Program
let program = parser.parse().program;

// Visitor 借用 AST 节点
impl<'a> Visit<'a> for MyVisitor {
    fn visit_program(&mut self, program: &Program<'a>) {
        //                                 ^ 不可变借用
        // 可以读取但不能修改
    }
}
```

---

## 智能指针

### Box<T> - 堆分配

```rust
// 标准库的 Box
let b = Box::new(5);  // 在堆上分配 5

// 用途：
// 1. 大型数据结构
// 2. 递归类型
// 3. trait 对象
```

### Box<'a, T> - Arena 分配（Oxc）

```rust
use oxc_allocator::{Allocator, Box};

let allocator = Allocator::default();

// Arena 分配的 Box
let b = Box::new_in(MyStruct { x: 10 }, &allocator);

// 特点：
// - 分配在 Arena 上，不是全局堆
// - 生命周期绑定到 allocator
// - 一次性释放所有内存
```

### Vec<'a, T> - Arena 分配的向量（Oxc）

```rust
use oxc_allocator::Vec;

let allocator = Allocator::default();
let mut vec = Vec::new_in(&allocator);

vec.push(item1);
vec.push(item2);

// 不是 std::vec::Vec！
// 在 Arena 上分配，生命周期为 'a
```

### 对比表格

| 类型         | 分配位置 | 释放时机          | 使用场景       |
| ------------ | -------- | ----------------- | -------------- |
| `Box<T>`     | 全局堆   | drop 时           | 通用场景       |
| `Box<'a, T>` | Arena    | allocator drop 时 | AST 节点       |
| `Vec<T>`     | 全局堆   | drop 时           | 通用集合       |
| `Vec<'a, T>` | Arena    | allocator drop 时 | AST 子节点列表 |

---

## 枚举与模式匹配

### 强大的枚举

```rust
// Rust 的枚举可以携带数据
pub enum Expression<'a> {
    NumericLiteral(Box<'a, NumericLiteral<'a>>),
    StringLiteral(Box<'a, StringLiteral<'a>>),
    BinaryExpression(Box<'a, BinaryExpression<'a>>),
    // ... 更多变体
}
```

### 模式匹配

```rust
// 完整匹配
match expr {
    Expression::NumericLiteral(lit) => {
        println!("数字: {}", lit.value);
    }
    Expression::BinaryExpression(bin) => {
        println!("操作符: {:?}", bin.operator);
    }
    _ => {}  // 处理其他情况
}

// if let - 只关心一种情况
if let Expression::NumericLiteral(lit) = expr {
    println!("数字: {}", lit.value);
}

// matches! 宏 - 只判断不提取
if matches!(expr, Expression::NumericLiteral(_)) {
    println!("这是数字字面量");
}
```

### 高级模式

```rust
// 嵌套模式
match statement {
    Statement::VariableDeclaration(decl)
        if decl.kind == VariableDeclarationKind::Const => {
        println!("const 声明");
    }
    _ => {}
}

// 多个模式
match value {
    0 | 1 => println!("0 或 1"),
    2..=10 => println!("2 到 10"),
    _ => println!("其他"),
}
```

---

## Trait 系统

### 什么是 Trait？

Trait 类似于其他语言的接口，定义共享行为。

```rust
// 定义 trait
pub trait Visitor {
    fn visit_node(&mut self, node: &Node);
}

// 实现 trait
struct MyVisitor;

impl Visitor for MyVisitor {
    fn visit_node(&mut self, node: &Node) {
        // 实现
    }
}
```

### 在 oxc_ast 中的应用

```rust
// Visit trait
pub trait Visit<'a> {
    fn visit_program(&mut self, program: &Program<'a>) {
        walk_program(self, program);
    }

    // 为每种 AST 节点提供方法
    fn visit_statement(&mut self, stmt: &Statement<'a>) {
        walk_statement(self, stmt);
    }
}

// 使用时实现 trait
struct MyAnalyzer;

impl<'a> Visit<'a> for MyAnalyzer {
    fn visit_function(&mut self, func: &Function<'a>) {
        // 自定义逻辑
        walk_function(self, func);
    }
}
```

### Trait 边界

```rust
// 泛型约束
fn process<T: Display>(value: T) {
    println!("{}", value);
}

// 多个约束
fn process<T: Display + Debug>(value: T) {
    // ...
}

// where 子句（更清晰）
fn process<T>(value: T)
where
    T: Display + Debug,
{
    // ...
}
```

---

## 宏与属性

### 属性宏

```rust
// derive 宏 - 自动实现 trait
#[derive(Debug, Clone)]
struct Point {
    x: i32,
    y: i32,
}

// 现在可以使用 Debug 和 Clone
let p = Point { x: 1, y: 2 };
println!("{:?}", p);    // Debug
let p2 = p.clone();     // Clone
```

### Oxc 的自定义属性

```rust
#[ast(visit)]  // 标记为可访问的 AST 节点
#[derive(Debug)]
#[generate_derive(CloneIn, Dummy, TakeIn)]  // Oxc 特有的派生
pub struct Program<'a> {
    pub span: Span,
    pub body: Vec<'a, Statement<'a>>,
}
```

### 常见属性

````rust
// 配置项
#[cfg(test)]         // 仅在测试时编译
#[cfg(feature = "serde")]  // 根据 feature 条件编译

// 文档
/// 这是文档注释
///
/// # Example
/// ```
/// let x = 5;
/// ```
pub fn example() {}

// 警告控制
#[allow(dead_code)]  // 允许未使用的代码
#[warn(missing_docs)]  // 警告缺少文档
````

---

## 内部可变性

### Cell<T> - 简单的内部可变性

```rust
use std::cell::Cell;

struct Container {
    value: Cell<i32>,  // 可以在不可变引用中修改
}

let container = Container { value: Cell::new(5) };
// container 是不可变的，但可以修改 value
container.value.set(10);
println!("{}", container.value.get());  // 10
```

### 在 AST 中的应用

```rust
pub struct Program<'a> {
    pub scope_id: Cell<Option<ScopeId>>,
    //            ^^^^ Cell 包装
    // ... 其他字段
}

// 使用
impl Program {
    fn set_scope(&self, scope_id: ScopeId) {
        // self 是 &self（不可变），但可以修改 scope_id
        self.scope_id.set(Some(scope_id));
    }
}
```

### RefCell<T> - 运行时检查

```rust
use std::cell::RefCell;

let data = RefCell::new(vec![1, 2, 3]);

// 借用检查在运行时进行
{
    let r1 = data.borrow();      // OK: 不可变借用
    let r2 = data.borrow();      // OK: 多个不可变借用
    // let r3 = data.borrow_mut();  // 运行时 panic!
}

{
    let mut r = data.borrow_mut();  // OK: 可变借用
    r.push(4);
}
```

### 选择指南

| 类型         | 使用场景               | 开销       |
| ------------ | ---------------------- | ---------- |
| `Cell<T>`    | T 实现 Copy，简单值    | 零开销     |
| `RefCell<T>` | 需要借用检查的复杂类型 | 运行时开销 |

---

## 常用模式速查

### 1. Option<T>

```rust
// 处理可选值
pub struct Function<'a> {
    pub id: Option<BindingIdentifier<'a>>,  // 函数名可选
}

// 使用
if let Some(id) = &func.id {
    println!("函数名: {}", id.name);
}

// 或者
let name = func.id.as_ref().map(|id| &id.name);

// 解包（不安全，确保有值时使用）
let id = func.id.unwrap();
```

### 2. Result<T, E>

```rust
// 处理错误
fn parse_file(path: &str) -> Result<Program, ParseError> {
    // ...
}

// 使用
match parse_file("test.js") {
    Ok(program) => println!("成功"),
    Err(e) => eprintln!("错误: {}", e),
}

// 或者使用 ? 操作符
fn process() -> Result<(), Error> {
    let program = parse_file("test.js")?;  // 错误自动传播
    // ...
    Ok(())
}
```

### 3. 迭代器

```rust
// 遍历
for stmt in &program.body {
    // ...
}

// 链式操作
let functions: Vec<_> = program.body.iter()
    .filter_map(|stmt| {
        if let Statement::FunctionDeclaration(func) = stmt {
            Some(func)
        } else {
            None
        }
    })
    .collect();

// 常用方法
let count = statements.len();
let first = statements.first();
let is_empty = statements.is_empty();
```

---

## 调试技巧

### 1. 打印调试

```rust
// Debug trait
println!("{:?}", expr);      // 调试格式
println!("{:#?}", expr);     // 美化输出

// 自定义格式
dbg!(expr);  // 打印表达式和值
```

### 2. 类型标注

```rust
// 当编译器推断不出类型时
let vec: Vec<i32> = Vec::new();

// 使用 turbofish 语法
let vec = Vec::<i32>::new();
```

### 3. 编译器提示

```rust
// 让编译器告诉你类型
let x = some_complex_expression;
let () = x;  // 编译错误会显示 x 的实际类型
```

---

## 学习资源

### 官方文档

- [The Rust Book](https://doc.rust-lang.org/book/) - Rust 圣经
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) - 示例学习
- [Standard Library](https://doc.rust-lang.org/std/) - 标准库文档

### 进阶资源

- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) - 高级 Rust
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/) - 设计模式

### 实用工具

- [Rust Playground](https://play.rust-lang.org/) - 在线运行
- [crates.io](https://crates.io/) - 包仓库
- [docs.rs](https://docs.rs/) - 文档集合

---

## 快速参考卡片

### 语法速查

```rust
// 变量声明
let x = 5;              // 不可变
let mut x = 5;          // 可变
const MAX: i32 = 100;   // 常量

// 函数
fn add(a: i32, b: i32) -> i32 {
    a + b  // 最后的表达式是返回值
}

// 结构体
struct Point { x: i32, y: i32 }
let p = Point { x: 1, y: 2 };

// 枚举
enum Result { Ok(T), Err(E) }

// 实现方法
impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}

// trait
trait Draw {
    fn draw(&self);
}
```

### 常用符号

| 符号   | 含义         |
| ------ | ------------ |
| `&`    | 借用（引用） |
| `&mut` | 可变借用     |
| `*`    | 解引用       |
| `'a`   | 生命周期参数 |
| `<T>`  | 泛型参数     |
| `::`   | 路径分隔符   |
| `?`    | 错误传播     |
| `_`    | 占位符/忽略  |

---

**提示**: 这个速查表会随着学习进度不断更新。建议保存并经常查阅！

遇到不懂的 Rust 概念时：

1. 先在这里查找
2. 查看 Rust Book 相关章节
3. 在 Rust Playground 实验
4. 在实际代码中应用
