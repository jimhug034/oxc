// 第五个示例：AST 模拟和实际应用场景
// 运行方式：cd learn_docs/examples && cargo run --bin 05_ast_simulation

use oxc_allocator::{Allocator, Vec as ArenaVec, HashMap as ArenaHashMap, Box as ArenaBox};
use std::time::Instant;

fn main() {
    println!("🌳 AST 模拟和实际应用场景");
    println!("{}", "=".repeat(50));

    // 简单 AST 模拟
    simple_ast_demo();

    // 复杂 AST 构建
    complex_ast_demo();

    // AST 遍历和分析
    ast_traversal_demo();

    // 真实场景模拟：JavaScript 解析
    javascript_parsing_simulation();

    // 性能对比：Arena vs 传统方式
    ast_performance_comparison();

    println!("\n🎉 AST 模拟示例完成！");
}

fn simple_ast_demo() {
    println!("\n🌱 简单 AST 演示:");

    let allocator = Allocator::default();

    // 定义简单的 AST 节点类型
    #[derive(Debug)]
    enum AstNodeType {
        Program,
        FunctionDeclaration,
        Parameter,
        BlockStatement,
        ReturnStatement,
        BinaryExpression,
        Identifier,
        Literal,
    }

    #[derive(Debug)]
    struct AstNode<'a> {
        node_type: AstNodeType,
        value: Option<&'a str>,
        children: ArenaVec<'a, &'a AstNode<'a>>,
    }

    impl<'a> AstNode<'a> {
        fn new_in(
            allocator: &'a Allocator,
            node_type: AstNodeType,
            value: Option<&'a str>,
        ) -> &'a mut Self {
            allocator.alloc(AstNode {
                node_type,
                value,
                children: ArenaVec::new_in(allocator),
            })
        }

        fn add_child(&mut self, child: &'a AstNode<'a>) {
            self.children.push(child);
        }
    }

    // 构建 AST：function add(a, b) { return a + b; }
    println!("   构建 AST: function add(a, b) {{ return a + b; }}");

    let program = AstNode::new_in(&allocator, AstNodeType::Program, None);
    let function = AstNode::new_in(&allocator, AstNodeType::FunctionDeclaration,
                                   Some(allocator.alloc_str("add")));
    let param_a = AstNode::new_in(&allocator, AstNodeType::Parameter,
                                  Some(allocator.alloc_str("a")));
    let param_b = AstNode::new_in(&allocator, AstNodeType::Parameter,
                                  Some(allocator.alloc_str("b")));
    let block = AstNode::new_in(&allocator, AstNodeType::BlockStatement, None);
    let return_stmt = AstNode::new_in(&allocator, AstNodeType::ReturnStatement, None);
    let binary_expr = AstNode::new_in(&allocator, AstNodeType::BinaryExpression,
                                      Some(allocator.alloc_str("+")));
    let id_a = AstNode::new_in(&allocator, AstNodeType::Identifier,
                               Some(allocator.alloc_str("a")));
    let id_b = AstNode::new_in(&allocator, AstNodeType::Identifier,
                               Some(allocator.alloc_str("b")));

    // 构建树结构
    program.add_child(function);
    function.add_child(param_a);
    function.add_child(param_b);
    function.add_child(block);
    block.add_child(return_stmt);
    return_stmt.add_child(binary_expr);
    binary_expr.add_child(id_a);
    binary_expr.add_child(id_b);

    // 打印 AST 结构
    print_ast(program, 0);

    println!("   🎯 所有 AST 节点都在同一个 Arena 中，内存连续！");
}

fn print_ast(node: &AstNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let value_str = node.value.map_or(String::new(), |v| format!(" ({})", v));
    println!("   {}├─ {:?}{}", indent, node.node_type, value_str);

    for child in &node.children {
        print_ast(child, depth + 1);
    }
}

fn complex_ast_demo() {
    println!("\n🌳 复杂 AST 演示:");

    let allocator = Allocator::default();

    // 更复杂的 AST 节点定义
    #[derive(Debug, Clone)]
    struct ComplexAstNode<'a> {
        id: u32,
        node_type: &'static str,
        value: Option<&'a str>,
        attributes: ArenaHashMap<'a, &'a str, &'a str>,
        children: ArenaVec<'a, ArenaBox<'a, ComplexAstNode<'a>>>,
        parent: Option<u32>, // 父节点 ID
    }

    impl<'a> ComplexAstNode<'a> {
        fn new_in(
            allocator: &'a Allocator,
            id: u32,
            node_type: &'static str,
            value: Option<&'a str>,
        ) -> ArenaBox<'a, Self> {
            allocator.alloc(ComplexAstNode {
                id,
                node_type,
                value,
                attributes: ArenaHashMap::new_in(allocator),
                children: ArenaVec::new_in(allocator),
                parent: None,
            })
        }

        fn add_child(&mut self, mut child: ArenaBox<'a, ComplexAstNode<'a>>) {
            child.parent = Some(self.id);
            self.children.push(child);
        }

        fn set_attribute(&mut self, key: &'a str, value: &'a str) {
            self.attributes.insert(key, value);
        }
    }

    // 构建一个复杂的 JavaScript 类的 AST
    println!("   构建复杂 AST: class Calculator {{ ... }}");

    let mut node_id = 0;
    let mut next_id = || { node_id += 1; node_id };

    let mut class_node = ComplexAstNode::new_in(
        &allocator,
        next_id(),
        "ClassDeclaration",
        Some(allocator.alloc_str("Calculator"))
    );
    class_node.set_attribute(
        allocator.alloc_str("access"),
        allocator.alloc_str("public")
    );

    // 构造函数
    let mut constructor = ComplexAstNode::new_in(
        &allocator,
        next_id(),
        "MethodDefinition",
        Some(allocator.alloc_str("constructor"))
    );
    constructor.set_attribute(
        allocator.alloc_str("kind"),
        allocator.alloc_str("constructor")
    );

    // 方法
    let mut add_method = ComplexAstNode::new_in(
        &allocator,
        next_id(),
        "MethodDefinition",
        Some(allocator.alloc_str("add"))
    );
    add_method.set_attribute(
        allocator.alloc_str("kind"),
        allocator.alloc_str("method")
    );

    let mut subtract_method = ComplexAstNode::new_in(
        &allocator,
        next_id(),
        "MethodDefinition",
        Some(allocator.alloc_str("subtract"))
    );
    subtract_method.set_attribute(
        allocator.alloc_str("kind"),
        allocator.alloc_str("method")
    );

    // 添加参数到方法
    let param1 = ComplexAstNode::new_in(
        &allocator,
        next_id(),
        "Parameter",
        Some(allocator.alloc_str("a"))
    );
    let param2 = ComplexAstNode::new_in(
        &allocator,
        next_id(),
        "Parameter",
        Some(allocator.alloc_str("b"))
    );

    add_method.add_child(param1);
    add_method.add_child(param2);

    // 构建类结构
    class_node.add_child(constructor);
    class_node.add_child(add_method);
    class_node.add_child(subtract_method);

    // 打印复杂 AST
    print_complex_ast(&class_node, 0);

    println!("   📊 AST 统计:");
    let stats = collect_ast_stats(&class_node);
    println!("     总节点数: {}", stats.total_nodes);
    println!("     最大深度: {}", stats.max_depth);
    println!("     节点类型: {:?}", stats.node_types);
}

#[derive(Debug)]
struct AstStats {
    total_nodes: usize,
    max_depth: usize,
    node_types: ArenaVec<'static, &'static str>,
}

fn collect_ast_stats<'a>(node: &ComplexAstNode<'a>) -> AstStats {
    let allocator = Allocator::default();
    let mut stats = AstStats {
        total_nodes: 0,
        max_depth: 0,
        node_types: ArenaVec::new_in(&allocator),
    };

    collect_stats_recursive(node, 0, &mut stats);
    stats
}

fn collect_stats_recursive(node: &ComplexAstNode, depth: usize, stats: &mut AstStats) {
    stats.total_nodes += 1;
    stats.max_depth = stats.max_depth.max(depth);

    if !stats.node_types.contains(&node.node_type) {
        stats.node_types.push(node.node_type);
    }

    for child in &node.children {
        collect_stats_recursive(child, depth + 1, stats);
    }
}

fn print_complex_ast(node: &ComplexAstNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let value_str = node.value.map_or(String::new(), |v| format!(" ({})", v));

    println!("   {}├─ {} [ID: {}]{}", indent, node.node_type, node.id, value_str);

    // 打印属性
    if !node.attributes.is_empty() {
        for (key, value) in &node.attributes {
            println!("   {}│  @{}: {}", indent, key, value);
        }
    }

    // 打印子节点
    for child in &node.children {
        print_complex_ast(child, depth + 1);
    }
}

fn ast_traversal_demo() {
    println!("\n🚶 AST 遍历演示:");

    let allocator = Allocator::default();

    // 创建一个表达式树：(a + b) * (c - d)
    #[derive(Debug)]
    struct ExprNode<'a> {
        op: &'static str,
        value: Option<&'a str>,
        left: Option<ArenaBox<'a, ExprNode<'a>>>,
        right: Option<ArenaBox<'a, ExprNode<'a>>>,
    }

    impl<'a> ExprNode<'a> {
        fn new_binary(
            allocator: &'a Allocator,
            op: &'static str,
            left: ExprNode<'a>,
            right: ExprNode<'a>,
        ) -> ArenaBox<'a, Self> {
            allocator.alloc(ExprNode {
                op,
                value: None,
                left: Some(allocator.alloc(left)),
                right: Some(allocator.alloc(right)),
            })
        }

        fn new_identifier(allocator: &'a Allocator, name: &'a str) -> Self {
            ExprNode {
                op: "identifier",
                value: Some(name),
                left: None,
                right: None,
            }
        }
    }

    // 构建表达式：(a + b) * (c - d)
    let a = ExprNode::new_identifier(&allocator, allocator.alloc_str("a"));
    let b = ExprNode::new_identifier(&allocator, allocator.alloc_str("b"));
    let c = ExprNode::new_identifier(&allocator, allocator.alloc_str("c"));
    let d = ExprNode::new_identifier(&allocator, allocator.alloc_str("d"));

    let add_expr = ExprNode {
        op: "+",
        value: None,
        left: Some(allocator.alloc(a)),
        right: Some(allocator.alloc(b)),
    };

    let sub_expr = ExprNode {
        op: "-",
        value: None,
        left: Some(allocator.alloc(c)),
        right: Some(allocator.alloc(d)),
    };

    let mul_expr = ExprNode::new_binary(&allocator, "*", add_expr, sub_expr);

    println!("   表达式: (a + b) * (c - d)");

    // 前序遍历
    println!("   前序遍历:");
    preorder_traversal(&mul_expr, 0);

    // 中序遍历
    println!("   中序遍历:");
    inorder_traversal(&mul_expr);
    println!();

    // 后序遍历
    println!("   后序遍历:");
    postorder_traversal(&mul_expr, 0);
}

fn preorder_traversal(node: &ExprNode, depth: usize) {
    let indent = "  ".repeat(depth);
    if let Some(value) = node.value {
        println!("     {}{}", indent, value);
    } else {
        println!("     {}{}", indent, node.op);
    }

    if let Some(left) = &node.left {
        preorder_traversal(left, depth + 1);
    }
    if let Some(right) = &node.right {
        preorder_traversal(right, depth + 1);
    }
}

fn inorder_traversal(node: &ExprNode) {
    if let Some(left) = &node.left {
        print!("(");
        inorder_traversal(left);
    }

    if let Some(value) = node.value {
        print!(" {} ", value);
    } else {
        print!(" {} ", node.op);
    }

    if let Some(right) = &node.right {
        inorder_traversal(right);
        print!(")");
    }
}

fn postorder_traversal(node: &ExprNode, depth: usize) {
    let indent = "  ".repeat(depth);

    if let Some(left) = &node.left {
        postorder_traversal(left, depth + 1);
    }
    if let Some(right) = &node.right {
        postorder_traversal(right, depth + 1);
    }

    if let Some(value) = node.value {
        println!("     {}{}", indent, value);
    } else {
        println!("     {}{}", indent, node.op);
    }
}

fn javascript_parsing_simulation() {
    println!("\n🔧 JavaScript 解析模拟:");

    let allocator = Allocator::default();

    // 模拟解析一个真实的 JavaScript 函数
    let source_code = r#"
        function fibonacci(n) {
            if (n <= 1) {
                return n;
            }
            return fibonacci(n - 1) + fibonacci(n - 2);
        }
    "#;

    println!("   源代码:");
    println!("{}", source_code);

    let start = Instant::now();

    // 模拟词法分析
    let mut tokens = ArenaVec::new_in(&allocator);
    let token_strings = [
        "function", "fibonacci", "(", "n", ")", "{",
        "if", "(", "n", "<=", "1", ")", "{",
        "return", "n", ";", "}",
        "return", "fibonacci", "(", "n", "-", "1", ")",
        "+", "fibonacci", "(", "n", "-", "2", ")", ";",
        "}"
    ];

    for token_str in token_strings {
        let token = allocator.alloc_str(token_str);
        tokens.push(token);
    }

    let lexing_time = start.elapsed();

    // 模拟语法分析 - 创建 AST 节点
    let start = Instant::now();
    let mut ast_nodes = ArenaVec::new_in(&allocator);

    // 创建各种类型的 AST 节点
    let node_types = [
        "FunctionDeclaration", "Identifier", "Parameter", "BlockStatement",
        "IfStatement", "BinaryExpression", "ReturnStatement", "CallExpression",
        "Literal", "ArithmeticExpression"
    ];

    for (i, node_type) in node_types.iter().enumerate() {
        let node = allocator.alloc_str(&format!("{}_{}", node_type, i));
        ast_nodes.push(node);
    }

    let parsing_time = start.elapsed();

    // 模拟语义分析
    let start = Instant::now();
    let mut symbol_table = ArenaHashMap::new_in(&allocator);
    symbol_table.insert(
        allocator.alloc_str("fibonacci"),
        allocator.alloc_str("function")
    );
    symbol_table.insert(
        allocator.alloc_str("n"),
        allocator.alloc_str("parameter")
    );

    let semantic_time = start.elapsed();

    println!("   解析统计:");
    println!("     词法分析: {} 个 tokens, 耗时 {:?}", tokens.len(), lexing_time);
    println!("     语法分析: {} 个 AST 节点, 耗时 {:?}", ast_nodes.len(), parsing_time);
    println!("     语义分析: {} 个符号, 耗时 {:?}", symbol_table.len(), semantic_time);

    let total_time = lexing_time + parsing_time + semantic_time;
    println!("     总耗时: {:?}", total_time);

    println!("   🎯 Arena 优势在解析中的体现:");
    println!("     - 所有 tokens、AST 节点、符号都在连续内存中");
    println!("     - 极快的分配速度，不影响解析性能");
    println!("     - 解析完成后，所有数据一起释放");
    println!("     - 非常适合编译器的工作流程");
}

fn ast_performance_comparison() {
    println!("\n🏁 AST 性能对比:");

    const NODE_COUNT: usize = 10_000;

    // 传统方式：使用 Box 和 Vec
    println!("   传统方式 (Box + Vec):");
    let start = Instant::now();

    #[derive(Debug)]
    struct TraditionalNode {
        id: usize,
        node_type: String,
        children: Vec<Box<TraditionalNode>>,
    }

    let mut traditional_nodes = Vec::new();
    for i in 0..NODE_COUNT {
        let node = Box::new(TraditionalNode {
            id: i,
            node_type: format!("Node_{}", i),
            children: Vec::new(),
        });
        traditional_nodes.push(node);
    }

    let traditional_time = start.elapsed();
    println!("     创建 {} 个节点耗时: {:?}", NODE_COUNT, traditional_time);

    // Arena 方式
    println!("   Arena 方式:");
    let allocator = Allocator::default();
    let start = Instant::now();

    #[derive(Debug)]
    struct ArenaNode<'a> {
        id: usize,
        node_type: &'a str,
        children: ArenaVec<'a, &'a ArenaNode<'a>>,
    }

    let mut arena_nodes = ArenaVec::new_in(&allocator);
    for i in 0..NODE_COUNT {
        let node_type = allocator.alloc_str(&format!("Node_{}", i));
        let node = allocator.alloc(ArenaNode {
            id: i,
            node_type,
            children: ArenaVec::new_in(&allocator),
        });
        arena_nodes.push(node);
    }

    let arena_time = start.elapsed();
    println!("     创建 {} 个节点耗时: {:?}", NODE_COUNT, arena_time);

    // 性能对比
    let speedup = traditional_time.as_nanos() as f64 / arena_time.as_nanos() as f64;
    println!("   🚀 Arena 方式速度提升: {:.2}x", speedup);

    // 内存使用对比
    let traditional_memory = NODE_COUNT * (
        std::mem::size_of::<Box<TraditionalNode>>() +
        std::mem::size_of::<TraditionalNode>()
    );

    let arena_memory = NODE_COUNT * std::mem::size_of::<ArenaNode>();

    println!("   💾 内存使用对比:");
    println!("     传统方式: ~{} KB", traditional_memory / 1024);
    println!("     Arena 方式: ~{} KB", arena_memory / 1024);

    let memory_efficiency = traditional_memory as f64 / arena_memory as f64;
    println!("     内存效率提升: {:.2}x", memory_efficiency);
}
