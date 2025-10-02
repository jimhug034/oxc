// 第一个示例：oxc_allocator 基础使用
// 运行方式：cd learn_docs/examples && cargo run --bin 01_allocator_basics

use oxc_allocator::Allocator;

fn main() {
    println!("🎯 oxc_allocator 基础使用示例");
    println!("{}", "=".repeat(50));

    // 1. 创建分配器
    let allocator = Allocator::default();
    println!("✅ 创建了一个新的 Allocator");

    // 2. 分配基本数据类型
    basic_allocation(&allocator);

    // 3. 观察内存地址和连续性
    memory_layout_observation(&allocator);

    // 4. 字符串分配
    string_allocation(&allocator);

    println!("\n🎉 基础示例完成！");
}

fn basic_allocation(allocator: &Allocator) {
    println!("\n📦 基本数据类型分配:");

    // 分配不同类型的数据
    let number = allocator.alloc(42i32);
    let float_num = allocator.alloc(3.14f64);
    let boolean = allocator.alloc(true);
    let character = allocator.alloc('A');

    println!("   整数: {}", number);
    println!("   浮点数: {}", float_num);
    println!("   布尔值: {}", boolean);
    println!("   字符: {}", character);

    // 分配数组
    let array = allocator.alloc([1, 2, 3, 4, 5]);
    println!("   数组: {:?}", array);

    // 分配结构体
    #[derive(Debug)]
    struct Point {
        x: i32,
        y: i32,
    }

    let point = allocator.alloc(Point { x: 10, y: 20 });
    println!("   结构体: {:?}", point);
}

fn memory_layout_observation(allocator: &Allocator) {
    println!("\n🔍 内存地址观察:");

    // 连续分配相同类型的数据
    let data1 = allocator.alloc(100u64);
    let data2 = allocator.alloc(200u64);
    let data3 = allocator.alloc(300u64);
    let data4 = allocator.alloc(400u64);

    println!("   连续分配的 u64 数据:");
    println!("     data1 (100): {:p}", data1);
    println!("     data2 (200): {:p}", data2);
    println!("     data3 (300): {:p}", data3);
    println!("     data4 (400): {:p}", data4);

    // 计算地址差
    let addr1 = data1 as *const u64 as usize;
    let addr2 = data2 as *const u64 as usize;
    let addr3 = data3 as *const u64 as usize;
    let addr4 = data4 as *const u64 as usize;

    println!("   地址差分析:");
    println!("     data2 - data1: {} bytes", addr2.abs_diff(addr1));
    println!("     data3 - data2: {} bytes", addr3.abs_diff(addr2));
    println!("     data4 - data3: {} bytes", addr4.abs_diff(addr3));
    println!("     u64 大小: {} bytes", std::mem::size_of::<u64>());

    // 验证内存连续性（可能是正向或反向）
    let diff1 = addr2.abs_diff(addr1);
    let diff2 = addr3.abs_diff(addr2);
    let diff3 = addr4.abs_diff(addr3);
    let u64_size = std::mem::size_of::<u64>();
    let is_continuous = diff1 == u64_size && diff2 == u64_size && diff3 == u64_size;

    println!("   内存是否连续: {}", if is_continuous { "✅ 是" } else { "❌ 否" });
}

fn string_allocation(allocator: &Allocator) {
    println!("\n📝 字符串分配:");

    // 使用 alloc_str 分配字符串
    let greeting = allocator.alloc_str("Hello, Oxc!");
    let name = allocator.alloc_str("Arena Allocator");
    let description = allocator.alloc_str("高性能内存分配器");

    println!("   字符串内容:");
    println!("     greeting: \"{}\"", greeting);
    println!("     name: \"{}\"", name);
    println!("     description: \"{}\"", description);

    println!("   字符串地址:");
    println!("     greeting: {:p}", greeting.as_ptr());
    println!("     name: {:p}", name.as_ptr());
    println!("     description: {:p}", description.as_ptr());

    // 字符串长度信息
    println!("   字符串长度:");
    println!("     greeting: {} bytes", greeting.len());
    println!("     name: {} bytes", name.len());
    println!("     description: {} bytes", description.len());
}
