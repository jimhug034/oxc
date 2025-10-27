//! # Walk 模块
//!
//! 本模块提供并行文件系统遍历功能，用于 oxlint 遍历需要检查的文件。
//!
//! ## 核心功能
//! - 并行遍历多个目录和文件
//! - 根据文件扩展名过滤文件
//! - 支持 .gitignore 等忽略规则
//! - 自动过滤掉压缩后的文件（如 .min.js, -min.js, _min.js）
//! - 使用多线程提高遍历性能

use std::{ffi::OsStr, path::PathBuf, sync::Arc, sync::mpsc};

use ignore::{DirEntry, overrides::Override};
use oxc_linter::LINTABLE_EXTENSIONS;

use crate::cli::IgnoreOptions;

/// 文件扩展名集合
///
/// 用于指定需要遍历的文件类型，默认包含所有可链接的文件扩展名
#[derive(Debug, Clone)]
pub struct Extensions(pub Vec<&'static str>);

impl Default for Extensions {
    fn default() -> Self {
        Self(LINTABLE_EXTENSIONS.to_vec())
    }
}

/// 并行文件遍历器
///
/// 基于 `ignore` crate 的并行遍历实现，用于高效地收集需要检查的文件路径。
/// 使用多线程并发遍历，大幅提升大型项目的文件收集性能。
pub struct Walk {
    /// 底层的并行遍历实例
    inner: ignore::WalkParallel,
    /// 需要包含的文件扩展名
    extensions: Extensions,
}

/// 并行访问者构建器
///
/// 为 `ignore::WalkParallel` 创建并行访问者实例，实现文件路径的并发收集
struct WalkBuilder {
    /// 用于发送收集到的文件路径的通道发送端
    sender: mpsc::Sender<Vec<Arc<OsStr>>>,
    /// 文件扩展名过滤器
    extensions: Extensions,
}

impl<'s> ignore::ParallelVisitorBuilder<'s> for WalkBuilder {
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(WalkCollector {
            paths: vec![],
            sender: self.sender.clone(),
            extensions: self.extensions.clone(),
        })
    }
}

/// 文件路径收集器
///
/// 在并行遍历过程中收集符合条件的文件路径。
/// 使用 Vec 批量收集路径，在 Drop 时一次性发送，减少通道通信开销。
struct WalkCollector {
    /// 临时存储收集到的文件路径
    paths: Vec<Arc<OsStr>>,
    /// 用于将收集到的路径发送给主线程的通道
    sender: mpsc::Sender<Vec<Arc<OsStr>>>,
    /// 文件扩展名过滤器
    extensions: Extensions,
}

impl Drop for WalkCollector {
    /// 在收集器销毁时，将收集到的所有路径发送给主线程
    fn drop(&mut self) {
        let paths = std::mem::take(&mut self.paths);
        self.sender.send(paths).unwrap();
    }
}

impl ignore::ParallelVisitor for WalkCollector {
    /// 并行访问者核心方法：处理每个遍历到的文件或目录
    ///
    /// - 对于符合条件的文件，将其路径添加到收集列表
    /// - 跳过目录和不符合条件的文件
    /// - 忽略遍历错误
    fn visit(&mut self, entry: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
        match entry {
            Ok(entry) => {
                if Walk::is_wanted_entry(&entry, &self.extensions) {
                    self.paths.push(entry.path().as_os_str().into());
                }
                ignore::WalkState::Continue
            }
            Err(_err) => ignore::WalkState::Skip,
        }
    }
}
impl Walk {
    /// 创建新的并行文件遍历器
    ///
    /// # 参数
    /// - `paths`: 要遍历的路径列表（至少需要一个路径）
    /// - `options`: 忽略选项配置
    /// - `override_builder`: 可选的忽略规则覆盖构建器
    ///
    /// # 行为
    /// - 不会规范化路径（不解析符号链接到绝对路径）
    /// - 支持多个路径同时遍历
    /// - 启用 .gitignore 等忽略规则（除非 `no_ignore` 为 true）
    /// - 遵循符号链接（不跟踪循环引用）
    /// - 不包含隐藏文件
    ///
    /// # Panics
    /// 如果 `paths` 为空会 panic
    pub fn new(
        paths: &[PathBuf],
        options: &IgnoreOptions,
        override_builder: Option<Override>,
    ) -> Self {
        assert!(!paths.is_empty(), "At least one path must be provided to Walk::new");

        // 取出第一个path作为根路径
        let mut inner = ignore::WalkBuilder::new(
            paths
                .iter()
                .next()
                .expect("Expected paths parameter to Walk::new() to contain at least one path."),
        );

        // 添加额外的路径（如果有多个）
        if let Some(paths) = paths.get(1..) {
            for path in paths {
                inner.add(path);
            }
        }

        // 配置忽略规则
        if !options.no_ignore {
            inner.add_custom_ignore_filename(&options.ignore_path);

            if let Some(override_builder) = override_builder {
                inner.overrides(override_builder);
            }
        }

        // 构建并行遍历器
        // ignore(false): 不忽略文件
        // git_global(false): 不使用全局 gitignore
        // follow_links(true): 跟随符号链接
        // hidden(false): 不包含隐藏文件
        let inner =
            inner.ignore(false).git_global(false).follow_links(true).hidden(false).build_parallel();
        Self { inner, extensions: Extensions::default() }
    }

    /// 执行并行遍历并收集所有符合条件的文件路径
    ///
    /// # 工作流程
    /// 1. 创建通道用于收集器与主线程通信
    /// 2. 启动并行遍历，每个线程使用 `WalkCollector` 收集路径
    /// 3. 各个线程批量发送收集到的路径
    /// 4. 主线程接收并合并所有路径
    ///
    /// # 返回
    /// 所有符合条件的文件路径列表
    pub fn paths(self) -> Vec<Arc<OsStr>> {
        let (sender, receiver) = mpsc::channel::<Vec<Arc<OsStr>>>();
        let mut builder = WalkBuilder { sender, extensions: self.extensions };
        self.inner.visit(&mut builder);
        drop(builder);
        receiver.into_iter().flatten().collect()
    }

    /// 设置自定义的文件扩展名过滤器
    ///
    /// 覆盖默认的可链接文件扩展名，只遍历指定扩展名的文件
    #[cfg_attr(not(test), expect(dead_code))]
    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }

    /// 判断一个目录条目是否是想要的文件
    ///
    /// # 过滤规则
    /// 1. 必须是文件（不是目录）
    /// 2. 文件扩展名必须在允许列表中
    /// 3. 不包含压缩文件标记（.min., -min., _min.）
    ///
    /// # 示例
    /// - `foo.js` -> true (符合扩展名)
    /// - `bar.min.js` -> false (压缩文件)
    /// - `baz.txt` -> false (扩展名不在列表中)
    fn is_wanted_entry(dir_entry: &DirEntry, extensions: &Extensions) -> bool {
        let Some(file_type) = dir_entry.file_type() else { return false };
        if file_type.is_dir() {
            return false;
        }
        let Some(file_name) = dir_entry.path().file_name() else { return false };
        // 过滤掉压缩文件（如 bundle.min.js, app-min.js, lib_min.js）
        if [".min.", "-min.", "_min."].iter().any(|e| file_name.to_string_lossy().contains(e)) {
            return false;
        }
        let Some(extension) = dir_entry.path().extension() else { return false };
        let extension = extension.to_string_lossy();
        extensions.0.contains(&extension.as_ref())
    }
}

#[cfg(test)]
mod test {
    use std::{env, ffi::OsString, path::Path};

    use ignore::overrides::OverrideBuilder;

    use super::{Extensions, Walk};
    use crate::cli::IgnoreOptions;

    #[test]
    fn test_walk_with_extensions() {
        let fixture = env::current_dir().unwrap().join("fixtures/walk_dir");
        let fixtures = vec![fixture.clone()];
        let ignore_options = IgnoreOptions {
            no_ignore: false,
            ignore_path: OsString::from(".gitignore"),
            ignore_pattern: vec![],
        };

        let override_builder = OverrideBuilder::new("/").build().unwrap();

        let mut paths = Walk::new(&fixtures, &ignore_options, Some(override_builder))
            .with_extensions(Extensions(["js", "vue"].to_vec()))
            .paths()
            .into_iter()
            .map(|path| {
                Path::new(&path).strip_prefix(&fixture).unwrap().to_string_lossy().to_string()
            })
            .collect::<Vec<_>>();
        paths.sort();

        assert_eq!(paths, vec!["bar.vue", "foo.js"]);
    }
}
