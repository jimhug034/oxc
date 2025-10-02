// Rust 概念学习 01: 生命周期 (Lifetimes)
// 运行方式：cd learn_docs/examples && cargo run --bin rust_concepts_01_lifetimes

use oxc_allocator::Allocator;

fn main() {
    println!("🦀 Rust 概念学习：生命周期 (Lifetimes)");
    println!("{}", "=".repeat(50));

    // 基础生命周期概念
    basic_lifetime_concepts();

    // Arena 分配器中的生命周期
    arena_lifetime_binding();

    // 生命周期省略规则
    lifetime_elision_rules();

    // 多个生命周期参数
    multiple_lifetime_parameters();

    // 静态生命周期
    static_lifetime_examples();

    println!("\n🎉 生命周期学习完成！");
}

fn basic_lifetime_concepts() {
    println!("\n📚 1. 基础生命周期概念");

    let allocator = Allocator::default();

    // 概念 1: 引用的生命周期不能超过被引用的数据
    {
        let data = allocator.alloc(42);
        println!("   分配的数据: {}", data);
        // data 的生命周期与 allocator 绑定
    } // data 在这里仍然有效，因为 allocator 还存在

    // 概念 2: 生命周期注解的作用
    let result = longest_lived_string(&allocator, "hello", "world!");
    println!("   最长的字符串: {}", result);
}

// 生命周期注解示例
fn longest_lived_string<'a>(
    allocator: &'a Allocator,
    s1: &str,
    s2: &str
) -> &'a str {
    // 返回在 allocator 中分配的字符串
    if s1.len() > s2.len() {
        allocator.alloc_str(s1)
    } else {
        allocator.alloc_str(s2)
    }
}

fn arena_lifetime_binding() {
    println!("\n📚 2. Arena 分配器中的生命周期绑定");

    // 演示：所有从 Arena 分配的数据都与 Arena 的生命周期绑定
    let allocator = Allocator::default();

    let numbers = create_number_sequence(&allocator, 5);
    println!("   数字序列: {:?}", numbers);

    // 演示生命周期传播
    let processed = process_numbers(&allocator, &numbers);
    println!("   处理后的数据: {:?}", processed);

    // 重要：所有这些引用都与 allocator 的生命周期绑定
    println!("   🎯 关键点：所有引用都与 allocator 生命周期绑定");
}

fn create_number_sequence<'a>(allocator: &'a Allocator, count: usize) -> Vec<&'a i32> {
    let mut sequence = Vec::new();
    for i in 0..count {
        let number = allocator.alloc(i as i32);
        sequence.push(number as &i32); // 转换为不可变引用
    }
    sequence
}

fn process_numbers<'a>(
    allocator: &'a Allocator,
    numbers: &[&'a i32]
) -> Vec<&'a i32> {
    let mut processed = Vec::new();
    for &number in numbers {
        let doubled = allocator.alloc(*number * 2);
        processed.push(doubled as &i32); // 转换为不可变引用
    }
    processed
}

fn lifetime_elision_rules() {
    println!("\n📚 3. 生命周期省略规则");

    let allocator = Allocator::default();

    // 规则 1: 每个引用参数都有自己的生命周期
    let result1 = simple_alloc(&allocator, 100);
    println!("   简单分配: {}", result1);

    // 规则 2: 如果只有一个输入生命周期，它被赋给所有输出生命周期
    let result2 = transform_value(&allocator, 200);
    println!("   转换值: {}", result2);

    // 规则 3: 如果有多个输入生命周期，但其中一个是 &self 或 &mut self，
    // 那么 self 的生命周期被赋给所有输出生命周期
    let helper = LifetimeHelper::new(&allocator);
    let result3 = helper.get_value();
    println!("   Helper 值: {}", result3);
}

// 生命周期省略：编译器自动推断
fn simple_alloc(allocator: &Allocator, value: i32) -> &i32 {
    // 等价于：fn simple_alloc<'a>(allocator: &'a Allocator, value: i32) -> &'a i32
    allocator.alloc(value)
}

fn transform_value(allocator: &Allocator, value: i32) -> &str {
    // 编译器自动推断生命周期
    allocator.alloc_str(&format!("transformed_{}", value))
}

struct LifetimeHelper<'a> {
    allocator: &'a Allocator,
    value: &'a i32,
}

impl<'a> LifetimeHelper<'a> {
    fn new(allocator: &'a Allocator) -> Self {
        let value = allocator.alloc(42);
        LifetimeHelper { allocator, value }
    }

    fn get_value(&self) -> &i32 {
        // 返回的生命周期与 &self 相同
        self.value
    }

    fn create_new_value(&self, val: i32) -> &i32 {
        // 在同一个 allocator 中创建新值
        self.allocator.alloc(val)
    }
}

fn multiple_lifetime_parameters() {
    println!("\n📚 4. 多个生命周期参数");

    let allocator1 = Allocator::default();
    let allocator2 = Allocator::default();

    let data1 = allocator1.alloc(100);
    let data2 = allocator2.alloc(200);

    // 比较来自不同 allocator 的数据
    let comparison = compare_values(data1, data2);
    println!("   比较结果: {}", comparison);

    // 演示生命周期约束
    demonstrate_lifetime_constraints(&allocator1, &allocator2);
}

// 多个生命周期参数
fn compare_values<'a, 'b>(val1: &'a i32, val2: &'b i32) -> bool {
    // 可以比较来自不同生命周期的值
    val1 > val2
}

fn demonstrate_lifetime_constraints(alloc1: &Allocator, alloc2: &Allocator) {
    let str1 = alloc1.alloc_str("from allocator 1");
    let str2 = alloc2.alloc_str("from allocator 2");

    println!("   字符串 1: {}", str1);
    println!("   字符串 2: {}", str2);

    // 注意：不能返回一个引用，它可能来自两个不同的 allocator
    // 这就是为什么需要明确的生命周期注解
}

fn static_lifetime_examples() {
    println!("\n📚 5. 静态生命周期");

    let allocator = Allocator::default();

    // 'static 生命周期：在整个程序运行期间都有效
    let static_str: &'static str = "这是静态字符串";
    println!("   静态字符串: {}", static_str);

    // 将静态字符串复制到 arena 中
    let arena_copy = allocator.alloc_str(static_str);
    println!("   Arena 副本: {}", arena_copy);

    // 演示静态生命周期与 arena 生命周期的区别
    demonstrate_static_vs_arena(&allocator, static_str);
}

fn demonstrate_static_vs_arena(allocator: &Allocator, static_str: &'static str) {
    // 静态字符串可以在任何地方使用
    let arena_str = allocator.alloc_str("arena string");

    println!("   静态字符串地址: {:p}", static_str.as_ptr());
    println!("   Arena 字符串地址: {:p}", arena_str.as_ptr());

    // 静态字符串存储在程序的数据段中
    // Arena 字符串存储在堆上的 arena 中

    // 可以将静态引用存储在需要任意生命周期的地方
    let mixed_data = MixedLifetimeData {
        static_data: static_str,
        arena_data: arena_str,
    };

    println!("   混合数据: static='{}', arena='{}'",
             mixed_data.static_data, mixed_data.arena_data);
}

struct MixedLifetimeData<'a> {
    static_data: &'static str,  // 静态生命周期
    arena_data: &'a str,        // Arena 生命周期
}

// 高级生命周期概念演示
fn advanced_lifetime_concepts() {
    println!("\n📚 6. 高级生命周期概念");

    let allocator = Allocator::default();

    // 生命周期子类型 (Lifetime Subtyping)
    demonstrate_lifetime_subtyping(&allocator);

    // 高阶生命周期 (Higher-Ranked Trait Bounds)
    demonstrate_higher_ranked_lifetimes(&allocator);
}

fn demonstrate_lifetime_subtyping(allocator: &Allocator) {
    // 较长的生命周期可以被强制转换为较短的生命周期
    let long_lived = allocator.alloc(42);

    {
        // 在内部作用域中使用
        let short_lived_ref: &i32 = long_lived; // 生命周期收缩
        println!("   子类型演示: {}", short_lived_ref);
    }

    // long_lived 仍然有效
    println!("   原始引用仍有效: {}", long_lived);
}

fn demonstrate_higher_ranked_lifetimes(allocator: &Allocator) {
    // 高阶生命周期：for<'a> 语法
    fn identity_function(s: &str) -> &str {
        // 这个函数可以接受任何生命周期的字符串引用
        s
    }

    let test_str = allocator.alloc_str("test");
    let result = identity_function(test_str);
    println!("   高阶生命周期演示: {}", result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifetime_binding() {
        let allocator = Allocator::default();
        let data = allocator.alloc(42);
        assert_eq!(*data, 42);

        // 测试生命周期绑定
        let processed = simple_alloc(&allocator, 100);
        assert_eq!(*processed, 100);
    }

    #[test]
    fn test_multiple_lifetimes() {
        let alloc1 = Allocator::default();
        let alloc2 = Allocator::default();

        let val1 = alloc1.alloc(10);
        let val2 = alloc2.alloc(20);

        assert!(compare_values(val2, val1)); // 20 > 10
    }
}
