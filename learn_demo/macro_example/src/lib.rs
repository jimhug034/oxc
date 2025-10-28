//! 一个简化的 proc macro 示例，演示如何自动生成枚举和方法
//!
//! 这个示例展示了 Oxc 中 `declare_all_lint_rules` 宏的核心思想

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Token};

/// 解析规则路径的结构
/// 例如：animal::dog 会被解析为 module="animal", name="dog"
struct RuleMeta {
    module: syn::Ident,
    name: syn::Ident,
}

impl syn::parse::Parse for RuleMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let module = input.parse()?;
        input.parse::<Token![::]>()?;
        let name = input.parse()?;
        Ok(RuleMeta { module, name })
    }
}

/// 解析多个规则的结构
struct AllRulesMeta {
    rules: Vec<RuleMeta>,
}

impl syn::parse::Parse for AllRulesMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let rules = input.parse_terminated(RuleMeta::parse, Token![,])?;
        Ok(AllRulesMeta {
            rules: rules.into_iter().collect(),
        })
    }
}

/// 声明所有规则的宏
///
/// # 使用示例
///
/// ```rust,ignore
/// declare_rules! {
///     animal::dog,
///     animal::cat,
/// }
/// ```
///
/// 这会生成：
/// - AnimalEnum 枚举
/// - impl AnimalEnum { ... } 方法
/// - RULES 静态变量
#[proc_macro]
pub fn declare_rules(input: TokenStream) -> TokenStream {
    let AllRulesMeta { rules } = parse_macro_input!(input as AllRulesMeta);

    // 收集所有需要的数据
    let mut enum_variants = Vec::new();
    let mut match_arms = Vec::new();
    let mut rule_instances = Vec::new();

    for (i, rule) in rules.iter().enumerate() {
        // 生成枚举变体名：AnimalDog, AnimalCat
        let variant_name = syn::Ident::new(
            &format!("{}{}",
                rule.module.to_string().to_case(),
                rule.name.to_string().to_case()
            ),
            rule.module.span(),
        );

        // 生成具体类型名：Dog, Cat
        let struct_name = syn::Ident::new(
            &rule.name.to_string().to_case(),
            rule.name.span(),
        );

        enum_variants.push(quote! {
            #variant_name(#struct_name),
        });

        // 生成 match 分支
        match_arms.push(quote! {
            #i => Self::#variant_name(_) => "OK",
        });

        // 生成规则实例
        rule_instances.push(quote! {
            RuleEnum::#variant_name(#struct_name),
        });
    }

    // 生成完整的代码
    let expanded = quote! {
        /// 自动生成的规则枚举
        #[derive(Debug, Clone)]
        pub enum RuleEnum {
            #(#enum_variants)*
        }

        impl RuleEnum {
            /// 获取规则的唯一 ID
            pub fn id(&self) -> usize {
                match self {
                    #(#match_arms,)*
                }
            }

            /// 运行规则检查
            pub fn run(&self, input: &str) -> bool {
                match self {
                    Self::AnimalDog(_) => !input.contains("cat"),
                    Self::AnimalCat(_) => !input.contains("dog"),
                }
            }
        }

        /// 所有规则的列表
        pub static RULES: &[RuleEnum] = &[
            #(#rule_instances,)*
        ];
    };

    TokenStream::from(expanded)
}

// 字符串转换工具函数
trait ToCase {
    fn to_case(&self) -> String;
}

impl ToCase for syn::Ident {
    fn to_case(&self) -> String {
        let s = self.to_string();
        // 简单的首字母大写转换
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

