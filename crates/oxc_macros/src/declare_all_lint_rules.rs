use convert_case::{Case, Casing};
use itertools::Itertools as _;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Result,
    parse::{Parse, ParseStream},
};

pub struct LintRuleMeta {
    rule_name: syn::Ident,
    enum_name: syn::Ident,
    path: syn::Path,
}

impl Parse for LintRuleMeta {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let path = input.parse::<syn::Path>()?;

        let segments = &path.segments;
        let combined = segments
            .iter()
            .rev()
            .take(2)
            .rev()
            .map(|seg| seg.ident.to_string().to_case(Case::Pascal))
            .join("");

        let combined = combined.to_case(Case::Pascal);

        let enum_name = syn::parse_str(&combined)?;
        let rule_name = syn::parse_str(
            &path.segments.iter().next_back().unwrap().ident.to_string().to_case(Case::Pascal),
        )?;
        Ok(Self { rule_name, enum_name, path })
    }
}

pub struct AllLintRulesMeta {
    rules: Vec<LintRuleMeta>,
}

impl Parse for AllLintRulesMeta {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let rules =
            input.parse_terminated(LintRuleMeta::parse, syn::Token![,])?.into_iter().collect();
        Ok(Self { rules })
    }
}

/// 生成所有 lint 规则的枚举和相关代码
///
/// # 工作流程
///
/// 1. **解析规则路径**：从 `eslint::no_console` 提取模块路径、枚举名等信息
/// 2. **收集元数据**：为每个规则收集所需的标识符和名称
/// 3. **生成代码**：使用 `quote!` 宏生成 Rust 代码
/// 4. **返回 TokenStream**：将生成的代码转换为编译器可处理的 TokenStream
///
/// # 生成的代码结构
///
/// - `RuleEnum` 枚举：包含所有规则的变体
/// - `RuleEnum` 实现：提供规则的执行、查询等方法
/// - trait 实现：Hash, PartialEq, Eq, Ord, PartialOrd
/// - `RULES` 静态变量：包含所有规则实例的列表
#[expect(clippy::cognitive_complexity, clippy::too_many_lines)]
pub fn declare_all_lint_rules(metadata: AllLintRulesMeta) -> TokenStream {
    let AllLintRulesMeta { rules } = metadata;

    // 预分配向量以存储每个规则的不同信息
    let mut use_stmts = Vec::with_capacity(rules.len()); // 模块路径（如 eslint::no_console）
    let mut struct_names = Vec::with_capacity(rules.len()); // 枚举变体名（如 EslintNoConsole）
    let mut struct_rule_names = Vec::with_capacity(rules.len()); // 规则结构体名（如 NoConsole）
    let mut plugin_names = Vec::with_capacity(rules.len()); // 插件名（如 "eslint"）
    let mut ids = Vec::with_capacity(rules.len()); // 规则 ID（索引号）

    // 遍历所有规则，提取元数据
    for (i, rule) in rules.iter().enumerate() {
        use_stmts.push(&rule.path);
        struct_names.push(&rule.enum_name);
        struct_rule_names.push(&rule.rule_name);

        // 提取插件名：从路径中取除了最后一个段之外的所有段
        // 例如：eslint::no_console -> "eslint"
        //       import::no_duplicates -> "import"
        plugin_names.push(
            rule.path
                .segments
                .iter()
                .take(rule.path.segments.len() - 1)
                .map(|s| format!("{}", s.ident))
                .join("/"),
        );
        ids.push(i); // 规则的唯一 ID（基于索引）
    }

    // 使用 quote! 宏生成 Rust 代码
    let expanded = quote! {
        // 为每个规则创建 use 语句和类型别名
        // 例如：pub use self::eslint::no_console::NoConsole as EslintNoConsole;
        #(pub use self::#use_stmts::#struct_rule_names as #struct_names;)*

        // 导入所需的依赖
        use crate::{
            context::{ContextHost, LintContext},
            rule::{Rule, RuleCategory, RuleFixMeta, RuleMeta},
            utils::PossibleJestNode,
            AstNode
        };
        use oxc_semantic::SymbolId;

        // 生成 RuleEnum 枚举，包含所有规则的变体
        // 例如：pub enum RuleEnum { EslintNoConsole(NoConsole), ... }
        #[derive(Debug, Clone)]
        #[expect(clippy::enum_variant_names)]
        pub enum RuleEnum {
            #(#struct_names(#struct_names)),*
        }

        // 为 RuleEnum 实现各种方法
        impl RuleEnum {
            /// 返回规则的唯一 ID（索引号）
            pub fn id(&self) -> usize {
                match self {
                    #(Self::#struct_names(_) => #ids),*
                }
            }

            /// 返回规则的名称（如 "no-console"）
            pub fn name(&self) -> &'static str {
                match self {
                    #(Self::#struct_names(_) => #struct_names::NAME),*
                }
            }

            /// 返回规则的类别（如 Correctness, Suspicious, Performance 等）
            pub fn category(&self) -> RuleCategory {
                match self {
                    #(Self::#struct_names(_) => #struct_names::CATEGORY),*
                }
            }

            /// 返回规则的自动修复能力
            pub fn fix(&self) -> RuleFixMeta {
                match self {
                    #(Self::#struct_names(_) => #struct_names::FIX),*
                }
            }

            /// 返回规则的文档（仅在 ruledocs feature 启用时）
            #[cfg(feature = "ruledocs")]
            pub fn documentation(&self) -> Option<&'static str> {
                match self {
                    #(Self::#struct_names(_) => #struct_names::documentation()),*
                }
            }

            /// 返回规则的配置模式（仅在 ruledocs feature 启用时）
            #[cfg(feature = "ruledocs")]
            pub fn schema(&self, generator: &mut schemars::SchemaGenerator) -> Option<schemars::schema::Schema> {
                match self {
                    #(Self::#struct_names(_) => #struct_names::config_schema(generator).or_else(||#struct_names::schema(generator))),*
                }
            }

            /// 返回规则所属的插件名（如 "eslint", "typescript"）
            pub fn plugin_name(&self) -> &'static str {
                match self {
                    #(Self::#struct_names(_) => #plugin_names),*
                }
            }

            /// 从 JSON 配置读取规则配置
            pub fn read_json(&self, value: serde_json::Value) -> Self {
                match self {
                    #(Self::#struct_names(_) => Self::#struct_names(
                        #struct_names::from_configuration(value),
                    )),*
                }
            }

            /// 在 AST 节点上运行规则检查（主入口）
            pub(super) fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
                match self {
                    #(Self::#struct_names(rule) => rule.run(node, ctx)),*
                }
            }

            /// 在符号上运行规则检查（用于某些需要语义信息的规则）
            pub(super) fn run_on_symbol<'a>(&self, symbol_id: SymbolId, ctx: &LintContext<'a>) {
                match self {
                    #(Self::#struct_names(rule) => rule.run_on_symbol(symbol_id, ctx)),*
                }
            }

            /// 运行一次性检查（在整个代码库扫描完成后执行）
            pub(super) fn run_once<'a>(&self, ctx: &LintContext<'a>) {
                match self {
                    #(Self::#struct_names(rule) => rule.run_once(ctx)),*
                }
            }

            /// 在 Jest 节点上运行规则检查（用于 Jest 相关规则）
            pub(super) fn run_on_jest_node<'a, 'c>(
                &self,
                jest_node: &PossibleJestNode<'a, 'c>,
                ctx: &'c LintContext<'a>,
            ) {
                match self {
                    #(Self::#struct_names(rule) => rule.run_on_jest_node(jest_node, ctx)),*
                }
            }

            /// 判断规则是否应该运行（基于配置和上下文）
            pub(super) fn should_run(&self, ctx: &ContextHost) -> bool {
                match self {
                    #(Self::#struct_names(rule) => rule.should_run(ctx)),*
                }
            }

            /// 判断是否是 tsgolint 规则
            pub fn is_tsgolint_rule(&self) -> bool {
                match self {
                    #(Self::#struct_names(rule) => #struct_names::IS_TSGOLINT_RULE),*
                }
            }
        }

        // 实现 Hash trait，使 RuleEnum 可以作为 HashMap/HashSet 的键
        impl std::hash::Hash for RuleEnum {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.id().hash(state);
            }
        }

        // 实现 PartialEq trait，用于比较规则是否相等
        impl PartialEq for RuleEnum {
            fn eq(&self, other: &Self) -> bool {
                self.id() == other.id()
            }
        }

        // 实现 Eq trait（因为 RuleEnum 有自反性、对称性、传递性）
        impl Eq for RuleEnum {}

        // 实现 Ord trait，用于排序规则
        impl Ord for RuleEnum {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.id().cmp(&other.id())
            }
        }

        // 实现 PartialOrd trait，用于部分比较
        impl PartialOrd for RuleEnum {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(&other))
            }
        }

        // 生成全局的 RULES 静态变量，包含所有规则的实例
        // 使用 LazyLock 实现延迟初始化，首次访问时才创建
        // 例如：vec![RuleEnum::EslintNoConsole(NoConsole::default()), ...]
        pub static RULES: std::sync::LazyLock<Vec<RuleEnum>> = std::sync::LazyLock::new(|| vec![
            #(RuleEnum::#struct_names(#struct_names::default())),*
        ]);
    };

    // 将生成的代码转换为 TokenStream 返回给编译器
    TokenStream::from(expanded)
}
