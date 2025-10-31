use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};

use oxc_diagnostics::DiagnosticSender;

use crate::Linter;

mod runtime;
use runtime::Runtime;
pub use runtime::RuntimeFileSystem;

/// Lint 服务运行所需的环境配置，负责描述工作目录、项目配置以及跨模块分析等行为。
pub struct LintServiceOptions {
    /// Current working directory
    cwd: Box<Path>,
    /// TypeScript `tsconfig.json` path for reading path alias and project references
    tsconfig: Option<PathBuf>,

    /// 是否开启跨模块分析（例如在项目模式下对依赖关系做更深层次的解析）
    cross_module: bool,
}

impl LintServiceOptions {
    #[must_use]
    /// 使用当前工作目录创建默认配置。
    pub fn new<T>(cwd: T) -> Self
    where
        T: Into<Box<Path>>,
    {
        Self { cwd: cwd.into(), tsconfig: None, cross_module: false }
    }

    #[inline]
    #[must_use]
    /// 指定 `tsconfig.json` 文件位置，用于解析路径别名与项目引用。
    pub fn with_tsconfig<T>(mut self, tsconfig: T) -> Self
    where
        T: Into<PathBuf>,
    {
        let tsconfig = tsconfig.into();
        // Should this be canonicalized?
        let tsconfig = if tsconfig.is_relative() { self.cwd.join(tsconfig) } else { tsconfig };
        debug_assert!(tsconfig.is_file());

        self.tsconfig = Some(tsconfig);
        self
    }

    #[inline]
    #[must_use]
    /// 开启或关闭跨模块分析能力。
    pub fn with_cross_module(mut self, cross_module: bool) -> Self {
        self.cross_module = cross_module;
        self
    }

    #[inline]
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }
}

/// Lint 服务的外层封装，负责组合运行时并对外提供统一的执行接口。
pub struct LintService {
    runtime: Runtime,
}

impl LintService {
    /// 根据给定的 `Linter` 与运行选项创建新的运行时实例。
    pub fn new(linter: Linter, options: LintServiceOptions) -> Self {
        let runtime = Runtime::new(linter, options);
        Self { runtime }
    }

    /// 为运行时替换自定义文件系统，实现与实际环境或测试环境的文件交互。
    pub fn with_file_system(
        &mut self,
        file_system: Box<dyn RuntimeFileSystem + Sync + Send>,
    ) -> &mut Self {
        self.runtime.with_file_system(file_system);
        self
    }

    /// 指定需要 lint 的路径集合。
    pub fn with_paths(&mut self, paths: Vec<Arc<OsStr>>) -> &mut Self {
        self.runtime.with_paths(paths);
        self
    }

    /// # Panics
    /// 执行 lint 过程并将产生的诊断信息通过 `DiagnosticSender` 发送出去。
    pub fn run(&mut self, tx_error: &DiagnosticSender) {
        self.runtime.run(tx_error);
    }

    #[cfg(feature = "language_server")]
    /// LSP 模式：返回按源码位置排序的诊断信息。
    pub fn run_source<'a>(
        &mut self,
        allocator: &'a mut oxc_allocator::Allocator,
    ) -> Vec<crate::MessageWithPosition<'a>> {
        self.runtime.run_source(allocator)
    }

    /// For tests
    #[cfg(test)]
    /// 测试模式：允许控制语法检查并获取测试场景下的诊断结果。
    pub(crate) fn run_test_source<'a>(
        &mut self,
        allocator: &'a mut oxc_allocator::Allocator,
        check_syntax_errors: bool,
        tx_error: &DiagnosticSender,
    ) -> Vec<crate::Message<'a>> {
        self.runtime.run_test_source(allocator, check_syntax_errors, tx_error)
    }
}
