//! Macros for declaring lints and secret scanners.
#![warn(missing_docs)]
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod declare_all_lint_rules;
mod declare_oxc_lint;

/// Macro used to declare an oxc lint rule
///
/// Every lint declaration consists of 4 parts:
///
/// 1. The documentation
/// 2. The lint's struct
/// 3. The lint's category
/// 4. What kind of auto-fixes the lint supports
///
/// ## Documentation
/// Lint rule documentation added here will be used to build documentation pages
/// for [our website](https://oxc.rs). Please make sure they are clear and
/// concise. Remember, end users will depend on it to understand the purpose of
/// the lint and how to use it!
///
/// ## Category
/// Please see the [rule category
/// documentation](https://oxc.rs/docs/contribute/linter.html#rule-category) for
/// a full list of categories and their descriptions.
///
/// ## Auto-fixes
///
/// Lints that support auto-fixes **must** specify what kind of auto-fixes they
/// support. Here are some examples:
/// - `none`: no auto-fixes are available (default)
/// - `pending`: auto-fixes are planned but not yet implemented
/// - `fix`: safe, automatic fixes are available
/// - `suggestion`: automatic fixes are available, but may not be safe
/// - `conditional_fix`: automatic fixes are available in some cases
/// - `dangerous_fix`: automatic fixes are available, but may be dangerous
///
/// More generally, auto-fix categories follow this pattern:
/// ```text
/// [conditional?]_[fix|suggestion|dangerous]
/// ```
/// ...meaning that these are also valid:
/// - `suggestion_fix` (supports safe fixes and suggestions)
/// - `conditional_dangerous_fix` (sometimes provides dangerous fixes)
/// - `dangerous_fix_dangerous_suggestion` (provides dangerous fixes and suggestions in all cases)
///
/// `pending` and `none` are special cases that do not follow this pattern.
///
/// ## Integration markers
/// You can optionally add an integration marker immediately after the rule's struct
/// name in parentheses. Currently the only supported marker is `tsgolint`:
///
/// ```rust,ignore
/// declare_oxc_lint!(
///     /// Docs...
///     MyRule(tsgolint),
///     eslint,
///     style,
///     fix
/// );
/// ```
///
/// Adding `(tsgolint)` sets an internal `IS_TSGOLINT_RULE` flag to `true`, which
/// allows the `oxlint` CLI to surface this rule to the external `tsgolint`
/// executable. Rules without the marker keep the default `false` value and are
/// ignored by that integration. Only one marker is allowed and any other value
/// will result in a compile error.
///
/// # Example
///
/// ```
/// use oxc_macros::declare_oxc_lint;
///
/// #[derive(Debug, Default, Clone)]
/// pub struct NoDebugger;
///
/// declare_oxc_lint!(
///     /// ### What it does
///     ///
///     /// Checks for usage of the `debugger` statement
///     ///
///     /// ### Why is this bad?
///     ///
///     /// `debugger` statements do not affect functionality when a debugger isn't attached.
///     /// They're most commonly an accidental debugging leftover.
///     ///
///     /// ### Examples
///     ///
///     /// Examples of **incorrect** code for this rule:
///     /// ```js
///     /// const data = await getData();
///     /// const result = complexCalculation(data);
///     /// debugger;
///     /// ```
///     ///
///     /// Examples of **correct** code for this rule:
///     /// ```js
///     /// // Not a debugger statement
///     /// var debug = require('foo');
///     /// ```
///     NoDebugger,
///     eslint,
///     correctness,
///     fix
/// );
/// ```
#[proc_macro]
pub fn declare_oxc_lint(input: TokenStream) -> TokenStream {
    let metadata = parse_macro_input!(input as declare_oxc_lint::LintRuleMeta);
    declare_oxc_lint::declare_oxc_lint(metadata)
}

/// Same as `declare_oxc_lint`, but doesn't do imports.
/// Enables multiple usages in a single file.
#[proc_macro]
pub fn declare_oxc_lint_test(input: TokenStream) -> TokenStream {
    let mut metadata = parse_macro_input!(input as declare_oxc_lint::LintRuleMeta);
    metadata.used_in_test = true;
    declare_oxc_lint::declare_oxc_lint(metadata)
}

/// 声明所有 lint 规则的统一宏
///
/// # 核心作用
///
/// 此宏接收一个规则路径列表（如 `eslint::no_console, eslint::eqeqeq`），自动生成：
///
/// 1. **`RuleEnum` 枚举**：包含所有规则的变体，类似于编译期虚函数表（v-table）
/// 2. **规则元数据方法**：为每个规则生成 ID、名称、类别、插件名等查询方法
/// 3. **规则执行方法**：提供统一的规则执行入口（`run`, `run_on_symbol`, `run_once` 等）
/// 4. **`RULES` 静态变量**：包含所有规则实例的列表，供运行时使用
///
/// # 利用的 Rust 核心原理
///
/// ## 1. 过程宏（Proc Macro）- 编译时代码生成
/// - 使用 `#[proc_macro]` 在编译期解析和生成代码
/// - 利用 `syn` 解析 TokenStream，`quote` 生成新代码
/// - 实现元编程：编写代码来生成代码
///
/// ## 2. 枚举 + 模式匹配 - 零成本抽象的多态
/// - **传统方式**：`Box<dyn Rule>` + trait 对象 → 动态分发（有性能开销）
/// - **本宏方式**：`RuleEnum` + match 表达式 → 静态分发（零开销）
/// - 编译期将 match 展开为直接函数调用，性能等同于内联
///
/// ## 3. 绕过对象安全性（Object Safety）限制
/// - Rust 的 trait 对象有严格限制（不能使用泛型参数、关联类型等）
/// - 通过枚举 + 具体类型绕过这些限制，支持任意类型的规则
/// - 每个规则变体都包含完整的类型信息，编译期全部分析
///
/// ## 4. 静态分发 vs 动态分发
/// ```rust,ignore
/// // 动态分发（传统方式）：运行时查找虚函数表
/// fn run_dynamic(rule: &dyn Rule, node: &AstNode) {
///     rule.run(node);  // 通过 vtable 查找实际函数
/// }
///
/// // 静态分发（本宏方式）：编译期直接内联
/// match rule {
///     RuleEnum::EslintNoConsole(r) => r.run(node),  // 直接调用，零开销
///     RuleEnum::EslintEqeqeq(r) => r.run(node),     // 直接调用，零开销
/// }
/// ```
///
/// ## 5. 类型安全 + 编译期保证
/// - 所有规则都在编译期确定，不存在运行时错误
/// - 枚举的 exhaustiveness 检查确保覆盖所有规则
/// - 利用 Rust 的类型系统在编译期发现错误
///
/// # 设计优势
///
/// - **性能**：零成本抽象，静态分发，编译器可完全优化
/// - **类型安全**：编译期类型检查，避免运行时错误
/// - **可扩展性**：添加新规则只需一行声明，宏自动生成代码
/// - **统一管理**：所有 600+ 条规则在单一文件中统一注册和维护
///
/// # 工作原理
///
/// ```rust,ignore
/// // 输入：规则路径列表
/// declare_all_lint_rules! {
///     eslint::no_console,
///     eslint::eqeqeq,
/// }
///
/// // 自动生成：
/// // 1. 枚举定义
/// pub enum RuleEnum {
///     EslintNoConsole(NoConsole),
///     EslintEqeqeq(Eqeqeq),
/// }
///
/// // 2. 方法实现
/// impl RuleEnum {
///     pub fn id(&self) -> usize { /* match 分发 */ }
///     pub fn name(&self) -> &str { /* match 分发 */ }
///     pub fn run(&self, node: &AstNode, ctx: &LintContext) { /* match 分发 */ }
/// }
///
/// // 3. 规则列表
/// pub static RULES: LazyLock<Vec<RuleEnum>> = LazyLock::new(|| vec![
///     RuleEnum::EslintNoConsole(NoConsole::default()),
///     RuleEnum::EslintEqeqeq(Eqeqeq::default()),
/// ]);
/// ```
///
/// # 使用示例
///
/// 见 `crates/oxc_linter/src/rules.rs` - 这是唯一使用此宏的地方
#[proc_macro]
pub fn declare_all_lint_rules(input: TokenStream) -> TokenStream {
    let metadata = parse_macro_input!(input as declare_all_lint_rules::AllLintRulesMeta);
    declare_all_lint_rules::declare_all_lint_rules(metadata)
}
