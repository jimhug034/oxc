use oxlint::{cli::CliRunResult, lint};

/// Oxlint 主入口点
///
/// 调用链: main() -> lint() -> LintRunner::new().run() -> LintService::run()
fn main() -> CliRunResult {
    // 调用 lint 函数，不传入外部 linter（仅用于 Node.js 绑定）
    lint(None)
}
