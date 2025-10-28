//! # Runtime - Linter 运行时执行引擎
//!
//! 这是 Oxc linter 的核心运行时模块，负责：
//!
//! 1. **并行处理多个文件**：使用 Rayon 实现并行解析和 linting
//! 2. **构建模块依赖图**：解析 import/export 语句，构建完整的模块依赖关系
//! 3. **内存优化**：使用分配器池和分组处理减少内存使用
//! 4. **增量处理**：支持多段文件（如 .vue, .astro）的处理
//!
//! ## 核心设计
//!
//! ### 并行处理策略
//!
//! - **分组处理**：将大量文件分成小组处理，避免内存溢出
//! - **依赖感知**：按依赖关系顺序处理，提前释放内存
//! - **双线程架构**：graph 线程负责构建依赖图，module 线程负责解析模块
//!
//! ### 内存管理
//!
//! - **分配器池**：为每个线程提供独立的分配器
//! - **及时释放**：处理完立即释放源文件和语义信息
//! - **Cow 优化**：使用 Cow 避免不必要的内存分配
//!
//! ## 工作流程
//!
//! ```text
//! 输入文件列表
//!     ↓
//! 分组处理（按深度排序）
//!     ↓
//! 并行解析 + 构建依赖图
//!     ↓
//! 解析依赖模块
//!     ↓
//! 并行执行 Linting
//!     ↓
//! 收集诊断信息
//! ```

use std::{
    borrow::Cow,
    ffi::OsStr,
    fs,
    mem::take,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};

use indexmap::IndexSet;
use rayon::iter::ParallelDrainRange;
use rayon::{Scope, iter::IntoParallelRefIterator, prelude::ParallelIterator};
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use self_cell::self_cell;
use smallvec::SmallVec;

use oxc_allocator::{Allocator, AllocatorGuard, AllocatorPool};
use oxc_diagnostics::{DiagnosticSender, DiagnosticService, Error, OxcDiagnostic};
use oxc_parser::{ParseOptions, Parser};
use oxc_resolver::Resolver;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::{CompactStr, SourceType, VALID_EXTENSIONS};

#[cfg(feature = "language_server")]
use crate::lsp::MessageWithPosition;

#[cfg(test)]
use crate::fixer::{Message, PossibleFixes};
use crate::{
    Fixer, Linter,
    context::ContextSubHost,
    loader::{JavaScriptSource, LINT_PARTIAL_LOADER_EXTENSIONS, PartialLoader},
    module_record::ModuleRecord,
    utils::read_to_arena_str,
};

use super::LintServiceOptions;

/// Linter 运行时引擎
///
/// # 核心职责
///
/// - 并行处理多个文件的 linting
/// - 构建和管理模块依赖图
/// - 优化内存使用和性能
///
/// # 设计亮点
///
/// - **分组处理**：避免一次性处理太多文件导致内存溢出
/// - **依赖感知**：深度优先处理，提前释放内存
/// - **双线程架构**：graph 线程 + module 线程，高效并行
pub struct Runtime {
    /// 当前工作目录
    cwd: Box<Path>,

    /// 所有待 lint 的文件路径集合
    /// 使用 IndexSet 保持顺序，使用 Arc<OsStr> 避免重复分配
    paths: IndexSet<Arc<OsStr>, FxBuildHasher>,

    /// Linter 实例
    pub(super) linter: Linter,

    /// 模块解析器（用于解析 import/export）
    /// 如果启用 cross_module 功能，则会使用此解析器
    resolver: Option<Resolver>,

    /// 文件系统抽象（用于测试和语言服务器）
    pub(super) file_system: Box<dyn RuntimeFileSystem + Sync + Send>,

    /// 分配器池：为每个线程提供独立的分配器
    /// 避免多线程竞争，提高性能
    allocator_pool: AllocatorPool,
}

/// `Runtime::process_path` 的输出
///
/// 这是模块线程和处理线程之间的通信结构
/// 包含处理后的模块信息和路径
struct ModuleProcessOutput<'alloc_pool> {
    /// 文件路径
    ///
    /// 注意：使用 `Arc<OsStr>` 而不是 `Path`，因为：
    /// - `OsStr` 的哈希计算比 `Path` 更快
    /// - `Arc` 避免重复分配，提高性能
    /// - 跨线程共享更安全
    path: Arc<OsStr>,

    /// 处理后的模块
    /// 包含模块记录、源代码、语义信息等
    processed_module: ProcessedModule<'alloc_pool>,
}

/// 从路径处理得到的模块
///
/// # 多段支持
///
/// 某些文件格式（如 .vue, .astro）可能包含多个源文件段：
/// - `.vue` 文件：`<script>`, `<template>`, `<style>` 三个段
/// - `.astro` 文件：支持多个 `<script>` 段
///
/// 普通 ts/js 文件只有一个段
#[derive(Default)]
struct ProcessedModule<'alloc_pool> {
    /// 各个源文件段的模块记录
    ///
    /// - `Ok(ResolvedModuleRecord)`: 解析成功，包含模块记录和解析后的依赖
    /// - `Err(Vec<OxcDiagnostic>)`: 解析失败，包含诊断信息
    ///
    /// 使用 `SmallVec` 优化：大多数文件只有一个段，避免堆分配
    section_module_records: SmallVec<[Result<ResolvedModuleRecord, Vec<OxcDiagnostic>>; 1]>,

    /// 源代码和语义信息
    ///
    /// # None 的情况
    ///
    /// 1. **依赖模块**：import 插件启用时，依赖模块只用于构建模块图，不需要 lint
    /// 2. **文件读取失败**：文件不存在或不是有效的 UTF-8
    ///
    /// # Some 的情况
    ///
    /// 即使解析失败，只要源文件是有效的 UTF-8，`content` 仍为 `Some`
    /// 这样设计是为了处理"部分段解析失败"的情况
    content: Option<ModuleContent<'alloc_pool>>,
}

/// 解析后的模块请求
///
/// 表示一个 import 语句的解析结果
struct ResolvedModuleRequest {
    /// import 语句中的原始 specifier（如 "./foo", "lodash"）
    specifier: CompactStr,

    /// 解析后的实际文件路径
    /// 例如："./foo" → "/path/to/src/foo.ts"
    resolved_requested_path: Arc<OsStr>,
}

/// 包含所有 import 语句解析结果的模块记录
///
/// 这是模块图和 linting 之间的桥梁
struct ResolvedModuleRecord {
    /// 模块记录（包含 AST、符号信息等）
    module_record: Arc<ModuleRecord>,

    /// 该模块的所有依赖模块的解析结果
    /// 用于构建模块依赖图
    resolved_module_requests: Vec<ResolvedModuleRequest>,
}

self_cell! {
    struct ModuleContent<'alloc_pool> {
        owner: AllocatorGuard<'alloc_pool>,
        #[not_covariant]
        dependent: ModuleContentDependent,
    }
}
struct ModuleContentDependent<'a> {
    source_text: &'a str,
    section_contents: SectionContents<'a>,
}

// Safety: dependent borrows from owner. They're safe to be sent together.
unsafe impl Send for ModuleContent<'_> {}

/// 每个源文件段的源文本和语义信息
///
/// 与 `ProcessedModule.section_module_records` 的顺序一致
type SectionContents<'a> = SmallVec<[SectionContent<'a>; 1]>;

/// 单个源文件段的内容
struct SectionContent<'a> {
    /// 源文本片段（包含偏移和类型信息）
    source: JavaScriptSource<'a>,

    /// 语义信息
    ///
    /// - `Some(semantic)`：解析成功，语义信息可用
    /// - `None`：解析失败，对应 `section_module_records` 中为 `Err(Vec<OxcDiagnostic>)`
    semantic: Option<Semantic<'a>>,
}

/// 准备好用于 linting 的模块
///
/// 这是从 `ProcessedModule` 转换来的，区别在于 `content` 是必有的（非 Option）
/// 只有入口文件（在 `runtime.paths` 中的文件）才有此结构
struct ModuleToLint<'alloc_pool> {
    /// 文件路径
    path: Arc<OsStr>,

    /// 各段的模块记录
    /// 注意：这里已经是 `Arc<ModuleRecord>`，而不再是 `ResolvedModuleRecord`
    /// 因为依赖信息已经用于构建模块图了
    section_module_records: SmallVec<[Result<Arc<ModuleRecord>, Vec<OxcDiagnostic>>; 1]>,

    /// 源代码和语义信息（必须存在）
    content: ModuleContent<'alloc_pool>,
}

impl<'alloc_pool> ModuleToLint<'alloc_pool> {
    /// 从 `ProcessedModule` 创建 `ModuleToLint`
    ///
    /// # 转换逻辑
    ///
    /// 1. 检查 `content` 是否存在（只有入口文件才有）
    /// 2. 提取 `module_record`，丢弃 `resolved_module_requests`（已用于构建模块图）
    /// 3. 保留源代码和语义信息用于 linting
    ///
    /// # 返回 None
    ///
    /// 如果 `content` 为 `None`，说明这是依赖模块，不需要 lint
    fn from_processed_module(
        path: Arc<OsStr>,
        processed_module: ProcessedModule<'alloc_pool>,
    ) -> Option<Self> {
        processed_module.content.map(|content| Self {
            path,
            // 提取 module_record，丢弃 resolved_module_requests
            section_module_records: processed_module
                .section_module_records
                .into_iter()
                .map(|record_result| record_result.map(|ok| ok.module_record))
                .collect(),
            content,
        })
    }
}

/// 文件系统抽象接口
///
/// # 设计目的
///
/// 允许 `Runtime` 使用不同的文件系统实现：
/// - **OsFileSystem**：默认实现，直接操作文件系统
/// - **内存文件系统**：测试时使用，避免磁盘 I/O
/// - **语言服务器**：从 IDE 提供源文本，支持编辑中的文件
///
/// # 使用场景
///
/// - 测试：提供内存中的内容，快速测试
/// - 语言服务器：处理编辑中的代码，不需要保存到磁盘
/// - 生产环境：使用操作系统的文件系统
pub trait RuntimeFileSystem {
    /// 从文件路径读取内容到分配器管理的字符串
    ///
    /// # 关键点
    ///
    /// - 返回的字符串的生命周期绑定到 `allocator`
    /// - 使用 arena allocator，避免频繁的内存分配
    /// - 支持 UTF-8 编码
    ///
    /// # 错误
    ///
    /// - 文件不存在
    /// - 不是有效的 UTF-8 编码
    /// - 权限不足
    fn read_to_arena_str<'a>(
        &'a self,
        path: &Path,
        allocator: &'a Allocator,
    ) -> Result<&'a str, std::io::Error>;

    /// 将内容写入文件系统
    ///
    /// # 用途
    ///
    /// 用于写入修复后的代码（当启用 auto-fix 时）
    ///
    /// # 错误
    ///
    /// - 没有写入权限
    /// - 磁盘空间不足
    fn write_file(&self, path: &Path, content: &str) -> Result<(), std::io::Error>;
}

/// 操作系统文件系统实现
///
/// 这是默认的文件系统实现，直接操作操作系统文件
struct OsFileSystem;

impl RuntimeFileSystem for OsFileSystem {
    /// 从文件系统读取文件内容
    ///
    /// 使用 `read_to_arena_str` 工具函数，将内容分配到 arena allocator
    fn read_to_arena_str<'a>(
        &self,
        path: &Path,
        allocator: &'a Allocator,
    ) -> Result<&'a str, std::io::Error> {
        read_to_arena_str(path, allocator)
    }

    /// 写入文件到文件系统
    ///
    /// 使用 `fs::write` 原子性地写入文件
    fn write_file(&self, path: &Path, content: &str) -> Result<(), std::io::Error> {
        fs::write(path, content)
    }
}

/// MessageCloner - 线程安全的 Allocator 包装器
///
/// # 设计目的
///
/// 允许在多线程环境中安全地使用 `Allocator` 来克隆 `Message`
///
/// # 问题背景
///
/// `Allocator` 不是线程安全的（不实现 `Sync`），不能跨线程共享
/// 从多个线程同时分配会导致未定义行为
///
/// # 解决方案
///
/// 使用 Mutex 同步访问：
/// 1. 构造时获取独占的 `&mut Allocator`
/// 2. 通过 Mutex 同步所有访问
/// 3. 确保任何时候只有一个线程使用 Allocator
///
/// # 模块封装
///
/// 将内部实现封装在模块中，防止外部代码直接访问 `UnsafeAllocatorRef`
/// 必须通过 `MessageCloner::clone_message` 方法使用
#[cfg(any(feature = "language_server", test))]
mod message_cloner {
    use std::sync::Mutex;

    use oxc_allocator::{Allocator, CloneIn};

    use crate::Message;

    /// 不安全地使 `&Allocator` 实现 `Send`
    ///
    /// # 安全性保证
    ///
    /// 实现 `Send` 是安全的，因为：
    /// 1. **独占引用**：构造时通过 `&mut Allocator` 确保无其他引用
    /// 2. **生命周期约束**：生命周期 `'a` 保证引用不超过原始借用
    /// 3. **同步访问**：所有访问都通过 `MessageCloner` 的 Mutex 同步
    /// 4. **模块封装**：外部无法直接访问，只能通过安全接口
    ///
    /// 因此，虽然 `Allocator` 不是 `Sync`，`UnsafeAllocatorRef` 可以安全地在线程间传递
    struct UnsafeAllocatorRef<'a>(&'a Allocator);

    unsafe impl Send for UnsafeAllocatorRef<'_> {}

    /// Allocator 的线程安全包装器
    ///
    /// 允许多个线程安全地使用同一 Allocator 来克隆 Message
    pub struct MessageCloner<'a>(Mutex<UnsafeAllocatorRef<'a>>);

    impl<'a> MessageCloner<'a> {
        /// 将 Allocator 包装在 MessageCloner 中
        ///
        /// # 参数
        ///
        /// - `allocator`: 独占的可变引用，确保无其他引用存在
        ///
        /// # 保证
        ///
        /// 在 `MessageCloner` 存在期间，没有其他线程可以使用该 Allocator
        #[inline]
        #[expect(clippy::needless_pass_by_ref_mut)]
        pub fn new(allocator: &'a mut Allocator) -> Self {
            Self(Mutex::new(UnsafeAllocatorRef(allocator)))
        }

        /// 将 Message 克隆到该 MessageCloner 持有的 Allocator 中
        ///
        /// # 线程安全
        ///
        /// 使用 Mutex 锁确保同时只有一个线程执行 `clone_in` 操作
        ///
        /// # Panics
        ///
        /// 如果底层 Mutex 被 poison，会 panic
        pub fn clone_message(&self, message: &Message) -> Message<'a> {
            // 获取独占锁，确保没有其他线程同时使用 Allocator
            let guard = self.0.lock().unwrap();
            let allocator = guard.0;
            message.clone_in(allocator)
        }
    }
}
#[cfg(any(feature = "language_server", test))]
use message_cloner::MessageCloner;

impl Runtime {
    /// 创建新的 Runtime 实例
    ///
    /// # 初始化流程
    ///
    /// 1. **初始化线程池**：确保 Rayon 线程池已创建
    /// 2. **创建分配器池**：为每个线程创建独立的分配器
    /// 3. **配置解析器**：如果启用 cross_module，创建模块解析器
    /// 4. **设置文件系统**：使用默认的 OsFileSystem
    ///
    /// # 线程池初始化
    ///
    /// 调用 `build_global()` 确保线程池已初始化：
    /// - 如果未初始化：创建线程池，线程数基于 CPU 核心数
    /// - 如果已初始化：忽略错误，继续使用现有线程池
    ///
    /// 这确保了 `rayon::current_num_threads()` 的返回值在整个 Runtime 生命周期中保持稳定
    pub(super) fn new(linter: Linter, options: LintServiceOptions) -> Self {
        // ========================================================================================
        // 步骤 1: 初始化 Rayon 全局线程池
        // ========================================================================================
        //
        // 目的：确保线程池已创建并"锁定"配置
        //
        // 重要说明：
        // - Rayon 默认使用 `std::thread::available_parallelism()` 确定线程数
        // - 线程池初始化后，线程数就不会改变
        // - 这确保了后续的分配器池大小与线程数一致
        //
        // 注意：即使线程池已初始化，`build_global()` 返回 Err，我们也可以忽略
        // 因为线程数已经被"锁定"了
        let _ = rayon::ThreadPoolBuilder::new().build_global();

        // ========================================================================================
        // 步骤 2: 创建分配器池
        // ========================================================================================
        // 为每个线程创建独立的分配器，避免多线程竞争
        let thread_count = rayon::current_num_threads();
        let allocator_pool = AllocatorPool::new(thread_count);

        // ========================================================================================
        // 步骤 3: 配置模块解析器
        // ========================================================================================
        // 如果启用 cross_module 功能，创建解析器用于解析 import/export
        let resolver = options.cross_module.then(|| {
            Self::get_resolver(options.tsconfig.or_else(|| Some(options.cwd.join("tsconfig.json"))))
        });

        // ========================================================================================
        // 步骤 4: 创建 Runtime 实例
        // ========================================================================================
        Self {
            allocator_pool,
            cwd: options.cwd,
            paths: IndexSet::with_capacity_and_hasher(0, FxBuildHasher),
            linter,
            resolver,
            file_system: Box::new(OsFileSystem),
        }
    }

    pub fn with_file_system(
        &mut self,
        file_system: Box<dyn RuntimeFileSystem + Sync + Send>,
    ) -> &mut Self {
        self.file_system = file_system;
        self
    }

    pub fn with_paths(&mut self, paths: Vec<Arc<OsStr>>) -> &mut Self {
        self.paths = paths.into_iter().collect();
        self
    }

    /// 创建模块解析器
    ///
    /// # 配置说明
    ///
    /// - **扩展名**：支持所有 JavaScript/TypeScript 扩展名
    /// - **主字段**：优先查找 "module"，然后 "main"
    /// - **导出条件**：支持 "module" 和 "import" 条件
    /// - **扩展别名**：如果是 TypeScript 项目，配置扩展名别名
    /// - **tsconfig**：如果提供了 tsconfig.json，会解析路径映射等配置
    ///
    /// # TypeScript 支持
    ///
    /// 如果存在 tsconfig.json，会自动配置：
    /// - 扩展名别名：`.js` → `.js` 或 `.ts`
    /// - 路径映射：根据 `paths` 配置解析
    /// - 引用：支持 project references
    fn get_resolver(tsconfig_path: Option<PathBuf>) -> Resolver {
        use oxc_resolver::{ResolveOptions, TsconfigOptions, TsconfigReferences};

        // 检查 tsconfig.json 是否存在
        let tsconfig = tsconfig_path.and_then(|path| {
            path.is_file().then_some(TsconfigOptions {
                config_file: path,
                references: TsconfigReferences::Auto,
            })
        });

        // 如果是 TypeScript 项目，配置扩展名别名
        // 这允许 import 语句使用 .js 扩展名，但实际解析到 .ts 文件
        let extension_alias = tsconfig.as_ref().map_or_else(Vec::new, |_| {
            vec![
                (".js".into(), vec![".js".into(), ".ts".into()]),
                (".mjs".into(), vec![".mjs".into(), ".mts".into()]),
                (".cjs".into(), vec![".cjs".into(), ".cts".into()]),
            ]
        });

        Resolver::new(ResolveOptions {
            // 支持的所有文件扩展名
            extensions: VALID_EXTENSIONS.iter().map(|ext| format!(".{ext}")).collect(),
            // package.json 中查找的主字段顺序
            main_fields: vec!["module".into(), "main".into()],
            // 支持的导出条件
            condition_names: vec!["module".into(), "import".into()],
            // TypeScript 扩展名别名
            extension_alias,
            // TypeScript 配置
            tsconfig,
            ..ResolveOptions::default()
        })
    }

    /// 获取文件的源代码类型和文本内容
    ///
    /// # 返回类型
    ///
    /// - `None`：文件扩展名不支持或不在支持列表中
    /// - `Some(Ok(...))`：成功读取，返回 SourceType 和源文本
    /// - `Some(Err(...))`：读取失败，返回错误信息
    ///
    /// # 文件类型处理
    ///
    /// - **JSX 自动启用**：JavaScript 文件自动启用 JSX 支持
    /// - **多段文件**：`.vue`, `.astro` 等特殊扩展名使用 PartialLoader
    /// - **未支持文件**：其他扩展名返回 None
    fn get_source_type_and_text<'a>(
        &'a self,
        path: &Path,
        ext: &str,
        allocator: &'a Allocator,
    ) -> Option<Result<(SourceType, &'a str), Error>> {
        // 尝试从路径推断源代码类型
        let source_type = SourceType::from_path(path);

        // 检查是否支持该文件类型
        // 如果不支持且不是特殊的多段文件，返回 None
        let not_supported_yet =
            source_type.as_ref().is_err_and(|_| !LINT_PARTIAL_LOADER_EXTENSIONS.contains(&ext));
        if not_supported_yet {
            return None;
        }

        // 获取源类型，默认为 JavaScript
        let mut source_type = source_type.unwrap_or_default();

        // 为 JavaScript 文件自动启用 JSX 支持
        // 这样可以最大化成功解析的可能性
        if source_type.is_javascript() {
            source_type = source_type.with_jsx(true);
        }

        // 从文件系统读取源文本
        let file_result = self.file_system.read_to_arena_str(path, allocator).map_err(|e| {
            Error::new(OxcDiagnostic::error(format!(
                "Failed to open file {} with error \"{e}\"",
                path.display()
            )))
        });

        Some(match file_result {
            Ok(source_text) => Ok((source_type, source_text)),
            Err(e) => Err(e),
        })
    }

    /// 准备入口模块用于 linting
    ///
    /// # 核心流程
    ///
    /// 1. **分组处理**：将文件分组，避免内存溢出
    /// 2. **并行解析**：graph 线程构建依赖图，module 线程解析模块
    /// 3. **依赖解析**：递归解析所有依赖模块
    /// 4. **回调执行**：当模块及其依赖都解析完成时，调用 `on_module_to_lint`
    ///
    /// # 性能优化
    ///
    /// - **深度优先排序**：更深路径先处理（可能是叶子节点）
    /// - **提前释放**：处理完立即释放源文件和语义信息
    /// - **双线程架构**：graph 线程管理依赖图，module 线程并行解析
    ///
    /// # 参数
    ///
    /// - `on_module_to_lint`: 当模块准备好 lint 时调用的回调
    ///   模块准备好意味着其所有依赖都已解析（如果启用了 import 插件）
    fn resolve_modules<'a>(
        &'a mut self,
        scope: &Scope<'a>,
        check_syntax_errors: bool,
        tx_error: &'a DiagnosticSender,
        on_module_to_lint: impl Fn(&'a Self, ModuleToLint) + Send + Sync + Clone + 'a,
    ) {
        if self.resolver.is_none() {
            self.paths.par_iter().for_each(|path| {
                let output = self.process_path(path, check_syntax_errors, tx_error);
                let Some(entry) =
                    ModuleToLint::from_processed_module(output.path, output.processed_module)
                else {
                    return;
                };
                on_module_to_lint(self, entry);
            });
            return;
        }
        // The goal of code below is to construct the module graph bootstrapped by the entry modules (`self.paths`),
        // and call `on_entry` when all dependencies of that entry is resolved. We want to call `on_entry` for each
        // entry as soon as possible, so that the memory for source texts and semantics can be released early.

        // ==============================================================================================
        // 优化策略：按路径深度排序
        // ==============================================================================================
        //
        // 原因：深度优先处理可以减少内存峰值
        //
        // 示例场景：
        // - src/index.js          (入口文件，依赖很多)
        // - src/a/foo.js          (依赖较少)
        // - src/b/bar.js          (依赖较少)
        // - src/very/deep/path/baz.js  (叶子节点，无依赖)
        //
        // 问题：如果从 index.js 开始构建模块图，所有文件的源文本和语义信息
        //      都必须保持在内存中，直到最后一个依赖处理完成
        //
        // 解决方案：从"叶子"模块开始处理
        // - 叶子模块的依赖更早准备好
        // - 可以更早运行 lint 并释放内存
        //
        // 启发式：更深的路径更可能是叶子模块
        // - src/very/deep/path/baz.js 比 src/index.js 的依赖更少
        // - 虽然不总是正确，但在真实代码库中效果很好
        self.paths.par_sort_unstable_by(|a, b| Path::new(b).cmp(Path::new(a)));

        // ==============================================================================================
        // 分组处理策略
        // ==============================================================================================
        //
        // 核心思想：分批处理模块及其依赖
        //
        // 优点：
        // 1. 内存可控：每组大小适中，避免内存溢出
        // 2. 充分利用线程池：组足够大以利用并行性
        // 3. 及时释放：处理完一组后立即释放源文本和语义信息
        //
        // 工作方式：
        // 1. 构建当前组的模块图
        // 2. 运行 lint
        // 3. 释放源文件和语义（保留模块图）
        // 4. 处理下一组
        //
        // 组大小：4 * 线程数（基于 AFFiNE@97cc814a 的经验值）
        let group_size = rayon::current_num_threads() * 4;

        // ==============================================================================================
        // 初始化数据结构
        // ==============================================================================================

        // 当前组中需要 lint 的入口模块列表
        // 这些模块将在步骤 4 中被并行 lint
        let mut modules_to_lint: Vec<ModuleToLint> = Vec::with_capacity(group_size);

        // 将 self 转为不可变引用，以便在并行任务中共享
        // 这确保了在 graph 线程中是安全的
        let me: &Self = self;

        // 模块图：以路径为键，模块记录为值
        //
        // 设计要点：
        // - 键：模块路径（Arc<OsStr>）
        // - 值：该模块的所有段的模块记录
        // - 跨组持久化：前面的组发现的模块可能被后面的组引用
        // - 用于在步骤 3 中填充 loaded_modules
        let mut modules_by_path =
            FxHashMap::<Arc<OsStr>, SmallVec<[Arc<ModuleRecord>; 1]>>::with_capacity_and_hasher(
                me.paths.len(),
                FxBuildHasher,
            );

        // 已遇到的路径集合，防止重复处理
        //
        // 这是 `modules_by_path` 的超集，因为：
        // - 包含已处理的模块
        // - 包含已排队的模块（正在处理中）
        let mut encountered_paths =
            FxHashSet::<Arc<OsStr>>::with_capacity_and_hasher(me.paths.len(), FxBuildHasher);

        // 当前组中模块的依赖请求
        //
        // 用途：暂存依赖关系，待所有依赖处理完成后写入 loaded_modules
        // 结构：(模块路径, 每个段的依赖请求列表)
        let mut module_paths_and_resolved_requests =
            Vec::<(Arc<OsStr>, SmallVec<[Vec<ResolvedModuleRequest>; 1]>)>::new();

        // ==============================================================================================
        // 双线程架构设计
        // ==============================================================================================
        //
        // Graph 线程（主线程）：
        // - 唯一的线程，负责调用 `resolve_modules`
        // - 负责更新模块图（无需锁，单线程更新）
        // - 使用 try_recv + yield_now 避免空闲等待
        //
        // Module 线程（并行线程）：
        // - 接收路径，生成 `ModuleProcessOutput`
        // - 在 Rayon 线程池中并行执行
        // - 处理逻辑在 `self.process_path` 中

        // 通信通道：module 线程 → graph 线程
        let (tx_process_output, rx_process_output) = mpsc::channel::<ModuleProcessOutput>();

        // 当前组的起始位置指针
        let mut group_start = 0usize;

        // ==============================================================================================
        // 分组处理主循环
        // ==============================================================================================
        // 外层循环：遍历所有组
        while group_start < me.paths.len() {
            // 当前组中已排队但未处理的模块数量
            let mut pending_module_count = 0;

            // ============================================================================================
            // 步骤 1: 启动入口模块的处理
            // ============================================================================================
            // 从 `self.paths` 中取出一个组的模块，启动并行处理
            while pending_module_count < group_size && group_start < me.paths.len() {
                let path = &me.paths[group_start];
                group_start += 1;

                // 检查该模块是否已经在之前的组中作为依赖被处理过了
                // 如果是，跳过；否则加入处理队列
                if encountered_paths.insert(Arc::clone(path)) {
                    pending_module_count += 1;
                    let path = Arc::clone(path);
                    let tx_process_output = tx_process_output.clone();

                    // 在 module 线程中处理该模块
                    scope.spawn(move |_| {
                        tx_process_output
                            .send(me.process_path(&path, check_syntax_errors, tx_error))
                            .unwrap();
                    });
                }
            }

            // ============================================================================================
            // 步骤 2: 处理所有已排队的模块（包括入口模块和依赖模块）
            // ============================================================================================
            // 内层循环：处理当前组的所有模块，直到全部完成
            // 每次迭代将一个新模块添加到模块图中
            while pending_module_count > 0 {
                // 非阻塞接收模块处理结果
                //
                // 性能优化：使用 try_recv 而不是 recv（阻塞）
                // - 模块线程负责重活（解析），graph 线程负责轻活（更新图）
                // - 如果阻塞等待，graph 线程会空闲浪费
                // - 使用 try_recv + yield_now 让 graph 线程可以参与模块处理
                let Ok(ModuleProcessOutput { path, mut processed_module }) =
                    rx_process_output.try_recv()
                else {
                    // 如果没有结果，让出 CPU 时间片
                    // 这样 Rayon 可以调度 graph 线程去执行模块处理或 linting
                    rayon::yield_now();
                    continue;
                };
                pending_module_count -= 1;

                // ========================================================================================
                // 步骤 2.1: 递归处理依赖模块
                // ========================================================================================
                // 遍历当前模块的所有依赖，如果还未处理，则启动处理
                for record_result in &processed_module.section_module_records {
                    let Ok(record) = record_result.as_ref() else {
                        continue;
                    };

                    // 遍历该段的所有依赖请求
                    for request in &record.resolved_module_requests {
                        let dep_path = &request.resolved_requested_path;

                        // 如果依赖模块还未处理过，加入处理队列
                        if encountered_paths.insert(Arc::clone(dep_path)) {
                            scope.spawn({
                                let tx_process_output = tx_process_output.clone();
                                let dep_path = Arc::clone(dep_path);
                                move |_| {
                                    tx_process_output
                                        .send(me.process_path(
                                            &dep_path,
                                            check_syntax_errors,
                                            tx_error,
                                        ))
                                        .unwrap();
                                }
                            });
                            pending_module_count += 1;
                        }
                    }
                }

                // ========================================================================================
                // 步骤 2.2: 更新模块图
                // ========================================================================================
                // 将模块记录添加到 `modules_by_path`，供后续依赖解析使用
                modules_by_path.insert(
                    Arc::clone(&path),
                    processed_module
                        .section_module_records
                        .iter()
                        .filter_map(|resolved_module_record| {
                            Some(Arc::clone(&resolved_module_record.as_ref().ok()?.module_record))
                        })
                        .collect(),
                );

                // ========================================================================================
                // 步骤 2.3: 暂存依赖关系
                // ========================================================================================
                // 注意：我们不能立即写入 `loaded_modules`，因为依赖模块可能还未处理完
                // 解决方案：先暂存依赖关系，待所有依赖处理完成后再写入
                module_paths_and_resolved_requests.push((
                    Arc::clone(&path),
                    processed_module
                        .section_module_records
                        .iter_mut()
                        .filter_map(|record_result| {
                            Some(take(&mut record_result.as_mut().ok()?.resolved_module_requests))
                        })
                        .collect(),
                ));

                // ========================================================================================
                // 步骤 2.4: 收集需要 lint 的入口模块
                // ========================================================================================
                // 检查该模块是否是入口文件（在 `self.paths` 中）
                // 如果是，将其添加到 `modules_to_lint` 列表
                if let Some(entry_module) =
                    ModuleToLint::from_processed_module(path, processed_module)
                {
                    modules_to_lint.push(entry_module);
                }
            } // while pending_module_count > 0

            // ============================================================================================
            // 步骤 3: 填充 loaded_modules（当前组的所有依赖已处理完成）
            // ============================================================================================
            // 现在依赖关系已明确，可以安全地写入 `loaded_modules` 了
            module_paths_and_resolved_requests.par_drain(..).for_each(|(path, requested_module_paths)| {
                if requested_module_paths.is_empty() {
                    return;
                }

                // 获取该模块的所有段记录
                let records = &modules_by_path[&path];

                // 断言：段记录数应与请求模块路径数相等
                assert_eq!(
                    records.len(), requested_module_paths.len(),
                    "This is an internal logic error. Please file an issue at https://github.com/oxc-project/oxc/issues",
                );

                // 为每个段填充其 loaded_modules
                for (record, requested_module_paths) in
                    records.iter().zip(requested_module_paths.into_iter())
                {
                    let mut loaded_modules = record.loaded_modules.write().unwrap();
                    for request in requested_module_paths {
                        // TODO: 需要重新设计多段文件在 loaded_modules 中的存储方式
                        // 目前只使用最后一段的模块记录
                        let Some(dep_module_record) =
                            modules_by_path[&request.resolved_requested_path].last()
                        else {
                            continue;
                        };
                        loaded_modules.insert(request.specifier, Arc::clone(dep_module_record));
                    }
                }
            });

            // ============================================================================================
            // 步骤 4: 并行执行 Linting
            // ============================================================================================
            // 所有依赖已准备好，现在可以对入口模块执行 linting
            #[expect(clippy::iter_with_drain)]
            for entry in modules_to_lint.drain(..) {
                let on_entry = on_module_to_lint.clone();
                scope.spawn(move |_| {
                    on_entry(me, entry);
                });
            }
        }
    }

    /// 运行 linter，处理所有文件
    ///
    /// # 工作流程
    ///
    /// 1. **解析模块**：解析所有文件和依赖
    /// 2. **执行 Linting**：对每个模块运行规则检查
    /// 3. **应用修复**：如果启用了 fix，自动修复代码
    /// 4. **收集诊断**：收集所有错误和警告
    ///
    /// # 并行处理
    ///
    /// 使用 Rayon 实现并行处理，充分利用多核 CPU
    ///
    /// # 错误处理
    ///
    /// 所有诊断信息通过 `tx_error` 通道发送，由调用者统一处理
    pub(super) fn run(&mut self, tx_error: &DiagnosticSender) {
        rayon::scope(|scope| {
            self.resolve_modules(scope, true, tx_error, |me, mut module_to_lint| {
                module_to_lint.content.with_dependent_mut(|allocator_guard, dep| {
                    // 修复处理：使用 Cow 避免不必要的内存分配
                    // 如果同一文件有多个段（如 .astro 文件），会累积所有修复后再写入文件
                    // 这样避免对同一文件进行多次写入
                    let mut new_source_text = Cow::from(dep.source_text);

                    let path = Path::new(&module_to_lint.path);

                    // 断言：段记录数和段内容数应该相等
                    assert_eq!(
                        module_to_lint.section_module_records.len(),
                        dep.section_contents.len()
                    );

                    // ====================================================================================
                    // 步骤 1: 准备上下文子主机（每个段一个）
                    // ====================================================================================
                    let context_sub_hosts: Vec<ContextSubHost<'_>> = module_to_lint
                        .section_module_records
                        .into_iter()
                        .zip(dep.section_contents.drain(..))
                        .filter_map(|(record_result, section)| match record_result {
                            Ok(module_record) => {
                                // 解析成功：创建上下文，用于 linting
                                Some(ContextSubHost::new_with_framework_options(
                                    section.semantic.unwrap(),
                                    Arc::clone(&module_record),
                                    section.source.start,
                                    section.source.framework_options,
                                ))
                            }
                            Err(messages) => {
                                // 解析失败：发送诊断信息
                                if !messages.is_empty() {
                                    let diagnostics = DiagnosticService::wrap_diagnostics(
                                        &me.cwd,
                                        path,
                                        dep.source_text,
                                        messages,
                                    );
                                    tx_error.send((path.to_path_buf(), diagnostics)).unwrap();
                                }
                                None
                            }
                        })
                        .collect();

                    // 如果所有段都解析失败，跳过 linting
                    if context_sub_hosts.is_empty() {
                        return;
                    }

                    // ====================================================================================
                    // 步骤 2: 执行 Linting
                    // ====================================================================================
                    let mut messages = me.linter.run(path, context_sub_hosts, allocator_guard);

                    // ====================================================================================
                    // 步骤 3: 应用自动修复（如果启用）
                    // ====================================================================================
                    if me.linter.options().fix.is_some() {
                        let fix_result = Fixer::new(dep.source_text, messages).fix();
                        if fix_result.fixed {
                            // 将修复后的代码替换原代码
                            // 注意：使用完整的范围替换（0..len），因为修复器已经考虑了所有段的偏移
                            let start = 0;
                            let end = start + dep.source_text.len();
                            new_source_text
                                .to_mut()
                                .replace_range(start..end, &fix_result.fixed_code);
                        }
                        messages = fix_result.messages;
                    }

                    // ====================================================================================
                    // 步骤 4: 收集诊断信息
                    // ====================================================================================
                    if !messages.is_empty() {
                        let errors = messages.into_iter().map(Into::into).collect();
                        let diagnostics = DiagnosticService::wrap_diagnostics(
                            &me.cwd,
                            path,
                            dep.source_text,
                            errors,
                        );
                        tx_error.send((path.to_path_buf(), diagnostics)).unwrap();
                    }

                    // ====================================================================================
                    // 步骤 5: 写入修复后的文件
                    // ====================================================================================
                    // 检查代码是否被修改（Cow::Owned 表示进行了修改）
                    if let Cow::Owned(new_source_text) = &new_source_text {
                        me.file_system.write_file(path, new_source_text).unwrap();
                    }
                });
            });
        });
    }

    // language_server: the language server needs line and character position
    // the struct not using `oxc_diagnostic::Error, because we are just collecting information
    // and returning it to the client to let him display it.
    #[cfg(feature = "language_server")]
    pub(super) fn run_source<'a>(
        &mut self,
        allocator: &'a mut oxc_allocator::Allocator,
    ) -> Vec<MessageWithPosition<'a>> {
        use std::sync::Mutex;

        use oxc_data_structures::rope::Rope;

        use crate::lsp::message_to_message_with_position;

        // Wrap allocator in `MessageCloner` so can clone `Message`s into it
        let message_cloner = MessageCloner::new(allocator);

        let messages = Mutex::new(Vec::<MessageWithPosition<'a>>::new());
        let (sender, _receiver) = mpsc::channel();
        rayon::scope(|scope| {
            self.resolve_modules(scope, true, &sender, |me, mut module_to_lint| {
                module_to_lint.content.with_dependent_mut(
                    |allocator_guard, ModuleContentDependent { source_text, section_contents }| {
                        assert_eq!(
                            module_to_lint.section_module_records.len(),
                            section_contents.len()
                        );

                        let rope = &Rope::from_str(source_text);

                        let context_sub_hosts: Vec<ContextSubHost<'_>> = module_to_lint
                            .section_module_records
                            .into_iter()
                            .zip(section_contents.drain(..))
                            .filter_map(|(record_result, section)| match record_result {
                                Ok(module_record) => {
                                    Some(ContextSubHost::new_with_framework_options(
                                        section.semantic.unwrap(),
                                        Arc::clone(&module_record),
                                        section.source.start,
                                        section.source.framework_options,
                                    ))
                                }
                                Err(diagnostics) => {
                                    if !diagnostics.is_empty() {
                                        messages
                                            .lock()
                                            .unwrap()
                                            .extend(diagnostics.into_iter().map(Into::into));
                                    }
                                    None
                                }
                            })
                            .collect();

                        if context_sub_hosts.is_empty() {
                            return;
                        }

                        let section_messages = me.linter.run(
                            Path::new(&module_to_lint.path),
                            context_sub_hosts,
                            allocator_guard,
                        );

                        messages.lock().unwrap().extend(section_messages.iter().map(|message| {
                            let message = message_cloner.clone_message(message);

                            message_to_message_with_position(&message, source_text, rope)
                        }));
                    },
                );
            });
        });

        // ToDo: oxc_diagnostic::Error is not compatible with MessageWithPosition
        // send use OxcDiagnostic or even better the MessageWithPosition struct
        // while let Ok(diagnostics) = receiver.recv() {
        //     if let Some(diagnostics) = diagnostics {
        //         messages.lock().unwrap().extend(
        //             diagnostics.1
        //                 .into_iter()
        //                 .map(|report| MessageWithPosition::from(report))
        //         );
        //     }
        // }

        messages.into_inner().unwrap()
    }

    #[cfg(test)]
    pub(super) fn run_test_source<'a>(
        &mut self,
        allocator: &'a mut Allocator,
        check_syntax_errors: bool,
        tx_error: &DiagnosticSender,
    ) -> Vec<Message<'a>> {
        use std::sync::Mutex;

        // Wrap allocator in `MessageCloner` so can clone `Message`s into it
        let message_cloner = MessageCloner::new(allocator);

        let messages = Mutex::new(Vec::<Message<'a>>::new());
        rayon::scope(|scope| {
            self.resolve_modules(scope, check_syntax_errors, tx_error, |me, mut module| {
                module.content.with_dependent_mut(
                    |allocator_guard, ModuleContentDependent { source_text: _, section_contents }| {
                        assert_eq!(module.section_module_records.len(), section_contents.len());

                        let context_sub_hosts: Vec<ContextSubHost<'_>> = module
                            .section_module_records
                            .into_iter()
                            .zip(section_contents.drain(..))
                            .filter_map(|(record_result, section)| match record_result {
                                Ok(module_record) => Some(ContextSubHost::new_with_framework_options(
                                    section.semantic.unwrap(),
                                    Arc::clone(&module_record),
                                    section.source.start,
                                    section.source.framework_options
                                )),
                                Err(errors) => {
                                    if !errors.is_empty() {
                                        messages
                                            .lock()
                                            .unwrap()
                                            .extend(errors
                                        .into_iter()
                                        .map(|err| Message::new(err, PossibleFixes::None))
                                    );
                                    }
                                    None
                                }
                            })
                            .collect();

                        if context_sub_hosts.is_empty() {
                            return;
                        }

                        messages.lock().unwrap().extend(
                            me.linter.run(
                                Path::new(&module.path),
                                context_sub_hosts,
                                allocator_guard
                            ).iter_mut()
                                .map(|message| {
                                    message_cloner.clone_message(message)
                                }),
                        );
                    },
                );
            });
        });
        messages.into_inner().unwrap()
    }

    /// 处理单个文件路径
    ///
    /// # 功能
    ///
    /// 读取文件、解析 AST、构建语义信息、解析依赖
    ///
    /// # 返回值
    ///
    /// 始终返回 `ModuleProcessOutput`，即使解析失败
    fn process_path(
        &self,
        path: &Arc<OsStr>,
        check_syntax_errors: bool,
        tx_error: &DiagnosticSender,
    ) -> ModuleProcessOutput<'_> {
        let processed_module =
            self.process_path_to_module(path, check_syntax_errors, tx_error).unwrap_or_default();
        ModuleProcessOutput { path: Arc::clone(path), processed_module }
    }

    /// 将文件路径转换为处理后的模块
    ///
    /// # 区别处理
    ///
    /// - **入口文件**（在 `self.paths` 中）：需要 lint，保存源文件和语义
    /// - **依赖文件**：只需要模块记录，不保存源文件（节省内存）
    ///
    /// # 返回 None
    ///
    /// - 文件扩展名不支持
    /// - 无法读取文件
    fn process_path_to_module(
        &self,
        path: &Arc<OsStr>,
        check_syntax_errors: bool,
        tx_error: &DiagnosticSender,
    ) -> Option<ProcessedModule<'_>> {
        // 获取文件扩展名
        let ext = Path::new(path).extension().and_then(OsStr::to_str)?;

        // 检查文件类型是否支持
        if SourceType::from_path(Path::new(path))
            .as_ref()
            .is_err_and(|_| !LINT_PARTIAL_LOADER_EXTENSIONS.contains(&ext))
        {
            return None;
        }

        // 从分配器池获取一个分配器
        let allocator_guard = self.allocator_pool.get();

        // ========================================================================================
        // 分支：入口文件 vs 依赖文件
        // ========================================================================================
        if self.paths.contains(path) {
            // ====================================================================================
            // 入口文件：需要 lint，保存源文件和语义
            // ====================================================================================
            let mut records =
                SmallVec::<[Result<ResolvedModuleRecord, Vec<OxcDiagnostic>>; 1]>::new();

            // 使用 self_cell 确保 source_text 和 section_contents 的生命周期一致
            let module_content = ModuleContent::try_new(allocator_guard, |allocator_guard| {
                let allocator = &**allocator_guard;

                // 读取源文件和确定类型
                let Some(stt) = self.get_source_type_and_text(Path::new(path), ext, allocator)
                else {
                    return Err(());
                };

                let (source_type, source_text) = match stt {
                    Ok(v) => v,
                    Err(e) => {
                        // 读取失败，发送错误诊断
                        tx_error.send((Path::new(path).to_path_buf(), vec![e])).unwrap();
                        return Err(());
                    }
                };

                // 解析源文件（可能包含多个段）
                let mut section_contents = SmallVec::new();
                records = self.process_source(
                    Path::new(path),
                    ext,
                    check_syntax_errors,
                    source_type,
                    source_text,
                    allocator,
                    Some(&mut section_contents), // 需要保存 section_contents
                );

                Ok(ModuleContentDependent { source_text, section_contents })
            });
            let module_content = module_content.ok()?;

            Some(ProcessedModule { section_module_records: records, content: Some(module_content) })
        } else {
            // ====================================================================================
            // 依赖文件：只需要模块记录，不保存源文件
            // ====================================================================================
            let allocator = &*allocator_guard;

            // 读取源文件（但不保留在 ProcessedModule 中）
            let stt = self.get_source_type_and_text(Path::new(path), ext, allocator)?;

            let (source_type, source_text) = match stt {
                Ok(v) => v,
                Err(e) => {
                    tx_error.send((Path::new(path).to_path_buf(), vec![e])).unwrap();
                    return None;
                }
            };

            // 解析源文件，只保留模块记录，不保存源文本和语义
            let records = self.process_source(
                Path::new(path),
                ext,
                check_syntax_errors,
                source_type,
                source_text,
                allocator,
                None, // 不需要保存 section_contents
            );

            Some(ProcessedModule { section_module_records: records, content: None })
        }
    }

    /// 处理源文件（可能包含多个段）
    ///
    /// # 多段文件支持
    ///
    /// 某些文件格式（如 .vue, .astro）包含多个源文件段：
    /// - 每个段独立解析
    /// - 每个段生成独立的模块记录
    /// - 解析失败的段会生成错误诊断
    ///
    /// # 参数
    ///
    /// - `out_sections`: 如果需要 lint，传入此参数保存源文本和语义
    ///                   None 表示这是依赖文件，只需要模块记录
    ///
    /// # 返回值
    ///
    /// 返回所有段的解析结果
    #[expect(clippy::too_many_arguments)]
    fn process_source<'a>(
        &self,
        path: &Path,
        ext: &str,
        check_syntax_errors: bool,
        source_type: SourceType,
        source_text: &'a str,
        allocator: &'a Allocator,
        mut out_sections: Option<&mut SectionContents<'a>>,
    ) -> SmallVec<[Result<ResolvedModuleRecord, Vec<OxcDiagnostic>>; 1]> {
        // 解析源文件为多个段
        // 如果不是多段文件，则返回一个默认段
        let section_sources = PartialLoader::parse(ext, source_text)
            .unwrap_or_else(|| vec![JavaScriptSource::partial(source_text, source_type, 0)]);

        let mut section_module_records = SmallVec::<
            [Result<ResolvedModuleRecord, Vec<OxcDiagnostic>>; 1],
        >::with_capacity(section_sources.len());

        // 处理每个段
        for section_source in section_sources {
            match self.process_source_section(
                path,
                allocator,
                section_source.source_text,
                section_source.source_type,
                check_syntax_errors,
            ) {
                Ok((record, semantic)) => {
                    // 解析成功：保存模块记录
                    section_module_records.push(Ok(record));

                    // 如果需要 lint，同时保存源文本和语义
                    if let Some(sections) = &mut out_sections {
                        sections.push(SectionContent {
                            source: section_source,
                            semantic: Some(semantic),
                        });
                    }
                }
                Err(err) => {
                    // 解析失败：保存错误诊断
                    section_module_records.push(Err(err));

                    // 如果需要 lint，保存源文本但无语义
                    if let Some(sections) = &mut out_sections {
                        sections.push(SectionContent { source: section_source, semantic: None });
                    }
                }
            }
        }
        section_module_records
    }

    /// 处理单个源文件段
    ///
    /// # 处理流程
    ///
    /// 1. **解析 AST**：将源代码解析为抽象语法树
    /// 2. **构建语义**：生成作用域、符号、类型信息
    /// 3. **创建模块记录**：提取 import/export 信息
    /// 4. **解析依赖**：如果启用 import 插件，解析所有依赖模块
    ///
    /// # 返回值
    ///
    /// - `Ok((record, semantic))`：解析成功
    /// - `Err(diagnostics)`：解析失败，返回诊断信息
    fn process_source_section<'a>(
        &self,
        path: &Path,
        allocator: &'a Allocator,
        source_text: &'a str,
        source_type: SourceType,
        check_syntax_errors: bool,
    ) -> Result<(ResolvedModuleRecord, Semantic<'a>), Vec<OxcDiagnostic>> {
        // ========================================================================================
        // 步骤 1: 解析 AST
        // ========================================================================================
        let ret = Parser::new(allocator, source_text, source_type)
            .with_options(ParseOptions {
                parse_regular_expression: true,          // 解析正则表达式
                allow_return_outside_function: true,      // 允许在函数外使用 return
                ..ParseOptions::default()
            })
            .parse();

        // 检查解析错误
        if !ret.errors.is_empty() {
            // Flow 语言错误被忽略（不支持 Flow）
            return Err(if ret.is_flow_language { vec![] } else { ret.errors });
        }

        // ========================================================================================
        // 步骤 2: 构建语义信息
        // ========================================================================================
        let semantic_ret = SemanticBuilder::new()
            .with_cfg(true)                          // 构建控制流图
            .with_scope_tree_child_ids(true)         // 保留作用域树子节点 ID
            .with_build_jsdoc(true)                  // 解析 JSDoc 注释
            .with_check_syntax_error(check_syntax_errors)  // 检查语法错误
            .build(allocator.alloc(ret.program));

        // 检查语义分析错误
        if !semantic_ret.errors.is_empty() {
            return Err(semantic_ret.errors);
        }

        let mut semantic = semantic_ret.semantic;
        // 设置不规则的空白字符（如 tab, nbsp）
        semantic.set_irregular_whitespaces(ret.irregular_whitespaces);

        // ========================================================================================
        // 步骤 3: 创建模块记录
        // ========================================================================================
        let module_record = Arc::new(ModuleRecord::new(path, &ret.module_record, &semantic));

        // ========================================================================================
        // 步骤 4: 解析依赖模块（如果启用 import 插件）
        // ========================================================================================
        let mut resolved_module_requests: Vec<ResolvedModuleRequest> = vec![];

        if let Some(resolver) = &self.resolver {
            // 获取当前模块所在目录
            let dir = path.parent().unwrap();

            // 解析该模块的所有 import 语句
            resolved_module_requests = module_record
                .requested_modules
                .keys()
                .filter_map(|specifier| {
                    // 解析 specifier 到实际文件路径
                    let resolution = resolver.resolve(dir, specifier).ok()?;
                    Some(ResolvedModuleRequest {
                        specifier: specifier.clone(),
                        resolved_requested_path: Arc::<OsStr>::from(resolution.path().as_os_str()),
                    })
                })
                .collect();
        }

        Ok((ResolvedModuleRecord { module_record, resolved_module_requests }, semantic))
    }
}
