//! # ConfigStoreBuilder - 配置构建器
//!
//! 这是 Oxc linter 配置系统的核心组件，负责从配置文件（`.oxlintrc.json`）构建最终的执行配置。
//!
//! ## 主要功能
//!
//! 1. **解析配置文件**：支持 JSON、YAML 等格式的配置文件
//! 2. **处理继承**：支持 `extends` 字段，从多个配置文件继承和合并设置
//! 3. **加载插件**：加载内置插件和外部插件（如 ESLint 插件）
//! 4. **配置规则**：设置规则的严重程度（allow/warn/deny/error）
//! 5. **处理覆盖**：支持基于文件路径的规则覆盖配置
//!
//! ## 工作流程
//!
//! ```text
//! 配置文件 (Oxlintrc)
//!     ↓
//! 1. 解析 extends 继承链
//!     ↓
//! 2. 加载外部插件
//!     ↓
//! 3. 应用规则配置
//!     ↓
//! 4. 处理类别过滤器
//!     ↓
//! 5. 处理覆盖配置
//!     ↓
//! ConfigStoreBuilder
//!     ↓
//! build()
//!     ↓
//! Config (最终配置)
//! ```
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! // 从配置文件创建
//! let config = ConfigStoreBuilder::from_oxlintrc(true, oxlintrc, None, &mut store)
//!     .unwrap()
//!     .build(&store)
//!     .unwrap();
//!
//! // 手动构建
//! let config = ConfigStoreBuilder::default()
//!     .with_filter(&LintFilter::warn(RuleCategory::Correctness))
//!     .with_filter(&LintFilter::deny("no-console"))
//!     .build(&store)
//!     .unwrap();
//! ```

use std::{
    fmt::{self, Debug, Display},
    path::{Path, PathBuf},
};

use itertools::Itertools;
use oxc_resolver::Resolver;
use rustc_hash::{FxHashMap, FxHashSet};

use oxc_span::{CompactStr, format_compact_str};

use crate::{
    AllowWarnDeny, ExternalPluginStore, LintConfig, LintFilter, LintFilterKind, Oxlintrc,
    RuleCategory, RuleEnum,
    config::{
        ESLintRule, LintPlugins, OxlintOverrides, OxlintRules, overrides::OxlintOverride,
        plugins::BuiltinLintPlugins,
    },
    external_linter::ExternalLinter,
    external_plugin_store::{ExternalRuleId, ExternalRuleLookupError},
    rules::RULES,
};

use super::{
    Config,
    categories::OxlintCategories,
    config_store::{ResolvedOxlintOverride, ResolvedOxlintOverrideRules, ResolvedOxlintOverrides},
};

/// 配置构建器，用于构建 linter 的最终配置
///
/// # 核心职责
///
/// 这个构建器负责：
/// - 解析和合并配置文件
/// - 加载和初始化插件
/// - 应用规则配置
/// - 处理文件覆盖
///
/// # 使用模式
///
/// 构建器采用建造者模式（Builder Pattern），允许链式调用：
/// ```rust,ignore
/// let config = ConfigStoreBuilder::default()
///     .with_filter(...)
///     .with_overrides(...)
///     .build(&store)
///     .unwrap();
/// ```
///
/// # 注意事项
///
/// - **必须调用 `build()`**：构建器被标记为 `#[must_use]`，忘记调用会编译期警告
/// - **配置不可变**：每次调用 `with_*` 方法都会创建新的构建器实例
/// - **延迟构建**：只有在调用 `build()` 时才会应用所有配置并创建最终配置
#[must_use = "You dropped your builder without building a Linter! Did you mean to call .build()?"]
pub struct ConfigStoreBuilder {
    /// 内置规则的配置映射：规则 -> 严重程度
    pub(super) rules: FxHashMap<RuleEnum, AllowWarnDeny>,

    /// 外部插件规则的配置映射：规则ID -> 严重程度
    pub(super) external_rules: FxHashMap<ExternalRuleId, AllowWarnDeny>,

    /// linter 配置（插件、设置、环境变量等）
    config: LintConfig,

    /// 规则类别的配置（correctness, suspicious, performance 等）
    categories: OxlintCategories,

    /// 基于文件路径的规则覆盖配置
    overrides: OxlintOverrides,

    /// 收集所有被 `extends` 引用的文件路径
    ///
    /// 语言服务器用这些路径来监听文件变化，当配置文件被修改时重新加载配置
    pub extended_paths: Vec<PathBuf>,
}

impl Default for ConfigStoreBuilder {
    fn default() -> Self {
        Self { rules: Self::warn_correctness(BuiltinLintPlugins::default()), ..Self::empty() }
    }
}

impl ConfigStoreBuilder {
    /// Create a [`ConfigStoreBuilder`] with default plugins enabled and no
    /// configured rules.
    ///
    /// You can think of this as `oxlint -A all`.
    pub fn empty() -> Self {
        let config = LintConfig::default();
        let rules = FxHashMap::default();
        let external_rules = FxHashMap::default();
        let categories: OxlintCategories = OxlintCategories::default();
        let overrides = OxlintOverrides::default();
        let extended_paths = Vec::new();

        Self { rules, external_rules, config, categories, overrides, extended_paths }
    }

    /// Warn on all rules in all plugins and categories, including those in `nursery`.
    /// This is the kitchen sink.
    ///
    /// You can think of this as `oxlint -W all -W nursery`.
    pub fn all() -> Self {
        let config =
            LintConfig { plugins: BuiltinLintPlugins::all().into(), ..LintConfig::default() };
        let overrides = OxlintOverrides::default();
        let categories: OxlintCategories = OxlintCategories::default();
        let rules = RULES.iter().map(|rule| (rule.clone(), AllowWarnDeny::Warn)).collect();
        let external_rules = FxHashMap::default();
        let extended_paths = Vec::new();
        Self { rules, external_rules, config, categories, overrides, extended_paths }
    }

    /// 从已加载或手动构建的 [`Oxlintrc`] 创建 [`ConfigStoreBuilder`]
    ///
    /// # 工作原理
    ///
    /// 该方法负责解析和合并配置文件，处理以下内容：
    /// 1. **extends 继承**：解析配置文件中的 `extends` 字段，递归加载和合并继承的配置
    /// 2. **外部插件**：加载和初始化外部插件（如 ESLint 插件）
    /// 3. **规则配置**：应用规则设置，包括内置规则和外部插件规则
    /// 4. **类别配置**：处理规则类别的启用/禁用
    /// 5. **覆盖配置**：处理基于文件路径的规则覆盖
    ///
    /// # 参数
    ///
    /// - `start_empty`: 当为 `true` 时，构建器只包含配置文件中的设置，不应用默认配置
    ///   当为 `false` 时，配置将应用在默认 [`Oxlintrc`] 之上
    /// - `oxlintrc`: 要解析的配置文件对象
    /// - `external_linter`: 可选的外部 linter 实例（用于加载外部插件）
    /// - `external_plugin_store`: 外部插件存储，用于管理和查找外部插件规则
    ///
    /// # 示例
    ///
    /// 从 `.oxlintrc.json` 文件创建配置：
    /// ```ignore
    /// use oxc_linter::{ConfigBuilder, Oxlintrc};
    /// let oxlintrc = Oxlintrc::from_file("path/to/.oxlintrc.json").unwrap();
    /// let config_store = ConfigStoreBuilder::from_oxlintrc(true, oxlintrc).build();
    /// // 也可以使用 `From` trait 作为简写，等价于 `from_oxlintrc(false, oxlintrc)`
    /// let config_store = ConfigStoreBuilder::from(oxlintrc).build();
    /// ```
    ///
    /// # 错误
    ///
    /// 如果引用的配置文件无效，返回 [`ConfigBuilderError::InvalidConfigFile`]
    pub fn from_oxlintrc(
        start_empty: bool,
        oxlintrc: Oxlintrc,
        external_linter: Option<&ExternalLinter>,
        external_plugin_store: &mut ExternalPluginStore,
    ) -> Result<Self, ConfigBuilderError> {
        // TODO: 可以缓存以避免重复计算相同的 oxlintrc

        /// 递归解析配置文件继承链
        ///
        /// 解析 `extends` 字段中指定的配置文件，并从最底层配置开始向上合并，
        /// 确保子配置可以覆盖父配置的设置。
        ///
        /// # 返回
        /// 返回合并后的配置和所有被加载的配置文件路径（用于监听文件变化）
        fn resolve_oxlintrc_config(
            config: Oxlintrc,
        ) -> Result<(Oxlintrc, Vec<PathBuf>), ConfigBuilderError> {
            let path = config.path.clone();
            let root_path = path.parent();
            let extends = config.extends.clone();
            let mut extended_paths = Vec::new();

            let mut oxlintrc = config;

            // 从后向前遍历 extends 数组（最底层配置在前）
            // 这样可以确保父配置先被加载，子配置后加载并覆盖父配置
            for path in extends.iter().rev() {
                // 跳过 ESLint 命名配置（不支持）
                if path.starts_with("eslint:") || path.starts_with("plugin:") {
                    continue;
                }

                // 启发式检查：如果路径不包含 "."，可能是命名配置，跳过
                if !path.to_string_lossy().contains('.') {
                    continue;
                }

                // 解析相对路径：如果有根路径，则拼接；否则使用原路径
                let path = match root_path {
                    Some(p) => &p.join(path),
                    None => path,
                };

                // 加载被继承的配置文件
                let extends_oxlintrc = Oxlintrc::from_file(path).map_err(|e| {
                    ConfigBuilderError::InvalidConfigFile {
                        file: path.display().to_string(),
                        reason: e.to_string(),
                    }
                })?;

                // 记录被加载的配置文件路径（用于文件监听）
                extended_paths.push(path.clone());

                // 递归解析继承链：被继承的配置也可能有自己的 extends
                let (extends, extends_paths) = resolve_oxlintrc_config(extends_oxlintrc)?;

                // 合并配置：子配置会覆盖父配置中相同的设置
                oxlintrc = oxlintrc.merge(extends);
                extended_paths.extend(extends_paths);
            }

            Ok((oxlintrc, extended_paths))
        }

        // ========================================================================================
        // 步骤 1: 解析配置文件继承链并合并配置
        // ========================================================================================
        let (oxlintrc, extended_paths) = resolve_oxlintrc_config(oxlintrc)?;

        // ========================================================================================
        // 步骤 2: 收集外部插件引用（来自基础配置和覆盖配置）
        // ========================================================================================
        let mut external_plugins = FxHashSet::default();

        // 从基础配置中收集外部插件
        if let Some(base_plugins) = oxlintrc.plugins.as_ref() {
            external_plugins.extend(base_plugins.external.iter().cloned());
        }

        // 从覆盖配置中收集外部插件
        for r#override in &oxlintrc.overrides {
            if let Some(override_plugins) = &r#override.plugins {
                external_plugins.extend(override_plugins.external.iter().cloned());
            }
        }

        // ========================================================================================
        // 步骤 3: 加载外部插件（如 ESLint 插件）
        // ========================================================================================
        if !external_plugins.is_empty() {
            // 如果没有配置外部 linter，则报错
            let external_linter =
                external_linter.ok_or(ConfigBuilderError::NoExternalLinterConfigured)?;

            let resolver = Resolver::default();

            #[expect(clippy::missing_panics_doc, reason = "oxlintrc.path is always a file path")]
            let oxlintrc_dir = oxlintrc.path.parent().unwrap();

            // 加载每个外部插件
            for plugin_specifier in &external_plugins {
                Self::load_external_plugin(
                    oxlintrc_dir,
                    plugin_specifier,
                    external_linter,
                    &resolver,
                    external_plugin_store,
                )?;
            }
        }

        // ========================================================================================
        // 步骤 4: 获取插件配置（如果没有则使用默认值）
        // ========================================================================================
        let plugins = oxlintrc.plugins.unwrap_or_default();

        // ========================================================================================
        // 步骤 5: 初始化规则映射
        // ========================================================================================
        // 如果 start_empty 为 true，则从空规则集开始；否则默认启用 correctness 类别的规则
        let rules = if start_empty {
            FxHashMap::default()
        } else {
            Self::warn_correctness(plugins.builtin)
        };

        // ========================================================================================
        // 步骤 6: 处理规则类别配置
        // ========================================================================================
        let mut categories = oxlintrc.categories.clone();

        // 如果不是从空配置开始，默认启用 correctness 类别
        if !start_empty {
            categories.insert(RuleCategory::Correctness, AllowWarnDeny::Warn);
        }

        // ========================================================================================
        // 步骤 7: 创建 LintConfig 对象
        // ========================================================================================
        let config = LintConfig {
            plugins,
            settings: oxlintrc.settings,
            env: oxlintrc.env,
            globals: oxlintrc.globals,
            path: Some(oxlintrc.path),
        };

        // ========================================================================================
        // 步骤 8: 创建构建器实例
        // ========================================================================================
        let mut builder = Self {
            rules,
            external_rules: FxHashMap::default(),
            config,
            categories,
            overrides: oxlintrc.overrides,
            extended_paths,
        };

        // ========================================================================================
        // 步骤 9: 应用配置中的类别过滤器
        // ========================================================================================
        for filter in oxlintrc.categories.filters() {
            builder = builder.with_filter(&filter);
        }

        // ========================================================================================
        // 步骤 10: 应用规则覆盖配置
        // ========================================================================================
        {
            let all_rules = builder.get_all_rules();

            oxlintrc
                .rules
                .override_rules(
                    &mut builder.rules,
                    &mut builder.external_rules,
                    &all_rules,
                    external_plugin_store,
                )
                .map_err(ConfigBuilderError::ExternalRuleLookupError)?;
        }

        Ok(builder)
    }

    /// 配置启用的 linter 插件
    ///
    /// 启用插件不会自动启用其中的任何规则。必须在启用插件后自行启用规则（使用 [`with_filters`]）。
    /// 注意：关闭已经启用的插件会导致该插件的所有规则被关闭，传递给这些规则的配置将丢失。
    /// 如果需要重新打开该插件，需要重新添加配置。
    ///
    /// 此方法设置哪些插件被启用和禁用，会覆盖现有的配置。
    /// 如果只是添加/移除某些插件，请使用 [`and_builtin_plugins`]
    ///
    /// [`with_filters`]: ConfigStoreBuilder::with_filters
    /// [`and_builtin_plugins`]: ConfigStoreBuilder::and_builtin_plugins
    #[inline]
    pub fn with_builtin_plugins(mut self, plugins: BuiltinLintPlugins) -> Self {
        self.config.plugins.builtin = plugins;
        self
    }

    /// 设置规则类别配置
    pub fn with_categories(mut self, categories: OxlintCategories) -> Self {
        self.categories = categories;
        self
    }

    /// 启用或禁用一组插件，不影响其他插件
    ///
    /// 详情参见 [`ConfigStoreBuilder::with_builtin_plugins`] 了解插件配置如何影响规则
    #[inline]
    pub fn and_builtin_plugins(mut self, plugins: BuiltinLintPlugins, enabled: bool) -> Self {
        self.config.plugins.builtin.set(plugins, enabled);
        self
    }

    #[inline]
    pub fn plugins(&self) -> &LintPlugins {
        &self.config.plugins
    }

    #[cfg(test)]
    pub(crate) fn with_rule(mut self, rule: RuleEnum, severity: AllowWarnDeny) -> Self {
        self.rules.insert(rule, severity);
        self
    }

    /// 向当前覆盖列表的末尾追加覆盖配置
    pub fn with_overrides<O: IntoIterator<Item = OxlintOverride>>(mut self, overrides: O) -> Self {
        self.overrides.extend(overrides);
        self
    }

    /// 批量应用过滤器配置
    pub fn with_filters<'a, I: IntoIterator<Item = &'a LintFilter>>(mut self, filters: I) -> Self {
        for filter in filters {
            self = self.with_filter(filter);
        }
        self
    }

    /// 应用单个过滤器配置
    ///
    /// 根据过滤器的类型和严重程度（allow/warn/deny）来修改规则配置
    ///
    /// # 工作原理
    ///
    /// 过滤器可以是以下类型：
    /// - **Category**：按规则类别（如 correctness, suspicious）
    /// - **Rule**：指定插件和规则名（如 "eslint/no-console"）
    /// - **Generic**：仅规则名（如 "no-console"）
    /// - **All**：所有规则
    ///
    /// # 行为说明
    ///
    /// - **Warn/Deny**：启用规则并设置严重程度（使用 `upsert_where`）
    /// - **Allow**：禁用规则（从 `rules` map 中移除）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 警告所有 correctness 类别的规则
    /// builder.with_filter(&LintFilter::warn(RuleCategory::Correctness));
    ///
    /// // 禁止某个特定规则
    /// builder.with_filter(&LintFilter::deny("no-console"));
    ///
    /// // 禁用所有规则
    /// builder.with_filter(&LintFilter::allow_all());
    /// ```
    pub fn with_filter(mut self, filter: &LintFilter) -> Self {
        let (severity, filter) = filter.into();

        match severity {
            // 启用规则（warn 或 deny）
            AllowWarnDeny::Deny | AllowWarnDeny::Warn => match filter {
                LintFilterKind::Category(category) => {
                    // 按类别筛选：匹配特定类别的所有规则
                    self.upsert_where(severity, |r| r.category() == *category);
                }
                LintFilterKind::Rule(plugin, rule) => {
                    // 指定插件和规则名：精确匹配
                    self.upsert_where(severity, |r| r.plugin_name() == plugin && r.name() == rule);
                }
                LintFilterKind::Generic(name) => {
                    // 仅规则名：匹配所有插件中同名的规则
                    self.upsert_where(severity, |r| r.name() == name);
                }
                LintFilterKind::All => {
                    // 所有规则：排除 nursery 类别的实验性规则
                    self.upsert_where(severity, |r| r.category() != RuleCategory::Nursery);
                }
            },
            // 禁用规则（allow）
            AllowWarnDeny::Allow => match filter {
                LintFilterKind::Category(category) => {
                    // 禁用特定类别的所有规则
                    self.rules.retain(|rule, _| rule.category() != *category);
                }
                LintFilterKind::Rule(plugin, rule) => {
                    // 禁用指定的规则
                    self.rules.retain(|r, _| r.plugin_name() != plugin || r.name() != rule);
                }
                LintFilterKind::Generic(name) => {
                    // 禁用所有同名规则
                    self.rules.retain(|rule, _| rule.name() != name);
                }
                LintFilterKind::All => {
                    // 禁用所有规则
                    self.rules.clear();
                }
            },
        }

        self
    }

    /// 获取所有可用的规则列表
    fn get_all_rules(&self) -> Vec<RuleEnum> {
        self.get_all_rules_for_plugins(None)
    }

    /// 获取指定插件的所有规则
    ///
    /// # 参数
    /// - `override_plugins`: 可选的插件覆盖配置，如果提供则使用该配置，否则使用默认插件配置
    fn get_all_rules_for_plugins(&self, override_plugins: Option<&LintPlugins>) -> Vec<RuleEnum> {
        let mut builtin_plugins = if let Some(override_plugins) = override_plugins {
            self.config.plugins.builtin | override_plugins.builtin
        } else {
            self.config.plugins.builtin
        };

        if builtin_plugins.is_all() {
            RULES.clone()
        } else {
            // we need to include some jest rules when vitest is enabled, see [`VITEST_COMPATIBLE_JEST_RULES`]
            if builtin_plugins.contains(BuiltinLintPlugins::VITEST) {
                builtin_plugins = builtin_plugins.union(BuiltinLintPlugins::JEST);
            }

            RULES
                .iter()
                .filter(|rule| {
                    builtin_plugins.contains(BuiltinLintPlugins::from(rule.plugin_name()))
                })
                .cloned()
                .collect()
        }
    }

    /// 根据条件批量更新或插入规则配置
    ///
    /// # 工作原理
    ///
    /// 1. 获取所有可用规则（基于当前启用的插件）
    /// 2. 使用 `query` 闭包筛选需要配置的规则
    /// 3. 对于每个匹配的规则：
    ///    - 如果规则已存在：更新其严重程度
    ///    - 如果规则不存在：插入新规则和严重程度
    ///
    /// # 参数
    ///
    /// - `severity`: 要设置的严重程度
    /// - `query`: 一个闭包，用于筛选需要配置的规则
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 将所有 correctness 规则的严重程度设置为 Deny
    /// builder.upsert_where(AllowWarnDeny::Deny, |r| r.category() == RuleCategory::Correctness);
    /// ```
    fn upsert_where<F>(&mut self, severity: AllowWarnDeny, query: F)
    where
        F: Fn(&&RuleEnum) -> bool,
    {
        // 获取所有可用规则（基于当前插件配置）
        let all_rules = self.get_all_rules();

        // 使用查询条件筛选需要配置的规则
        // 注意：我们可能应该警告用户配置了不存在的规则
        let rules_to_configure = all_rules.iter().filter(query);

        for rule in rules_to_configure {
            // 如果规则已存在，更新其严重程度
            // 否则，插入新规则
            if let Some(existing_rule) = self.rules.get_mut(rule) {
                *existing_rule = severity;
            } else {
                self.rules.insert(rule.clone(), severity);
            }
        }
    }

    /// 从构建器的当前状态构建 [`Config`]
    ///
    /// 这是构建流程的最后一步，将所有配置合并并创建最终的 `Config` 对象
    ///
    /// # 工作流程
    ///
    /// 1. **处理插件兼容性**：Vitest 插件会隐式启用 Jest 插件
    /// 2. **解析覆盖配置**：处理基于文件路径的规则覆盖
    /// 3. **过滤规则**：移除已禁用插件的规则
    /// 4. **排序规则**：按规则 ID 排序，确保执行顺序一致
    /// 5. **创建配置**：生成最终的 `Config` 对象
    ///
    /// # 注意事项
    ///
    /// - 构建器在此方法中被消费（`self` 的所有权被转移）
    /// - 规则会被排序以保持稳定的执行顺序
    /// - 未启用的插件规则会被自动过滤掉
    ///
    /// # 错误
    ///
    /// 如果有无法匹配的规则或外部规则查找失败，返回相应的错误
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let config = ConfigStoreBuilder::default()
    ///     .with_filter(&LintFilter::warn(RuleCategory::Correctness))
    ///     .build(&external_plugin_store)
    ///     .unwrap();
    /// ```
    pub fn build(
        mut self,
        external_plugin_store: &ExternalPluginStore,
    ) -> Result<Config, ConfigBuilderError> {
        // 获取当前启用的插件
        // 注意：如果插件在配置后被禁用，相关的规则需要在这里被过滤掉
        let mut plugins = self.plugins().builtin;

        // ====================================================================
        // 步骤 1: 处理插件兼容性
        // ====================================================================
        // Vitest 插件需要 Jest 插件支持，自动启用 Jest
        if plugins.contains(BuiltinLintPlugins::VITEST) {
            plugins = plugins.union(BuiltinLintPlugins::JEST);
        }

        // ====================================================================
        // 步骤 2: 解析覆盖配置
        // ====================================================================
        let overrides = std::mem::take(&mut self.overrides);
        let resolved_overrides = self
            .resolve_overrides(overrides, external_plugin_store)
            .map_err(ConfigBuilderError::ExternalRuleLookupError)?;

        // ====================================================================
        // 步骤 3: 过滤和排序内置规则
        // ====================================================================
        // 只保留已启用插件的规则
        let mut rules: Vec<_> = self
            .rules
            .into_iter()
            .filter(|(r, _)| plugins.contains(r.plugin_name().into()))
            .collect();
        // 按规则 ID 排序，确保执行顺序稳定
        rules.sort_unstable_by_key(|(r, _)| r.id());

        // ====================================================================
        // 步骤 4: 排序外部规则
        // ====================================================================
        let mut external_rules: Vec<_> = self.external_rules.into_iter().collect();
        external_rules.sort_unstable_by_key(|(r, _)| *r);

        // ====================================================================
        // 步骤 5: 创建最终配置
        // ====================================================================
        Ok(Config::new(rules, external_rules, self.categories, self.config, resolved_overrides))
    }

    fn resolve_overrides(
        &self,
        overrides: OxlintOverrides,
        external_plugin_store: &ExternalPluginStore,
    ) -> Result<ResolvedOxlintOverrides, ExternalRuleLookupError> {
        let resolved = overrides
            .into_iter()
            .map(|override_config| {
                let mut builtin_rules = Vec::new();
                let mut external_rules = Vec::new();
                let mut rules_map = FxHashMap::default();
                let mut external_rules_map = FxHashMap::default();

                let all_rules = self.get_all_rules_for_plugins(override_config.plugins.as_ref());

                // Resolve rules for this override
                override_config.rules.override_rules(
                    &mut rules_map,
                    &mut external_rules_map,
                    &all_rules,
                    external_plugin_store,
                )?;

                // Convert to vectors
                builtin_rules.extend(rules_map.into_iter());
                external_rules.extend(external_rules_map.into_iter());

                Ok(ResolvedOxlintOverride {
                    files: override_config.files,
                    env: override_config.env,
                    globals: override_config.globals,
                    plugins: override_config.plugins,
                    rules: ResolvedOxlintOverrideRules { builtin_rules, external_rules },
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ResolvedOxlintOverrides::new(resolved))
    }

    /// 为给定插件集中的所有 correctness 规则设置为 warn 级别
    ///
    /// 这是默认配置，用于确保重要的正确性规则默认被启用
    ///
    /// # 工作原理
    ///
    /// 1. 处理 Vitest 插件的特殊需求（需要 Jest 支持）
    /// 2. 从全局 `RULES` 列表中筛选出 correctness 类别的规则
    /// 3. 只包含已启用插件的规则
    /// 4. 将所有规则设置为 `Warn` 级别
    ///
    /// # 注意事项
    ///
    /// - 这确保了 correctness 规则默认被启用
    /// - 用户可以通过配置文件或过滤器禁用这些规则
    /// - ESLint 的 correctness 规则无法被完全禁用（这是有意为之）
    fn warn_correctness(mut plugins: BuiltinLintPlugins) -> FxHashMap<RuleEnum, AllowWarnDeny> {
        // Vitest 插件需要 Jest 插件支持
        if plugins.contains(BuiltinLintPlugins::VITEST) {
            plugins = plugins.union(BuiltinLintPlugins::JEST);
        }

        // 从全局规则列表中筛选并配置 correctness 规则
        RULES
            .iter()
            .filter(|rule| {
                // 只包含 correctness 类别的规则
                // 并且该规则所属的插件已被启用
                rule.category() == RuleCategory::Correctness
                    && plugins.contains(BuiltinLintPlugins::from(rule.plugin_name()))
            })
            .map(|rule| (rule.clone(), AllowWarnDeny::Warn))
            .collect()
    }

    /// # Panics
    /// This function will panic if the `oxlintrc` is not valid JSON.
    pub fn resolve_final_config_file(&self, oxlintrc: Oxlintrc) -> String {
        let mut oxlintrc = oxlintrc;
        let previous_rules = std::mem::take(&mut oxlintrc.rules);

        let rule_name_to_rule = previous_rules
            .rules
            .into_iter()
            .map(|r| (get_name(&r.plugin_name, &r.rule_name), r))
            .collect::<rustc_hash::FxHashMap<_, _>>();

        let new_rules = self
            .rules
            .iter()
            .sorted_by_key(|(r, _)| (r.plugin_name(), r.name()))
            .map(|(r, severity)| ESLintRule {
                plugin_name: r.plugin_name().to_string(),
                rule_name: r.name().to_string(),
                severity: *severity,
                config: rule_name_to_rule
                    .get(&get_name(r.plugin_name(), r.name()))
                    .and_then(|r| r.config.clone()),
            })
            .collect();

        oxlintrc.rules = OxlintRules::new(new_rules);
        serde_json::to_string_pretty(&oxlintrc).unwrap()
    }

    #[cfg(not(all(feature = "oxlint2", not(feature = "disable_oxlint2"))))]
    #[expect(unused_variables, clippy::needless_pass_by_ref_mut)]
    fn load_external_plugin(
        oxlintrc_dir_path: &Path,
        plugin_specifier: &str,
        external_linter: &ExternalLinter,
        resolver: &Resolver,
        external_plugin_store: &mut ExternalPluginStore,
    ) -> Result<(), ConfigBuilderError> {
        unreachable!()
    }

    #[cfg(all(feature = "oxlint2", not(feature = "disable_oxlint2")))]
    fn load_external_plugin(
        oxlintrc_dir_path: &Path,
        plugin_specifier: &str,
        external_linter: &ExternalLinter,
        resolver: &Resolver,
        external_plugin_store: &mut ExternalPluginStore,
    ) -> Result<(), ConfigBuilderError> {
        use crate::PluginLoadResult;

        let resolved = resolver.resolve(oxlintrc_dir_path, plugin_specifier).map_err(|e| {
            ConfigBuilderError::PluginLoadFailed {
                plugin_specifier: plugin_specifier.to_string(),
                error: e.to_string(),
            }
        })?;
        // TODO: We should support paths which are not valid UTF-8. How?
        let plugin_path = resolved.full_path().to_str().unwrap().to_string();

        if external_plugin_store.is_plugin_registered(&plugin_path) {
            return Ok(());
        }

        let result = {
            let plugin_path = plugin_path.clone();
            tokio::task::block_in_place(move || {
                tokio::runtime::Handle::current()
                    .block_on((external_linter.load_plugin)(plugin_path))
            })
            .map_err(|e| ConfigBuilderError::PluginLoadFailed {
                plugin_specifier: plugin_specifier.to_string(),
                error: e.to_string(),
            })
        }?;

        match result {
            PluginLoadResult::Success { name, offset, rule_names } => {
                external_plugin_store.register_plugin(plugin_path, name, offset, rule_names);
                Ok(())
            }
            PluginLoadResult::Failure(e) => Err(ConfigBuilderError::PluginLoadFailed {
                plugin_specifier: plugin_specifier.to_string(),
                error: e,
            }),
        }
    }
}

fn get_name(plugin_name: &str, rule_name: &str) -> CompactStr {
    if plugin_name == "eslint" {
        CompactStr::from(rule_name)
    } else {
        format_compact_str!("{plugin_name}/{rule_name}")
    }
}

impl Debug for ConfigStoreBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfigStoreBuilder")
            .field("rules", &self.rules)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

/// 从 [`Oxlintrc`] 构建 [`Config`] 时可能发生的错误
///
/// # 错误类型
///
/// - **UnknownRules**: 配置文件中引用了不存在的规则
/// - **InvalidConfigFile**: 配置文件格式错误或无法解析
/// - **PluginLoadFailed**: 外部插件加载失败（通常是路径或依赖问题）
/// - **ExternalRuleLookupError**: 外部规则查找失败
/// - **NoExternalLinterConfigured**: 需要外部 linter 但未配置
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ConfigBuilderError {
    /// 存在无法匹配到任何已知插件/规则的未知规则
    ///
    /// 当配置文件中指定了不存在的规则时会发生此错误
    UnknownRules { rules: Vec<ESLintRule> },

    /// 引用的配置文件由于某种原因无效
    ///
    /// 常见原因：
    /// - 文件不存在
    /// - JSON/YAML 格式错误
    /// - 文件权限问题
    InvalidConfigFile { file: String, reason: String },

    /// 外部插件加载失败
    ///
    /// 加载外部插件（如 ESLint 插件）时出错
    PluginLoadFailed { plugin_specifier: String, error: String },

    /// 外部规则查找错误
    ///
    /// 在外部插件中查找规则定义时出错
    ExternalRuleLookupError(ExternalRuleLookupError),

    /// 未配置外部 linter
    ///
    /// 配置文件需要外部 linter 支持，但当前环境未配置外部 linter
    NoExternalLinterConfigured,
}

impl Display for ConfigBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigBuilderError::UnknownRules { rules } => {
                f.write_str("unknown rules: ")?;
                for (i, rule) in rules.iter().enumerate() {
                    if i == 0 {
                        Display::fmt(&rule.full_name(), f)?;
                    } else {
                        write!(f, ", {}", rule.full_name())?;
                    }
                }
                Ok(())
            }
            ConfigBuilderError::InvalidConfigFile { file, reason } => {
                write!(f, "invalid config file {file}: {reason}")
            }
            ConfigBuilderError::PluginLoadFailed { plugin_specifier, error } => {
                write!(f, "Failed to load external plugin: {plugin_specifier}\n  {error}")?;
                Ok(())
            }
            ConfigBuilderError::NoExternalLinterConfigured => {
                f.write_str("Failed to load external plugin because no external linter was configured. This means the Oxlint binary was executed directly rather than via napi bindings.")?;
                Ok(())
            }
            ConfigBuilderError::ExternalRuleLookupError(e) => std::fmt::Display::fmt(&e, f),
        }
    }
}

impl std::error::Error for ConfigBuilderError {}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use rustc_hash::FxHashSet;

    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = ConfigStoreBuilder::default();
        assert_eq!(*builder.plugins(), LintPlugins::default());

        // populated with all correctness-level ESLint rules at a "warn" severity
        assert!(!builder.rules.is_empty());
        for (rule, severity) in &builder.rules {
            assert_eq!(rule.category(), RuleCategory::Correctness);
            assert_eq!(*severity, AllowWarnDeny::Warn);
            let plugin = rule.plugin_name();
            let name = rule.name();
            assert!(
                builder.plugins().builtin.contains(plugin.into()),
                "{plugin}/{name} is in the default rule set but its plugin is not enabled"
            );
        }
    }

    #[test]
    fn test_builder_empty() {
        let builder = ConfigStoreBuilder::empty();
        assert_eq!(*builder.plugins(), LintPlugins::default());
        assert!(builder.rules.is_empty());
    }

    #[test]
    fn test_filter_deny_on_default() {
        let builder = ConfigStoreBuilder::default();
        let initial_rule_count = builder.rules.len();

        let builder = builder.with_filter(&LintFilter::deny(RuleCategory::Correctness));
        let rule_count_after_deny = builder.rules.len();

        // By default, all correctness rules are set to warn. the above filter should only
        // re-configure those rules, and not add/remove any others.
        assert!(!builder.rules.is_empty());
        assert_eq!(initial_rule_count, rule_count_after_deny);

        for (rule, severity) in &builder.rules {
            assert_eq!(rule.category(), RuleCategory::Correctness);
            assert_eq!(*severity, AllowWarnDeny::Deny);

            let plugin = rule.plugin_name();
            let name = rule.name();
            assert!(
                builder.plugins().builtin.contains(plugin.into()),
                "{plugin}/{name} is in the default rule set but its plugin is not enabled"
            );
        }
    }

    // change a rule already set to "warn" to "deny"
    #[test]
    fn test_filter_deny_single_enabled_rule_on_default() {
        for filter_string in ["no-const-assign", "eslint/no-const-assign"] {
            let builder = ConfigStoreBuilder::default();
            let initial_rule_count = builder.rules.len();

            let builder =
                builder.with_filter(&LintFilter::new(AllowWarnDeny::Deny, filter_string).unwrap());
            let rule_count_after_deny = builder.rules.len();
            assert_eq!(
                initial_rule_count, rule_count_after_deny,
                "Changing a single rule from warn to deny should not add a new one, just modify what's already there."
            );

            let (_, severity) = builder
                .rules
                .iter()
                .find(|(r, _)| r.plugin_name() == "eslint" && r.name() == "no-const-assign")
                .expect("Could not find eslint/no-const-assign after configuring it to 'deny'");
            assert_eq!(*severity, AllowWarnDeny::Deny);
        }
    }
    // turn on a rule that isn't configured yet and set it to "warn"
    // note that this is an eslint rule, a plugin that's already turned on.
    #[test]
    fn test_filter_warn_single_disabled_rule_on_default() {
        for filter_string in ["no-console", "eslint/no-console"] {
            let filter = LintFilter::new(AllowWarnDeny::Warn, filter_string).unwrap();
            let builder = ConfigStoreBuilder::default();
            // sanity check: not already turned on
            assert!(!builder.rules.iter().any(|(r, _)| r.name() == "no-console"));
            let builder = builder.with_filter(&filter);
            let (_, severity) = builder
                .rules
                .iter()
                .find(|(r, _)| r.plugin_name() == "eslint" && r.name() == "no-console")
                .expect("Could not find eslint/no-console after configuring it to 'warn'");

            assert_eq!(*severity, AllowWarnDeny::Warn);
        }
    }

    #[test]
    fn test_filter_allow_all_then_warn() {
        let builder = ConfigStoreBuilder::default()
            .with_filter(&LintFilter::new(AllowWarnDeny::Allow, "all").unwrap());
        assert!(builder.rules.is_empty(), "Allowing all rules should empty out the rules list");

        let builder = builder.with_filter(&LintFilter::warn(RuleCategory::Correctness));
        assert!(
            !builder.rules.is_empty(),
            "warning on categories after allowing all rules should populate the rules set"
        );
        for (rule, severity) in &builder.rules {
            let plugin = rule.plugin_name();
            let name = rule.name();
            assert_eq!(
                *severity,
                AllowWarnDeny::Warn,
                "{plugin}/{name} should have a warning severity"
            );
            assert_eq!(
                rule.category(),
                RuleCategory::Correctness,
                "{plugin}/{name} should not be enabled b/c we only enabled correctness rules"
            );
        }
    }

    #[test]
    fn test_rules_after_plugin_added() {
        let builder = ConfigStoreBuilder::default();
        let initial_rule_count = builder.rules.len();

        let builder = builder.and_builtin_plugins(BuiltinLintPlugins::IMPORT, true);
        assert_eq!(
            initial_rule_count,
            builder.rules.len(),
            "Enabling a plugin should not add any rules, since we don't know which categories to turn on."
        );
    }

    #[test]
    fn test_rules_after_plugin_removal() {
        // sanity check: the plugin we're removing is, in fact, enabled by default.
        assert!(LintPlugins::default().builtin.contains(BuiltinLintPlugins::TYPESCRIPT));

        let mut desired_plugins = LintPlugins::default();
        desired_plugins.builtin.set(BuiltinLintPlugins::TYPESCRIPT, false);

        let external_plugin_store = ExternalPluginStore::default();
        let linter = ConfigStoreBuilder::default()
            .with_builtin_plugins(desired_plugins.builtin)
            .build(&external_plugin_store)
            .unwrap();
        for (rule, _) in linter.base.rules.iter() {
            let name = rule.name();
            let plugin = rule.plugin_name();
            assert_ne!(
                BuiltinLintPlugins::from(plugin),
                BuiltinLintPlugins::TYPESCRIPT,
                "{plugin}/{name} is in the rules list after typescript plugin has been disabled"
            );
        }
    }

    #[test]
    fn test_plugin_configuration() {
        let builder = ConfigStoreBuilder::default();
        let initial_plugins = builder.plugins().clone();

        // ==========================================================================================
        // Test ConfigStoreBuilder::and_plugins, which deltas the plugin list instead of overriding it
        // ==========================================================================================

        // Enable eslint plugin. Since it's already enabled, this does nothing.

        assert!(initial_plugins.builtin.contains(BuiltinLintPlugins::ESLINT)); // sanity check that eslint is
        // enabled
        let builder = builder.and_builtin_plugins(BuiltinLintPlugins::ESLINT, true);
        assert_eq!(initial_plugins, *builder.plugins());

        // Disable import plugin. Since it's not already enabled, this is also a no-op.
        assert!(!builder.plugins().builtin.contains(BuiltinLintPlugins::IMPORT)); // sanity check that it's not
        // already enabled
        let builder = builder.and_builtin_plugins(BuiltinLintPlugins::IMPORT, false);
        assert_eq!(initial_plugins, *builder.plugins());

        // Enable import plugin. Since it's not already enabled, this turns it on.
        let builder = builder.and_builtin_plugins(BuiltinLintPlugins::IMPORT, true);
        assert_eq!(
            BuiltinLintPlugins::default().union(BuiltinLintPlugins::IMPORT),
            builder.plugins().builtin
        );
        assert_ne!(initial_plugins, *builder.plugins());

        // Turn import back off, resetting plugins to the initial state
        let builder = builder.and_builtin_plugins(BuiltinLintPlugins::IMPORT, false);
        assert_eq!(initial_plugins, *builder.plugins());

        // ==========================================================================================
        // Test ConfigStoreBuilder::with_plugins, which _does_ override plugins
        // ==========================================================================================

        let builder = builder.with_builtin_plugins(BuiltinLintPlugins::ESLINT);
        assert_eq!(BuiltinLintPlugins::ESLINT, builder.plugins().builtin);

        let expected_plugins = BuiltinLintPlugins::ESLINT
            .union(BuiltinLintPlugins::TYPESCRIPT)
            .union(BuiltinLintPlugins::NEXTJS);
        let builder = builder.with_builtin_plugins(expected_plugins);
        assert_eq!(expected_plugins, builder.plugins().builtin);
    }

    #[test]
    fn test_categories() {
        let oxlintrc: Oxlintrc = serde_json::from_str(
            r#"
        {
            "categories": {
                "correctness": "warn",
                "suspicious": "deny"
            },
            "rules": {
                "no-const-assign": "error"
            }
        }
        "#,
        )
        .unwrap();
        let builder = {
            let mut external_plugin_store = ExternalPluginStore::default();
            ConfigStoreBuilder::from_oxlintrc(false, oxlintrc, None, &mut external_plugin_store)
                .unwrap()
        };
        for (rule, severity) in &builder.rules {
            let name = rule.name();
            let plugin = rule.plugin_name();
            let category = rule.category();
            match category {
                RuleCategory::Correctness => {
                    if name == "no-const-assign" {
                        assert_eq!(
                            *severity,
                            AllowWarnDeny::Deny,
                            "no-const-assign should be denied",
                        );
                    } else {
                        assert_eq!(
                            *severity,
                            AllowWarnDeny::Warn,
                            "{plugin}/{name} should be a warning"
                        );
                    }
                }
                RuleCategory::Suspicious => {
                    assert_eq!(*severity, AllowWarnDeny::Deny, "{plugin}/{name} should be denied");
                }
                invalid => {
                    panic!("Found rule {plugin}/{name} with an unexpected category {invalid:?}");
                }
            }
        }
    }

    #[test]
    fn test_extends_rules_single() {
        let base_config = config_store_from_path("fixtures/extends_config/rules_config.json");
        let derived_config = config_store_from_str(
            r#"
        {
            "extends": [
                "fixtures/extends_config/rules_config.json"
            ]
        }
        "#,
        );

        assert_eq!(base_config.rules(), derived_config.rules());

        let update_rules_config = config_store_from_str(
            r#"
        {
            "extends": [
                "fixtures/extends_config/rules_config.json"
            ],
            "rules": {
                "no-debugger": "warn",
                "no-console": "warn",
                "unicorn/no-null": "off",
                "typescript/prefer-as-const": "warn"
            }
        }
        "#,
        );

        assert!(
            update_rules_config
                .rules()
                .iter()
                .any(|(r, severity)| r.name() == "no-debugger" && *severity == AllowWarnDeny::Warn)
        );
        assert!(
            update_rules_config
                .rules()
                .iter()
                .any(|(r, severity)| r.name() == "no-console" && *severity == AllowWarnDeny::Warn)
        );
        assert!(
            !update_rules_config
                .rules()
                .iter()
                .any(|(r, severity)| r.name() == "no-null" && *severity == AllowWarnDeny::Allow)
        );
        assert!(
            update_rules_config
                .rules()
                .iter()
                .any(|(r, severity)| r.name() == "prefer-as-const"
                    && *severity == AllowWarnDeny::Warn)
        );
    }

    #[test]
    fn test_extends_rules_multiple() {
        let warn_all = config_store_from_str(
            r#"
        {
            "extends": [
                "fixtures/extends_config/rules_multiple/allow_all.json",
                "fixtures/extends_config/rules_multiple/deny_all.json",
                "fixtures/extends_config/rules_multiple/warn_all.json"
            ]
        }
        "#,
        );
        assert!(warn_all.rules().iter().all(|(_, severity)| *severity == AllowWarnDeny::Warn));

        let deny_all = config_store_from_str(
            r#"
        {
            "extends": [
                "fixtures/extends_config/rules_multiple/allow_all.json",
                "fixtures/extends_config/rules_multiple/warn_all.json",
                "fixtures/extends_config/rules_multiple/deny_all.json"
            ]
        }
        "#,
        );
        assert!(deny_all.rules().iter().all(|(_, severity)| *severity == AllowWarnDeny::Deny));

        let allow_all = config_store_from_str(
            r#"
        {
            "extends": [
                "fixtures/extends_config/rules_multiple/warn_all.json",
                "fixtures/extends_config/rules_multiple/deny_all.json",
                "fixtures/extends_config/rules_multiple/allow_all.json"
            ]
        }
        "#,
        );
        assert!(allow_all.rules().iter().all(|(_, severity)| *severity == AllowWarnDeny::Allow));
        assert_eq!(allow_all.number_of_rules(), 0);

        let allow_and_override_config = config_store_from_str(
            r#"
        {
            "extends": [
                "fixtures/extends_config/rules_multiple/deny_all.json",
                "fixtures/extends_config/rules_multiple/allow_all.json"
            ],
            "rules": {
                "no-var": "warn",
                "oxc/approx-constant": "error",
                "unicorn/no-null": "error"
            }
        }
        "#,
        );
        assert!(
            allow_and_override_config
                .rules()
                .iter()
                .any(|(r, severity)| r.name() == "no-var" && *severity == AllowWarnDeny::Warn)
        );
        assert!(
            allow_and_override_config
                .rules()
                .iter()
                .any(|(r, severity)| r.name() == "approx-constant"
                    && *severity == AllowWarnDeny::Deny)
        );
        assert!(
            allow_and_override_config
                .rules()
                .iter()
                .any(|(r, severity)| r.name() == "no-null" && *severity == AllowWarnDeny::Deny)
        );
    }

    #[test]
    fn test_extends_invalid() {
        let invalid_config = {
            let mut external_plugin_store = ExternalPluginStore::default();
            ConfigStoreBuilder::from_oxlintrc(
                true,
                Oxlintrc::from_file(&PathBuf::from(
                    "fixtures/extends_config/extends_invalid_config.json",
                ))
                .unwrap(),
                None,
                &mut external_plugin_store,
            )
        };
        let err = invalid_config.unwrap_err();
        assert!(matches!(err, ConfigBuilderError::InvalidConfigFile { .. }));
        if let ConfigBuilderError::InvalidConfigFile { file, reason } = err {
            assert!(file.ends_with("invalid_config.json"));
            assert!(reason.contains("Failed to parse"));
        }
    }

    #[test]
    fn test_extends_plugins() {
        // Test 1: Default plugins when none are specified
        let default_config = config_store_from_str(
            r#"
            {
                "rules": {}
            }
            "#,
        );
        // Check that default plugins are correctly set
        assert_eq!(*default_config.plugins(), LintPlugins::default());

        // Test 2: Parent config with explicitly specified plugins
        let parent_config = config_store_from_str(
            r#"
            {
                "plugins": ["react", "typescript"]
            }
            "#,
        );
        assert_eq!(
            *parent_config.plugins(),
            LintPlugins::new(
                BuiltinLintPlugins::REACT | BuiltinLintPlugins::TYPESCRIPT,
                FxHashSet::default()
            )
        );

        // Test 3: Child config that extends parent without specifying plugins
        // Should inherit parent's plugins
        let child_no_plugins_config =
            config_store_from_path("fixtures/extends_config/plugins/child_no_plugins.json");
        assert_eq!(
            *child_no_plugins_config.plugins(),
            LintPlugins::new(
                BuiltinLintPlugins::REACT | BuiltinLintPlugins::TYPESCRIPT,
                FxHashSet::default()
            )
        );

        // Test 4: Child config that extends parent and specifies additional plugins
        // Should have parent's plugins plus its own
        let child_with_plugins_config =
            config_store_from_path("fixtures/extends_config/plugins/child_with_plugins.json");
        assert_eq!(
            *child_with_plugins_config.plugins(),
            LintPlugins::new(
                BuiltinLintPlugins::REACT
                    | BuiltinLintPlugins::TYPESCRIPT
                    | BuiltinLintPlugins::JEST,
                FxHashSet::default()
            )
        );

        // Test 5: Empty plugins array should result in empty plugins
        let empty_plugins_config = config_store_from_str(
            r#"
            {
                "plugins": []
            }
            "#,
        );
        assert_eq!(
            *empty_plugins_config.plugins(),
            LintPlugins::new(BuiltinLintPlugins::empty(), FxHashSet::default())
        );

        // Test 6: Extending multiple config files with plugins
        let config = config_store_from_str(
            r#"
            {
                "extends": [
                    "fixtures/extends_config/plugins/jest.json",
                    "fixtures/extends_config/plugins/react.json"
                ]
            }
            "#,
        );
        assert!(config.plugins().builtin.contains(BuiltinLintPlugins::JEST));
        assert!(config.plugins().builtin.contains(BuiltinLintPlugins::REACT));

        // Test 7: Adding more plugins to extended configs
        let config = config_store_from_str(
            r#"
            {
                "extends": [
                    "fixtures/extends_config/plugins/jest.json",
                    "fixtures/extends_config/plugins/react.json"
                ],
                "plugins": ["typescript"]
            }
            "#,
        );
        assert_eq!(
            *config.plugins(),
            LintPlugins::new(
                BuiltinLintPlugins::JEST
                    | BuiltinLintPlugins::REACT
                    | BuiltinLintPlugins::TYPESCRIPT,
                FxHashSet::default()
            )
        );

        // Test 8: Extending a config with a plugin is the same as adding it directly
        let plugin_config = config_store_from_str(r#"{ "plugins": ["jest", "react"] }"#);
        let extends_plugin_config = config_store_from_str(
            r#"
            {
                "extends": [
                    "fixtures/extends_config/plugins/jest.json",
                    "fixtures/extends_config/plugins/react.json"
                ]
            }
            "#,
        );
        assert_eq!(
            plugin_config.plugins(),
            extends_plugin_config.plugins(),
            "Extending a config with a plugin is the same as adding it directly"
        );
    }

    #[test]
    fn test_not_extends_named_configs() {
        // For now, test that extending named configs is just ignored
        let config = config_store_from_str(
            r#"
        {
            "extends": [
                "next/core-web-vitals",
                "eslint:recommended",
                "plugin:@typescript-eslint/strict-type-checked",
                "prettier",
                "plugin:unicorn/recommended"
            ]
        }
        "#,
        );
        assert_eq!(*config.plugins(), LintPlugins::default());
        assert!(config.rules().is_empty());
    }

    fn config_store_from_path(path: &str) -> Config {
        let mut external_plugin_store = ExternalPluginStore::default();
        ConfigStoreBuilder::from_oxlintrc(
            true,
            Oxlintrc::from_file(&PathBuf::from(path)).unwrap(),
            None,
            &mut external_plugin_store,
        )
        .unwrap()
        .build(&external_plugin_store)
        .unwrap()
    }

    fn config_store_from_str(s: &str) -> Config {
        let mut external_plugin_store = ExternalPluginStore::default();
        ConfigStoreBuilder::from_oxlintrc(
            true,
            serde_json::from_str(s).unwrap(),
            None,
            &mut external_plugin_store,
        )
        .unwrap()
        .build(&external_plugin_store)
        .unwrap()
    }
}
