//! Rust 零成本抽象模式演示
//!
//! 本演示展示了如何使用枚举 + 模式匹配实现高性能的多态，
//! 避免使用 trait 对象带来的性能开销。

/// ========================================
/// 第一部分：传统 OOP 方式（动态分发）
/// ========================================

// 定义一个 trait（接口）
trait Animal {
    fn make_sound(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
}

// 实现具体类型
struct Dog {
    name: String,
}

impl Animal for Dog {
    fn make_sound(&self) -> &'static str {
        "Woof!"
    }

    fn get_name(&self) -> &'static str {
        &self.name
    }
}

struct Cat {
    name: String,
}

impl Animal for Cat {
    fn make_sound(&self) -> &'static str {
        "Meow!"
    }

    fn get_name(&self) -> &'static str {
        &self.name
    }
}

// 传统方式：使用 trait 对象（动态分发）
fn make_sound_dynamic(animal: &dyn Animal) -> &'static str {
    // 运行时通过 vtable 查找实际方法
    // 有性能开销：指针解引用 + 虚函数表查找
    animal.make_sound()
}

/// ========================================
/// 第二部分：Oxc 方式（静态分发）
/// ========================================

// 使用枚举将具体类型包装起来
#[derive(Debug, Clone)]
enum AnimalEnum {
    Dog(Dog),
    Cat(Cat),
}

// 为枚举实现方法，使用 match 进行静态分发
impl AnimalEnum {
    /// 获取动物的叫声（静态分发）
    fn make_sound(&self) -> &'static str {
        match self {
            // 编译期直接内联，零开销
            Self::Dog(dog) => dog.make_sound(),
            Self::Cat(cat) => cat.make_sound(),
        }
    }

    /// 获取动物名称（静态分发）
    fn get_name(&self) -> &str {
        match self {
            Self::Dog(dog) => &dog.name,
            Self::Cat(cat) => &cat.name,
        }
    }

    /// 获取动物类型（静态分发）
    fn get_type(&self) -> &'static str {
        match self {
            Self::Dog(_) => "dog",
            Self::Cat(_) => "cat",
        }
    }
}

/// ========================================
/// 第三部分：模拟宏生成（手动生成）
/// ========================================

// 在实际项目中，这部分代码由 proc macro 自动生成
mod generated {
    use super::*;

    // 模拟宏生成的枚举
    #[derive(Debug, Clone)]
    pub enum AnimalEnum {
        Dog(Dog),
        Cat(Cat),
    }

    // 模拟宏生成的方法实现
    impl AnimalEnum {
        pub fn make_sound(&self) -> &'static str {
            match self {
                Self::Dog(dog) => dog.make_sound(),
                Self::Cat(cat) => cat.make_sound(),
            }
        }

        pub fn get_name(&self) -> &str {
            match self {
                Self::Dog(dog) => &dog.name,
                Self::Cat(cat) => &cat.name,
            }
        }

        pub fn get_type(&self) -> &'static str {
            match self {
                Self::Dog(_) => "dog",
                Self::Cat(_) => "cat",
            }
        }
    }

    // 模拟宏生成的静态列表
    pub static ANIMALS: &[AnimalEnum] = &[
        AnimalEnum::Dog(Dog { name: "Buddy".to_string() }),
        AnimalEnum::Cat(Cat { name: "Whiskers".to_string() }),
    ];
}

/// ========================================
/// 第四部分：性能对比演示
/// ========================================

fn performance_comparison() {
    let dog = Dog { name: "Buddy".to_string() };
    let cat = Cat { name: "Whiskers".to_string() };

    println!("=== 动态分发方式 ===");
    println!("Dog says: {}", make_sound_dynamic(&dog));
    println!("Cat says: {}", make_sound_dynamic(&cat));

    println!("\n=== 静态分发方式 ===");
    let dog_enum = AnimalEnum::Dog(dog);
    let cat_enum = AnimalEnum::Cat(cat);

    println!("Dog says: {}", dog_enum.make_sound());
    println!("Cat says: {}", cat_enum.make_sound());
    println!("Dog type: {}", dog_enum.get_type());
    println!("Cat type: {}", cat_enum.get_type());

    println!("\n=== 遍历所有动物（模拟规则运行）===");
    for animal in generated::ANIMALS {
        println!("{} ({}) says: {}", animal.get_name(), animal.get_type(), animal.make_sound());
    }
}

/// ========================================
/// 第五部分：更复杂的场景 - 规则系统
/// ========================================

// 模拟 Lint 规则系统
trait Rule {
    fn name(&self) -> &'static str;
    fn check(&self, input: &str) -> bool;
}

struct NoConsoleRule;
impl Rule for NoConsoleRule {
    fn name(&self) -> &'static str {
        "no-console"
    }

    fn check(&self, input: &str) -> bool {
        !input.contains("console.log")
    }
}

struct NoDebuggerRule;
impl Rule for NoDebuggerRule {
    fn name(&self) -> &'static str {
        "no-debugger"
    }

    fn check(&self, input: &str) -> bool {
        !input.contains("debugger")
    }
}

// 使用枚举实现静态分发
#[derive(Debug, Clone)]
enum RuleEnum {
    NoConsole(NoConsoleRule),
    NoDebugger(NoDebuggerRule),
}

impl RuleEnum {
    fn name(&self) -> &'static str {
        match self {
            Self::NoConsole(rule) => rule.name(),
            Self::NoDebugger(rule) => rule.name(),
        }
    }

    fn check(&self, input: &str) -> bool {
        match self {
            Self::NoConsole(rule) => rule.check(input),
            Self::NoDebugger(rule) => rule.check(input),
        }
    }

    fn category(&self) -> &'static str {
        match self {
            Self::NoConsole(_) => "correctness",
            Self::NoDebugger(_) => "correctness",
        }
    }
}

// 模拟宏生成的静态规则列表
static RULES: &[RuleEnum] = &[
    RuleEnum::NoConsole(NoConsoleRule),
    RuleEnum::NoDebugger(NoDebuggerRule),
];

fn lint_demo() {
    let code_samples = vec![
        "console.log('Hello');",
        "let x = 1;",
        "debugger; let y = 2;",
    ];

    println!("\n=== Lint 规则检查演示 ===");
    for code in code_samples {
        println!("\n检查代码: {}", code);
        for rule in RULES {
            let passed = rule.check(code);
            let status = if passed { "✓ 通过" } else { "✗ 失败" };
            println!("  [{}] {} ({})", status, rule.name(), rule.category());
        }
    }
}

/// ========================================
/// 第六部分：手动实现"宏"（简化版）
/// ========================================

fn manual_macro_emulation() {
    println!("\n=== 手动模拟宏的工作流程 ===");

    // 1. 定义规则列表（类似宏的输入）
    let rule_paths = vec!["eslint::no_console", "eslint::no_debugger"];

    println!("步骤 1: 解析规则路径");
    for path in &rule_paths {
        println!("  - {}", path);
    }

    // 2. 生成枚举变体名称
    println!("\n步骤 2: 生成枚举变体名称");
    for path in &rule_paths {
        let parts: Vec<&str> = path.split("::").collect();
        let rule_name = parts.last().unwrap();
        let enum_name = format!("Eslint{}",
            rule_name.split('_')
                .map(|s| s.chars().next().unwrap().to_uppercase().to_string() + &s[1..])
                .collect::<String>()
        );
        println!("  {} -> {}", path, enum_name);
    }

    // 3. 生成 match 表达式
    println!("\n步骤 3: 生成 match 表达式");
    println!("  match self {{");
    for (i, path) in rule_paths.iter().enumerate() {
        let parts: Vec<&str> = path.split("::").collect();
        let rule_name = parts.last().unwrap();
        println!("    Self::Eslint{} => {{ /* 执行规则 {} */ }}",
                 rule_name, i);
    }
    println!("  }}");
}

fn main() {
    println!("==========================================");
    println!("  Rust 零成本抽象模式演示");
    println!("==========================================\n");

    performance_comparison();
    lint_demo();
    manual_macro_emulation();

    println!("\n=== 总结 ===");
    println!("1. 使用枚举 + match 代替 trait 对象");
    println!("2. 编译期静态分发，零开销");
    println!("3. 类型安全，编译期保证");
    println!("4. Proc macro 自动生成重复代码");
    println!("5. 性能优于动态分发方式");
}

