// 第六个示例：高级特性和实用技巧
// 运行方式：cd learn_docs/examples && cargo run --bin 06_advanced_features

use oxc_allocator::{Allocator, Vec as ArenaVec, HashMap as ArenaHashMap, Box as ArenaBox};
use std::time::Instant;

fn main() {
    println!("🔬 oxc_allocator 高级特性和实用技巧");
    println!("{}", "=".repeat(50));

    // 内存对齐演示
    memory_alignment_demo();

    // 大对象分配策略
    large_object_allocation();

    // 字符串构建器使用
    string_builder_demo();

    // 自定义分配器模式
    custom_allocator_patterns();

    // 错误处理和边界情况
    error_handling_demo();

    // 最佳实践总结
    best_practices_demo();

    println!("\n🎉 高级特性示例完成！");
}

fn memory_alignment_demo() {
    println!("\n🎯 内存对齐演示:");

    let allocator = Allocator::default();

    // 分配不同对齐要求的数据类型
    let byte_data = allocator.alloc(0x42u8);
    let short_data = allocator.alloc(0x1234u16);
    let int_data = allocator.alloc(0x12345678u32);
    let long_data = allocator.alloc(0x123456789ABCDEFu64);

    println!("   不同类型的内存地址:");
    println!("     u8  (1 byte):  {:p} (对齐: {})", byte_data, byte_data as *const u8 as usize % 1);
    println!("     u16 (2 bytes): {:p} (对齐: {})", short_data, short_data as *const u16 as usize % 2);
    println!("     u32 (4 bytes): {:p} (对齐: {})", int_data, int_data as *const u32 as usize % 4);
    println!("     u64 (8 bytes): {:p} (对齐: {})", long_data, long_data as *const u64 as usize % 8);

    // 验证对齐
    let u16_aligned = (short_data as *const u16 as usize) % 2 == 0;
    let u32_aligned = (int_data as *const u32 as usize) % 4 == 0;
    let u64_aligned = (long_data as *const u64 as usize) % 8 == 0;

    println!("   对齐验证:");
    println!("     u16 正确对齐: {}", if u16_aligned { "✅" } else { "❌" });
    println!("     u32 正确对齐: {}", if u32_aligned { "✅" } else { "❌" });
    println!("     u64 正确对齐: {}", if u64_aligned { "✅" } else { "❌" });

    // 结构体对齐
    #[repr(C)]
    struct AlignedStruct {
        a: u8,
        b: u32,
        c: u16,
    }

    let struct_data = allocator.alloc(AlignedStruct { a: 1, b: 2, c: 3 });
    let struct_aligned = (struct_data as *const AlignedStruct as usize) % std::mem::align_of::<AlignedStruct>() == 0;

    println!("   结构体对齐:");
    println!("     AlignedStruct 地址: {:p}", struct_data);
    println!("     对齐要求: {} bytes", std::mem::align_of::<AlignedStruct>());
    println!("     正确对齐: {}", if struct_aligned { "✅" } else { "❌" });
}

fn large_object_allocation() {
    println!("\n📦 大对象分配策略:");

    let allocator = Allocator::default();

    // 分配不同大小的对象
    let sizes = [
        ("小对象", 64),
        ("中对象", 1024),
        ("大对象", 64 * 1024),
        ("超大对象", 1024 * 1024),
    ];

    let mut allocations = Vec::new();

    for (name, size) in sizes {
        println!("   分配 {} ({} bytes):", name, size);

        let start = Instant::now();
        let data = allocator.alloc(vec![0u8; size]);
        let alloc_time = start.elapsed();

        println!("     地址: {:p}", data.as_ptr());
        println!("     分配耗时: {:?}", alloc_time);

        allocations.push((name, data.as_ptr()));
    }

    // 分析地址分布
    println!("   地址分布分析:");
    for i in 1..allocations.len() {
        let (prev_name, prev_addr) = allocations[i-1];
        let (curr_name, curr_addr) = allocations[i];

        let addr_diff = curr_addr as usize - prev_addr as usize;
        println!("     {} 到 {} 的地址差: {} bytes",
                 prev_name, curr_name, addr_diff);
    }

    // 测试连续分配大对象的性能
    println!("   连续分配性能测试:");
    let start = Instant::now();
    let mut large_objects = Vec::new();

    for i in 0..100 {
        let obj = allocator.alloc(vec![i as u8; 4096]); // 4KB 对象
        large_objects.push(obj);
    }

    let batch_time = start.elapsed();
    println!("     连续分配 100 个 4KB 对象耗时: {:?}", batch_time);
    println!("     平均每个对象: {:?}", batch_time / 100);
}

fn string_builder_demo() {
    println!("\n📝 字符串构建演示:");

    let allocator = Allocator::default();

    // 使用 alloc_str 构建字符串
    println!("   基本字符串分配:");
    let greeting = allocator.alloc_str("Hello");
    let target = allocator.alloc_str("Arena");
    let punctuation = allocator.alloc_str("!");

    println!("     部分 1: \"{}\" at {:p}", greeting, greeting.as_ptr());
    println!("     部分 2: \"{}\" at {:p}", target, target.as_ptr());
    println!("     部分 3: \"{}\" at {:p}", punctuation, punctuation.as_ptr());

    // 构建复杂字符串
    println!("   复杂字符串构建:");
    let mut parts = ArenaVec::new_in(&allocator);

    for i in 0..10 {
        let part = allocator.alloc_str(&format!("Part_{}", i));
        parts.push(part);
    }

    println!("     构建了 {} 个字符串部分", parts.len());
    for (i, part) in parts.iter().enumerate() {
        println!("       {}: \"{}\"", i, part);
    }

    // 字符串拼接模拟
    println!("   字符串拼接模拟:");
    let base = "Generated code: ";
    let mut full_strings = ArenaVec::new_in(&allocator);

    for i in 0..5 {
        let full_string = allocator.alloc_str(&format!("{}{}", base, i));
        full_strings.push(full_string);
    }

    for (i, s) in full_strings.iter().enumerate() {
        println!("     字符串 {}: \"{}\"", i, s);
    }

    // 长字符串处理
    println!("   长字符串处理:");
    let long_content = "A".repeat(10000);
    let start = Instant::now();
    let long_string = allocator.alloc_str(&long_content);
    let alloc_time = start.elapsed();

    println!("     长度: {} 字符", long_string.len());
    println!("     分配耗时: {:?}", alloc_time);
    println!("     前 50 字符: \"{}...\"", &long_string[..50]);
}

fn custom_allocator_patterns() {
    println!("\n🎨 自定义分配器模式:");

    // 模式 1: 分层分配器
    hierarchical_allocation_pattern();

    // 模式 2: 类型化分配器
    typed_allocation_pattern();

    // 模式 3: 批量分配模式
    batch_allocation_pattern();
}

fn hierarchical_allocation_pattern() {
    println!("   模式 1: 分层分配器");

    // 为不同的处理阶段使用不同的分配器
    struct CompilerPhases {
        lexer_allocator: Allocator,
        parser_allocator: Allocator,
        semantic_allocator: Allocator,
    }

    let phases = CompilerPhases {
        lexer_allocator: Allocator::default(),
        parser_allocator: Allocator::default(),
        semantic_allocator: Allocator::default(),
    };

    // 词法分析阶段
    let mut tokens = ArenaVec::new_in(&phases.lexer_allocator);
    for i in 0..100 {
        let token = phases.lexer_allocator.alloc_str(&format!("token_{}", i));
        tokens.push(token);
    }

    // 语法分析阶段
    let mut ast_nodes = ArenaVec::new_in(&phases.parser_allocator);
    for i in 0..50 {
        let node = phases.parser_allocator.alloc_str(&format!("ast_node_{}", i));
        ast_nodes.push(node);
    }

    // 语义分析阶段
    let mut symbols = ArenaHashMap::new_in(&phases.semantic_allocator);
    for i in 0..25 {
        let name = phases.semantic_allocator.alloc_str(&format!("symbol_{}", i));
        let type_info = phases.semantic_allocator.alloc_str(&format!("type_{}", i));
        symbols.insert(name, type_info);
    }

    println!("     词法分析: {} tokens", tokens.len());
    println!("     语法分析: {} AST 节点", ast_nodes.len());
    println!("     语义分析: {} 符号", symbols.len());
    println!("     优势: 每个阶段可以独立管理和释放内存");
}

fn typed_allocation_pattern() {
    println!("   模式 2: 类型化分配器");

    // 为特定类型创建专门的分配函数
    struct TypedAllocator<'a> {
        allocator: &'a Allocator,
    }

    impl<'a> TypedAllocator<'a> {
        fn new(allocator: &'a Allocator) -> Self {
            Self { allocator }
        }

        fn alloc_identifier(&self, name: &str) -> &'a str {
            self.allocator.alloc_str(&format!("id:{}", name))
        }

        fn alloc_literal(&self, value: &str) -> &'a str {
            self.allocator.alloc_str(&format!("lit:{}", value))
        }

        fn alloc_operator(&self, op: &str) -> &'a str {
            self.allocator.alloc_str(&format!("op:{}", op))
        }
    }

    let allocator = Allocator::default();
    let typed_alloc = TypedAllocator::new(&allocator);

    let identifiers = [
        typed_alloc.alloc_identifier("variable"),
        typed_alloc.alloc_identifier("function"),
        typed_alloc.alloc_identifier("class"),
    ];

    let literals = [
        typed_alloc.alloc_literal("42"),
        typed_alloc.alloc_literal("\"hello\""),
        typed_alloc.alloc_literal("true"),
    ];

    let operators = [
        typed_alloc.alloc_operator("+"),
        typed_alloc.alloc_operator("-"),
        typed_alloc.alloc_operator("*"),
    ];

    println!("     标识符: {:?}", identifiers);
    println!("     字面量: {:?}", literals);
    println!("     操作符: {:?}", operators);
    println!("     优势: 类型安全，语义清晰");
}

fn batch_allocation_pattern() {
    println!("   模式 3: 批量分配模式");

    let allocator = Allocator::default();

    // 批量分配相同类型的对象
    fn batch_alloc_numbers(allocator: &Allocator, count: usize) -> ArenaVec<&i32> {
        let mut numbers = ArenaVec::new_in(allocator);
        for i in 0..count {
            numbers.push(allocator.alloc(i as i32));
        }
        numbers
    }

    fn batch_alloc_strings(allocator: &Allocator, prefix: &str, count: usize) -> ArenaVec<&str> {
        let mut strings = ArenaVec::new_in(allocator);
        for i in 0..count {
            strings.push(allocator.alloc_str(&format!("{}_{}", prefix, i)));
        }
        strings
    }

    let start = Instant::now();
    let numbers = batch_alloc_numbers(&allocator, 1000);
    let number_time = start.elapsed();

    let start = Instant::now();
    let strings = batch_alloc_strings(&allocator, "item", 1000);
    let string_time = start.elapsed();

    println!("     批量分配 1000 个数字: {:?}", number_time);
    println!("     批量分配 1000 个字符串: {:?}", string_time);
    println!("     数字范围: {} 到 {}", numbers[0], numbers[999]);
    println!("     字符串示例: \"{}\" 到 \"{}\"", strings[0], strings[999]);
    println!("     优势: 高效的批量操作，内存局部性好");
}

fn error_handling_demo() {
    println!("\n⚠️ 错误处理和边界情况:");

    let allocator = Allocator::default();

    // 大量分配测试
    println!("   大量分配测试:");
    let start = Instant::now();
    let mut large_allocation_test = Vec::new();

    for i in 0..10000 {
        let data = allocator.alloc(vec![i as u8; 100]);
        large_allocation_test.push(data);
    }

    let mass_alloc_time = start.elapsed();
    println!("     分配 10,000 个 100-byte 对象: {:?}", mass_alloc_time);

    // 零大小分配
    println!("   零大小类型分配:");
    #[derive(Debug)]
    struct ZeroSized;

    let zero_sized = allocator.alloc(ZeroSized);
    println!("     ZeroSized 地址: {:p}", zero_sized);

    // 空字符串分配
    println!("   空字符串分配:");
    let empty_string = allocator.alloc_str("");
    println!("     空字符串长度: {}", empty_string.len());
    println!("     空字符串地址: {:p}", empty_string.as_ptr());

    // 非常大的对象分配
    println!("   大对象分配测试:");
    let start = Instant::now();
    let huge_array = allocator.alloc([0u8; 1024 * 1024]); // 1MB
    let huge_alloc_time = start.elapsed();

    println!("     1MB 数组分配耗时: {:?}", huge_alloc_time);
    println!("     1MB 数组地址: {:p}", huge_array.as_ptr());

    // 内存使用估算
    println!("   内存使用估算:");
    let estimated_usage =
        10000 * 100 +  // 大量分配测试
        1024 * 1024 +  // 1MB 数组
        1000;          // 其他小对象

    println!("     估算总内存使用: ~{} MB", estimated_usage / (1024 * 1024));
}

fn best_practices_demo() {
    println!("\n💡 最佳实践总结:");

    println!("   ✅ 推荐做法:");
    println!("     1. 重用 Allocator 实例，避免频繁创建/销毁");
    println!("     2. 在适当的时候使用 reset() 释放内存");
    println!("     3. 为不同的处理阶段使用不同的 Allocator");
    println!("     4. 利用 Arena 数据结构 (ArenaVec, ArenaHashMap)");
    println!("     5. 批量分配相同类型的对象");

    println!("   ❌ 避免的做法:");
    println!("     1. 在循环中创建新的 Allocator");
    println!("     2. 忘记在长时间运行的程序中调用 reset()");
    println!("     3. 混合使用 Arena 和标准分配器");
    println!("     4. 在 Arena 中存储需要 Drop 的类型");

    // 实际的最佳实践示例
    demonstrate_best_practices();
}

fn demonstrate_best_practices() {
    println!("   🎯 最佳实践示例:");

    // 好的模式：重用 Allocator
    let mut allocator = Allocator::default();

    for batch in 0..3 {
        println!("     批次 {}: 处理数据", batch + 1);

        // 处理一批数据
        let mut batch_data = ArenaVec::new_in(&allocator);
        for i in 0..1000 {
            let item = allocator.alloc_str(&format!("batch_{}_item_{}", batch, i));
            batch_data.push(item);
        }

        // 模拟处理
        let processed_count = batch_data.len();
        println!("       处理了 {} 个项目", processed_count);

        // 处理完成后重置，为下一批准备
        allocator.reset();
        println!("       内存已重置");
    }

    // 展示生命周期管理
    {
        let scoped_allocator = Allocator::default();
        let _scoped_data = scoped_allocator.alloc("作用域数据");
        println!("     作用域内分配数据");
        // scoped_allocator 在这里自动释放
    }
    println!("     作用域结束，数据自动清理");

    // 展示类型安全的使用
    let type_safe_allocator = Allocator::default();

    #[derive(Debug)]
    struct SafeData<'a> {
        content: &'a str,
        metadata: ArenaHashMap<'a, &'a str, &'a str>,
    }

    let safe_data = type_safe_allocator.alloc(SafeData {
        content: type_safe_allocator.alloc_str("安全的数据"),
        metadata: ArenaHashMap::new_in(&type_safe_allocator),
    });

    println!("     类型安全的数据: {:?}", safe_data.content);

    println!("   🎉 遵循这些最佳实践，你就能充分发挥 Arena 分配器的优势！");
}
