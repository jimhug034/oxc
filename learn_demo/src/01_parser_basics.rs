// Day 1-2: Parser 基础示例
// 运行: cargo run --bin 01_parser_basics

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

fn main() {
    println!("🎯 Oxc Parser 基础示例");
    println!("{}", "=".repeat(60));

    // 示例 1: 解析简单的变量声明
    example_1_variable_declaration();

    // 示例 2: 解析函数
    example_2_function();

    // 示例 3: 解析表达式
    example_3_expressions();

    // 示例 4: 解析 TypeScript
    example_4_typescript();

    // 示例 5: 错误处理
    example_5_error_handling();

    println!("\n🎉 Parser 基础示例完成！");
    println!("\n💡 提示:");
    println!("   - 查看输出，理解 AST 的结构");
    println!("   - 尝试修改源代码，观察 AST 的变化");
    println!("   - 参考文档: learn_demo/docs/第一周_Day1-2_Parser基础.md");
}

fn example_1_variable_declaration() {
    println!("\n📦 示例 1: 变量声明");
    println!("{}", "-".repeat(60));

    let allocator = Allocator::default();
    let source_code = r#"
        const sum = 1 + 2 + 3;
    "#;

    let source_type = SourceType::default();
    let ret = Parser::new(&allocator, source_code, source_type).parse();

    println!("源代码:");
    println!("{}", source_code);
    println!("\n解析结果:");
    println!("  - 错误数: {}", ret.errors.len());
    println!("  - 语句数: {}", ret.program.body.len());

    // 遍历顶层语句
    for (i, stmt) in ret.program.body.iter().enumerate() {
        println!("  - 语句 {}: {:?}", i + 1, stmt);
    }

    // TODO: 添加更详细的 AST 分析
    // 提示: 你可以匹配不同的语句类型，打印更多信息
}

fn example_2_function() {
    println!("\n📦 示例 2: 函数");
    println!("{}", "-".repeat(60));

    let allocator = Allocator::default();
    let source_code = r#"
        function greet(name) {
            return "Hello, " + name;
        }

        const result = greet("World");
    "#;

    let source_type = SourceType::default();
    let ret = Parser::new(&allocator, source_code, source_type).parse();

    println!("源代码:");
    println!("{}", source_code);
    println!("\n解析结果:");
    println!("  - 错误数: {}", ret.errors.len());
    println!("  - 语句数: {}", ret.program.body.len());

    // TODO: 分析函数声明
    // 提示: 查看函数的参数、函数体等信息
}

fn example_3_expressions() {
    println!("\n📦 示例 3: 表达式");
    println!("{}", "-".repeat(60));

    let allocator = Allocator::default();
    let source_code = r#"
        const sum = 1 + 2 + 3;
        const product = x * y;
        const result = numbers.map(n => n * 2);
    "#;

    let source_type = SourceType::default();
    let ret = Parser::new(&allocator, source_code, source_type).parse();

    println!("源代码:");
    println!("{}", source_code);
    println!("\n解析结果:");
    println!("  - 错误数: {}", ret.errors.len());
    println!("  - 语句数: {}", ret.program.body.len());

    // TODO: 分析不同类型的表达式
    // 提示: 二元表达式、调用表达式、箭头函数等
}

fn example_4_typescript() {
    println!("\n📦 示例 4: TypeScript");
    println!("{}", "-".repeat(60));

    let allocator = Allocator::default();
    let source_code = r#"
        interface User {
            name: string;
            age: number;
        }

        const user: User = {
            name: "Alice",
            age: 30
        };
    "#;

    // 注意: TypeScript 需要指定 source_type
    let source_type = SourceType::from_path("test.ts").unwrap();
    let ret = Parser::new(&allocator, source_code, source_type).parse();

    println!("源代码:");
    println!("{}", source_code);
    println!("\n解析结果:");
    println!("  - 错误数: {}", ret.errors.len());
    println!("  - 语句数: {}", ret.program.body.len());

    // TODO: 分析 TypeScript 特有的节点
    // 提示: Interface 声明、类型注解等
}

fn example_5_error_handling() {
    println!("\n📦 示例 5: 错误处理");
    println!("{}", "-".repeat(60));

    let allocator = Allocator::default();
    // 故意包含语法错误的代码
    let source_code = r#"
        const x = ;  // 语法错误
        function foo( {  // 语法错误
            return 1;
        }
    "#;

    let source_type = SourceType::default();
    let ret = Parser::new(&allocator, source_code, source_type).parse();

    println!("源代码 (包含错误):");
    println!("{}", source_code);
    println!("\n解析结果:");
    println!("  - 错误数: {}", ret.errors.len());

    // 打印所有错误
    for (i, error) in ret.errors.iter().enumerate() {
        println!("  - 错误 {}: {:?}", i + 1, error);
    }

    println!("\n💡 注意:");
    println!("   Oxc Parser 具有错误恢复能力，即使遇到错误也会继续解析");
}

// ============================================================================
// 练习区域
// ============================================================================

// 练习 1: 编写一个函数，统计代码中的函数声明数量
#[allow(dead_code)]
fn count_functions(source_code: &str) -> usize {
    let allocator = Allocator::default();
    let source_type = SourceType::default();
    let ret = Parser::new(&allocator, source_code, source_type).parse();

    // TODO: 实现函数统计
    // 提示: 遍历 program.body，检查每个语句的类型

    0
}

// 练习 2: 编写一个函数，提取所有变量名
#[allow(dead_code)]
fn extract_variable_names(source_code: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::default();
    let _ret = Parser::new(&allocator, source_code, source_type).parse();

    // TODO: 实现变量名提取
    // 提示: 查找 VariableDeclaration，提取 id.name

    vec![]
}

// 练习 3: 编写一个函数，检测代码中是否使用了箭头函数
#[allow(dead_code)]
fn has_arrow_function(source_code: &str) -> bool {
    // TODO: 实现箭头函数检测
    // 提示: 需要深度遍历 AST，可以使用 visitor 模式

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_functions() {
        let code = r#"
            function foo() {}
            function bar() {}
            const baz = function() {};
        "#;

        // 应该找到 3 个函数
        assert_eq!(count_functions(code), 3);
    }

    #[test]
    fn test_extract_variable_names() {
        let code = r#"
            const x = 1;
            let y = 2;
            var z = 3;
        "#;

        let names = extract_variable_names(code);
        assert_eq!(names, vec!["x", "y", "z"]);
    }

    #[test]
    fn test_has_arrow_function() {
        let code_with_arrow = "const fn = () => {}";
        let code_without_arrow = "function fn() {}";

        assert!(has_arrow_function(code_with_arrow));
        assert!(!has_arrow_function(code_without_arrow));
    }
}

