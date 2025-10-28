//! Rust 零成本抽象模式 - 简化演示
//!
//! 这个文件展示了如何使用枚举 + match 实现高性能多态

/// ========================================
/// 第一部分：定义 trait 和具体实现
/// ========================================

// 定义一个处理器的 trait
trait Processor {
    fn process(&self, input: &str) -> String;
    fn name(&self) -> &'static str;
}

// 实现不同类型的处理器
#[derive(Debug, Clone)]
struct UppercaseProcessor;
impl Processor for UppercaseProcessor {
    fn process(&self, input: &str) -> String {
        input.to_uppercase()
    }
    fn name(&self) -> &'static str {
        "uppercase"
    }
}

#[derive(Debug, Clone)]
struct LowercaseProcessor;
impl Processor for LowercaseProcessor {
    fn process(&self, input: &str) -> String {
        input.to_lowercase()
    }
    fn name(&self) -> &'static str {
        "lowercase"
    }
}

#[derive(Debug, Clone)]
struct ReverseProcessor;
impl Processor for ReverseProcessor {
    fn process(&self, input: &str) -> String {
        input.chars().rev().collect()
    }
    fn name(&self) -> &'static str {
        "reverse"
    }
}

/// ========================================
/// 第二部分：传统方式 - 使用 trait 对象（动态分发）
/// ========================================

// 这种方式有性能开销，因为需要在运行时查找虚函数表
fn process_with_trait_object(processor: &dyn Processor, input: &str) -> String {
    println!("  [动态分发] 使用处理器: {}", processor.name());
    processor.process(input)
}

/// ========================================
/// 第三部分：Oxc 方式 - 使用枚举（静态分发）
/// ========================================

// 使用枚举包装所有具体类型
#[derive(Debug, Clone)]
enum ProcessorEnum {
    Uppercase(UppercaseProcessor),
    Lowercase(LowercaseProcessor),
    Reverse(ReverseProcessor),
}

// 为枚举实现方法，使用 match 进行静态分发
impl ProcessorEnum {
    /// 处理输入（静态分发，零开销）
    fn process(&self, input: &str) -> String {
        match self {
            // 编译期直接内联，编译器可以完全优化
            Self::Uppercase(p) => p.process(input),
            Self::Lowercase(p) => p.process(input),
            Self::Reverse(p) => p.process(input),
        }
    }

    /// 获取处理器名称（静态分发）
    fn name(&self) -> &'static str {
        match self {
            Self::Uppercase(p) => p.name(),
            Self::Lowercase(p) => p.name(),
            Self::Reverse(p) => p.name(),
        }
    }

    /// 获取处理器 ID（静态分发）
    fn id(&self) -> usize {
        match self {
            Self::Uppercase(_) => 0,
            Self::Lowercase(_) => 1,
            Self::Reverse(_) => 2,
        }
    }
}

/// ========================================
/// 第四部分：模拟宏生成的全局列表
/// ========================================

// 在实际项目中，这部分由 proc macro 自动生成
static PROCESSORS: &[ProcessorEnum] = &[
    ProcessorEnum::Uppercase(UppercaseProcessor),
    ProcessorEnum::Lowercase(LowercaseProcessor),
    ProcessorEnum::Reverse(ReverseProcessor),
];

/// ========================================
/// 第五部分：演示和性能对比
/// ========================================

fn main() {
    println!("==========================================");
    println!("  Rust 零成本抽象模式演示");
    println!("==========================================\n");

    let test_input = "Hello World";

    // 动态分发方式
    println!("【方式 1：动态分发 (传统 OOP)】");
    let dyn_upper = &UppercaseProcessor as &dyn Processor;
    let dyn_lower = &LowercaseProcessor as &dyn Processor;
    let dyn_reverse = &ReverseProcessor as &dyn Processor;

    println!("  结果: {}", process_with_trait_object(dyn_upper, test_input));
    println!("  结果: {}", process_with_trait_object(dyn_lower, test_input));
    println!("  结果: {}", process_with_trait_object(dyn_reverse, test_input));

    println!("\n【方式 2：静态分发 (Oxc 模式)】");
    // 静态分发方式
    let static_upper = ProcessorEnum::Uppercase(UppercaseProcessor);
    let static_lower = ProcessorEnum::Lowercase(LowercaseProcessor);
    let static_reverse = ProcessorEnum::Reverse(ReverseProcessor);

    println!("  结果: {}", static_upper.process(test_input));
    println!("  结果: {}", static_lower.process(test_input));
    println!("  结果: {}", static_reverse.process(test_input));

    // 遍历所有处理器（模拟规则遍历）
    println!("\n【遍历所有处理器】");
    for processor in PROCESSORS {
        let result = processor.process(test_input);
        println!("  [{:2}] {} -> {}", processor.id(), processor.name(), result);
    }

    println!("\n==========================================");
    println!("  总结：关键优势");
    println!("==========================================");
    println!("✓ 零成本抽象：无运行时开销");
    println!("✓ 类型安全：编译期保证正确性");
    println!("✓ 易于优化：编译器可以完全内联");
    println!("✓ 避免 vtable：不需要虚函数表查找");
    println!("✓ 统一管理：所有处理器集中管理");
}

/// ========================================
/// 第六部分：更复杂的场景 - 规则链
/// ========================================

// 演示如何串联使用多个处理器
fn pipeline_demo() {
    println!("\n==========================================");
    println!("  高级用法：处理管道");
    println!("==========================================");

    let input = "Hello World";

    // 创建处理管道
    let processors = vec![
        ProcessorEnum::Uppercase(UppercaseProcessor),
        ProcessorEnum::Reverse(ReverseProcessor),
        ProcessorEnum::Lowercase(LowercaseProcessor),
    ];

    let mut result = input.to_string();
    for (i, processor) in processors.iter().enumerate() {
        result = processor.process(&result);
        println!("步骤 {}: {} -> {}", i + 1, processor.name(), result);
    }
}

// 取消注释下面的行来运行管道演示
// pipeline_demo();

