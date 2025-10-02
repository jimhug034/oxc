# 第一周 Day 3-4: Linter 基础

> 理解代码检查规则的工作原理，编写自己的 Lint 规则

## 📖 学习目标

- [ ] 理解 Linter 的工作原理
- [ ] 掌握访问者模式 (Visitor Pattern)
- [ ] 能够阅读现有的 Lint 规则
- [ ] 创建并实现简单的自定义规则

## 🚀 快速开始

### 1. 运行 oxlint

```bash
# 进入 Oxc 项目根目录
cd /Users/makeblock/Developer/my-git/oxc

# 检查一些代码文件
cargo run -p oxc_linter --example linter -- apps/oxlint/src/

# 或者运行我们的学习示例
cargo run --bin 02_linter_basics
```

### 2. 体验 Linter

创建 `test_lint.js`：

```javascript
debugger;  // 应该被 no-debugger 规则检测到

console.log("test");  // 如果启用 no-console 会被检测到

if (x == null) {  // 应该用 === 而不是 ==
    console.log("null check");
}
```

运行检查：
```bash
cargo run -p oxc_linter --example linter -- test_lint.js
```

## 📚 核心概念

### Linter 的工作流程

```
源代码
  ↓
Parser 解析 → AST
  ↓
遍历 AST (Visitor Pattern)
  ↓
应用 Lint 规则
  ↓
收集诊断信息 (Diagnostics)
  ↓
输出错误/警告
```

### 访问者模式 (Visitor Pattern)

访问者模式允许你在不修改 AST 节点定义的情况下，对节点执行操作。

#### 概念图

```
Visitor                    AST 节点
  ↓                          ↓
visit_program()  ────→    Program
visit_statement() ────→   Statement
visit_expression() ───→   Expression
```

#### 代码示例

```rust
impl<'a> Visit<'a> for MyRule {
    // 访问每个函数
    fn visit_function(&mut self, func: &Function<'a>) {
        // 检查函数名
        if let Some(id) = &func.id {
            println!("Found function: {}", id.name);
        }

        // 继续遍历子节点
        walk_function(self, func);
    }

    // 访问每个变量声明
    fn visit_variable_declarator(&mut self, decl: &VariableDeclarator<'a>) {
        println!("Found variable: {}", decl.id);
        walk_variable_declarator(self, decl);
    }
}
```

### Lint 规则的结构

一个典型的 Lint 规则包含：

1. **规则元数据**
   - 名称、分类、严重程度
   - 文档链接

2. **检测逻辑**
   - 实现 `Visit` trait
   - 在特定节点类型上检查

3. **诊断生成**
   - 创建错误/警告信息
   - 提供修复建议（可选）

## 🔬 阅读现有规则

### 规则 1: `no-debugger` - 最简单的规则

位置: `crates/oxc_linter/src/rules/eslint/no_debugger.rs`

```rust
// 简化版本
impl Rule for NoDebugger {
    fn run_once(&self, ctx: &LintContext) {
        // 遍历所有语句
        for stmt in &ctx.semantic().program().body {
            // 检查是否是 debugger 语句
            if matches!(stmt, Statement::DebuggerStatement(_)) {
                ctx.diagnostic(
                    no_debugger_diagnostic(stmt.span())
                );
            }
        }
    }
}
```

**学习要点**:
- 最简单的规则：只检查一种语句类型
- 使用 `run_once` 而不是 visitor（因为只需要扫描一次）
- 创建诊断信息

---

### 规则 2: `no-console` - 检查成员表达式

位置: `crates/oxc_linter/src/rules/eslint/no_console.rs`

```rust
impl Rule for NoConsole {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        // 只关心调用表达式
        if let AstKind::CallExpression(call_expr) = node.kind() {
            // 检查是否是 console.xxx()
            if let Expression::StaticMemberExpression(member) = &call_expr.callee {
                if member.object.is_specific_id("console") {
                    ctx.diagnostic(
                        no_console_diagnostic(member.span)
                    );
                }
            }
        }
    }
}
```

**学习要点**:
- 使用 `run` 方法遍历每个节点
- 检查节点类型和结构
- 判断标识符名称

---

### 规则 3: `eqeqeq` - 更复杂的逻辑

位置: `crates/oxc_linter/src/rules/eslint/eqeqeq.rs`

这个规则检查是否使用了 `==` 或 `!=` 而不是 `===` 或 `!==`。

**学习要点**:
- 检查二元操作符
- 提供自动修复建议
- 配置选项支持

## 🛠️ 创建自己的规则

### 练习 1: `no-magic-numbers`

目标：检测代码中的魔术数字（没有命名的数字常量）

#### 步骤 1: 生成规则模板

```bash
cd /Users/makeblock/Developer/my-git/oxc
just new-rule no-magic-numbers
```

#### 步骤 2: 实现规则逻辑

```rust
use oxc_ast::AstKind;
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;
use crate::{context::LintContext, rule::Rule, AstNode};

declare_oxc_lint!(
    /// ### What it does
    /// 禁止使用魔术数字
    ///
    /// ### Why is this bad?
    /// 魔术数字降低代码可读性
    ///
    /// ### Example
    /// ```javascript
    /// // Bad
    /// const area = width * 3.14;
    ///
    /// // Good
    /// const PI = 3.14;
    /// const area = width * PI;
    /// ```
    NoMagicNumbers,
    restriction,
    pending
);

impl Rule for NoMagicNumbers {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        if let AstKind::NumericLiteral(lit) = node.kind() {
            // 允许 0 和 1（常见的非魔术数字）
            if lit.value == 0.0 || lit.value == 1.0 {
                return;
            }

            // 报告魔术数字
            ctx.diagnostic(
                OxcDiagnostic::warn("不要使用魔术数字")
                    .with_label(lit.span)
            );
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        "const x = 0;",
        "const y = 1;",
        "const PI = 3.14;",  // 在声明中可以接受
    ];

    let fail = vec![
        "const area = width * 3.14;",
        "if (count > 100) {}",
        "setTimeout(fn, 5000);",
    ];

    Tester::new(NoMagicNumbers::NAME, NoMagicNumbers::PLUGIN, pass, fail)
        .test_and_snapshot();
}
```

#### 步骤 3: 注册规则

在 `crates/oxc_linter/src/rules.rs` 中添加：

```rust
mod no_magic_numbers;
pub use no_magic_numbers::NoMagicNumbers;
```

#### 步骤 4: 测试规则

```bash
cargo test -p oxc_linter no_magic_numbers
```

---

### 练习 2: `no-var` (简单版本)

目标：禁止使用 `var` 声明变量

<details>
<summary>点击查看实现提示</summary>

```rust
impl Rule for NoVar {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        if let AstKind::VariableDeclaration(decl) = node.kind() {
            // 检查是否是 var
            if decl.kind == VariableDeclarationKind::Var {
                ctx.diagnostic(
                    OxcDiagnostic::warn("使用 let 或 const 代替 var")
                        .with_label(decl.span)
                );
            }
        }
    }
}
```
</details>

---

### 练习 3: `require-await`

目标：检查 async 函数是否使用了 await

这个练习更复杂，需要：
1. 跟踪是否在 async 函数内部
2. 检测是否有 await 表达式
3. 在函数结束时报告

<details>
<summary>点击查看实现提示</summary>

```rust
#[derive(Default)]
struct RequireAwait {
    async_function_stack: Vec<(Span, bool)>,  // (span, has_await)
}

impl Rule for RequireAwait {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        match node.kind() {
            // 进入 async 函数
            AstKind::Function(func) if func.r#async => {
                self.async_function_stack.push((func.span, false));
            }

            // 发现 await 表达式
            AstKind::AwaitExpression(_) => {
                if let Some(last) = self.async_function_stack.last_mut() {
                    last.1 = true;  // 标记有 await
                }
            }

            _ => {}
        }
    }

    fn run_on_exit<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        // 离开 async 函数时检查
        if let AstKind::Function(func) = node.kind() {
            if func.r#async {
                if let Some((span, has_await)) = self.async_function_stack.pop() {
                    if !has_await {
                        ctx.diagnostic(
                            OxcDiagnostic::warn("async 函数应该使用 await")
                                .with_label(span)
                        );
                    }
                }
            }
        }
    }
}
```
</details>

## 📝 实践任务

### 任务清单

- [ ] 阅读至少 3 个现有的 Lint 规则
- [ ] 理解访问者模式的工作原理
- [ ] 使用 `just new-rule` 创建一个规则
- [ ] 实现 `no-magic-numbers` 规则
- [ ] 为规则编写测试用例
- [ ] （可选）实现 `no-var` 规则
- [ ] （挑战）实现 `require-await` 规则

### 推荐阅读的规则

按难度排序：

1. **入门级**
   - `no_debugger.rs` - 检查 debugger 语句
   - `no_with.rs` - 检查 with 语句
   - `no_empty.rs` - 检查空代码块

2. **初级**
   - `no_console.rs` - 检查 console 调用
   - `no_var.rs` - 检查 var 声明
   - `use_isnan.rs` - 检查 NaN 比较

3. **中级**
   - `eqeqeq.rs` - 检查相等运算符
   - `no_unused_vars.rs` - 检查未使用变量
   - `prefer_const.rs` - 推荐使用 const

4. **高级**
   - `no_this_before_super.rs` - 检查 super 调用
   - `no_shadow_restricted_names.rs` - 检查变量名遮蔽

## 🔍 深入理解

### Semantic 信息

Linter 不仅可以访问 AST，还可以使用 Semantic 分析提供的信息：

```rust
// 获取符号信息
let semantic = ctx.semantic();

// 检查变量是否被使用
if semantic.symbol_references(symbol_id).count() == 0 {
    // 未使用的变量
}

// 获取作用域信息
let scope = semantic.scope(scope_id);
```

### 自动修复 (Auto-fix)

一些规则可以提供自动修复：

```rust
ctx.diagnostic_with_fix(
    diagnostic,
    |fixer| {
        fixer.replace(span, "修复后的代码")
    }
);
```

### 配置选项

规则可以接受配置：

```rust
#[derive(Debug, Default, Deserialize)]
struct NoConsoleConfig {
    allow: Vec<String>,  // 允许的 console 方法
}

impl Rule for NoConsole {
    fn from_configuration(value: serde_json::Value) -> Self {
        let config = serde_json::from_value(value).unwrap_or_default();
        Self(Box::new(config))
    }
}
```

## 🎯 检查点

完成以下任务，检验学习效果：

- [ ] 能够运行 oxlint 检查代码
- [ ] 理解访问者模式的基本原理
- [ ] 能够阅读和理解简单的 Lint 规则
- [ ] 成功创建并实现一个自定义规则
- [ ] 为规则编写了测试用例
- [ ] 测试通过

## 🔗 相关资源

### 代码位置
- Linter 核心: `crates/oxc_linter/src/`
- 规则实现: `crates/oxc_linter/src/rules/`
- 测试工具: `crates/oxc_linter/src/tester.rs`

### 文档
- [ESLint 规则文档](https://eslint.org/docs/latest/rules/)
- [贡献 Lint 规则](../../../../CONTRIBUTING.md)

### 工具
- `just new-rule <name>` - 创建新规则
- `cargo test -p oxc_linter` - 运行测试

---

## ➡️ 下一步

完成 Day 3-4 的学习后，继续：
- [Day 5-7: 其他工具初探](./第一周_Day5-7_其他工具.md)

---

**学习日期**: ___________
**完成情况**: ⬜ 未开始 / ⬜ 进行中 / ⬜ 已完成

