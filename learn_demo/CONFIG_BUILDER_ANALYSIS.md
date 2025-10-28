# ConfigStoreBuilder 配置构建器分析

## 📄 文件概述

**文件路径**：`crates/oxc_linter/src/config/config_builder.rs`

**作用**：这是 Oxc linter 配置系统的核心组件，负责从配置文件（`.oxlintrc.json`）构建最终的执行配置。

## 🎯 核心功能

### 1. 解析配置文件
- 支持 JSON、YAML 等格式
- 解析规则配置、插件配置、覆盖配置等

### 2. 处理继承（extends）
- 支持 `extends` 字段，从多个配置文件继承设置
- 递归解析继承链
- 子配置可以覆盖父配置

### 3. 加载插件
- 内置插件：eslint, typescript, react, jest 等
- 外部插件：支持加载 ESLint 插件
- 动态解析插件依赖

### 4. 配置规则
- 设置规则的严重程度（allow/warn/deny/error）
- 按类别配置（correctness, suspicious, performance 等）
- 支持精确规则配置

### 5. 处理覆盖
- 基于文件路径的规则覆盖
- 不同文件可以有不同的规则配置

## 🔄 工作流程

```
配置文件 (Oxlintrc)
    ↓
1. 解析 extends 继承链
    ↓
2. 加载外部插件
    ↓
3. 应用规则配置
    ↓
4. 处理类别过滤器
    ↓
5. 处理覆盖配置
    ↓
ConfigStoreBuilder
    ↓
build()
    ↓
Config (最终配置)
```

## 📊 数据结构

### ConfigStoreBuilder

```rust
pub struct ConfigStoreBuilder {
    /// 内置规则的配置映射：规则 -> 严重程度
    pub(super) rules: FxHashMap<RuleEnum, AllowWarnDeny>,

    /// 外部插件规则的配置映射：规则ID -> 严重程度
    pub(super) external_rules: FxHashMap<ExternalRuleId, AllowWarnDeny>,

    /// linter 配置（插件、设置、环境变量等）
    config: LintConfig,

    /// 规则类别的配置
    categories: OxlintCategories,

    /// 基于文件路径的规则覆盖配置
    overrides: OxlintOverrides,

    /// 收集所有被 extends 引用的文件路径
    pub extended_paths: Vec<PathBuf>,
}
```

## 🛠️ 关键方法

### 构造函数

1. **`default()`** - 默认配置
   - 启用默认插件
   - 所有 correctness 规则设置为 warn

2. **`empty()`** - 空配置
   - 不启用任何规则
   - 等价于 `oxlint -A all`

3. **`all()`** - 全部规则
   - 启用所有插件和规则
   - 等价于 `oxlint -W all -W nursery`

4. **`from_oxlintrc()`** - 从配置文件创建
   - 解析配置文件
   - 处理继承和覆盖
   - 加载外部插件

### 配置方法

1. **`with_filter()`** - 应用过滤器
   - 根据类别、规则名等筛选规则
   - 设置规则的严重程度

2. **`with_overrides()`** - 添加覆盖配置
   - 基于文件路径的规则覆盖

3. **`build()`** - 构建最终配置
   - 合并所有配置
   - 过滤未启用的规则
   - 排序规则以保持稳定的执行顺序

## 💡 设计模式

### 建造者模式（Builder Pattern）

```rust
let config = ConfigStoreBuilder::default()
    .with_filter(&LintFilter::warn(RuleCategory::Correctness))
    .with_filter(&LintFilter::deny("no-console"))
    .with_overrides(vec![...])
    .build(&external_plugin_store)
    .unwrap();
```

**优势**：
- 灵活性：可以随意组合配置
- 可读性：代码清晰表达配置意图
- 安全性：编译期检查配置完整性

## 🎨 核心算法

### 1. 继承链解析

```rust
fn resolve_oxlintrc_config(config: Oxlintrc) -> Result<(Oxlintrc, Vec<PathBuf>)> {
    // 从后向前遍历 extends 数组
    for path in extends.iter().rev() {
        // 加载被继承的配置
        let extends_oxlintrc = Oxlintrc::from_file(path)?;

        // 递归解析继承链
        let (extends, extends_paths) = resolve_oxlintrc_config(extends_oxlintrc)?;

        // 合并配置：子配置覆盖父配置
        oxlintrc = oxlintrc.merge(extends);
    }

    Ok((oxlintrc, extended_paths))
}
```

### 2. 规则过滤

```rust
pub fn with_filter(mut self, filter: &LintFilter) -> Self {
    match severity {
        AllowWarnDeny::Warn | AllowWarnDeny::Deny => {
            // 启用规则并设置严重程度
            self.upsert_where(severity, |r| /* 查询条件 */);
        }
        AllowWarnDeny::Allow => {
            // 禁用规则（从 map 中移除）
            self.rules.retain(|rule, _| /* 保留条件 */);
        }
    }
    self
}
```

### 3. 构建最终配置

```rust
pub fn build(mut self, external_plugin_store: &ExternalPluginStore) -> Result<Config> {
    // 1. 处理插件兼容性
    if plugins.contains(BuiltinLintPlugins::VITEST) {
        plugins = plugins.union(BuiltinLintPlugins::JEST);
    }

    // 2. 解析覆盖配置
    let resolved_overrides = self.resolve_overrides(overrides, external_plugin_store)?;

    // 3. 过滤规则
    let mut rules: Vec<_> = self.rules
        .into_iter()
        .filter(|(r, _)| plugins.contains(r.plugin_name().into()))
        .collect();

    // 4. 排序规则
    rules.sort_unstable_by_key(|(r, _)| r.id());

    // 5. 创建最终配置
    Ok(Config::new(rules, external_rules, categories, config, resolved_overrides))
}
```

## 🔍 特殊处理

### Vitest 插件兼容性

Vitest 插件需要 Jest 插件支持，构建器会自动启用 Jest：

```rust
if plugins.contains(BuiltinLintPlugins::VITEST) {
    plugins = plugins.union(BuiltinLintPlugins::JEST);
}
```

### 默认 Correctness 规则

默认配置会启用所有 correctness 类别的规则：

```rust
fn warn_correctness(plugins: BuiltinLintPlugins) -> FxHashMap<RuleEnum, AllowWarnDeny> {
    RULES
        .iter()
        .filter(|rule| {
            rule.category() == RuleCategory::Correctness
                && plugins.contains(BuiltinLintPlugins::from(rule.plugin_name()))
        })
        .map(|rule| (rule.clone(), AllowWarnDeny::Warn))
        .collect()
}
```

## 📝 使用示例

### 从配置文件创建

```rust
let config = ConfigStoreBuilder::from_oxlintrc(
    true,                                    // start_empty
    oxlintrc,                               // 配置文件
    None,                                    // external_linter
    &mut external_plugin_store,
)
.unwrap()
.build(&external_plugin_store)
.unwrap();
```

### 手动构建

```rust
let config = ConfigStoreBuilder::default()
    .with_filter(&LintFilter::warn(RuleCategory::Correctness))
    .with_filter(&LintFilter::deny("no-console"))
    .with_filter(&LintFilter::allow("no-var"))
    .with_overrides(vec![/* ... */])
    .build(&external_plugin_store)
    .unwrap();
```

## 🚨 错误处理

### ConfigBuilderError

```rust
pub enum ConfigBuilderError {
    /// 未知规则
    UnknownRules { rules: Vec<ESLintRule> },

    /// 无效配置文件
    InvalidConfigFile { file: String, reason: String },

    /// 插件加载失败
    PluginLoadFailed { plugin_specifier: String, error: String },

    /// 外部规则查找错误
    ExternalRuleLookupError(ExternalRuleLookupError),

    /// 未配置外部 linter
    NoExternalLinterConfigured,
}
```

## 🎓 设计要点

### 1. 建造者模式
- 允许链式调用
- 配置不可变
- 延迟构建

### 2. 配置合并
- 子配置覆盖父配置
- 顺序重要（后配置覆盖先配置）

### 3. 插件系统
- 内置插件优先级
- 外部插件动态加载
- 插件依赖解析

### 4. 规则管理
- 按类别管理
- 支持精确配置
- 支持通配符配置

## 📚 相关文件

- `config.rs` - Config 结构定义
- `config_store.rs` - 配置存储
- `overrides.rs` - 覆盖配置
- `plugins.rs` - 插件定义
- `rules.rs` - 规则注册

## 💭 总结

`ConfigStoreBuilder` 是 Oxc linter 配置系统的核心，它：

1. ✅ **灵活**：支持多种配置方式（文件、代码、继承）
2. ✅ **强大**：处理复杂的配置场景（继承、覆盖、插件）
3. ✅ **高效**：编译期检查，运行时零开销
4. ✅ **易用**：清晰的 API，优雅的错误处理

通过建造者模式和精心设计的配置合并算法，它实现了强大而灵活的配置系统。

