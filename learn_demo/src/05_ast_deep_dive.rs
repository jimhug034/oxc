//! Day 5-7: 深入 oxc_ast 与 Rust 进阶
//!
//! 本示例展示如何深入理解和操作 AST 节点
//!
//! 运行方式：
//! ```bash
//! cargo run --bin 05_ast_deep_dive
//! ```

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::scope::ScopeFlags;
use std::collections::HashMap;

fn main() {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║     Day 5-7: 深入 oxc_ast 与 Rust 进阶                   ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Day 5: 理解 AST 节点结构
    day5_ast_structure();

    println!("\n\n");

    // Day 6: Visitor 模式应用
    day6_visitor_pattern();

    println!("\n\n");

    // Day 7: 手动构建 AST
    day7_ast_builder();
}

// ============================================================================
// Day 5: 理解 AST 节点结构
// ============================================================================

fn day5_ast_structure() {
    println!("🎯 Day 5: 理解 AST 节点结构\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let source_code = r#"
        const x = 1 + 2;
        let name = "Oxc";

        function greet(msg) {
            return "Hello, " + msg;
        }

        if (x > 0) {
            console.log(name);
        }
    "#;

    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let ret = Parser::new(&allocator, source_code, source_type).parse();

    if !ret.errors.is_empty() {
        println!("❌ 解析错误:");
        for error in ret.errors {
            println!("  {error:?}");
        }
        return;
    }

    let program = ret.program;

    println!("✅ 成功解析代码\n");

    // 1. 分析 Program 结构
    println!("📦 Program 节点分析:");
    println!("  - source_type: {:?}", program.source_type);
    println!("  - 语句数量: {}", program.body.len());
    println!("  - 指令数量: {}", program.directives.len());
    println!();

    // 2. 分析每个顶层语句
    println!("📝 顶层语句分析:");
    for (idx, stmt) in program.body.iter().enumerate() {
        println!("  [{}] {}", idx, describe_statement(stmt));
    }
    println!();

    // 3. 深入分析第一个变量声明
    if let Some(Statement::VariableDeclaration(decl)) = program.body.first() {
        println!("🔍 深入分析第一个变量声明:");
        println!("  - 声明类型: {:?}", decl.kind);
        println!("  - 声明器数量: {}", decl.declarations.len());

        if let Some(declarator) = decl.declarations.first() {
            println!("  - 变量名: {}", describe_binding_pattern(&declarator.id));
            if let Some(init) = &declarator.init {
                println!("  - 初始值: {}", describe_expression(init));
            }
        }
    }
}

// 辅助函数：描述语句类型
fn describe_statement(stmt: &Statement) -> String {
    match stmt {
        Statement::VariableDeclaration(decl) => {
            format!("变量声明 ({:?})", decl.kind)
        }
        Statement::FunctionDeclaration(func) => {
            let name = func.id.as_ref().map(|id| id.name.as_str()).unwrap_or("<anonymous>");
            format!("函数声明: {}", name)
        }
        Statement::IfStatement(_) => "If 语句".to_string(),
        Statement::ExpressionStatement(_) => "表达式语句".to_string(),
        Statement::BlockStatement(_) => "代码块".to_string(),
        _ => format!("{:?}", stmt),
    }
}

// 辅助函数：描述表达式
fn describe_expression(expr: &Expression) -> String {
    match expr {
        Expression::NumericLiteral(lit) => format!("数字字面量: {}", lit.value),
        Expression::StringLiteral(lit) => format!("字符串字面量: \"{}\"", lit.value),
        Expression::BooleanLiteral(lit) => format!("布尔字面量: {}", lit.value),
        Expression::Identifier(id) => format!("标识符: {}", id.name),
        Expression::BinaryExpression(bin) => {
            format!(
                "二元表达式: {} {:?} {}",
                describe_expression(&bin.left),
                bin.operator,
                describe_expression(&bin.right)
            )
        }
        Expression::CallExpression(call) => {
            format!("函数调用: {}(...)", describe_expression(&call.callee))
        }
        Expression::StaticMemberExpression(member) => {
            format!("成员访问: {}.{}", describe_expression(&member.object), member.property.name)
        }
        _ => format!("{:?}", expr),
    }
}

// 辅助函数：描述绑定模式
fn describe_binding_pattern(pattern: &BindingPattern) -> String {
    match &pattern.kind {
        BindingPatternKind::BindingIdentifier(id) => id.name.to_string(),
        BindingPatternKind::ObjectPattern(_) => "对象解构".to_string(),
        BindingPatternKind::ArrayPattern(_) => "数组解构".to_string(),
        BindingPatternKind::AssignmentPattern(_) => "赋值模式".to_string(),
    }
}

// ============================================================================
// Day 6: Visitor 模式应用
// ============================================================================

fn day6_visitor_pattern() {
    println!("🎯 Day 6: Visitor 模式应用\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let source_code = r#"
        const x = 1 + 2 + 3;
        let y = x * 2;

        function add(a, b) {
            return a + b;
        }

        function multiply(a, b) {
            const result = a * b;
            return result;
        }

        async function fetchData() {
            const data = await fetch('/api');
            return data;
        }

        if (x > 0) {
            console.log("positive");
        } else {
            console.log("negative");
        }

        for (let i = 0; i < 10; i++) {
            console.log(i);
        }
    "#;

    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true);
    let ret = Parser::new(&allocator, source_code, source_type).parse();

    if !ret.errors.is_empty() {
        println!("❌ 解析错误");
        return;
    }

    let program = ret.program;

    // 练习 1: 表达式计数器
    println!("📊 练习 1: 统计表达式数量");
    let mut counter = ExpressionCounter::default();
    counter.visit_program(&program);
    println!("  表达式总数: {}\n", counter.count);

    // 练习 2: 函数收集器
    println!("📋 练习 2: 收集所有函数");
    let mut collector = FunctionCollector::default();
    collector.visit_program(&program);
    println!("  找到 {} 个函数:", collector.functions.len());
    for (name, is_async) in &collector.functions {
        let async_marker = if *is_async { " (async)" } else { "" };
        println!("    - {}{}", name, async_marker);
    }
    println!();

    // 练习 3: 变量声明分析器
    println!("🔢 练习 3: 分析变量声明");
    let mut analyzer = VariableAnalyzer::default();
    analyzer.visit_program(&program);
    println!("  const 声明: {}", analyzer.const_count);
    println!("  let 声明: {}", analyzer.let_count);
    println!("  var 声明: {}", analyzer.var_count);
    println!();

    // 练习 4: console.log 查找器
    println!("🔍 练习 4: 查找 console.log");
    let mut finder = ConsoleLogFinder::default();
    finder.visit_program(&program);
    println!("  找到 {} 处 console.log 调用\n", finder.count);

    // 练习 5: 循环复杂度分析
    println!("📈 练习 5: 计算函数复杂度");
    let mut complexity = ComplexityAnalyzer::default();
    complexity.visit_program(&program);
    println!("  函数复杂度:");
    for (name, score) in &complexity.complexity_map {
        println!("    - {}: {}", name, score);
    }
}

// 练习 1: 表达式计数器
#[derive(Default)]
struct ExpressionCounter {
    count: usize,
}

impl<'a> Visit<'a> for ExpressionCounter {
    fn visit_expression(&mut self, _expr: &Expression<'a>) {
        self.count += 1;
        // 注意：不调用 walk_expression，避免重复计数
    }
}

// 练习 2: 函数收集器
#[derive(Default)]
struct FunctionCollector {
    functions: Vec<(String, bool)>, // (name, is_async)
}

impl<'a> Visit<'a> for FunctionCollector {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if let Some(id) = &it.id {
            self.functions.push((id.name.to_string(), it.r#async));
        }
    }
}

// 练习 3: 变量声明分析器
#[derive(Default)]
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
    }
}

// 练习 4: console.log 查找器
#[derive(Default)]
struct ConsoleLogFinder {
    count: usize,
}

impl<'a> Visit<'a> for ConsoleLogFinder {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        // 检查是否是 console.log
        if let Expression::StaticMemberExpression(member) = &call.callee {
            if let Expression::Identifier(obj) = &member.object {
                if obj.name == "console" && member.property.name == "log" {
                    self.count += 1;
                }
            }
        }
    }
}

// 练习 5: 循环复杂度分析器
#[derive(Default)]
struct ComplexityAnalyzer {
    current_function: Option<String>,
    complexity_map: HashMap<String, usize>,
    current_complexity: usize,
}

impl<'a> Visit<'a> for ComplexityAnalyzer {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        // 初始化新函数
        if let Some(id) = &it.id {
            self.current_function = Some(id.name.to_string());
            self.current_complexity = 1; // 基础复杂度
        }

        // 注意：Visit trait 会自动遍历子节点，我们不需要手动调用
        // 在离开函数时保存结果会在 leave_node 中处理
    }

    fn visit_if_statement(&mut self, _it: &IfStatement<'a>) {
        if self.current_function.is_some() {
            self.current_complexity += 1;
        }
    }

    fn visit_for_statement(&mut self, _it: &ForStatement<'a>) {
        if self.current_function.is_some() {
            self.current_complexity += 1;
        }
    }

    fn visit_while_statement(&mut self, _it: &WhileStatement<'a>) {
        if self.current_function.is_some() {
            self.current_complexity += 1;
        }
    }

    fn leave_node(&mut self, kind: oxc_ast::AstKind<'a>) {
        // 离开函数节点时保存复杂度
        if let oxc_ast::AstKind::Function(func) = kind {
            if let Some(id) = &func.id {
                if self.current_function.as_ref().map(|s| s.as_str()) == Some(&id.name) {
                    self.complexity_map.insert(id.name.to_string(), self.current_complexity);
                    self.current_function = None;
                    self.current_complexity = 0;
                }
            }
        }
    }
}

// ============================================================================
// Day 7: 手动构建 AST
// ============================================================================

fn day7_ast_builder() {
    println!("🎯 Day 7: Arena 分配器与内存管理\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 演示 Arena 分配器的基本概念
    println!("📦 Arena 分配器概念演示\n");

    let _allocator = Allocator::default();

    println!("1️⃣ 创建 Allocator");
    println!("   let allocator = Allocator::default();\n");

    println!("2️⃣ 在 Arena 上分配内存");
    println!("   所有 AST 节点都在这个 Arena 上分配");
    println!("   使用 Box::new_in(..., &allocator) 或 Vec::new_in(&allocator)\n");

    println!("3️⃣ 生命周期绑定");
    println!("   所有节点的生命周期 'a 与 allocator 绑定");
    println!("   确保节点不会超过 allocator 的生命周期\n");

    println!("4️⃣ 一次性释放");
    println!("   当 allocator drop 时，所有节点一次性释放");
    println!("   非常高效，无需逐个 drop！\n");

    println!("💡 Arena 分配器的优势：\n");
    println!("   ✅ 快速分配：几乎零开销");
    println!("   ✅ 缓存友好：内存连续，提升 CPU 缓存命中率");
    println!("   ✅ 简单释放：一次性释放所有内存");
    println!("   ✅ 无内存碎片：避免频繁分配/释放导致的碎片\n");

    println!("📚 实际使用示例：");
    println!("   // 解析代码时");
    println!("   let allocator = Allocator::default();");
    println!("   let parser = Parser::new(&allocator, source_code, source_type);");
    println!("   let program = parser.parse().program;");
    println!("   // program 中的所有节点都在 allocator 上\n");

    println!("   // 使用 AstBuilder");
    println!("   let ast = AstBuilder::new(&allocator);");
    println!("   let expr = ast.expression_numeric_literal(...);");
    println!("   // expr 也在 allocator 上\n");

    println!("🎓 学习要点：");
    println!("   1. 理解为什么需要 Arena 分配器");
    println!("   2. 知道 Box<'a, T> 和 Vec<'a, T> 与标准库版本的区别");
    println!("   3. 理解生命周期 'a 的作用");
    println!("   4. 了解 AstBuilder 的作用\n");

    println!("📖 深入学习：");
    println!("   查看文档 Day5-7_深入oxc_ast与Rust进阶.md");
    println!("   其中详细讲解了 Arena 分配器的原理和使用");
    println!("   以及如何使用 AstBuilder 手动构建 AST 节点\n");

    println!("✨ 完成 Day 7！");
    println!("   恭喜你完成了第一周的学习！");
    println!("   你已经掌握了 oxc_ast 的核心知识。");
}
