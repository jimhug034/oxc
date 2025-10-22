//! 命令行参数解析模块
//!
//! 这个模块包含 Oxlint 命令行工具的所有参数定义和解析逻辑。
//!
//! # 模块结构
//!
//! - `lint` - Lint 命令的完整参数定义
//! - `ignore` - 文件忽略相关的参数
//! - `mod` - 公共参数和工具函数

mod ignore;
mod lint;

use std::path::PathBuf;

use bpaf::Bpaf;

pub use self::{
    ignore::IgnoreOptions,
    lint::{LintCommand, OutputOptions, ReportUnusedDirectives, WarningOptions, lint_command},
};

/// Oxlint 版本号
///
/// 优先使用编译时的 `OXC_VERSION` 环境变量，否则使用 "dev"。
/// 在 CI/CD 构建时会设置 `OXC_VERSION` 为实际的版本号。
const VERSION: &str = match option_env!("OXC_VERSION") {
    Some(v) => v,
    None => "dev",
};

/// 杂项选项
///
/// 包含不属于其他特定类别的命令行选项。
#[derive(Debug, Clone, Bpaf)]
pub struct MiscOptions {
    /// 静默模式 - 不显示任何诊断信息
    ///
    /// 使用 `--silent` 标志启用。
    /// 在静默模式下，Oxlint 仍会执行 linting，但不会输出任何诊断信息。
    /// 只有退出码会指示是否发现了问题。
    ///
    /// # 使用场景
    /// - CI/CD 管道中只需要检查退出码
    /// - 作为其他工具的一部分运行时
    ///
    /// # 示例
    /// ```bash
    /// oxlint --silent src/ && echo "No errors found"
    /// ```
    #[bpaf(switch, hide_usage)]
    pub silent: bool,

    /// 使用的线程数
    ///
    /// 设置为 1 则只使用 1 个 CPU 核心（单线程模式）。
    /// 如果不指定，默认使用系统可用的 CPU 核心数。
    ///
    /// # 使用场景
    /// - 限制 CPU 使用率：`--threads 2`
    /// - 调试时使用单线程：`--threads 1`
    /// - 在资源受限的环境中运行
    ///
    /// # 示例
    /// ```bash
    /// # 使用 4 个线程
    /// oxlint --threads 4 src/
    ///
    /// # 单线程运行（便于调试）
    /// oxlint --threads 1 src/
    /// ```
    #[bpaf(argument("INT"), hide_usage)]
    pub threads: Option<usize>,

    /// 输出将要使用的配置
    ///
    /// 当此选项存在时：
    /// - 不执行实际的 linting
    /// - 只有配置相关的选项有效
    /// - 输出最终合并后的配置（包括默认值、配置文件、命令行参数）
    ///
    /// # 使用场景
    /// - 验证配置文件是否正确加载
    /// - 调试配置问题
    /// - 了解默认配置
    ///
    /// # 示例
    /// ```bash
    /// # 查看当前配置
    /// oxlint --print-config
    ///
    /// # 查看特定配置文件的效果
    /// oxlint --config custom.json --print-config
    /// ```
    #[bpaf(switch, hide_usage)]
    pub print_config: bool,
}

/// 验证路径是否有效
///
/// 确保所有路径都不包含 `..` (父目录) 组件，防止遍历到父目录。
///
/// # 安全性
///
/// 这是一个安全检查，防止用户意外或恶意地访问项目目录之外的文件。
/// 例如：
/// - ✅ 允许：`src/`, `./test.js`, `subfolder/file.js`
/// - ❌ 拒绝：`../parent/`, `src/../../etc/passwd`
///
/// # 参数
///
/// - `paths` - 要验证的路径列表
///
/// # 返回值
///
/// - `true` - 所有路径都有效（或列表为空）
/// - `false` - 至少有一个路径包含 `..`
///
/// # 示例
///
/// ```rust
/// let valid = vec![PathBuf::from("src/"), PathBuf::from("test.js")];
/// assert!(validate_paths(&valid));
///
/// let invalid = vec![PathBuf::from("../parent/")];
/// assert!(!validate_paths(&invalid));
/// ```
#[expect(clippy::ptr_arg)]
fn validate_paths(paths: &Vec<PathBuf>) -> bool {
    if paths.is_empty() {
        // 空路径列表视为有效
        true
    } else {
        // 检查所有路径的所有组件，确保没有 ParentDir (..)
        paths.iter().all(|p| p.components().all(|c| c != std::path::Component::ParentDir))
    }
}

/// 路径验证失败时的错误消息
///
/// 在命令行参数解析时显示给用户。
const PATHS_ERROR_MESSAGE: &str = "PATH must not contain \"..\"";

/// 测试模块：杂项选项
#[cfg(test)]
mod misc_options {
    use super::{MiscOptions, lint::lint_command};

    /// 辅助函数：从命令行字符串解析 MiscOptions
    ///
    /// 将空格分隔的参数字符串解析为 MiscOptions 结构体。
    fn get_misc_options(arg: &str) -> MiscOptions {
        let args = arg.split(' ').map(std::string::ToString::to_string).collect::<Vec<_>>();
        lint_command().run_inner(args.as_slice()).unwrap().misc_options
    }

    /// 测试：默认情况下线程数未设置
    #[test]
    fn default() {
        let options = get_misc_options(".");
        assert!(options.threads.is_none());
    }

    /// 测试：--threads 参数正确解析
    #[test]
    fn threads() {
        let options = get_misc_options("--threads 4 .");
        assert_eq!(options.threads, Some(4));
    }
}
