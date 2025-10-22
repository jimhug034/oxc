use std::{fmt::Debug, pin::Pin, sync::Arc};

use serde::{Deserialize, Serialize};

use oxc_allocator::Allocator;

/// 外部 Linter 的插件加载回调函数类型
///
/// 这是一个异步回调函数，用于从 JavaScript 端加载外部 lint 插件（例如 ESLint 插件）。
///
/// # 参数
/// - `String`: 插件名称或路径
///
/// # 返回值
/// - `PluginLoadResult`: 插件加载结果（成功或失败）
///
/// # 使用场景
/// 当使用 Node.js 绑定（NAPI）时，Rust 侧可以通过此回调调用 JavaScript 代码来加载
/// ESLint 插件，这些插件可能包含 Oxc 尚未实现的规则。
pub type ExternalLinterLoadPluginCb = Arc<
    dyn Fn(
            String,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<PluginLoadResult, Box<dyn std::error::Error + Send + Sync>>,
                    > + Send,
            >,
        > + Send
        + Sync
        + 'static,
>;

/// 外部 Linter 的文件检查回调函数类型
///
/// 这是一个同步回调函数，用于将 AST 传递给 JavaScript 端进行 linting。
///
/// # 参数
/// - `String`: 文件路径
/// - `Vec<u32>`: 要运行的规则 ID 列表
/// - `&Allocator`: 包含 AST 的内存分配器（通过共享内存传递 AST）
///
/// # 返回值
/// - `Vec<LintFileResult>`: JavaScript 端返回的 lint 诊断结果列表
///
/// # 工作原理
/// Rust 侧已经完成了文件解析和语义分析，生成了完整的 AST。
/// 通过共享内存的方式，将 AST 传递给 JavaScript 端，让 JS 端的 ESLint 规则
/// 可以直接读取 AST 并执行检查，避免了 AST 的序列化/反序列化开销。
pub type ExternalLinterLintFileCb =
    Arc<dyn Fn(String, Vec<u32>, &Allocator) -> Result<Vec<LintFileResult>, String> + Sync + Send>;

/// 插件加载结果
///
/// 表示从 JavaScript 端加载外部插件的结果。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PluginLoadResult {
    /// 插件加载成功
    #[serde(rename_all = "camelCase")]
    Success {
        /// 插件名称
        name: String,
        /// 规则在全局规则列表中的偏移量
        offset: usize,
        /// 插件提供的所有规则名称列表
        rule_names: Vec<String>,
    },
    /// 插件加载失败
    Failure(String),
}

/// 单个文件的 Lint 结果
///
/// 表示 JavaScript 端对单个文件执行 lint 后返回的诊断信息。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LintFileResult {
    /// 触发此诊断的规则索引（在传入的规则列表中的索引）
    pub rule_index: u32,
    /// 诊断消息内容
    pub message: String,
    /// 诊断位置（源码中的字节偏移量）
    pub loc: Loc,
}

/// 源码位置范围
///
/// 使用 UTF-16 字节偏移量，因为 JavaScript 使用 UTF-16 编码。
/// Rust 侧需要在传递给 JS 前将 UTF-8 偏移量转换为 UTF-16，
/// 并在接收结果后转换回 UTF-8。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Loc {
    /// 起始位置（UTF-16 字节偏移）
    pub start: u32,
    /// 结束位置（UTF-16 字节偏移）
    pub end: u32,
}

/// 外部 Linter 桥接器
///
/// 这个结构体允许 Oxc 的 Rust 侧与 JavaScript 侧的 lint 引擎（如 ESLint）进行通信。
///
/// # 使用场景
///
/// 当 Oxc 作为 Node.js 模块（通过 NAPI）运行时，可以使用此机制来：
/// 1. 运行 Oxc 尚未实现的 ESLint 规则
/// 2. 运行来自第三方 ESLint 插件的规则
/// 3. 在 Oxc 的高性能解析和语义分析基础上，复用 ESLint 生态系统
///
/// # 工作流程
///
/// ```text
/// Rust 侧 (Oxc)                      JavaScript 侧 (ESLint)
///     │                                      │
///     ├─ 1. 解析文件生成 AST                │
///     ├─ 2. 语义分析                         │
///     ├─ 3. 运行 Oxc 内置规则               │
///     │                                      │
///     ├─ 4. 调用 load_plugin ───────────────>│
///     │                                      ├─ 加载 ESLint 插件
///     │<──────────── 返回插件信息 ────────────┤
///     │                                      │
///     ├─ 5. 通过共享内存传递 AST ────────────>│
///     │    调用 lint_file                    │
///     │                                      ├─ ESLint 规则检查 AST
///     │<──────────── 返回诊断结果 ────────────┤
///     │                                      │
///     ├─ 6. 转换并合并诊断结果               │
///     ├─ 7. 输出最终结果                     │
/// ```
///
/// # 性能优势
///
/// - **零拷贝 AST 传递**: 通过共享内存，避免 AST 序列化/反序列化
/// - **高效解析**: 使用 Oxc 的快速解析器，而不是 ESLint 的解析器
/// - **选择性运行**: 只在需要时调用 JavaScript，大部分规则在 Rust 侧运行
///
/// # 注意事项
///
/// 此功能只在启用 `oxlint2` feature 时可用，且仅用于 NAPI 绑定。
#[derive(Clone)]
#[cfg_attr(not(all(feature = "oxlint2", not(feature = "disable_oxlint2"))), expect(dead_code))]
pub struct ExternalLinter {
    /// 加载外部插件的回调函数（异步）
    pub(crate) load_plugin: ExternalLinterLoadPluginCb,
    /// 对文件执行 lint 的回调函数（同步）
    pub(crate) lint_file: ExternalLinterLintFileCb,
}

impl ExternalLinter {
    /// 创建一个新的 ExternalLinter 实例
    ///
    /// # 参数
    /// - `load_plugin`: 用于加载 JavaScript 插件的回调
    /// - `lint_file`: 用于执行 JavaScript lint 规则的回调
    pub fn new(
        load_plugin: ExternalLinterLoadPluginCb,
        lint_file: ExternalLinterLintFileCb,
    ) -> Self {
        Self { load_plugin, lint_file }
    }
}

impl Debug for ExternalLinter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 不打印回调函数的内容，因为它们是不可调试的
        f.debug_struct("ExternalLinter").finish()
    }
}
