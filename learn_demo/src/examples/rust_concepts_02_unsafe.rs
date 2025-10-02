// Rust 概念学习 02: 不安全 Rust (Unsafe)
// 运行方式：cd learn_docs/examples && cargo run --bin rust_concepts_02_unsafe

use oxc_allocator::Allocator;
use std::ptr;

fn main() {
    println!("🦀 Rust 概念学习：不安全 Rust (Unsafe)");
    println!("{}", "=".repeat(50));

    // 理解为什么需要 unsafe
    why_unsafe_is_needed();

    // Arena 分配器中的 unsafe 使用
    unsafe_in_arena_allocator();

    // 编译时检查 vs 运行时检查
    compile_time_vs_runtime_checks();

    // 内存安全的保证
    memory_safety_guarantees();

    // 正确使用 unsafe 的原则
    unsafe_best_practices();

    println!("\n🎉 Unsafe Rust 学习完成！");
}

fn why_unsafe_is_needed() {
    println!("\n📚 1. 为什么需要 unsafe？");

    let allocator = Allocator::default();

    // Rust 的安全保证有时过于严格
    println!("   Rust 的安全检查有时会阻止合法但复杂的操作");

    // 示例：直接内存操作
    let data = allocator.alloc([1, 2, 3, 4, 5]);
    println!("   原始数据: {:?}", data);

    // 安全的方式访问
    println!("   安全访问 data[0]: {}", data[0]);

    // 有时我们需要更底层的控制
    demonstrate_low_level_access(data);
}

fn demonstrate_low_level_access(data: &mut [i32; 5]) {
    // 获取原始指针（这是安全的）
    let ptr = data.as_mut_ptr();

    println!("   原始指针地址: {:p}", ptr);

    // 使用 unsafe 进行原始指针操作
    unsafe {
        // 直接通过指针修改数据
        *ptr = 100;
        *ptr.add(1) = 200;
    }

    println!("   修改后的数据: {:?}", data);
    println!("   🎯 unsafe 允许我们进行底层内存操作");
}

fn unsafe_in_arena_allocator() {
    println!("\n📚 2. Arena 分配器中的 unsafe 使用");

    let allocator = Allocator::default();

    // 演示 Arena 分配器内部可能的 unsafe 操作
    demonstrate_arena_internals(&allocator);

    // 类型安全检查
    demonstrate_type_safety_checks(&allocator);
}

fn demonstrate_arena_internals(allocator: &Allocator) {
    println!("   Arena 分配器内部的 unsafe 操作：");

    // 分配一些数据
    let data1 = allocator.alloc(42i32);
    let data2 = allocator.alloc(84i32);
    let data3 = allocator.alloc(126i32);

    println!("   分配的数据: {}, {}, {}", data1, data2, data3);

    // 观察内存布局
    let addr1 = data1 as *const i32 as usize;
    let addr2 = data2 as *const i32 as usize;
    let addr3 = data3 as *const i32 as usize;

    println!("   内存地址:");
    println!("     data1: 0x{:x}", addr1);
    println!("     data2: 0x{:x}", addr2);
    println!("     data3: 0x{:x}", addr3);

    // 验证内存连续性
    let diff1 = addr2.abs_diff(addr1);
    let diff2 = addr3.abs_diff(addr2);
    println!("   地址差: {} bytes, {} bytes", diff1, diff2);

    // 这种底层内存分析需要 unsafe 操作
    unsafe_memory_analysis(data1, data2, data3);
}

unsafe fn unsafe_memory_analysis(data1: &i32, data2: &i32, data3: &i32) {
    println!("   🔍 Unsafe 内存分析:");

    // 将引用转换为原始指针
    let ptr1 = data1 as *const i32;
    let ptr2 = data2 as *const i32;
    let ptr3 = data3 as *const i32;

    // 计算指针偏移（这需要 unsafe）
    let offset1to2 = ptr2.offset_from(ptr1);
    let offset2to3 = ptr3.offset_from(ptr2);

    println!("     指针偏移: {}, {}", offset1to2, offset2to3);

    // 验证数据完整性
    assert_eq!(*ptr1, 42);
    assert_eq!(*ptr2, 84);
    assert_eq!(*ptr3, 126);

    println!("     ✅ 数据完整性验证通过");
}

fn demonstrate_type_safety_checks(allocator: &Allocator) {
    println!("   类型安全检查:");

    // 这些类型可以安全分配（不需要 Drop）
    let _safe_int = allocator.alloc(42);
    let _safe_array = allocator.alloc([1, 2, 3]);
    let _safe_tuple = allocator.alloc((1, 2, 3));

    println!("     ✅ 基本类型分配成功");

    // 编译时检查：这些类型不能分配（需要 Drop）
    // let _unsafe_vec = allocator.alloc(Vec::new()); // 编译错误！
    // let _unsafe_string = allocator.alloc(String::new()); // 编译错误！

    println!("     ✅ 编译时类型安全检查生效");

    // 演示编译时常量检查
    demonstrate_const_checks();
}

fn demonstrate_const_checks() {
    println!("   编译时常量检查:");

    // 这些检查在编译时进行
    const SAFE_TYPE_CHECK: bool = !std::mem::needs_drop::<i32>();
    const UNSAFE_TYPE_CHECK: bool = std::mem::needs_drop::<Vec<i32>>();

    println!("     i32 需要 Drop: {}", !SAFE_TYPE_CHECK);
    println!("     Vec<i32> 需要 Drop: {}", UNSAFE_TYPE_CHECK);

    // 编译时断言
    const _: () = assert!(SAFE_TYPE_CHECK, "i32 should not need drop");
    const _: () = assert!(UNSAFE_TYPE_CHECK, "Vec<i32> should need drop");

    println!("     ✅ 编译时断言通过");
}

fn compile_time_vs_runtime_checks() {
    println!("\n📚 3. 编译时检查 vs 运行时检查");

    let allocator = Allocator::default();

    // 编译时检查的例子
    demonstrate_compile_time_checks(&allocator);

    // 运行时检查的例子
    demonstrate_runtime_checks(&allocator);
}

fn demonstrate_compile_time_checks(allocator: &Allocator) {
    println!("   编译时检查:");

    // 这些检查在编译时完成，没有运行时开销
    let data = allocator.alloc(42);

    // 编译器知道这些信息
    println!("     数据大小: {} bytes", std::mem::size_of_val(data));
    println!("     数据对齐: {} bytes", std::mem::align_of_val(data));
    println!("     需要 Drop: {}", std::mem::needs_drop_val(data));

    // 这些都是零成本抽象
    println!("     ✅ 零成本的编译时检查");
}

fn demonstrate_runtime_checks(allocator: &Allocator) {
    println!("   运行时检查:");

    // 创建一个数组
    let array = allocator.alloc([1, 2, 3, 4, 5]);

    // 安全的数组访问（有边界检查）
    for i in 0..array.len() {
        println!("     array[{}] = {}", i, array[i]);
    }

    // 不安全的数组访问（无边界检查）
    unsafe {
        println!("   Unsafe 数组访问:");
        let ptr = array.as_ptr();
        for i in 0..5 {
            println!("     *ptr.add({}) = {}", i, *ptr.add(i));
        }
    }

    println!("     ✅ 运行时检查 vs 无检查对比");
}

fn memory_safety_guarantees() {
    println!("\n📚 4. 内存安全保证");

    let allocator = Allocator::default();

    // Rust 的内存安全保证
    demonstrate_memory_safety(&allocator);

    // Arena 分配器的额外保证
    demonstrate_arena_safety(&allocator);
}

fn demonstrate_memory_safety(allocator: &Allocator) {
    println!("   Rust 的内存安全保证:");

    let data = allocator.alloc(42);

    // 1. 没有空指针解引用
    println!("     1. 引用永远不为空: {}", data);

    // 2. 没有悬垂指针
    {
        let local_data = allocator.alloc(100);
        println!("     2. 局部数据: {}", local_data);
        // local_data 的生命周期与 allocator 绑定，不会悬垂
    }

    // 3. 没有缓冲区溢出（在安全代码中）
    let array = allocator.alloc([1, 2, 3]);
    // array[10]; // 这会 panic，而不是未定义行为

    println!("     3. 数组访问受保护: {:?}", array);

    // 4. 没有数据竞争（在单线程中自动保证）
    println!("     4. 单线程中无数据竞争");

    println!("     ✅ 内存安全保证有效");
}

fn demonstrate_arena_safety(allocator: &Allocator) {
    println!("   Arena 分配器的额外安全保证:");

    // 1. 统一的生命周期管理
    let data1 = allocator.alloc(1);
    let data2 = allocator.alloc(2);
    let data3 = allocator.alloc(3);

    println!("     1. 统一生命周期: {}, {}, {}", data1, data2, data3);

    // 2. 无内存泄漏（整个 arena 一起释放）
    println!("     2. 无内存泄漏保证");

    // 3. 缓存友好的内存布局
    println!("     3. 缓存友好的连续内存");

    // 4. 无碎片化
    println!("     4. 无内存碎片");

    println!("     ✅ Arena 额外安全保证");
}

fn unsafe_best_practices() {
    println!("\n📚 5. Unsafe 最佳实践");

    let allocator = Allocator::default();

    // 最佳实践 1: 最小化 unsafe 块
    demonstrate_minimal_unsafe(&allocator);

    // 最佳实践 2: 清晰的安全不变量
    demonstrate_safety_invariants(&allocator);

    // 最佳实践 3: 文档化假设
    demonstrate_documented_assumptions(&allocator);
}

fn demonstrate_minimal_unsafe(allocator: &Allocator) {
    println!("   最佳实践 1: 最小化 unsafe 块");

    let data = allocator.alloc([1, 2, 3, 4, 5]);

    // 好的做法：只在必要时使用 unsafe
    let result = safe_wrapper_for_unsafe_operation(data);
    println!("     安全包装的结果: {}", result);

    // 坏的做法：整个函数都是 unsafe
    // unsafe fn bad_entire_function_unsafe() { ... }

    println!("     ✅ 最小化 unsafe 使用");
}

fn safe_wrapper_for_unsafe_operation(data: &[i32; 5]) -> i32 {
    // 安全的前置检查
    if data.is_empty() {
        return 0;
    }

    // 最小的 unsafe 块
    unsafe {
        // 我们知道数组不为空，所以这是安全的
        *data.get_unchecked(0)
    }
}

fn demonstrate_safety_invariants(allocator: &Allocator) {
    println!("   最佳实践 2: 清晰的安全不变量");

    let data = allocator.alloc([1, 2, 3, 4, 5]);

    // 安全不变量：索引必须在有效范围内
    let index = 2;
    let value = unsafe_get_with_invariant(data, index);
    println!("     安全访问 data[{}] = {}", index, value);

    println!("     ✅ 安全不变量明确");
}

/// 安全不变量：index 必须 < data.len()
unsafe fn unsafe_get_with_invariant(data: &[i32], index: usize) -> i32 {
    // 调用者必须保证 index < data.len()
    debug_assert!(index < data.len(), "Index out of bounds");
    *data.get_unchecked(index)
}

fn demonstrate_documented_assumptions(allocator: &Allocator) {
    println!("   最佳实践 3: 文档化假设");

    let data = allocator.alloc([1, 2, 3, 4, 5]);

    // 有文档的 unsafe 函数
    let sum = unsafe_sum_array(data);
    println!("     数组和: {}", sum);

    println!("     ✅ 假设已文档化");
}

/// 计算数组的和
///
/// # Safety
///
/// 调用者必须保证：
/// - `data` 指向有效的内存
/// - `data` 包含至少 `len` 个有效的 i32 值
/// - 内存在函数调用期间不会被修改
unsafe fn unsafe_sum_array(data: &[i32]) -> i32 {
    let mut sum = 0;
    let ptr = data.as_ptr();

    for i in 0..data.len() {
        sum += *ptr.add(i);
    }

    sum
}

// 高级 unsafe 概念演示
fn advanced_unsafe_concepts() {
    println!("\n📚 6. 高级 Unsafe 概念");

    let allocator = Allocator::default();

    // 原始指针操作
    demonstrate_raw_pointers(&allocator);

    // 内存传输
    demonstrate_memory_transmutation(&allocator);
}

fn demonstrate_raw_pointers(allocator: &Allocator) {
    println!("   原始指针操作:");

    let data = allocator.alloc(42i32);

    // 获取原始指针
    let raw_ptr: *const i32 = data;
    let mut_ptr: *mut i32 = data as *const i32 as *mut i32;

    println!("     原始指针: {:p}", raw_ptr);
    println!("     可变指针: {:p}", mut_ptr);

    unsafe {
        // 通过原始指针读取
        let value = *raw_ptr;
        println!("     读取值: {}", value);

        // 通过可变指针写入
        *mut_ptr = 100;
        println!("     修改后: {}", *data);
    }
}

fn demonstrate_memory_transmutation(allocator: &Allocator) {
    println!("   内存传输 (transmutation):");

    let int_data = allocator.alloc(0x41424344u32);

    unsafe {
        // 将 u32 重新解释为 [u8; 4]
        let bytes: [u8; 4] = std::mem::transmute(*int_data);
        println!("     u32 as bytes: {:?}", bytes);

        // 注意：这种操作需要极其小心！
        println!("     ⚠️  transmute 是极其危险的操作");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_unsafe_operations() {
        let allocator = Allocator::default();
        let data = allocator.alloc([1, 2, 3, 4, 5]);

        // 测试安全包装
        let result = safe_wrapper_for_unsafe_operation(data);
        assert_eq!(result, 1);

        // 测试 unsafe 求和
        let sum = unsafe { unsafe_sum_array(data) };
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_memory_layout() {
        let allocator = Allocator::default();
        let data1 = allocator.alloc(1i32);
        let data2 = allocator.alloc(2i32);

        // 验证内存连续性
        let addr1 = data1 as *const i32 as usize;
        let addr2 = data2 as *const i32 as usize;
        let diff = addr2.abs_diff(addr1);

        // 应该相差一个 i32 的大小
        assert_eq!(diff, std::mem::size_of::<i32>());
    }
}


