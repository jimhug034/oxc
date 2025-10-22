#![expect(clippy::self_named_module_files)] // for rules.rs
#![allow(clippy::literal_string_with_formatting_args)]

use std::{path::Path, rc::Rc};

use oxc_allocator::Allocator;
use oxc_semantic::AstNode;

#[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
use oxc_ast_macros::ast;
#[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
use oxc_ast_visit::utf8_to_utf16::Utf8ToUtf16;

#[cfg(test)]
mod tester;

mod ast_util;
mod config;
mod context;
mod disable_directives;
mod external_linter;
mod external_plugin_store;
mod fixer;
mod frameworks;
mod globals;
#[cfg(feature = "language_server")]
mod lsp;
mod module_graph_visitor;
mod module_record;
mod options;
mod rule;
mod service;
mod tsgolint;
mod utils;

pub mod loader;
pub mod rules;
pub mod table;

#[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
mod generated {
    #[cfg(debug_assertions)]
    pub mod assert_layouts;
}

pub use crate::{
    config::{
        BuiltinLintPlugins, Config, ConfigBuilderError, ConfigStore, ConfigStoreBuilder,
        ESLintRule, LintIgnoreMatcher, LintPlugins, Oxlintrc, ResolvedLinterState,
    },
    context::{ContextSubHost, LintContext},
    external_linter::{
        ExternalLinter, ExternalLinterLintFileCb, ExternalLinterLoadPluginCb, LintFileResult,
        PluginLoadResult,
    },
    external_plugin_store::{ExternalPluginStore, ExternalRuleId},
    fixer::FixKind,
    frameworks::FrameworkFlags,
    loader::LINTABLE_EXTENSIONS,
    module_record::ModuleRecord,
    options::LintOptions,
    options::{AllowWarnDeny, InvalidFilterKind, LintFilter, LintFilterKind},
    rule::{RuleCategory, RuleFixMeta, RuleMeta},
    service::{LintService, LintServiceOptions, RuntimeFileSystem},
    tsgolint::TsGoLintState,
    utils::{read_to_arena_str, read_to_string},
};
use crate::{
    config::{LintConfig, OxlintEnv, OxlintGlobals, OxlintSettings},
    context::ContextHost,
    fixer::{Fixer, Message},
    rules::RuleEnum,
    utils::iter_possible_jest_call_node,
};

#[cfg(feature = "language_server")]
pub use crate::lsp::{FixWithPosition, MessageWithPosition, PossibleFixesWithPosition};

#[cfg(target_pointer_width = "64")]
#[test]
fn size_asserts() {
    // `RuleEnum` runs in a really tight loop, make sure it is small for CPU cache.
    // A reduction from 168 bytes to 16 results 15% performance improvement.
    // See codspeed in https://github.com/oxc-project/oxc/pull/1783
    assert_eq!(size_of::<RuleEnum>(), 16);
}

#[derive(Debug, Clone)]
#[expect(clippy::struct_field_names)]
pub struct Linter {
    options: LintOptions,
    config: ConfigStore,
    #[cfg_attr(not(all(feature = "oxlint2", not(feature = "disable_oxlint2"))), expect(dead_code))]
    external_linter: Option<ExternalLinter>,
}

impl Linter {
    pub fn new(
        options: LintOptions,
        config: ConfigStore,
        external_linter: Option<ExternalLinter>,
    ) -> Self {
        Self { options, config, external_linter }
    }

    /// Set the kind of auto fixes to apply.
    #[must_use]
    pub fn with_fix(mut self, kind: FixKind) -> Self {
        self.options.fix = kind;
        self
    }

    #[must_use]
    pub fn with_report_unused_directives(mut self, report_config: Option<AllowWarnDeny>) -> Self {
        self.options.report_unused_directive = report_config;
        self
    }

    pub(crate) fn options(&self) -> &LintOptions {
        &self.options
    }

    /// Returns the number of rules that will are being used, unless there
    /// nested configurations in use, in which case it returns `None` since the
    /// number of rules depends on which file is being linted.
    pub fn number_of_rules(&self, type_aware: bool) -> Option<usize> {
        self.config.number_of_rules(type_aware)
    }

    pub fn run<'a>(
        &self,
        path: &Path,
        context_sub_hosts: Vec<ContextSubHost<'a>>,
        allocator: &'a Allocator,
    ) -> Vec<Message<'a>> {
        let ResolvedLinterState { rules, config, external_rules } = self.config.resolve(path);

        let mut ctx_host = Rc::new(ContextHost::new(path, context_sub_hosts, self.options, config));

        loop {
            let rules = rules
                .iter()
                .filter(|(rule, _)| rule.should_run(&ctx_host) && !rule.is_tsgolint_rule())
                .map(|(rule, severity)| (rule, Rc::clone(&ctx_host).spawn(rule, *severity)));

            let semantic = ctx_host.semantic();

            let should_run_on_jest_node =
                ctx_host.plugins().has_test() && ctx_host.frameworks().is_test();

            // IMPORTANT: We have two branches here for performance reasons:
            //
            // 1) Branch where we iterate over each node, then each rule
            // 2) Branch where we iterate over each rule, then each node
            //
            // When the number of nodes is relatively small, most of them can fit
            // in the cache and we can save iterating over the rules multiple times.
            // But for large files, the number of nodes can be so large that it
            // starts to not fit into the cache and pushes out other data, like the rules.
            // So we end up thrashing the cache with each rule iteration. In this case,
            // it's better to put rules in the inner loop, as the rules data is smaller
            // and is more likely to fit in the cache.
            //
            // The threshold here is chosen to balance between performance improvement
            // from not iterating over rules multiple times, but also ensuring that we
            // don't thrash the cache too much. Feel free to tweak based on benchmarking.
            //
            // See https://github.com/oxc-project/oxc/pull/6600 for more context.
            if semantic.nodes().len() > 200_000 {
                // Collect rules into a Vec so that we can iterate over the rules multiple times
                let rules = rules.collect::<Vec<_>>();

                for (rule, ctx) in &rules {
                    rule.run_once(ctx);
                }

                for symbol in semantic.scoping().symbol_ids() {
                    for (rule, ctx) in &rules {
                        rule.run_on_symbol(symbol, ctx);
                    }
                }

                for node in semantic.nodes() {
                    for (rule, ctx) in &rules {
                        rule.run(node, ctx);
                    }
                }

                if should_run_on_jest_node {
                    for jest_node in iter_possible_jest_call_node(semantic) {
                        for (rule, ctx) in &rules {
                            rule.run_on_jest_node(&jest_node, ctx);
                        }
                    }
                }
            } else {
                for (rule, ref ctx) in rules {
                    rule.run_once(ctx);

                    for symbol in semantic.scoping().symbol_ids() {
                        rule.run_on_symbol(symbol, ctx);
                    }

                    for node in semantic.nodes() {
                        rule.run(node, ctx);
                    }

                    if should_run_on_jest_node {
                        for jest_node in iter_possible_jest_call_node(semantic) {
                            rule.run_on_jest_node(&jest_node, ctx);
                        }
                    }
                }
            }

            #[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
            self.run_external_rules(&external_rules, path, &mut ctx_host, allocator);

            // Stop clippy complaining about unused vars
            #[cfg(not(all(feature = "oxlint2", not(feature = "disable_oxlint2"))))]
            let (_, _, _) = (&external_rules, &mut ctx_host, allocator);

            if let Some(severity) = self.options.report_unused_directive {
                if severity.is_warn_deny() {
                    ctx_host.report_unused_directives(severity.into());
                }
            }

            // no next `<script>` block found, the complete file is finished linting
            if !ctx_host.next_sub_host() {
                break;
            }
        }

        ctx_host.take_diagnostics()
    }

    /// 运行外部 lint 规则（来自 JavaScript 端）
    ///
    /// 此方法只在启用 `oxlint2` feature 时编译。它允许 Rust 侧调用 JavaScript 端的
    /// ESLint 规则，通过共享内存传递 AST，实现高性能的跨语言 linting。
    ///
    /// # 工作流程
    ///
    /// 1. **准备 AST**: 获取已解析的 AST，并将其 span（UTF-8）转换为 UTF-16
    /// 2. **共享内存**: 通过 `Allocator` 的共享内存机制传递 AST 给 JavaScript
    /// 3. **调用 JS**: 通过 `external_linter.lint_file` 回调执行 JavaScript 规则
    /// 4. **收集结果**: 接收 JavaScript 返回的诊断信息
    /// 5. **转换位置**: 将 UTF-16 偏移量转换回 UTF-8
    /// 6. **合并诊断**: 将外部规则的诊断结果添加到 `ctx_host` 中
    ///
    /// # 参数
    ///
    /// - `external_rules`: 要运行的外部规则及其严重性级别列表
    /// - `path`: 正在检查的文件路径
    /// - `ctx_host`: 上下文宿主，用于存储诊断结果
    /// - `allocator`: 包含 AST 的内存分配器
    ///
    /// # 注意事项
    ///
    /// - 此方法涉及复杂的内存操作和指针转换
    /// - AST 必须在共享内存中，以便 JavaScript 可以直接访问
    /// - 需要进行 UTF-8 ↔ UTF-16 位置转换，因为 Rust 使用 UTF-8，JS 使用 UTF-16
    #[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
    fn run_external_rules<'a>(
        &self,
        external_rules: &[(ExternalRuleId, AllowWarnDeny)],
        path: &Path,
        ctx_host: &mut Rc<ContextHost<'a>>,
        allocator: &'a Allocator,
    ) {
        use std::{
            mem,
            ptr::{self, NonNull},
        };

        use oxc_ast::ast::Program;
        use oxc_diagnostics::OxcDiagnostic;
        use oxc_span::Span;

        use crate::fixer::PossibleFixes;

        // 如果没有外部规则需要运行，直接返回
        if external_rules.is_empty() {
            return;
        }

        // 获取外部 linter 实例
        // 当启用 `oxlint2` feature 时，这个字段总是 Some
        let external_linter = self.external_linter.as_ref().unwrap();

        // ====== 阶段 1: 准备 AST 并转换为 UTF-16 ======
        let (program_offset, span_converter) = {
            // 从 `ContextHost` 中提取 `Semantic`，并获取 `Program` 的可变引用。
            //
            // 为什么不能直接获取 `&mut Program`？
            // - `Semantic` 包含 `AstNodes`，它为每个 AST 节点存储了 `AstKind`
            // - 每个 `AstKind` 都包含对 AST 节点的不可变引用 `&`
            // - 在 `Semantic` 存在时获取 `&mut Program` 会违反 Rust 的别名规则
            //
            // 解决方案（使用指针技巧）：
            // 1. 从 `Semantic` 中获取指向 `Program` 的指针
            // 2. 创建一个新指针，继承 `data_end_ptr` 的来源（provenance），允许修改
            // 3. 删除 `Semantic`，此时不再有对 AST 节点的引用
            // 4. 现在可以安全地将指针转换为 `&mut Program`
            //
            // 内存布局保证：
            // - `Program` 在 `allocator` 中创建，使用 `FixedSizeAllocator`
            // - `FixedSizeAllocator` 只有一个内存块
            // - `data_end_ptr` 和 `Program` 在同一分配中
            // - 所有 `Linter::run` 的调用者都从 `ModuleContent` 获取 `allocator` 和 `Semantic`
            //
            // 注意：这在技术上是不健全的（unsound），因为没有静态保证
            // TODO: 最好避免这里需要 `&mut Program`，从而避免这种危险行为
            // 获取 ctx_host 的可变引用（Rc::get_mut 确保没有其他引用）
            let ctx_host = Rc::get_mut(ctx_host).unwrap();
            // 从 ctx_host 中取出 Semantic（替换为默认值）
            let semantic = mem::take(ctx_host.semantic_mut());
            // 获取 Program 的地址
            let program_addr = NonNull::from(semantic.nodes().program()).addr();
            // 创建新指针，继承 data_end_ptr 的来源，允许修改
            let mut program_ptr =
                allocator.data_end_ptr().cast::<Program>().with_addr(program_addr);
            // 删除 Semantic，释放所有对 AST 的引用
            drop(semantic);
            // SAFETY: 现在 Semantic 已被删除，不再有对任何 AST 节点的引用，
            // 因此可以安全地获取 Program 的可变引用，不会违反别名规则
            let program = unsafe { program_ptr.as_mut() };

            // 将所有 span 从 UTF-8 转换为 UTF-16
            // JavaScript 使用 UTF-16 编码，所以需要进行转换
            let span_converter = Utf8ToUtf16::new(program.source_text);
            span_converter.convert_program(program);

            // 获取 Program 在缓冲区中的偏移量（指针的低 32 位）
            // JavaScript 端将使用此偏移量来定位 AST
            let program_offset = ptr::from_ref(program) as u32;

            (program_offset, span_converter)
        };

        // ====== 阶段 2: 在缓冲区末尾写入元数据 ======
        // 将 Program 的偏移量写入缓冲区末尾的元数据区域
        // JavaScript 端需要这个偏移量来定位 AST 的起始位置
        let metadata = RawTransferMetadata::new(program_offset);
        let metadata_ptr = allocator.end_ptr().cast::<RawTransferMetadata>();
        // SAFETY: `Allocator` 由 `FixedSizeAllocator` 创建，它在 `end_ptr` 后面
        // 预留了 `RawTransferMetadata` 的空间。`end_ptr` 已对齐到 `RawTransferMetadata`。
        unsafe { metadata_ptr.write(metadata) };

        // ====== 阶段 3: 调用 JavaScript 端执行 lint ======
        // 通过共享内存传递 AST 和规则 ID 列表给 JavaScript
        // JavaScript 端将读取共享内存中的 AST，执行相应的规则，并返回诊断结果
        let result = (external_linter.lint_file)(
            path.to_str().unwrap().to_string(),      // 文件路径
            external_rules.iter().map(|(rule_id, _)| rule_id.raw()).collect(), // 规则 ID 列表
            allocator,                                 // 包含 AST 的内存分配器
        );
        // ====== 阶段 4: 处理 JavaScript 返回的结果 ======
        match result {
            Ok(diagnostics) => {
                // 成功：遍历所有诊断结果
                for diagnostic in diagnostics {
                    // 将 UTF-16 偏移量转换回 UTF-8
                    // JavaScript 返回的位置是 UTF-16 编码的，需要转换为 Rust 的 UTF-8
                    let mut span = Span::new(diagnostic.loc.start, diagnostic.loc.end);
                    span_converter.convert_span_back(&mut span);

                    // 获取触发此诊断的规则信息
                    let (external_rule_id, severity) =
                        external_rules[diagnostic.rule_index as usize];
                    let (plugin_name, rule_name) =
                        self.config.resolve_plugin_rule_names(external_rule_id);

                    // 创建诊断消息并添加到上下文中
                    ctx_host.push_diagnostic(Message::new(
                        OxcDiagnostic::error(diagnostic.message)
                            .with_label(span)                    // 错误位置
                            .with_error_code(plugin_name.to_string(), rule_name.to_string()) // 规则名
                            .with_severity(severity.into()),     // 严重性级别
                        PossibleFixes::None,                     // 暂无修复建议
                    ));
                }
            }
            Err(_err) => {
                // 失败：JavaScript 端执行出错
                // TODO: 应该报告诊断错误
            }
        }
    }
}

#[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
/// Metadata written to end of buffer.
///
/// Duplicate of `RawTransferMetadata` in `napi/parser/src/raw_transfer_types.rs`.
/// Any changes made here also need to be made there.
/// `oxc_ast_tools` checks that the 2 copies are identical.
#[ast]
struct RawTransferMetadata2 {
    /// Offset of `Program` within buffer.
    /// Note: In `RawTransferMetadata` (in `napi/parser`), this field is offset of `RawTransferData`,
    /// but here it's offset of `Program`.
    pub data_offset: u32,
    /// `true` if AST is TypeScript.
    pub is_ts: bool,
    /// Padding to pad struct to size 16.
    pub(crate) _padding: u64,
}

#[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
use RawTransferMetadata2 as RawTransferMetadata;

#[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
impl RawTransferMetadata {
    pub fn new(data_offset: u32) -> Self {
        Self { data_offset, is_ts: false, _padding: 0 }
    }
}

#[cfg(test)]
mod test {
    use super::Oxlintrc;

    #[test]
    fn test_schema_json() {
        use std::fs;

        use project_root::get_project_root;
        let path = get_project_root().unwrap().join("npm/oxlint/configuration_schema.json");
        let schema = schemars::schema_for!(Oxlintrc);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        let existing_json = fs::read_to_string(&path).unwrap_or_default();
        if existing_json.trim() != json.trim() {
            std::fs::write(&path, &json).unwrap();
        }
        insta::with_settings!({ prepend_module_to_snapshot => false }, {
            insta::assert_snapshot!(json);
        });
    }
}
