use std::io::BufWriter;

pub use oxc_linter::{
    ExternalLinter, ExternalLinterLintFileCb, ExternalLinterLoadPluginCb, LintFileResult,
    PluginLoadResult,
};

mod command;
mod lint;
mod output_formatter;
mod result;
mod tester;
mod walk;

pub mod cli {
    pub use crate::{command::*, lint::LintRunner, result::CliRunResult};
}

use cli::{CliRunResult, LintRunner};

#[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
mod raw_fs;

#[cfg(all(feature = "allocator", not(miri), not(target_family = "wasm")))]
#[global_allocator]
static GLOBAL: mimalloc_safe::MiMalloc = mimalloc_safe::MiMalloc;

/// Oxlint 核心启动函数
///
/// 这是从 main() 调用的主要入口点，负责：
/// 1. 初始化运行环境（日志、错误报告）
/// 2. 解析命令行参数
/// 3. 配置线程池
/// 4. 创建并运行 LintRunner
///
/// # 调用链
/// main() -> lint() -> LintRunner::run() -> Linter + LintService
///
/// # 参数
/// * `external_linter` - 可选的外部 linter，主要用于 NAPI 绑定
///
/// # 返回值
/// 返回 CliRunResult，会被转换为进程退出码
pub fn lint(external_linter: Option<ExternalLinter>) -> CliRunResult {
    // ====== 阶段 1: 初始化环境 ======
    // 初始化日志追踪（用于 OXC_LOG 环境变量）
    init_tracing();
    // 初始化错误报告系统（提供美观的错误输出）
    init_miette();

    // ====== 阶段 2: 解析命令行参数 ======
    let mut args = std::env::args_os();
    for argument in args.by_ref() {
        println!("{argument:?}");
    }
    // 如果第一个参数是 `node`，则跳过脚本路径 (`node script.js ...`)
    // 否则，只跳过第一个参数（即 `oxlint` 本身）
    if args.next().is_some_and(|arg| arg == "node") {
        args.next();
    }
    let args = args.collect::<Vec<_>>();

    // 使用 bpaf 库解析命令行参数（解析成 LintCommand 结构）
    let cmd = crate::cli::lint_command();
    let command = match cmd.run_inner(&*args) {
        Ok(cmd) => cmd,
        Err(e) => {
            // 参数解析失败，打印错误信息并返回适当的退出码
            e.print_message(100);
            return if e.exit_code() == 0 {
                CliRunResult::LintSucceeded
            } else {
                CliRunResult::InvalidOptionConfig
            };
        }
    };

    // ====== 阶段 3: 初始化 Rayon 线程池 ======
    // 根据 --threads 参数或 CPU 核心数设置并行度
    command.handle_threads();

    // stdio 被 LineWriter 阻塞，使用 BufWriter 减少系统调用，提高性能
    // 参见 `https://github.com/rust-lang/rust/issues/60673`
    let mut stdout = BufWriter::new(std::io::stdout());

    // ====== 阶段 4: 创建 LintRunner 并执行 linting ======
    // LintRunner::run() 内部会：
    // 1. 加载配置文件（.oxlintrc.json）
    // 2. 遍历文件系统收集要检查的文件
    // 3. 创建 oxc_linter::Linter 实例
    // 4. 通过 LintService 并行执行 linting
    LintRunner::new(command, external_linter).run(&mut stdout)
}

/// 初始化 miette 错误报告系统
///
/// 初始化依赖于 `is_atty` 系统调用的数据，确保它们不会阻塞后续线程。
/// miette 提供美观的、上下文丰富的错误诊断输出。
fn init_miette() {
    miette::set_hook(Box::new(|_| Box::new(miette::MietteHandlerOpts::new().build()))).unwrap();
}

/// 初始化 tracing 日志追踪系统
///
/// 通过 `OXC_LOG` 环境变量控制日志输出。
///
/// # 使用示例
/// 调试 `oxc_resolver`: `OXC_LOG=oxc_resolver oxlint --import-plugin`
/// 调试多个模块: `OXC_LOG=oxc_resolver,oxc_linter oxlint`
fn init_tracing() {
    use tracing_subscriber::{filter::Targets, prelude::*};

    // 不使用 `regex` 特性的用法
    // 参见 <https://github.com/tokio-rs/tracing/issues/1436#issuecomment-918528013>
    tracing_subscriber::registry()
        .with(std::env::var("OXC_LOG").map_or_else(
            |_| Targets::new(), // 环境变量不存在时，不输出任何日志
            |env_var| {
                // 解析环境变量中的日志目标（如 "oxc_resolver,oxc_linter"）
                use std::str::FromStr;
                Targets::from_str(&env_var).unwrap()
            },
        ))
        .with(tracing_subscriber::fmt::layer()) // 添加格式化输出层
        .init();
}
