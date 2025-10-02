// 第四个示例：内存管理和生命周期
// 运行方式：cd learn_docs/examples && cargo run --bin 04_memory_management

use oxc_allocator::{Allocator, Vec as ArenaVec, HashMap as ArenaHashMap};
use std::time::Instant;

fn main() {
    println!("⏰ 内存管理和生命周期示例");
    println!("{}", "=".repeat(50));

    // Allocator 重置功能
    allocator_reset_demo();

    // 生命周期演示
    lifetime_demo();

    // 内存增长和管理
    memory_growth_demo();

    // 批处理场景
    batch_processing_demo();

    // 内存使用统计
    memory_usage_stats();

    println!("\n🎉 内存管理示例完成！");
}

fn allocator_reset_demo() {
    println!("\n🔄 Allocator Reset 功能演示:");

    let mut allocator = Allocator::default();

    // 第一轮分配
    println!("   第一轮分配:");
    let mut first_round = Vec::new();
    for i in 0..1000 {
        let data = allocator.alloc(format!("数据_{}", i));
        first_round.push(data);
    }
    println!("     分配了 1000 个字符串");
    println!("     第一个字符串: {}", first_round[0]);
    println!("     最后一个字符串: {}", first_round[999]);

    // 重置分配器
    println!("   重置分配器...");
    allocator.reset();
    println!("     ✅ reset() 调用完成");

    // 注意：此时 first_round 中的引用已经无效，不能再使用
    // 这就是为什么 Rust 的生命周期系统会阻止我们这样做

    // 第二轮分配
    println!("   第二轮分配:");
    let mut second_round = Vec::new();
    for i in 0..500 {
        let data = allocator.alloc(format!("新数据_{}", i));
        second_round.push(data);
    }
    println!("     分配了 500 个新字符串");
    println!("     第一个新字符串: {}", second_round[0]);
    println!("     最后一个新字符串: {}", second_round[499]);

    println!("   🎯 重点：reset() 后内存被重用，分配速度依然很快！");
}

fn lifetime_demo() {
    println!("\n⏰ 生命周期演示:");

    // 演示作用域和生命周期
    let outer_data = {
        let allocator = Allocator::default();
        let data = allocator.alloc("作用域内的数据");

        println!("   作用域内:");
        println!("     数据内容: {}", data);
        println!("     数据地址: {:p}", data);

        // 这里我们不能返回 data，因为它的生命周期绑定到 allocator
        // 当 allocator 被 drop 时，data 也会失效

        "作用域结束"
    };

    println!("   作用域外:");
    println!("     外部数据: {}", outer_data);
    println!("     🎯 allocator 和其中的数据已经被释放");

    // 正确的使用方式：让 allocator 的生命周期足够长
    let allocator = Allocator::default();
    let long_lived_data = process_with_allocator(&allocator);

    println!("   正确的生命周期管理:");
    println!("     处理结果: {:?}", long_lived_data);
}

fn process_with_allocator(allocator: &Allocator) -> ArenaVec<&str> {
    let mut results = ArenaVec::new_in(allocator);

    // 模拟一些处理过程
    let words = ["Hello", "Arena", "Allocator", "World"];
    for word in words {
        let processed = allocator.alloc_str(&format!("处理过的_{}", word));
        results.push(processed);
    }

    results
}

fn memory_growth_demo() {
    println!("\n📈 内存增长演示:");

    let allocator = Allocator::default();

    // 模拟内存逐渐增长的场景
    println!("   逐步分配大量数据:");

    let mut total_allocated = 0;
    let chunk_size = 10_000;

    for round in 1..=5 {
        let start = Instant::now();

        // 每轮分配一定数量的数据
        for i in 0..chunk_size {
            let data = allocator.alloc(format!("Round_{}_Item_{}", round, i));
            total_allocated += data.len();
        }

        let elapsed = start.elapsed();

        println!("     第 {} 轮: 分配 {} 个对象，耗时 {:?}",
                 round, chunk_size, elapsed);
        println!("       累计分配: ~{} KB", total_allocated / 1024);
    }

    println!("   🎯 观察：即使分配了大量数据，速度依然保持稳定！");

    // 演示内存块的概念
    demonstrate_memory_chunks(&allocator);
}

fn demonstrate_memory_chunks(allocator: &Allocator) {
    println!("\n🧱 内存块概念演示:");

    // 分配一些小对象
    let small_objects: Vec<_> = (0..10).map(|i| allocator.alloc(i)).collect();

    // 分配一个大对象，可能会触发新的内存块分配
    let large_object = allocator.alloc([0u8; 65536]); // 64KB

    // 再分配一些小对象
    let more_small_objects: Vec<_> = (10..20).map(|i| allocator.alloc(i)).collect();

    println!("   内存地址分析:");
    println!("     前 10 个小对象:");
    for (i, obj) in small_objects.iter().enumerate() {
        println!("       对象 {}: {:p}", i, *obj);
    }

    println!("     大对象 (64KB): {:p}", large_object.as_ptr());

    println!("     后 10 个小对象:");
    for (i, obj) in more_small_objects.iter().enumerate() {
        println!("       对象 {}: {:p}", i + 10, *obj);
    }

    // 分析地址连续性
    let first_addr = small_objects[0] as *const i32 as usize;
    let last_small_addr = small_objects[9] as *const i32 as usize;
    let large_addr = large_object.as_ptr() as usize;
    let new_small_addr = more_small_objects[0] as *const i32 as usize;

    println!("   地址连续性分析:");
    println!("     前 10 个对象是否连续: {}",
             (last_small_addr - first_addr) == 9 * std::mem::size_of::<i32>());
    println!("     大对象后的小对象可能在新的内存块中");

    if new_small_addr > large_addr + 65536 {
        println!("     ✅ 确实，新的小对象在大对象之后的新内存块中");
    }
}

fn batch_processing_demo() {
    println!("\n⚡ 批处理场景演示:");

    let mut allocator = Allocator::default();

    // 模拟处理多个文件的场景
    let files = [
        "config.json",
        "main.js",
        "utils.ts",
        "component.tsx",
        "styles.css"
    ];

    for (index, filename) in files.iter().enumerate() {
        println!("   处理文件 {}: {}", index + 1, filename);

        let start = Instant::now();

        // 模拟文件处理：词法分析、语法分析等
        let file_content = allocator.alloc_str(&format!("文件内容: {}", filename));

        let mut tokens = ArenaVec::new_in(&allocator);
        for token_id in 0..100 { // 假设每个文件有100个token
            let token = allocator.alloc_str(&format!("token_{}_{}", filename, token_id));
            tokens.push(token);
        }

        let mut ast_nodes = ArenaVec::new_in(&allocator);
        for node_id in 0..50 { // 假设每个文件有50个AST节点
            let node = allocator.alloc(format!("ASTNode_{}_{}", filename, node_id));
            ast_nodes.push(node);
        }

        let processing_time = start.elapsed();

        println!("     文件内容: {}", file_content);
        println!("     生成 {} 个 tokens, {} 个 AST 节点",
                 tokens.len(), ast_nodes.len());
        println!("     处理耗时: {:?}", processing_time);

        // 处理完一个文件后重置，释放内存供下一个文件使用
        allocator.reset();
        println!("     ✅ 内存已重置，准备处理下一个文件");
    }

    println!("   🎯 批处理优势:");
    println!("     - 每个文件处理完后立即释放内存");
    println!("     - 内存使用量保持稳定");
    println!("     - 避免了内存泄漏");
    println!("     - 处理速度始终保持高效");
}

fn memory_usage_stats() {
    println!("\n📊 内存使用统计:");

    let allocator = Allocator::default();

    // 分配不同类型和大小的数据
    let mut allocations = Vec::new();

    // 小对象
    for i in 0..1000 {
        allocations.push(("small", allocator.alloc(i)));
    }

    // 中等对象
    for i in 0..100 {
        allocations.push(("medium", allocator.alloc([i; 64])));
    }

    // 大对象
    for i in 0..10 {
        allocations.push(("large", allocator.alloc([i as u8; 4096])));
    }

    // 字符串
    for i in 0..500 {
        let s = allocator.alloc_str(&format!("字符串数据_{}", i));
        allocations.push(("string", s));
    }

    println!("   分配统计:");
    println!("     小对象 (4 bytes): 1000 个");
    println!("     中等对象 (256 bytes): 100 个");
    println!("     大对象 (4KB): 10 个");
    println!("     字符串: 500 个");

    // 计算理论内存使用
    let small_memory = 1000 * 4;
    let medium_memory = 100 * 256;
    let large_memory = 10 * 4096;
    let string_memory = 500 * 20; // 估算平均字符串长度

    let total_estimated = small_memory + medium_memory + large_memory + string_memory;

    println!("   估算内存使用:");
    println!("     小对象: {} bytes", small_memory);
    println!("     中等对象: {} bytes", medium_memory);
    println!("     大对象: {} bytes", large_memory);
    println!("     字符串: ~{} bytes", string_memory);
    println!("     总计: ~{} KB", total_estimated / 1024);

    println!("   🎯 Arena 优势:");
    println!("     - 所有数据在连续内存中");
    println!("     - 没有额外的指针开销");
    println!("     - 一次性释放所有内存");
    println!("     - 极高的缓存命中率");
}
