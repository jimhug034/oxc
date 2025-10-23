use std::{
    env,
    ffi::OsStr,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf, absolute},
    sync::Arc,
    time::Instant,
};

use cow_utils::CowUtils;
use ignore::{gitignore::Gitignore, overrides::OverrideBuilder};
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

use oxc_diagnostics::{DiagnosticSender, DiagnosticService, GraphicalReportHandler, OxcDiagnostic};
use oxc_linter::{
    AllowWarnDeny, Config, ConfigStore, ConfigStoreBuilder, ExternalLinter, ExternalPluginStore,
    InvalidFilterKind, LintFilter, LintOptions, LintService, LintServiceOptions, Linter, Oxlintrc,
    TsGoLintState,
};

use crate::{
    cli::{CliRunResult, LintCommand, MiscOptions, ReportUnusedDirectives, WarningOptions},
    output_formatter::{LintCommandInfo, OutputFormatter},
    walk::Walk,
};
use oxc_linter::LintIgnoreMatcher;

#[derive(Debug)]
pub struct LintRunner {
    options: LintCommand,
    cwd: PathBuf,
    external_linter: Option<ExternalLinter>,
}

impl LintRunner {
    pub(crate) fn new(options: LintCommand, external_linter: Option<ExternalLinter>) -> Self {
        Self {
            options,
            cwd: env::current_dir().expect("Failed to get current working directory"),
            external_linter,
        }
    }

    /// 执行 lint 检查的主方法
    ///
    /// 这是 oxlint 的核心执行流程，负责：
    /// 1. 初始化输出格式化器
    /// 2. 解析和验证配置
    /// 3. 扫描需要检查的文件
    /// 4. 执行 lint 规则
    /// 5. 收集并输出诊断结果
    ///
    /// # 参数
    /// - `self`: 消费 LintRunner，执行 linting
    /// - `stdout`: 可变的 Write trait 对象，用于输出结果
    ///
    /// # 返回
    /// `CliRunResult`: 表示 lint 检查的执行结果和退出状态
    pub(crate) fn run(self, stdout: &mut dyn Write) -> CliRunResult {
        // ====== 步骤 1: 初始化输出格式化器 ======
        // 根据用户指定的格式（如 "stylish", "json" 等）创建格式化器
        // 用于后续输出诊断信息
        let format_str = self.options.output_options.format;
        let output_formatter = OutputFormatter::new(format_str);

        // ====== 步骤 2: 处理列出规则的请求 ======
        // 如果用户使用了 --list-rules 选项，直接列出所有可用规则并返回
        // 这是一个快速退出路径，不需要进行实际的 lint 检查
        if self.options.list_rules {
            if let Some(output) = output_formatter.all_rules() {
                print_and_flush_stdout(stdout, &output);
            }
            return CliRunResult::None;
        }

        // ====== 步骤 3: 解构 LintCommand 选项 ======
        // 从 self.options 中提取所有需要的配置选项
        // 这些选项包括文件路径、过滤器、警告级别、忽略规则等
        //
        // 🔍 paths 的来源追踪：
        // 1. 用户在命令行输入: oxlint src/ test.js
        // 2. bpaf 在 lib.rs:76 解析命令行参数，创建 LintCommand
        // 3. LintCommand 通过 lib.rs:103 传递给 LintRunner::new()
        // 4. LintRunner 将 LintCommand 存储在 self.options 中
        // 5. 这里通过结构体解构将 paths 提取出来
        //
        // 解构前: self.options.paths (类型: Vec<PathBuf>)
        // 解构后: paths (类型: Vec<PathBuf>)
        let LintCommand {
            paths,                 // 要检查的文件或目录路径（从命令行解析）
            filter,                // 规则过滤器（如 -A all, -D no-debugger）
            basic_options,         // 基础选项（如配置文件路径、tsconfig 路径）
            warning_options,       // 警告相关选项（quiet, max-warnings 等）
            ignore_options,        // 忽略相关选项（ignore-pattern, no-ignore 等）
            fix_options,           // 自动修复选项
            enable_plugins,        // 启用的插件列表
            misc_options,          // 其他杂项选项（silent, print-config 等）
            disable_nested_config, // 是否禁用嵌套配置
            inline_config_options, // 内联配置选项（如注释中的 eslint-disable）
            ..
        } = self.options;

        // 获取外部 linter 的引用（可能为 None）
        // 外部 linter 主要用于处理一些需要额外上下文的情况
        let external_linter = self.external_linter.as_ref();

        // ====== 步骤 4: 准备路径和计时 ======
        // 保存路径列表（后续可能被修改）
        let mut paths = paths;
        // 记录用户提供的原始路径数量，用于后续判断是否有文件被过滤掉
        let provided_path_count = paths.len();
        // 记录开始时间，用于计算整个 lint 过程的耗时
        let now = Instant::now();

        // ====== 步骤 5: 解析和验证过滤器 ======
        // 将 CLI 传入的过滤器字符串（如 "all", "no-debugger"）解析为 LintFilter 对象
        // 过滤器用于启用/禁用特定的 lint 规则
        // 如果解析失败，打印错误信息并返回相应的错误状态
        //
        // 注意：这里使用 Self::get_filters 而不是 self.get_filters
        // - Self (大写) 表示类型别名，指向 LintRunner
        // - Self::method() 表示调用静态方法（associated function），不需要实例
        // - self.method() 表示调用实例方法，需要实例
        // get_filters 的第一个参数不是 self，所以是静态方法
        let filters = match Self::get_filters(filter) {
            Ok(filters) => filters,
            Err((result, message)) => {
                print_and_flush_stdout(stdout, &message);
                return result;
            }
        };

        // ====== 步骤 6: 创建诊断报告处理器 ======
        // 用于格式化错误消息，使其更易读
        // 在测试模式下使用无主题版本以保持输出稳定
        let handler = if cfg!(any(test, feature = "force_test_reporter")) {
            GraphicalReportHandler::new_themed(miette::GraphicalTheme::none())
        } else {
            GraphicalReportHandler::new()
        };

        // ====== 步骤 7: 查找和加载配置文件 ======
        // 从当前工作目录查找 oxlint 配置文件（如 .oxlintrc.json）
        // 如果用户通过 --config 指定了配置文件，则使用指定的文件
        let config_search_result =
            Self::find_oxlint_config(&self.cwd, basic_options.config.as_ref());

        // 解析配置文件，如果失败则输出错误并返回
        let mut oxlintrc = match config_search_result {
            Ok(config) => config,
            Err(err) => {
                print_and_flush_stdout(
                    stdout,
                    &format!(
                        "Failed to parse configuration file.\n{}\n",
                        render_report(&handler, &err)
                    ),
                );

                return CliRunResult::InvalidOptionConfig;
            }
        };

        // ====== 步骤 8: 处理 ignore 选项和路径过滤 ======
        // 根据 --ignore-pattern 和 .gitignore 文件过滤不需要检查的文件
        let mut override_builder = None;

        // 如果用户没有使用 --no-ignore 选项，则需要应用 ignore 规则
        if !ignore_options.no_ignore {
            // 创建 override builder，用于处理通过 CLI 传入的 ignore-pattern
            let mut builder = OverrideBuilder::new(&self.cwd);

            // 添加用户指定的 ignore-pattern
            // 注意：ignore crate 的逻辑是反向的，所以需要在模式前加上 "!"
            if !ignore_options.ignore_pattern.is_empty() {
                for pattern in &ignore_options.ignore_pattern {
                    // ignore crate 的模式含义是反向的，需要加 "!" 前缀
                    // 参考：https://docs.rs/ignore/latest/ignore/overrides/struct.OverrideBuilder.html#method.add
                    let pattern = format!("!{pattern}");
                    builder.add(&pattern).unwrap();
                }
            }

            let builder = builder.build().unwrap();

            // ignore crate 允许通过显式路径，但应该优先考虑 ignore 文件
            // 许多用户使用工具自动传递已更改的文件列表
            // 除非传递了 --no-ignore，否则预先过滤路径
            if !paths.is_empty() {
                // 创建 Gitignore 对象，读取 .gitignore 或自定义的 ignore 文件
                let (ignore, _err) = Gitignore::new(&ignore_options.ignore_path);

                // 过滤路径：移除所有被 ignore 文件匹配的文件
                paths.retain_mut(|p| {
                    // 尝试将 cwd 附加到所有路径前，获取绝对路径
                    let Ok(mut path) = absolute(self.cwd.join(&p)) else {
                        return false;
                    };

                    // 交换 path 和 p，使用绝对路径替换相对路径
                    std::mem::swap(p, &mut path);

                    // 如果是目录，总是保留
                    if path.is_dir() {
                        true
                    } else {
                        // 文件需要检查是否被 ignore
                        // 如果被 CLI pattern 或 ignore 文件匹配，则过滤掉
                        !(builder.matched(p, false).is_ignore()
                            || ignore.matched(path, false).is_ignore())
                    }
                });
            }

            override_builder = Some(builder);
        }

        // ====== 步骤 9: 处理空路径情况 ======
        // 如果在过滤后没有路径了，需要特殊处理
        if paths.is_empty() {
            // 如果用户提供了显式路径，但所有路径都被过滤掉了，则提前返回
            if provided_path_count > 0 {
                // 输出统计信息（0 个文件）
                if let Some(end) = output_formatter.lint_command_info(&LintCommandInfo {
                    number_of_files: 0,
                    number_of_rules: None,
                    threads_count: rayon::current_num_threads(),
                    start_time: now.elapsed(),
                }) {
                    print_and_flush_stdout(stdout, &end);
                }

                return CliRunResult::LintNoFilesFound;
            }

            // 如果没有提供任何路径，默认检查当前工作目录
            paths.push(self.cwd.clone());
        }

        // ====== 步骤 10: 创建文件遍历器 ======
        // Walk 类递归遍历目录，找到所有需要检查的文件
        let walker = Walk::new(&paths, &ignore_options, override_builder);
        let paths = walker.paths();

        // ====== 步骤 11: 处理嵌套配置 ======
        // 创建一个外部插件存储，用于管理从嵌套配置中加载的插件
        let mut external_plugin_store = ExternalPluginStore::default();

        // 决定是否搜索嵌套配置文件
        // 只有在以下条件都满足时才搜索：
        // 1. 用户没有禁用嵌套配置
        // 2. 用户没有显式指定 --config 选项（显式指定的配置具有绝对优先级）
        let search_for_nested_configs = !disable_nested_config &&
            // 如果显式传递了 `--config` 选项，不应该搜索嵌套配置文件
            // 因为传递的配置文件具有绝对优先级
            basic_options.config.is_none();

        // 收集嵌套配置文件中的 ignore patterns
        let mut nested_ignore_patterns = Vec::new();

        // 查找并解析所有嵌套配置文件
        // 嵌套配置允许不同目录有不同的 lint 规则
        let nested_configs = if search_for_nested_configs {
            match Self::get_nested_configs(
                stdout,
                &handler,
                &filters,
                &paths,
                external_linter,
                &mut external_plugin_store,
                &mut nested_ignore_patterns,
            ) {
                Ok(v) => v,
                Err(v) => return v,
            }
        } else {
            FxHashMap::default()
        };

        // ====== 步骤 12: 创建 ignore 匹配器 ======
        // 用于判断文件是否应该被忽略
        // 结合主配置和嵌套配置中的 ignore patterns
        let ignore_matcher = {
            LintIgnoreMatcher::new(&oxlintrc.ignore_patterns, &self.cwd, nested_ignore_patterns)
        };

        // ====== 步骤 13: 应用插件启用覆盖 ======
        // 根据 CLI 选项（如 --jest-plugin, --vitest-plugin）启用或禁用插件
        {
            let mut plugins = oxlintrc.plugins.unwrap_or_default();
            enable_plugins.apply_overrides(&mut plugins);
            oxlintrc.plugins = Some(plugins);
        }

        // ====== 步骤 14: 准备配置用于打印或初始化 ======
        // 如果用户使用了 --print-config 或 --init 选项，保存一份配置副本
        let oxlintrc_for_print = if misc_options.print_config || basic_options.init {
            Some(oxlintrc.clone())
        } else {
            None
        };

        // ====== 步骤 15: 构建配置存储 ======
        // 从 oxlintrc 配置创建 ConfigStoreBuilder
        // ConfigStoreBuilder 会将配置文件转换为内部规则配置
        let config_builder = match ConfigStoreBuilder::from_oxlintrc(
            false,
            oxlintrc,
            external_linter,
            &mut external_plugin_store,
        ) {
            Ok(builder) => builder,
            Err(e) => {
                print_and_flush_stdout(
                    stdout,
                    &format!(
                        "Failed to parse configuration file.\n{}\n",
                        render_report(&handler, &OxcDiagnostic::error(e.to_string()))
                    ),
                );
                return CliRunResult::InvalidOptionConfig;
            }
        }
        .with_filters(&filters); // 应用过滤器（-A, -D, -W 等选项）

        // ====== 步骤 16: 处理打印配置或初始化配置 ======
        // 如果用户使用了 --print-config 或 --init 选项，在这里处理
        if let Some(basic_config_file) = oxlintrc_for_print {
            // 解析最终的配置文件内容
            let config_file = config_builder.resolve_final_config_file(basic_config_file);

            // 如果使用 --print-config，直接打印配置并返回
            if misc_options.print_config {
                print_and_flush_stdout(stdout, &config_file);
                print_and_flush_stdout(stdout, "\n");

                return CliRunResult::PrintConfigResult;
            }
            // 如果使用 --init，创建默认配置文件
            else if basic_options.init {
                let schema_relative_path = "node_modules/oxlint/configuration_schema.json";

                // 如果有 schema 文件，添加 $schema 引用以便 IDE 提供智能提示
                let configuration = if self.cwd.join(schema_relative_path).is_file() {
                    let mut config_json: Value = serde_json::from_str(&config_file).unwrap();
                    if let Value::Object(ref mut obj) = config_json {
                        let mut json_object = serde_json::Map::new();
                        // 添加 $schema 字段
                        json_object.insert(
                            "$schema".to_string(),
                            format!("./{schema_relative_path}").into(),
                        );
                        json_object.extend(obj.clone());
                        *obj = json_object;
                    }
                    serde_json::to_string_pretty(&config_json).unwrap()
                } else {
                    config_file
                };

                // 写入配置文件到 .oxlintrc.json
                if fs::write(Self::DEFAULT_OXLINTRC, configuration).is_ok() {
                    print_and_flush_stdout(stdout, "Configuration file created\n");
                    return CliRunResult::ConfigFileInitSucceeded;
                }

                // 写入失败的情况
                print_and_flush_stdout(stdout, "Failed to create configuration file\n");
                return CliRunResult::ConfigFileInitFailed;
            }
        }

        // ====== 步骤 17: 配置跨模块分析 ======
        // TODO(refactor): 提取到共享函数，以便语言服务器可以复用相同的功能
        // 检查是否启用了 import 插件，启用时需要跨模块分析来追踪导入依赖
        let use_cross_module = config_builder.plugins().has_import()
            || nested_configs.values().any(|config| config.plugins().has_import());
        // 创建 LintServiceOptions，配置是否启用跨模块分析
        let mut options = LintServiceOptions::new(self.cwd).with_cross_module(use_cross_module);

        // ====== 步骤 18: 构建最终的 lint 配置 ======
        // 从 ConfigStoreBuilder 构建最终的 Config 对象
        // Config 包含了所有规则的状态（开启/关闭/警告）
        let lint_config = match config_builder.build(&external_plugin_store) {
            Ok(config) => config,
            Err(e) => {
                print_and_flush_stdout(
                    stdout,
                    &format!(
                        "Failed to build configuration.\n{}\n",
                        render_report(&handler, &OxcDiagnostic::error(e.to_string()))
                    ),
                );
                return CliRunResult::InvalidOptionConfig;
            }
        };

        // ====== 步骤 19: 配置未使用指令报告 ======
        // 处理 --report-unused-disable-directives 选项
        // 这个选项会报告哪些 eslint-disable 注释没有被使用（即规则实际没有被禁用）
        let report_unused_directives = match inline_config_options.report_unused_directives {
            ReportUnusedDirectives::WithoutSeverity(true) => Some(AllowWarnDeny::Warn),
            ReportUnusedDirectives::WithSeverity(Some(severity)) => Some(severity),
            _ => None,
        };

        // ====== 步骤 20: 创建诊断服务 ======
        // 诊断服务负责收集和格式化 lint 错误/警告
        // tx_error 是发送诊断消息的通道
        let (mut diagnostic_service, tx_error) =
            Self::get_diagnostic_service(&output_formatter, &warning_options, &misc_options);

        // ====== 步骤 21: 创建配置存储 ======
        // ConfigStore 包含所有 lint 规则的配置，支持嵌套配置文件
        let config_store = ConfigStore::new(lint_config, nested_configs, external_plugin_store);

        // ====== 步骤 22: 过滤要检查的文件 ======
        // 应用 ignore 模式，过滤掉不需要检查的文件
        let files_to_lint = paths
            .into_iter()
            .filter(|path| !ignore_matcher.should_ignore(Path::new(path)))
            .collect::<Vec<Arc<OsStr>>>();

        // ====== 步骤 23: 类型感知 linting（通过 tsgolint）======
        // tsgolint 是用 Go 编写的外部工具，用于需要类型信息的规则
        // TODO: 如果启用了类型感知规则但找不到 `tsgolint`，应添加警告消息
        if self.options.type_aware {
            if let Err(err) = TsGoLintState::new(options.cwd(), config_store.clone())
                .with_silent(misc_options.silent)
                .lint(&files_to_lint, tx_error.clone())
            {
                print_and_flush_stdout(stdout, &err);
                return CliRunResult::TsGoLintError;
            }
        }

        // ====== 步骤 24: 🔥 关键：创建 oxc_linter::Linter 实例 ======
        // 这是真正的 linter 对象，来自 oxc_linter crate
        // 配置了：
        // 1. 默认 lint 选项
        // 2. 配置存储（包含所有规则）
        // 3. 外部 linter（可选）
        // 4. 是否自动修复
        // 5. 是否报告未使用的指令
        let linter = Linter::new(LintOptions::default(), config_store, self.external_linter)
            .with_fix(fix_options.fix_kind())
            .with_report_unused_directives(report_unused_directives);

        let number_of_files = files_to_lint.len();

        // ====== 步骤 25: 配置 tsconfig 路径 ======
        // 用于 import 插件解析路径别名和项目引用
        let tsconfig = basic_options.tsconfig;
        if let Some(path) = tsconfig.as_ref() {
            if path.is_file() {
                options = options.with_tsconfig(path);
            } else {
                let path = if path.is_relative() { options.cwd().join(path) } else { path.clone() };

                print_and_flush_stdout(
                    stdout,
                    &format!(
                        "The tsconfig file {:?} does not exist, Please provide a valid tsconfig file.\n",
                        path.to_string_lossy().cow_replace('\\', "/")
                    ),
                );

                return CliRunResult::InvalidOptionTsConfig;
            }
        }

        let number_of_rules = linter.number_of_rules(self.options.type_aware);

        // ====== 步骤 26: 🔥 关键：在独立线程中执行 linting ======
        // 在另一个线程中生成 linting 任务，这样诊断信息可以立即从 diagnostic_service.run 打印出来
        // 这实现了边检查边输出的效果，提升用户体验
        rayon::spawn(move || {
            // 创建 LintService（来自 oxc_linter crate）
            // LintService 负责：
            // 1. 遍历所有文件
            // 2. 解析每个文件（调用 oxc_parser）
            // 3. 进行语义分析（调用 oxc_semantic）
            // 4. 对每个文件运行所有 lint 规则
            // 5. 收集诊断信息并发送到 tx_error 通道
            let mut lint_service = LintService::new(linter, options);
            lint_service.with_paths(files_to_lint);

            // 如果启用了 `oxlint2` 特性，使用 RawTransferFileSystem
            // 这会将源文本读取到分配器的开始位置，而不是结束位置（性能优化）
            #[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
            {
                use crate::raw_fs::RawTransferFileSystem;
                lint_service.with_file_system(Box::new(RawTransferFileSystem));
            }

            // 🔥🔥🔥 这里是真正执行 linting 的地方！🔥🔥🔥
            // lint_service.run() 会：
            // 1. 并行处理所有文件（使用 Rayon）
            // 2. 每个文件调用 oxc_parser 解析
            // 3. 调用 oxc_semantic 进行语义分析
            // 4. 调用 Linter.run() 执行所有规则
            // 5. 将诊断结果发送到 tx_error 通道
            lint_service.run(&tx_error);
        });

        // ====== 步骤 27: 收集并输出诊断结果 ======
        // diagnostic_service 在主线程中运行，接收来自 lint_service 的诊断消息
        // 这允许实时打印 lint 错误，而不是等待所有文件都检查完毕
        let diagnostic_result = diagnostic_service.run(stdout);

        // ====== 步骤 28: 输出统计信息 ======
        // 打印检查的文件数、规则数、线程数和耗时
        if let Some(end) = output_formatter.lint_command_info(&LintCommandInfo {
            number_of_files,
            number_of_rules,
            threads_count: rayon::current_num_threads(),
            start_time: now.elapsed(),
        }) {
            print_and_flush_stdout(stdout, &end);
        }

        // ====== 步骤 29: 确定退出状态 ======
        // 根据诊断结果返回适当的退出码
        // 退出码决定了程序的成功或失败状态
        if diagnostic_result.errors_count() > 0 {
            CliRunResult::LintFoundErrors
        } else if warning_options.deny_warnings && diagnostic_result.warnings_count() > 0 {
            CliRunResult::LintNoWarningsAllowed
        } else if diagnostic_result.max_warnings_exceeded() {
            CliRunResult::LintMaxWarningsExceeded
        } else {
            CliRunResult::LintSucceeded
        }
    }
}

impl LintRunner {
    const DEFAULT_OXLINTRC: &'static str = ".oxlintrc.json";

    #[must_use]
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = cwd;
        self
    }

    fn get_diagnostic_service(
        reporter: &OutputFormatter,
        warning_options: &WarningOptions,
        misc_options: &MiscOptions,
    ) -> (DiagnosticService, DiagnosticSender) {
        let (service, sender) = DiagnosticService::new(reporter.get_diagnostic_reporter());
        (
            service
                .with_quiet(warning_options.quiet)
                .with_silent(misc_options.silent)
                .with_max_warnings(warning_options.max_warnings),
            sender,
        )
    }

    /// 解析和验证规则过滤器
    ///
    /// 这个方法将命令行传入的过滤器字符串（如 "all", "no-debugger", "eslint/no-unused-vars"）
    /// 解析为 `LintFilter` 对象，并在解析失败时返回详细的错误信息。
    ///
    /// # 什么是过滤器？
    ///
    /// 过滤器用于启用或禁用特定的 lint 规则，通过 `-A`、`-W`、`-D` 标志指定：
    /// - `-A` (Allow): 允许规则（通常是关闭规则）
    /// - `-W` (Warn): 将规则设为警告级别
    /// - `-D` (Deny): 将规则设为错误级别（通常是开启规则）
    ///
    /// # 用法示例
    ///
    /// ```bash
    /// # 允许所有规则，但拒绝 no-debugger
    /// oxlint -A all -D no-debugger src/
    ///
    /// # 警告级别启用 no-console
    /// oxlint -W no-console src/
    ///
    /// # 使用插件规则
    /// oxlint -D eslint/no-unused-vars src/
    /// ```
    ///
    /// # 参数
    ///
    /// - `filters_arg`: 从命令行解析的过滤器列表
    ///   - 每个元素是一个元组 `(AllowWarnDeny, String)`
    ///   - `AllowWarnDeny` 是严重性级别（Allow/Warn/Deny）
    ///   - `String` 是规则名称（如 "all", "no-debugger", "eslint/no-unused-vars"）
    ///
    /// # 返回值
    ///
    /// - `Ok(Vec<LintFilter>)`: 成功解析的所有过滤器
    /// - `Err((CliRunResult, String))`: 解析失败，返回错误码和用户友好的错误消息
    ///
    /// # 错误情况
    ///
    /// 1. **空过滤器**: 用户提供了严重性级别但没有指定规则名称
    ///    - 例如：`oxlint -D`（没有规则名）
    ///    - 错误码：`InvalidOptionSeverityWithoutFilter`
    ///
    /// 2. **缺少插件名**: 规则名格式不完整，缺少 `plugin/rule` 的前半部分
    ///    - 例如：`oxlint -D /rule-name`
    ///    - 错误码：`InvalidOptionSeverityWithoutPluginName`
    ///
    /// 3. **缺少规则名**: 规则名格式不完整，缺少 `plugin/rule` 的后半部分
    ///    - 例如：`oxlint -D plugin/`
    ///    - 错误码：`InvalidOptionSeverityWithoutRuleName`
    ///
    /// # 设计说明
    ///
    /// 这个方法被单独提取出来是为了提高代码可读性。
    /// 虽然目前只在一个地方使用，但将复杂的验证逻辑与主流程分离
    /// 使得代码更容易理解和测试。
    fn get_filters(
        filters_arg: Vec<(AllowWarnDeny, String)>,
    ) -> Result<Vec<LintFilter>, (CliRunResult, String)> {
        // 预分配容量，避免后续 push 时多次重新分配内存
        let mut filters = Vec::with_capacity(filters_arg.len());

        // 遍历每个过滤器参数，逐个解析
        for (severity, filter_arg) in filters_arg {
            match LintFilter::new(severity, filter_arg) {
                // ✅ 解析成功：将过滤器添加到列表中
                Ok(filter) => {
                    filters.push(filter);
                }
                // ❌ 错误 1: 空过滤器（用户没有提供规则名称）
                Err(InvalidFilterKind::Empty) => {
                    return Err((
                        CliRunResult::InvalidOptionSeverityWithoutFilter,
                        format!("Cannot {severity} an empty filter.\n"),
                    ));
                }
                // ❌ 错误 2: 缺少插件名（规则名格式应为 plugin/rule）
                Err(InvalidFilterKind::PluginMissing(filter)) => {
                    return Err((
                        CliRunResult::InvalidOptionSeverityWithoutPluginName,
                        format!(
                            "Failed to {severity} filter {filter}: Plugin name is missing. Expected <plugin>/<rule>\n"
                        ),
                    ));
                }
                // ❌ 错误 3: 缺少规则名（规则名格式应为 plugin/rule）
                Err(InvalidFilterKind::RuleMissing(filter)) => {
                    return Err((
                        CliRunResult::InvalidOptionSeverityWithoutRuleName,
                        format!(
                            "Failed to {severity} filter {filter}: Rule name is missing. Expected <plugin>/<rule>\n"
                        ),
                    ));
                }
            }
        }

        // 返回成功解析的所有过滤器
        Ok(filters)
    }

    fn get_nested_configs(
        stdout: &mut dyn Write,
        handler: &GraphicalReportHandler,
        filters: &Vec<LintFilter>,
        paths: &Vec<Arc<OsStr>>,
        external_linter: Option<&ExternalLinter>,
        external_plugin_store: &mut ExternalPluginStore,
        nested_ignore_patterns: &mut Vec<(Vec<String>, PathBuf)>,
    ) -> Result<FxHashMap<PathBuf, Config>, CliRunResult> {
        // TODO(perf): benchmark whether or not it is worth it to store the configurations on a
        // per-file or per-directory basis, to avoid calling `.parent()` on every path.
        let mut nested_oxlintrc = FxHashMap::<&Path, Oxlintrc>::default();
        let mut nested_configs = FxHashMap::<PathBuf, Config>::default();
        // get all of the unique directories among the paths to use for search for
        // oxlint config files in those directories and their ancestors
        // e.g. `/some/file.js` will check `/some` and `/`
        //      `/some/other/file.js` will check `/some/other`, `/some`, and `/`
        let mut directories = FxHashSet::default();
        for path in paths {
            let path = Path::new(path);
            // Start from the file's parent directory and walk up the tree
            let mut current = path.parent();
            while let Some(dir) = current {
                // NOTE: Initial benchmarking showed that it was faster to iterate over the directories twice
                // rather than constructing the configs in one iteration. It's worth re-benchmarking that though.
                let inserted = directories.insert(dir);
                if !inserted {
                    break;
                }
                current = dir.parent();
            }
        }
        for directory in directories {
            #[expect(clippy::match_same_arms)]
            match Self::find_oxlint_config_in_directory(directory) {
                Ok(Some(v)) => {
                    nested_oxlintrc.insert(directory, v);
                }
                Ok(None) => {}
                Err(_) => {
                    // TODO(camc314): report this error
                }
            }
        }

        // iterate over each config and build the ConfigStore
        for (dir, oxlintrc) in nested_oxlintrc {
            // Collect ignore patterns and their root
            nested_ignore_patterns.push((
                oxlintrc.ignore_patterns.clone(),
                oxlintrc.path.parent().unwrap().to_path_buf(),
            ));
            // TODO(refactor): clean up all of the error handling in this function
            let builder = match ConfigStoreBuilder::from_oxlintrc(
                false,
                oxlintrc,
                external_linter,
                external_plugin_store,
            ) {
                Ok(builder) => builder,
                Err(e) => {
                    print_and_flush_stdout(
                        stdout,
                        &format!(
                            "Failed to parse configuration file.\n{}\n",
                            render_report(handler, &OxcDiagnostic::error(e.to_string()))
                        ),
                    );
                    return Err(CliRunResult::InvalidOptionConfig);
                }
            }
            .with_filters(filters);

            let config = match builder.build(external_plugin_store) {
                Ok(config) => config,
                Err(e) => {
                    print_and_flush_stdout(
                        stdout,
                        &format!(
                            "Failed to build configuration.\n{}\n",
                            render_report(handler, &OxcDiagnostic::error(e.to_string()))
                        ),
                    );
                    return Err(CliRunResult::InvalidOptionConfig);
                }
            };
            nested_configs.insert(dir.to_path_buf(), config);
        }

        Ok(nested_configs)
    }

    // finds the oxlint config
    // when config is provided, but not found, an String with the formatted error is returned, else the oxlintrc config file is returned
    // when no config is provided, it will search for the default file names in the current working directory
    // when no file is found, the default configuration is returned
    fn find_oxlint_config(cwd: &Path, config: Option<&PathBuf>) -> Result<Oxlintrc, OxcDiagnostic> {
        let path: &Path = config.map_or(Self::DEFAULT_OXLINTRC.as_ref(), PathBuf::as_ref);
        let full_path = cwd.join(path);

        if config.is_some() || full_path.exists() {
            return Oxlintrc::from_file(&full_path);
        }
        Ok(Oxlintrc::default())
    }

    /// Looks in a directory for an oxlint config file, returns the oxlint config if it exists
    /// and returns `Err` if none exists or the file is invalid. Does not apply the default
    /// config file.
    fn find_oxlint_config_in_directory(dir: &Path) -> Result<Option<Oxlintrc>, OxcDiagnostic> {
        let possible_config_path = dir.join(Self::DEFAULT_OXLINTRC);
        if possible_config_path.is_file() {
            Oxlintrc::from_file(&possible_config_path).map(Some)
        } else {
            Ok(None)
        }
    }
}

pub fn print_and_flush_stdout(stdout: &mut dyn Write, message: &str) {
    stdout.write_all(message.as_bytes()).or_else(check_for_writer_error).unwrap();
    stdout.flush().unwrap();
}

fn check_for_writer_error(error: std::io::Error) -> Result<(), std::io::Error> {
    // Do not panic when the process is killed (e.g. piping into `less`).
    if matches!(error.kind(), ErrorKind::Interrupted | ErrorKind::BrokenPipe) {
        Ok(())
    } else {
        Err(error)
    }
}

fn render_report(handler: &GraphicalReportHandler, diagnostic: &OxcDiagnostic) -> String {
    let mut err = String::new();
    handler.render_report(&mut err, diagnostic).unwrap();
    err
}

#[cfg(test)]
mod test {
    use std::{fs, path::PathBuf};

    use super::LintRunner;
    use crate::tester::Tester;

    // lints the full directory of fixtures,
    // so do not snapshot it, test only
    #[test]
    fn no_arg() {
        let args = &[];
        Tester::new().test(args);
    }

    #[test]
    fn dir() {
        let args = &["fixtures/linter"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn cwd() {
        let args = &["debugger.js"];
        Tester::new().with_cwd("fixtures/linter".into()).test_and_snapshot(args);
    }

    #[test]
    fn file() {
        let args = &["fixtures/linter/debugger.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn multi_files() {
        let args = &["fixtures/linter/debugger.js", "fixtures/linter/nan.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn wrong_extension() {
        let args = &["foo.asdf"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn ignore_pattern() {
        let args =
            &["--ignore-pattern", "**/*.js", "--ignore-pattern", "**/*.vue", "fixtures/linter"];
        Tester::new().test_and_snapshot(args);
    }

    /// When a file is explicitly passed as a path and `--no-ignore`
    /// is not present, the ignore file should take precedence.
    /// See https://github.com/oxc-project/oxc/issues/1124
    #[test]
    fn ignore_file_overrides_explicit_args() {
        let args = &["--ignore-path", "fixtures/linter/.customignore", "fixtures/linter/nan.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn ignore_file_no_ignore() {
        let args = &[
            "--ignore-path",
            "fixtures/linter/.customignore",
            "--no-ignore",
            "fixtures/linter/nan.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn ignore_flow() {
        let args = &["--import-plugin", "fixtures/flow/index.mjs"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    // https://github.com/oxc-project/oxc/issues/7406
    fn ignore_flow_import_plugin_directory() {
        let args = &["--import-plugin", "-A all", "-D no-cycle", "fixtures/flow/"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    // https://github.com/oxc-project/oxc/issues/9023
    fn ignore_file_current_dir() {
        let args1 = &[];
        let args2 = &["."];
        Tester::new()
            .with_cwd("fixtures/ignore_file_current_dir".into())
            .test_and_snapshot_multiple(&[args1, args2]);
    }

    #[test]
    // https://github.com/oxc-project/oxc/issues/13204
    fn ignore_pattern_non_glob_syntax() {
        let args1 = &[];
        let args2 = &["."];
        Tester::new()
            .with_cwd("fixtures/ignore_pattern_non_glob_syntax".into())
            .test_and_snapshot_multiple(&[args1, args2]);
    }

    #[test]
    fn ignore_patterns_empty_nested() {
        let args1 = &[];
        let args2 = &["."];
        Tester::new()
            .with_cwd("fixtures/ignore_patterns_empty_nested".into())
            .test_and_snapshot_multiple(&[args1, args2]);
    }

    #[test]
    fn ignore_patterns_relative() {
        let args1 = &[];
        let args2 = &["."];
        Tester::new()
            .with_cwd("fixtures/ignore_patterns_relative".into())
            .test_and_snapshot_multiple(&[args1, args2]);
    }

    #[test]
    fn ignore_patterns_with_symlink() {
        let args1 = &[];
        let args2 = &["."];
        Tester::new()
            .with_cwd("fixtures/ignore_patterns_symlink".into())
            .test_and_snapshot_multiple(&[args1, args2]);
    }

    #[test]
    fn filter_allow_all() {
        let args = &["-A", "all", "fixtures/linter"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn filter_allow_one() {
        let args = &["-W", "correctness", "-A", "no-debugger", "fixtures/linter/debugger.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn filter_error() {
        let args = &["-D", "correctness", "fixtures/linter/debugger.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn eslintrc_error() {
        let args = &["-c", "fixtures/linter/eslintrc.json", "fixtures/linter/debugger.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn eslintrc_off() {
        let args = &["-c", "fixtures/eslintrc_off/eslintrc.json", "fixtures/eslintrc_off/test.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn oxlint_config_auto_detection() {
        let args = &["debugger.js"];
        Tester::new().with_cwd("fixtures/auto_config_detection".into()).test_and_snapshot(args);
    }

    #[test]
    #[cfg(not(target_os = "windows"))] // Skipped on Windows due to snapshot diffs from path separators (`/` vs `\`)
    fn oxlint_config_auto_detection_parse_error() {
        let args = &["debugger.js"];
        Tester::new().with_cwd("fixtures/auto_config_parse_error".into()).test_and_snapshot(args);
    }

    #[test]
    fn eslintrc_no_undef() {
        let args = &[
            "-W",
            "no-undef",
            "-c",
            "fixtures/no_undef/eslintrc.json",
            "fixtures/no_undef/test.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn eslintrc_no_env() {
        let args = &[
            "-W",
            "no-undef",
            "-c",
            "fixtures/eslintrc_env/eslintrc_no_env.json",
            "fixtures/eslintrc_env/test.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn eslintrc_with_env() {
        let args = &[
            "-c",
            "fixtures/eslintrc_env/eslintrc_env_browser.json",
            "fixtures/eslintrc_env/test.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn no_empty_allow_empty_catch() {
        let args = &[
            "-c",
            "fixtures/no_empty_allow_empty_catch/eslintrc.json",
            "-W",
            "no-empty",
            "fixtures/no_empty_allow_empty_catch/test.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn no_empty_disallow_empty_catch() {
        let args = &[
            "-c",
            "fixtures/no_empty_disallow_empty_catch/eslintrc.json",
            "-W",
            "no-empty",
            "fixtures/no_empty_disallow_empty_catch/test.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn no_console_off() {
        let args =
            &["-c", "fixtures/no_console_off/eslintrc.json", "fixtures/no_console_off/test.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn typescript_eslint() {
        let args = &[
            "-c",
            "fixtures/typescript_eslint/eslintrc.json",
            "fixtures/typescript_eslint/test.ts",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn typescript_eslint_off() {
        let args = &[
            "-c",
            "fixtures/typescript_eslint/eslintrc.json",
            "--disable-typescript-plugin",
            "fixtures/typescript_eslint/test.ts",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn js_and_jsx() {
        let args = &["fixtures/linter/js_as_jsx.js"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn lint_vue_file() {
        let args = &["fixtures/vue/debugger.vue"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn lint_empty_vue_file() {
        let args = &["fixtures/vue/empty.vue"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn lint_astro_file() {
        let args = &["fixtures/astro/debugger.astro"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn lint_svelte_file() {
        let args = &["fixtures/svelte/debugger.svelte"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_tsconfig_option() {
        // passed
        Tester::new().test(&["--tsconfig", "fixtures/tsconfig/tsconfig.json"]);

        // failed
        Tester::new().test_and_snapshot(&["--tsconfig", "oxc/tsconfig.json"]);
    }

    #[test]
    fn test_enable_vitest_rule_without_plugin() {
        let args = &[
            "-c",
            "fixtures/eslintrc_vitest_replace/eslintrc.json",
            "fixtures/eslintrc_vitest_replace/foo.test.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_enable_vitest_plugin() {
        let args = &[
            "--vitest-plugin",
            "-c",
            "fixtures/eslintrc_vitest_replace/eslintrc.json",
            "fixtures/eslintrc_vitest_replace/foo.test.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_import_plugin_enabled_in_config() {
        let args_1 = &["-c", ".oxlintrc.json", "test.js"];
        // support import-x namespace see #8779
        let args_2 = &["-c", ".oxlintrc-import-x.json", "test.js"];
        Tester::new()
            .with_cwd("fixtures/import".into())
            .test_and_snapshot_multiple(&[args_1, args_2]);
    }

    #[test]
    fn test_fix() {
        Tester::test_fix("fixtures/fix_argument/fix.js", "debugger\n", "\n");
        Tester::test_fix(
            "fixtures/fix_argument/fix.vue",
            "<script>debugger;</script>\n<script>debugger;</script>\n",
            "<script></script>\n<script></script>\n",
        );
    }

    #[test]
    fn test_print_config_ban_all_rules() {
        let args = &["-A", "all", "--print-config"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_print_config_ban_rules() {
        let args = &[
            "-c",
            "fixtures/print_config/ban_rules/eslintrc.json",
            "-A",
            "all",
            "-D",
            "eqeqeq",
            "--print-config",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_init_config() {
        assert!(!fs::exists(LintRunner::DEFAULT_OXLINTRC).unwrap());

        let args = &["--init"];
        Tester::new().test(args);

        assert!(fs::exists(LintRunner::DEFAULT_OXLINTRC).unwrap());

        fs::remove_file(LintRunner::DEFAULT_OXLINTRC).unwrap();
    }

    #[test]
    fn test_overrides() {
        let args_1 = &["-c", "fixtures/overrides/.oxlintrc.json", "fixtures/overrides/test.js"];
        let args_2 = &["-c", "fixtures/overrides/.oxlintrc.json", "fixtures/overrides/test.ts"];
        let args_3 = &["-c", "fixtures/overrides/.oxlintrc.json", "fixtures/overrides/other.jsx"];
        Tester::new().test_and_snapshot_multiple(&[args_1, args_2, args_3]);
    }

    #[test]
    fn test_overrides_directories() {
        let args = &["-c", "fixtures/overrides/directories-config.json", "fixtures/overrides"];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_overrides_envs_and_global() {
        let args = &["-c", ".oxlintrc.json", "."];
        Tester::new().with_cwd("fixtures/overrides_env_globals".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_ignore_patterns() {
        let args = &["-c", "./test/eslintrc.json", "--ignore-pattern", "*.ts", "."];

        Tester::new()
            .with_cwd("fixtures/config_ignore_patterns/with_oxlintrc".into())
            .test_and_snapshot(args);
    }

    #[test]
    fn test_config_ignore_patterns_extension() {
        let args = &[
            "-c",
            "fixtures/config_ignore_patterns/ignore_extension/eslintrc.json",
            "fixtures/config_ignore_patterns/ignore_extension",
        ];

        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_config_ignore_patterns_special_extension() {
        let args = &[
            "-c",
            "fixtures/config_ignore_patterns/ignore_extension/eslintrc.json",
            "fixtures/config_ignore_patterns/ignore_extension/main.js",
        ];

        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_config_ignore_patterns_directory() {
        let args = &["-c", "eslintrc.json"];
        Tester::new()
            .with_cwd("fixtures/config_ignore_patterns/ignore_directory".into())
            .test_and_snapshot(args);
    }

    // Issue: <https://github.com/oxc-project/oxc/pull/7566>
    #[test]
    fn ignore_path_with_relative_files() {
        let args = &[
            "--ignore-path",
            "fixtures/issue_7566/.oxlintignore",
            "fixtures/issue_7566/tests/main.js",
            "fixtures/issue_7566/tests/function/main.js",
        ];
        Tester::new().test_and_snapshot(args);
    }

    #[test]
    fn test_jest_and_vitest_alias_rules() {
        let args_1 = &["-c", "oxlint-jest.json", "test.js"];
        let args_2 = &["-c", "oxlint-vitest.json", "test.js"];
        Tester::new()
            .with_cwd("fixtures/jest_and_vitest_alias_rules".into())
            .test_and_snapshot_multiple(&[args_1, args_2]);
    }

    #[test]
    fn test_eslint_and_typescript_alias_rules() {
        let args_1 = &["-c", "oxlint-eslint.json", "test.js"];
        let args_2 = &["-c", "oxlint-typescript.json", "test.js"];
        Tester::new()
            .with_cwd("fixtures/eslint_and_typescript_alias_rules".into())
            .test_and_snapshot_multiple(&[args_1, args_2]);
    }

    #[test]
    fn test_disable_eslint_and_unicorn_alias_rules() {
        let args_1 = &["-c", ".oxlintrc-eslint.json", "test.js"];
        let args_2 = &["-c", ".oxlintrc-unicorn.json", "test.js"];
        Tester::new()
            .with_cwd("fixtures/disable_eslint_and_unicorn_alias_rules".into())
            .test_and_snapshot_multiple(&[args_1, args_2]);
    }

    #[test]
    fn test_two_rules_with_same_rule_name_from_different_plugins() {
        // Issue: <https://github.com/oxc-project/oxc/issues/8485>
        let args = &["-c", ".oxlintrc.json", "test.js"];
        Tester::new()
            .with_cwd("fixtures/two_rules_with_same_rule_name".into())
            .test_and_snapshot(args);
    }

    #[test]
    fn test_report_unused_directives() {
        let args = &["-c", ".oxlintrc.json", "--report-unused-disable-directives", "test.js"];

        Tester::new().with_cwd("fixtures/report_unused_directives".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_nested_config() {
        let args = &[];
        Tester::new().with_cwd("fixtures/nested_config".into()).test_and_snapshot(args);

        let args = &["--disable-nested-config"];
        Tester::new().with_cwd("fixtures/extends_config".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_nested_config_subdirectory() {
        // This tests the specific scenario from issue #10156
        // where a file is located in a subdirectory of a directory with a config file
        let args = &["package3-deep-config"];
        Tester::new().with_cwd("fixtures/nested_config".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_nested_config_explicit_config_precedence() {
        // `--config` takes absolute precedence over nested configs, and will be used for
        // linting all files rather than the nested configuration files.
        let args = &["--config", "oxlint-no-console.json"];
        Tester::new().with_cwd("fixtures/nested_config".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_nested_config_filter_precedence() {
        // CLI arguments take precedence over nested configs, but apply over top of the nested
        // config files, rather than replacing them.
        let args = &["-A", "no-console"];
        Tester::new().with_cwd("fixtures/nested_config".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_nested_config_explicit_config_and_filter_precedence() {
        // Combining `--config` and CLI filters should make the passed config file be
        // used for all files, but still override any rules specified in the config file.
        let args = &["-A", "no-console", "--config", "oxlint-no-console.json"];
        Tester::new().with_cwd("fixtures/nested_config".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_extends_explicit_config() {
        // Check that referencing a config file that extends other config files works as expected
        let args = &["--config", "extends_rules_config.json", "console.js"];
        Tester::new().with_cwd("fixtures/extends_config".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_extends_extends_config() {
        // Check that using a config that extends a config which extends a config works
        let args = &["--config", "relative_paths/extends_extends_config.json", "console.js"];
        Tester::new().with_cwd("fixtures/extends_config".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_extends_overrides() {
        // Check that using a config with overrides works as expected
        let args = &["overrides"];
        Tester::new().with_cwd("fixtures/extends_config".into()).test_and_snapshot(args);

        // Check that using a config which extends a config with overrides works as expected
        let args = &["overrides_same_directory"];
        Tester::new().with_cwd("fixtures/extends_config".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_nested_config_multi_file_analysis_imports() {
        let args = &["issue_10054"];
        Tester::new().with_cwd("fixtures".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_config_path_with_parent_references() {
        let cwd = std::env::current_dir().unwrap();

        // Test case 1: Invalid path that should fail
        let invalid_config = PathBuf::from("child/../../fixtures/linter/eslintrc.json");
        let result = LintRunner::find_oxlint_config(&cwd, Some(&invalid_config));
        assert!(result.is_err(), "Expected config lookup to fail with invalid path");

        // Test case 2: Valid path that should pass
        let valid_config = PathBuf::from("fixtures/linter/eslintrc.json");
        let result = LintRunner::find_oxlint_config(&cwd, Some(&valid_config));
        assert!(result.is_ok(), "Expected config lookup to succeed with valid path");

        // Test case 3: Valid path using parent directory (..) syntax that should pass
        let valid_parent_config = PathBuf::from("fixtures/linter/../linter/eslintrc.json");
        let result = LintRunner::find_oxlint_config(&cwd, Some(&valid_parent_config));
        assert!(result.is_ok(), "Expected config lookup to succeed with parent directory syntax");

        // Verify the resolved path is correct
        if let Ok(config) = result {
            assert_eq!(
                config.path.file_name().unwrap().to_str().unwrap(),
                "eslintrc.json",
                "Config file name should be preserved after path resolution"
            );
        }
    }

    #[test]
    fn test_cross_modules_with_nested_config() {
        let args = &[];
        Tester::new()
            .with_cwd("fixtures/cross_module_nested_config".into())
            .test_and_snapshot(args);
    }

    #[test]
    fn test_cross_modules_with_extended_config() {
        let args = &[];
        Tester::new()
            .with_cwd("fixtures/cross_module_extended_config".into())
            .test_and_snapshot(args);
    }

    #[test]
    fn test_import_plugin_being_enabled_correctly() {
        // https://github.com/oxc-project/oxc/pull/10597
        let args = &["--import-plugin", "-D", "import/no-cycle"];
        Tester::new().with_cwd("fixtures/import-cycle".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_rule_config_being_enabled_correctly() {
        let args = &["-c", ".oxlintrc.json"];
        Tester::new().with_cwd("fixtures/issue_11054".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_plugins_in_overrides_enabled_correctly() {
        let args = &["-c", ".oxlintrc.json"];
        Tester::new().with_cwd("fixtures/overrides_with_plugin".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_plugins_inside_overrides_categories_enabled_correctly() {
        let args = &["-c", ".oxlintrc.json"];
        Tester::new().with_cwd("fixtures/issue_10394".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_jsx_a11y_label_has_associated_control() {
        let args = &["-c", ".oxlintrc.json"];
        Tester::new().with_cwd("fixtures/issue_11644".into()).test_and_snapshot(args);
    }

    #[test]
    fn test_dot_folder() {
        Tester::new().with_cwd("fixtures/dot_folder".into()).test_and_snapshot(&[]);
    }

    // ToDo: `tsgolint` does not support `big-endian`?
    #[test]
    #[cfg(not(target_endian = "big"))]
    fn test_tsgolint() {
        // TODO: test with other rules as well once diagnostics are more stable
        let args = &["--type-aware", "no-floating-promises"];
        Tester::new().with_cwd("fixtures/tsgolint".into()).test_and_snapshot(args);
    }

    #[test]
    #[cfg(not(target_endian = "big"))]
    fn test_tsgolint_silent() {
        // TODO: test with other rules as well once diagnostics are more stable
        let args = &["--type-aware", "--silent", "no-floating-promises"];
        Tester::new().with_cwd("fixtures/tsgolint".into()).test_and_snapshot(args);
    }

    #[test]
    #[cfg(not(target_endian = "big"))]
    fn test_tsgolint_config() {
        // TODO: test with other rules as well once diagnostics are more stable
        let args = &["--type-aware", "no-floating-promises", "-c", "config-test.json"];
        Tester::new().with_cwd("fixtures/tsgolint".into()).test_and_snapshot(args);
    }

    #[test]
    #[cfg(not(target_endian = "big"))]
    fn test_tsgolint_no_typescript_files() {
        // tsgolint shouldn't run when no files need type aware linting
        let args = &["--type-aware", "test.svelte"];
        Tester::new().with_cwd("fixtures/tsgolint".into()).test_and_snapshot(args);
    }
}
