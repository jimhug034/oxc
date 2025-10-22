use std::path::PathBuf;

use bpaf::Bpaf;
use oxc_linter::{AllowWarnDeny, BuiltinLintPlugins, FixKind, LintPlugins};

use crate::output_formatter::OutputFormat;

use super::{
    MiscOptions, PATHS_ERROR_MESSAGE, VERSION,
    ignore::{IgnoreOptions, ignore_options},
    misc_options, validate_paths,
};

/// Lint 命令的完整参数结构
///
/// 这个结构体包含了 `oxlint` 命令行工具的所有参数。
/// 使用 `bpaf` 库进行命令行参数解析（一个高性能的参数解析库）。
///
/// # 参数分组
///
/// 参数被组织成多个逻辑组：
/// - 基本配置：配置文件、tsconfig 等
/// - 规则过滤：启用/禁用特定规则或类别
/// - 插件管理：启用/禁用各种插件
/// - 修复选项：自动修复相关设置
/// - 忽略选项：.gitignore 和 ignore 模式
/// - 警告选项：警告的处理方式
/// - 输出选项：输出格式
/// - 杂项：线程数、静默模式等
///
/// # 示例
///
/// ```bash
/// # 基本用法
/// oxlint src/
///
/// # 带参数
/// oxlint -D correctness -A no-debugger --fix src/
/// ```
#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version(VERSION))]
pub struct LintCommand {
    /// 基本配置选项
    /// 包括：--config, --tsconfig, --init
    #[bpaf(external)]
    pub basic_options: BasicOptions,

    /// 规则过滤器列表
    /// 从命令行收集的所有 -A/-W/-D 参数
    /// 例如：-D correctness -A no-debugger
    #[bpaf(external(lint_filter), map(LintFilter::into_tuple), many, hide_usage)]
    pub filter: Vec<(AllowWarnDeny, String)>,

    /// 插件启用/禁用设置
    /// 包括所有 --*-plugin 和 --disable-*-plugin 标志
    #[bpaf(external)]
    pub enable_plugins: EnablePlugins,

    /// 自动修复选项
    /// --fix, --fix-suggestions, --fix-dangerously
    #[bpaf(external)]
    pub fix_options: FixOptions,

    /// 文件忽略选项
    /// --no-ignore, --ignore-path, --ignore-pattern
    #[bpaf(external)]
    pub ignore_options: IgnoreOptions,

    /// 警告处理选项
    /// --quiet, --deny-warnings, --max-warnings
    #[bpaf(external)]
    pub warning_options: WarningOptions,

    /// 输出格式选项
    /// --format (-f): default, json, checkstyle, etc.
    #[bpaf(external)]
    pub output_options: OutputOptions,

    /// 列出所有已注册的规则
    /// 使用 --rules 标志
    #[bpaf(long("rules"), switch, hide_usage)]
    pub list_rules: bool,

    /// 杂项选项
    /// --silent, --threads, --print-config
    #[bpaf(external)]
    pub misc_options: MiscOptions,

    /// 禁用自动加载嵌套配置文件
    /// 默认情况下，Oxlint 会在子目录中查找配置文件
    #[bpaf(switch, hide_usage)]
    pub disable_nested_config: bool,

    /// 启用需要类型信息的规则
    /// 这会调用 tsgolint（Go 实现）来执行类型感知的规则
    #[bpaf(switch, hide_usage)]
    pub type_aware: bool,

    /// 内联配置注释选项
    /// --report-unused-disable-directives 相关
    #[bpaf(external)]
    pub inline_config_options: InlineConfigOptions,

    /// 要检查的文件或目录路径列表
    /// 位置参数，可以有多个
    /// 例如：oxlint src/ test/ utils/helper.js
    #[bpaf(positional("PATH"), many, guard(validate_paths, PATHS_ERROR_MESSAGE))]
    pub paths: Vec<PathBuf>,
}

impl LintCommand {
    /// 处理线程配置
    ///
    /// 在执行 linting 之前必须调用此方法来初始化 Rayon 线程池。
    pub fn handle_threads(&self) {
        Self::init_rayon_thread_pool(self.misc_options.threads);
    }

    /// 初始化 Rayon 全局线程池
    ///
    /// 根据 `--threads` 选项或 CPU 核心数设置线程数。
    ///
    /// # 行为
    ///
    /// - 如果指定了 `--threads N` 且 N > 0：使用 N 个线程
    /// - 如果没有指定或指定了 `--threads 0`：使用 CPU 核心数
    /// - 如果无法确定 CPU 核心数：默认为 1 个线程
    ///
    /// # 为什么总是显式初始化？
    ///
    /// 即使使用默认线程数，我们也总是显式初始化线程池，以确保：
    /// 1. 线程数在程序运行期间保持不变（"锁定"）
    /// 2. 避免 Rayon 未来版本可能引入的动态线程管理
    ///
    /// 详见：<https://docs.rs/rayon/1.11.0/rayon/struct.ThreadPoolBuilder.html#method.num_threads>
    #[expect(clippy::print_stderr)]
    fn init_rayon_thread_pool(threads: Option<usize>) {
        // Always initialize thread pool, even if using default thread count,
        // to ensure thread pool's thread count is locked after this point.
        // `rayon::current_num_threads()` will always return the same number after this point.
        //
        // If you don't initialize the global thread pool explicitly, or don't specify `num_threads`,
        // Rayon will initialize the thread pool when it's first used, with a thread count of
        // `std::thread::available_parallelism()`, and that thread count won't change thereafter.
        // So we don't *need* to initialize the thread pool here if we just want the default thread count.
        //
        // However, Rayon's docs state that:
        // > In the future, the default behavior may change to dynamically add or remove threads as needed.
        // https://docs.rs/rayon/1.11.0/rayon/struct.ThreadPoolBuilder.html#method.num_threads
        //
        // To ensure we continue to have a "locked" thread count, even after future Rayon upgrades,
        // we always initialize the thread pool and explicitly specify thread count here.

        // 确定要使用的线程数
        let thread_count = if let Some(thread_count) = threads
            && thread_count > 0
        {
            // 用户明确指定了线程数
            thread_count
        } else if let Ok(thread_count) = std::thread::available_parallelism() {
            // 使用系统 CPU 核心数
            thread_count.get()
        } else {
            // 无法确定 CPU 核心数，使用单线程
            eprintln!(
                "Unable to determine available thread count. Defaulting to 1.\nConsider specifying the number of threads explicitly with `--threads` option."
            );
            1
        };

        // 构建并设置全局线程池
        // 注意：这会 panic 如果全局线程池已被初始化（但在 Oxlint 中不应该发生）
        rayon::ThreadPoolBuilder::new().num_threads(thread_count).build_global().unwrap();
    }
}

/// 基本配置选项
///
/// 包含与配置文件和初始化相关的选项。
#[derive(Debug, Clone, Bpaf)]
pub struct BasicOptions {
    /// Oxlint 配置文件路径 (实验性)
    ///
    /// # 特性
    /// - 只支持 `.json` 扩展名
    /// - 可以在配置文件中使用注释（JSON5 格式）
    /// - 尽量兼容 ESLint v8 的配置格式
    ///
    /// # 查找规则
    /// 如果未提供，Oxlint 会在当前工作目录查找 `.oxlintrc.json`
    ///
    /// # 使用
    /// ```bash
    /// oxlint --config custom.json src/
    /// ```
    #[bpaf(long, short, argument("./.oxlintrc.json"))]
    pub config: Option<PathBuf>,

    /// TypeScript `tsconfig.json` 文件路径
    ///
    /// 用于 import 插件读取路径别名（path alias）和项目引用（project references）。
    ///
    /// # 使用
    /// ```bash
    /// oxlint --tsconfig ./tsconfig.json --import-plugin src/
    /// ```
    #[bpaf(argument("./tsconfig.json"), hide_usage)]
    pub tsconfig: Option<PathBuf>,

    /// 初始化 Oxlint 配置文件
    ///
    /// 使用默认值创建 `.oxlintrc.json` 配置文件。
    ///
    /// # 使用
    /// ```bash
    /// oxlint --init
    /// ```
    #[bpaf(switch, hide_usage)]
    pub init: bool,
}

/// 规则过滤器：允许/警告/拒绝 Lint 规则
///
/// 从命令行左到右累积规则和类别。
///
/// # 规则类别
///
/// - `correctness` - 明显错误或无用的代码（默认启用）
/// - `suspicious`  - 很可能错误或无用的代码
/// - `pedantic`    - 相当严格的规则，偶尔会有误报
/// - `style`       - 应该用更符合习惯的方式编写的代码
/// - `nursery`     - 正在开发中的新规则
/// - `restriction` - 防止使用特定语言和库功能的规则
/// - `all`         - 上述所有类别（除了 nursery）。不会自动启用插件
///
/// # 使用示例
///
/// ```bash
/// # 启用 correctness 类别，但禁用 no-debugger 规则
/// oxlint -D correctness -A no-debugger src/
///
/// # 禁用所有规则，只启用 no-debugger
/// oxlint -A all -D no-debugger src/
///
/// # 启用 suspicious 和 pedantic
/// oxlint -D suspicious -D pedantic src/
/// ```
///
/// # 优先级
///
/// 后面的参数会覆盖前面的。例如：
/// - `-D all -A no-var` - 启用所有规则，但禁用 no-var
/// - `-A all -D no-var` - 禁用所有规则，但启用 no-var
#[derive(Debug, Clone, Bpaf)]
pub enum LintFilter {
    /// 允许规则或类别（抑制 lint）
    ///
    /// 使用 `-A` 或 `--allow` 标志
    Allow(
        #[bpaf(short('A'), long("allow"), argument("NAME"))]
        String,
    ),
    /// 将规则或类别设为警告级别
    ///
    /// 使用 `-W` 或 `--warn` 标志
    Warn(
        #[bpaf(short('W'), long("warn"), argument("NAME"))]
        String,
    ),
    /// 拒绝规则或类别（发出错误）
    ///
    /// 使用 `-D` 或 `--deny` 标志
    Deny(
        #[bpaf(short('D'), long("deny"), argument("NAME"))]
        String,
    ),
}

impl LintFilter {
    /// 将 LintFilter 转换为元组格式
    ///
    /// 内部使用，将命令行参数转换为 Linter 可以理解的格式。
    fn into_tuple(self) -> (AllowWarnDeny, String) {
        match self {
            Self::Allow(s) => (AllowWarnDeny::Allow, s),
            Self::Warn(s) => (AllowWarnDeny::Warn, s),
            Self::Deny(s) => (AllowWarnDeny::Deny, s),
        }
    }
}

/// 自动修复选项
///
/// 控制 Oxlint 如何自动修复发现的问题。
#[derive(Debug, Clone, Bpaf)]
pub struct FixOptions {
    /// 修复尽可能多的问题
    ///
    /// 只有无法修复的问题会在输出中报告。
    /// 只应用"安全"的修复，不会改变程序行为。
    ///
    /// # 使用
    /// ```bash
    /// oxlint --fix src/
    /// ```
    #[bpaf(switch, hide_usage)]
    pub fix: bool,

    /// 应用可自动修复的建议
    ///
    /// ⚠️ 注意：可能会改变程序行为！
    /// 这包括一些不确定安全的修复建议。
    ///
    /// # 使用
    /// ```bash
    /// oxlint --fix-suggestions src/
    /// ```
    #[bpaf(switch, hide_usage)]
    pub fix_suggestions: bool,

    /// 应用危险的修复和建议
    ///
    /// ⚠️ 警告：可能会破坏代码！
    /// 这包括所有类型的修复，即使可能不安全。
    ///
    /// # 使用
    /// ```bash
    /// oxlint --fix-dangerously src/
    /// ```
    #[bpaf(switch, hide_usage)]
    pub fix_dangerously: bool,
}

impl FixOptions {
    /// 将命令行选项转换为 FixKind 标志
    ///
    /// 根据用户指定的选项组合，确定应该应用哪些类型的修复。
    pub fn fix_kind(&self) -> FixKind {
        let mut kind = FixKind::None;

        // --fix: 启用安全修复
        if self.fix {
            kind.set(FixKind::SafeFix, true);
        }

        // --fix-suggestions: 启用建议修复
        if self.fix_suggestions {
            kind.set(FixKind::Suggestion, true);
        }

        // --fix-dangerously: 启用危险修复
        if self.fix_dangerously {
            // 如果没有其他修复选项，启用所有修复
            if kind.is_none() {
                kind.set(FixKind::Fix, true);
            }
            // 标记为危险模式
            kind.set(FixKind::Dangerous, true);
        }

        kind
    }

    /// 检查是否启用了任何修复选项
    pub fn is_enabled(&self) -> bool {
        self.fix || self.fix_suggestions || self.fix_dangerously
    }
}

/// Handle Warnings
#[derive(Debug, Clone, Bpaf)]
pub struct WarningOptions {
    /// Disable reporting on warnings, only errors are reported
    #[bpaf(switch, hide_usage)]
    pub quiet: bool,

    /// Ensure warnings produce a non-zero exit code
    #[bpaf(switch, hide_usage)]
    pub deny_warnings: bool,

    /// Specify a warning threshold,
    /// which can be used to force exit with an error status if there are too many warning-level rule violations in your project
    #[bpaf(argument("INT"), hide_usage)]
    pub max_warnings: Option<usize>,
}

/// Output
#[derive(Debug, Clone, Bpaf)]
pub struct OutputOptions {
    /// Use a specific output format. Possible values:
    /// `checkstyle`, `default`, `github`, `gitlab`, `json`, `junit`, `stylish`, `unix`
    #[bpaf(long, short, fallback(OutputFormat::Default), hide_usage)]
    pub format: OutputFormat,
}

/// Enable Plugins
#[expect(clippy::struct_field_names)]
#[derive(Debug, Default, Clone, Bpaf)]
pub struct EnablePlugins {
    /// Disable unicorn plugin, which is turned on by default
    #[bpaf(
        long("disable-unicorn-plugin"),
        flag(OverrideToggle::Disable, OverrideToggle::NotSet),
        hide_usage
    )]
    pub unicorn_plugin: OverrideToggle,

    /// Disable oxc unique rules, which is turned on by default
    #[bpaf(
        long("disable-oxc-plugin"),
        flag(OverrideToggle::Disable, OverrideToggle::NotSet),
        hide_usage
    )]
    pub oxc_plugin: OverrideToggle,

    /// Disable TypeScript plugin, which is turned on by default
    #[bpaf(
        long("disable-typescript-plugin"),
        flag(OverrideToggle::Disable, OverrideToggle::NotSet),
        hide_usage
    )]
    pub typescript_plugin: OverrideToggle,

    /// Enable the experimental import plugin and detect ESM problems.
    /// It is recommended to use along side with the `--tsconfig` option.
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub import_plugin: OverrideToggle,

    /// Enable react plugin, which is turned off by default
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub react_plugin: OverrideToggle,

    /// Enable the experimental jsdoc plugin and detect JSDoc problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub jsdoc_plugin: OverrideToggle,

    /// Enable the Jest plugin and detect test problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub jest_plugin: OverrideToggle,

    /// Enable the Vitest plugin and detect test problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub vitest_plugin: OverrideToggle,

    /// Enable the JSX-a11y plugin and detect accessibility problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub jsx_a11y_plugin: OverrideToggle,

    /// Enable the Next.js plugin and detect Next.js problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub nextjs_plugin: OverrideToggle,

    /// Enable the React performance plugin and detect rendering performance problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub react_perf_plugin: OverrideToggle,

    /// Enable the promise plugin and detect promise usage problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub promise_plugin: OverrideToggle,

    /// Enable the node plugin and detect node usage problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub node_plugin: OverrideToggle,

    /// Enable the regex plugin and detect regex usage problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub regex_plugin: OverrideToggle,

    /// Enable the vue plugin and detect vue usage problems
    #[bpaf(flag(OverrideToggle::Enable, OverrideToggle::NotSet), hide_usage)]
    pub vue_plugin: OverrideToggle,
}

/// 三态开关：启用/禁用/未设置
///
/// 用于表示命令行标志的三种状态，允许区分：
/// - 用户明确要求启用
/// - 用户明确要求禁用
/// - 用户没有指定（使用配置文件或默认值）
///
/// # 设计原因
///
/// 我们希望 CLI 标志能够覆盖用户配置文件中的设置，但如果用户没有
/// 明确传递标志，则不改变默认行为。这个方案虽然有点复杂，但由于
/// `bpaf` 库的架构限制，这是必要的。
///
/// # 示例
///
/// ```bash
/// # 启用 React 插件（覆盖配置文件）
/// oxlint --react-plugin src/
///
/// # 禁用 Unicorn 插件（覆盖配置文件）
/// oxlint --disable-unicorn-plugin src/
///
/// # 不指定（使用配置文件或默认值）
/// oxlint src/
/// ```
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideToggle {
    /// 覆盖为启用状态
    Enable,
    /// 覆盖为禁用状态
    Disable,
    /// 不覆盖（使用默认值或配置文件值）
    #[default]
    NotSet,
}

impl From<Option<bool>> for OverrideToggle {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::Enable,
            Some(false) => Self::Disable,
            None => Self::NotSet,
        }
    }
}

impl From<OverrideToggle> for Option<bool> {
    fn from(value: OverrideToggle) -> Self {
        match value {
            OverrideToggle::Enable => Some(true),
            OverrideToggle::Disable => Some(false),
            OverrideToggle::NotSet => None,
        }
    }
}

impl OverrideToggle {
    /// 检查是否被明确设置为启用
    #[inline]
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Enable)
    }

    /// 检查是否未被设置
    #[inline]
    pub fn is_not_set(self) -> bool {
        matches!(self, Self::NotSet)
    }

    /// 如果已设置，则执行闭包
    ///
    /// 只有当开关被明确设置（Enable 或 Disable）时，才会调用闭包。
    /// 如果是 NotSet，则不执行任何操作。
    pub fn inspect<F>(self, f: F)
    where
        F: FnOnce(bool),
    {
        if let Some(v) = self.into() {
            f(v);
        }
    }
}

impl EnablePlugins {
    /// 将命令行插件覆盖应用到插件配置
    ///
    /// 遍历所有插件开关，如果用户明确设置了，就覆盖配置文件中的值。
    ///
    /// # 特殊处理
    ///
    /// - Vitest 插件：如果启用 Vitest 且未明确禁用 Jest，会自动启用 Jest
    ///   （因为 Vitest 插件复用了 Jest 规则）
    pub fn apply_overrides(&self, plugins: &mut LintPlugins) {
        // 对每个插件，如果命令行有明确设置，就覆盖
        self.react_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::REACT, yes));
        self.unicorn_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::UNICORN, yes));
        self.oxc_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::OXC, yes));
        self.typescript_plugin
            .inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::TYPESCRIPT, yes));
        self.import_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::IMPORT, yes));
        self.jsdoc_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::JSDOC, yes));
        self.jest_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::JEST, yes));
        self.vitest_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::VITEST, yes));
        self.jsx_a11y_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::JSX_A11Y, yes));
        self.nextjs_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::NEXTJS, yes));
        self.react_perf_plugin
            .inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::REACT_PERF, yes));
        self.promise_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::PROMISE, yes));
        self.node_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::NODE, yes));
        self.regex_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::REGEX, yes));
        self.vue_plugin.inspect(|yes| plugins.builtin.set(BuiltinLintPlugins::VUE, yes));

        // 特殊处理：Vitest 依赖 Jest 规则
        // 如果启用了 Vitest 但没有明确禁用 Jest，自动启用 Jest
        if self.vitest_plugin.is_enabled() && self.jest_plugin.is_not_set() {
            plugins.builtin.set(BuiltinLintPlugins::JEST, true);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Bpaf)]
pub enum ReportUnusedDirectives {
    WithoutSeverity(
        /// Report directive comments like `// eslint-disable-line` when no errors would have been reported on that line anyway.
        // More information at <https://eslint.org/docs/latest/use/command-line-interface#--report-unused-disable-directives>
        #[bpaf(long("report-unused-disable-directives"), switch, hide_usage)]
        bool,
    ),
    WithSeverity(
        /// Same as `--report-unused-disable-directives`, but allows you to specify the severity level of the reported errors.
        /// Only one of these two options can be used at a time.
        #[bpaf(
            long("report-unused-disable-directives-severity"),
            argument::<String>("SEVERITY"),
            guard(|s| AllowWarnDeny::try_from(s.as_str()).is_ok(), "Invalid severity value"),
            map(|s| AllowWarnDeny::try_from(s.as_str()).unwrap()), // guard ensures try_from will be Ok
            optional,
            hide_usage
        )]
        Option<AllowWarnDeny>,
    ),
}

/// Inline Configuration Comments
#[derive(Debug, Clone, Bpaf)]
pub struct InlineConfigOptions {
    #[bpaf(external)]
    pub report_unused_directives: ReportUnusedDirectives,
}

#[cfg(test)]
mod plugins {
    use rustc_hash::FxHashSet;

    use oxc_linter::{BuiltinLintPlugins, LintPlugins};

    use super::{EnablePlugins, OverrideToggle};

    #[test]
    fn test_override_default() {
        let mut plugins = LintPlugins::default();
        let enable = EnablePlugins::default();

        enable.apply_overrides(&mut plugins);
        assert_eq!(plugins, LintPlugins::default());
    }

    #[test]
    fn test_overrides() {
        let mut plugins = LintPlugins::default();
        let enable = EnablePlugins {
            react_plugin: OverrideToggle::Enable,
            unicorn_plugin: OverrideToggle::Disable,
            ..EnablePlugins::default()
        };
        let expected = BuiltinLintPlugins::default()
            .union(BuiltinLintPlugins::REACT)
            .difference(BuiltinLintPlugins::UNICORN);

        enable.apply_overrides(&mut plugins);
        assert_eq!(plugins, LintPlugins::new(expected, FxHashSet::default()));
    }

    #[test]
    fn test_override_vitest() {
        let mut plugins = LintPlugins::default();
        let enable =
            EnablePlugins { vitest_plugin: OverrideToggle::Enable, ..EnablePlugins::default() };
        let expected = LintPlugins::new(
            BuiltinLintPlugins::default() | BuiltinLintPlugins::VITEST | BuiltinLintPlugins::JEST,
            FxHashSet::default(),
        );

        enable.apply_overrides(&mut plugins);
        assert_eq!(plugins, expected);
    }
}

#[cfg(test)]
mod warning_options {
    use super::{WarningOptions, lint_command};

    fn get_warning_options(arg: &str) -> WarningOptions {
        let args = arg.split(' ').map(std::string::ToString::to_string).collect::<Vec<_>>();
        lint_command().run_inner(args.as_slice()).unwrap().warning_options
    }

    #[test]
    fn default() {
        let options = get_warning_options(".");
        assert!(!options.quiet);
        assert_eq!(options.max_warnings, None);
    }

    #[test]
    fn quiet() {
        let options = get_warning_options("--quiet .");
        assert!(options.quiet);
    }

    #[test]
    fn max_warnings() {
        let options = get_warning_options("--max-warnings 10 .");
        assert_eq!(options.max_warnings, Some(10));
    }
}

#[cfg(test)]
mod lint_options {
    use std::{fs::File, path::PathBuf};

    use oxc_linter::AllowWarnDeny;

    use super::{LintCommand, OutputFormat, lint_command};

    fn get_lint_options(arg: &str) -> LintCommand {
        let args = arg.split(' ').map(std::string::ToString::to_string).collect::<Vec<_>>();
        lint_command().run_inner(args.as_slice()).unwrap()
    }

    #[test]
    fn default() {
        let options = get_lint_options(".");
        assert_eq!(options.paths, vec![PathBuf::from(".")]);
        assert!(!options.fix_options.fix);
        assert!(!options.list_rules);
        assert_eq!(options.output_options.format, OutputFormat::Default);
    }

    #[test]
    fn multiple_paths() {
        let temp_dir = tempfile::tempdir().expect("Could not create a temp dir");
        let file_foo = temp_dir.path().join("foo.js");
        File::create(&file_foo).expect("Could not create foo.js temp file");
        let file_name_foo =
            file_foo.to_str().expect("Could not get path string for foo.js temp file");
        let file_bar = temp_dir.path().join("bar.js");
        File::create(&file_bar).expect("Could not create bar.js temp file");
        let file_name_bar =
            file_bar.to_str().expect("Could not get path string for bar.js temp file");
        let file_baz = temp_dir.path().join("baz");
        File::create(&file_baz).expect("Could not create baz temp file");
        let file_name_baz = file_baz.to_str().expect("Could not get path string for baz temp file");

        let options =
            get_lint_options(format!("{file_name_foo} {file_name_bar} {file_name_baz}").as_str());
        assert_eq!(options.paths, [file_foo, file_bar, file_baz]);
    }

    #[test]
    fn no_parent_path() {
        match lint_command().run_inner(&["../parent_dir"]) {
            Ok(_) => panic!("Should not allow parent dir"),
            Err(err) => match err {
                bpaf::ParseFailure::Stderr(doc) => {
                    assert_eq!("`../parent_dir`: PATH must not contain \"..\"", format!("{doc}"));
                }
                _ => unreachable!(),
            },
        }
    }

    #[test]
    fn fix() {
        let options = get_lint_options("--fix test.js");
        assert!(options.fix_options.fix);
    }

    #[test]
    fn filter() {
        let options =
            get_lint_options("-D suspicious --deny pedantic -A no-debugger --allow no-var src");
        assert_eq!(
            options.filter,
            [
                (AllowWarnDeny::Deny, "suspicious".into()),
                (AllowWarnDeny::Deny, "pedantic".into()),
                (AllowWarnDeny::Allow, "no-debugger".into()),
                (AllowWarnDeny::Allow, "no-var".into())
            ]
        );
    }

    #[test]
    fn format() {
        let options = get_lint_options("-f json");
        assert_eq!(options.output_options.format, OutputFormat::Json);
        assert!(options.paths.is_empty());
    }

    #[test]
    fn format_error() {
        let args = "-f asdf".split(' ').map(std::string::ToString::to_string).collect::<Vec<_>>();
        let result = lint_command().run_inner(args.as_slice());
        assert!(result.is_err_and(
            |err| err.unwrap_stderr() == "couldn't parse `asdf`: 'asdf' is not a known format"
        ));
    }

    #[test]
    fn list_rules() {
        let options = get_lint_options("--rules");
        assert!(options.list_rules);
    }

    #[test]
    fn disable_nested_config() {
        let options = get_lint_options("--disable-nested-config");
        assert!(options.disable_nested_config);
        let options = get_lint_options(".");
        assert!(!options.disable_nested_config);
    }

    #[test]
    fn type_aware() {
        let options = get_lint_options("--type-aware");
        assert!(options.type_aware);
        let options = get_lint_options(".");
        assert!(!options.type_aware);
    }
}

#[cfg(test)]
mod inline_config_options {
    use oxc_linter::AllowWarnDeny;

    use super::{LintCommand, ReportUnusedDirectives, lint_command};

    fn get_lint_options(arg: &str) -> LintCommand {
        let args = arg.split(' ').map(std::string::ToString::to_string).collect::<Vec<_>>();
        lint_command().run_inner(args.as_slice()).unwrap()
    }

    #[test]
    fn default() {
        let options = get_lint_options(".");
        assert_eq!(
            options.inline_config_options.report_unused_directives,
            ReportUnusedDirectives::WithoutSeverity(false)
        );
    }

    #[test]
    fn without_severity() {
        let options = get_lint_options("--report-unused-disable-directives");
        assert_eq!(
            options.inline_config_options.report_unused_directives,
            ReportUnusedDirectives::WithoutSeverity(true)
        );
    }

    #[test]
    fn with_severity_warn() {
        let options = get_lint_options("--report-unused-disable-directives-severity=warn");
        assert_eq!(
            options.inline_config_options.report_unused_directives,
            ReportUnusedDirectives::WithSeverity(Some(AllowWarnDeny::Warn))
        );
    }

    #[test]
    fn with_severity_error() {
        let options = get_lint_options("--report-unused-disable-directives-severity error");
        assert_eq!(
            options.inline_config_options.report_unused_directives,
            ReportUnusedDirectives::WithSeverity(Some(AllowWarnDeny::Deny))
        );
    }
}
