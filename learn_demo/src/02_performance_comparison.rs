// 第二个示例：性能对比分析
// 运行方式：cd learn_docs/examples && cargo run --bin 02_performance_comparison --release

use oxc_allocator::Allocator;
use std::time::Instant;

fn main() {
    println!("🚀 oxc_allocator 性能对比分析");
    println!("{}", "=".repeat(50));

    // 不同规模的性能测试
    performance_test_small();
    performance_test_medium();
    performance_test_large();

    // 不同数据类型的性能测试
    different_types_performance();

    // 内存使用效率对比
    memory_efficiency_comparison();

    println!("\n🎉 性能对比完成！");
    println!("💡 提示：使用 --release 模式运行可以看到更明显的性能差异");
}

fn performance_test_small() {
    println!("\n📊 小规模测试 (10,000 次分配):");
    const COUNT: usize = 10_000;

    // 传统 Box 分配
    let start = Instant::now();
    let mut boxes = Vec::new();
    for i in 0..COUNT {
        boxes.push(Box::new(i));
    }
    let box_time = start.elapsed();

    // Arena 分配
    let allocator = Allocator::default();
    let start = Instant::now();
    let mut arena_refs = Vec::new();
    for i in 0..COUNT {
        arena_refs.push(allocator.alloc(i));
    }
    let arena_time = start.elapsed();

    print_comparison("小规模", COUNT, box_time, arena_time);
}

fn performance_test_medium() {
    println!("\n📊 中规模测试 (100,000 次分配):");
    const COUNT: usize = 100_000;

    // 传统 Box 分配
    let start = Instant::now();
    let mut boxes = Vec::new();
    for i in 0..COUNT {
        boxes.push(Box::new(i));
    }
    let box_time = start.elapsed();

    // Arena 分配
    let allocator = Allocator::default();
    let start = Instant::now();
    let mut arena_refs = Vec::new();
    for i in 0..COUNT {
        arena_refs.push(allocator.alloc(i));
    }
    let arena_time = start.elapsed();

    print_comparison("中规模", COUNT, box_time, arena_time);
}

fn performance_test_large() {
    println!("\n📊 大规模测试 (1,000,000 次分配):");
    const COUNT: usize = 1_000_000;

    // 传统 Box 分配
    let start = Instant::now();
    let mut boxes = Vec::new();
    for i in 0..COUNT {
        boxes.push(Box::new(i));
    }
    let box_time = start.elapsed();

    // Arena 分配
    let allocator = Allocator::default();
    let start = Instant::now();
    let mut arena_refs = Vec::new();
    for i in 0..COUNT {
        arena_refs.push(allocator.alloc(i));
    }
    let arena_time = start.elapsed();

    print_comparison("大规模", COUNT, box_time, arena_time);
}

fn different_types_performance() {
    println!("\n📊 不同数据类型性能测试:");
    const COUNT: usize = 100_000;

    // 测试小对象 (u8)
    test_type_performance::<u8>("u8 (1 byte)", COUNT, 42);

    // 测试中等对象 (u64)
    test_type_performance::<u64>("u64 (8 bytes)", COUNT, 42);

    // 测试大对象 (数组)
    test_type_performance::<[u8; 64]>("Array (64 bytes)", COUNT, [0; 64]);

    // 测试更大对象
    test_type_performance::<[u8; 1024]>("Array (1KB)", COUNT / 10, [0; 1024]);
}

fn test_type_performance<T: Clone>(type_name: &str, count: usize, value: T) {
    println!("   {} 测试 ({} 次):", type_name, count);

    // Box 分配
    let start = Instant::now();
    let mut boxes = Vec::new();
    for _ in 0..count {
        boxes.push(Box::new(value.clone()));
    }
    let box_time = start.elapsed();

    // Arena 分配
    let allocator = Allocator::default();
    let start = Instant::now();
    let mut arena_refs = Vec::new();
    for _ in 0..count {
        arena_refs.push(allocator.alloc(value.clone()));
    }
    let arena_time = start.elapsed();

    let speedup = box_time.as_nanos() as f64 / arena_time.as_nanos() as f64;
    println!("     Box 耗时: {:?}", box_time);
    println!("     Arena 耗时: {:?}", arena_time);
    println!("     速度提升: {:.2}x", speedup);
}

fn memory_efficiency_comparison() {
    println!("\n💾 内存使用效率对比:");

    const COUNT: usize = 100_000;

    // 计算 Box 方式的内存使用
    let box_memory_per_item = std::mem::size_of::<Box<usize>>();
    let box_total_memory = COUNT * box_memory_per_item;

    println!("   Box 方式:");
    println!("     每个 Box<usize>: {} bytes", box_memory_per_item);
    println!("     {} 个对象总计: {} KB", COUNT, box_total_memory / 1024);
    println!("     额外开销: 每个对象都有指针和堆分配元数据");

    // Arena 方式的内存使用
    let arena_memory_per_item = std::mem::size_of::<usize>();
    let arena_total_memory = COUNT * arena_memory_per_item;

    println!("   Arena 方式:");
    println!("     每个 usize: {} bytes", arena_memory_per_item);
    println!("     {} 个对象总计: {} KB", COUNT, arena_total_memory / 1024);
    println!("     额外开销: 几乎没有，连续存储");

    let memory_efficiency = box_total_memory as f64 / arena_total_memory as f64;
    println!("   内存效率提升: {:.2}x", memory_efficiency);

    // 缓存友好性演示
    cache_friendliness_demo();
}

fn cache_friendliness_demo() {
    println!("\n🧠 缓存友好性演示:");

    const COUNT: usize = 10_000;

    // Box 方式 - 分散的内存访问
    let mut boxes = Vec::new();
    for i in 0..COUNT {
        boxes.push(Box::new(i));
    }

    let start = Instant::now();
    let mut sum = 0;
    for boxed_value in &boxes {
        sum += **boxed_value;
    }
    let box_traverse_time = start.elapsed();

    // Arena 方式 - 连续的内存访问
    let allocator = Allocator::default();
    let mut arena_refs = Vec::new();
    for i in 0..COUNT {
        arena_refs.push(allocator.alloc(i));
    }

    let start = Instant::now();
    let mut sum2 = 0;
    for arena_ref in &arena_refs {
        sum2 += **arena_ref;
    }
    let arena_traverse_time = start.elapsed();

    println!("   遍历 {} 个对象:", COUNT);
    println!("     Box 方式遍历耗时: {:?}", box_traverse_time);
    println!("     Arena 方式遍历耗时: {:?}", arena_traverse_time);

    let traverse_speedup = box_traverse_time.as_nanos() as f64 / arena_traverse_time.as_nanos() as f64;
    println!("     遍历速度提升: {:.2}x", traverse_speedup);

    // 验证结果一致性
    assert_eq!(sum, sum2);
    println!("     ✅ 计算结果一致: {}", sum);
}

fn print_comparison(test_name: &str, count: usize, box_time: std::time::Duration, arena_time: std::time::Duration) {
    let speedup = box_time.as_nanos() as f64 / arena_time.as_nanos() as f64;

    println!("   {} ({} 次分配):", test_name, count);
    println!("     Box 分配耗时: {:?}", box_time);
    println!("     Arena 分配耗时: {:?}", arena_time);
    println!("     速度提升: {:.2}x", speedup);

    // 计算每次分配的平均时间
    let box_avg = box_time.as_nanos() / count as u128;
    let arena_avg = arena_time.as_nanos() / count as u128;

    println!("     平均每次分配:");
    println!("       Box: {} ns", box_avg);
    println!("       Arena: {} ns", arena_avg);
}
