# OXC Linter 模块详解

OXC Linter 是 OXC 项目中的代码检查工具，它提供了与 ESLint 兼容的规则系统，但使用 Rust 实现，因此具有更高的性能。本文将详细介绍 OXC Linter 的功能、结构和使用方法。

## 功能特点

OXC Linter 具有以下主要功能特点：

1. **高性能**: 使用 Rust 编写，比 JavaScript 实现的 ESLint 快数十倍
2. **兼容性**: 支持 ESLint 配置文件和规则
3. **并行处理**: 可以并行检查多个文件
4. **自动修复**: 支持自动修复代码中的问题
5. **插件系统**: 支持扩展规则集

## 架构设计

OXC Linter 的架构主要包括以下几个部分：

### 核心组件

1. **Linter**: 主要的 lint 执行器，负责运行规则检查
2. **LintContext**: 为规则提供上下文信息，包括 AST、源代码等
3. **Rule**: 规则定义和实现
4. **Config**: 配置系统，支持从文件加载配置
5. **Fixer**: 自动修复功能的实现

### 规则分类

OXC Linter 的规则按照不同的功能和框架进行分类，包括：

- **eslint**: 基本的 ESLint 规则
- **typescript**: TypeScript 相关规则
- **react**: React 框架相关规则
- **react_perf**: React 性能相关规则
- **jsx_a11y**: JSX 可访问性规则
- **import**: 导入语句相关规则
- **jest**: Jest 测试框架相关规则
- **vitest**: Vitest 测试框架相关规则
- **node**: Node.js 相关规则
- **promise**: Promise 相关规则
- **unicorn**: 其他实用规则
- **nextjs**: Next.js 框架相关规则
- **jsdoc**: JSDoc 文档相关规则
- **oxc**: OXC 特有规则

## 使用方法

### 基本用法

以下是使用 OXC Linter 的基本步骤：

1. 解析源代码生成 AST
2. 创建语义分析器
3. 配置 Linter 选项和规则
4. 运行 Linter 获取诊断信息
5. 处理诊断信息（显示错误或自动修复）

### 示例代码

下面是一个简单的使用示例：

```rust
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_linter::{Linter, LintOptions, ConfigStore};

// 读取源代码
let source_text = "const x = 1; debugger;";
let path = Path::new("file.js");

// 创建内存分配器和解析器
let allocator = Allocator::default();
let source_type = SourceType::from_path(path).unwrap();
let parser_result = Parser::new(&allocator, source_text, source_type).parse();

// 创建语义分析器
let semantic = SemanticBuilder::new().build(&parser_result.program);

// 配置 Linter
let options = LintOptions::default();
let config = ConfigStore::default(); // 也可以从文件加载配置
let linter = Linter::new(options, config);

// 运行 Linter
let diagnostics = linter.run(path, semantic.semantic, parser_result.module_record);

// 处理诊断信息
for diagnostic in diagnostics {
    println!("{}", diagnostic.with_source_code(source_text.to_string()));
}
```

### 自定义规则

OXC Linter 允许开发者创建自定义规则。一个规则通常包括以下部分：

1. **元数据**: 规则的名称、类别、描述等
2. **检测逻辑**: 检测代码中的问题
3. **修复逻辑**: 如何自动修复问题

以下是一个简单的自定义规则示例：

```rust
struct NoDebugger;

impl Rule for NoDebugger {
    fn meta(&self) -> RuleMeta {
        RuleMeta {
            name: "no-debugger",
            category: RuleCategory::Best_Practices,
            description: "禁止使用 debugger 语句",
            ..Default::default()
        }
    }

    fn run(&self, node: &AstNode, ctx: &LintContext) {
        if let AstKind::DebuggerStatement(stmt) = node.kind() {
            ctx.diagnostic(
                OxcDiagnostic::error("debugger 语句不允许使用")
                    .with_label(stmt.span)
            );
        }
    }
}
```

## 配置系统

OXC Linter 支持多种配置方式：

1. **配置文件**: 支持 `.eslintrc.js`, `.eslintrc.json` 等格式
2. **内联注释**: 支持通过注释禁用特定行或文件的规则
3. **编程式配置**: 通过代码配置规则和选项

### 配置文件示例

```json
{
  "rules": {
    "no-debugger": "error",
    "no-console": "warn",
    "react/no-unused-state": "error"
  },
  "env": {
    "browser": true,
    "node": true
  }
}
```

### 内联注释

```javascript
// eslint-disable-next-line no-console
console.log("This won't trigger a warning");

/* eslint-disable */
debugger; // 这里不会报错
/* eslint-enable */
```

## 与其他模块的集成

OXC Linter 与 OXC 项目的其他模块紧密集成：

1. **oxc_parser**: 提供 AST 用于检查
2. **oxc_semantic**: 提供语义分析信息
3. **oxc_diagnostics**: 用于生成错误报告
4. **oxc_span**: 用于定位源代码位置

## 性能优化

OXC Linter 采用了多种性能优化技术：

1. **并行处理**: 使用 Rayon 库进行并行文件检查
2. **缓存优化**: 针对大文件和小文件使用不同的遍历策略
3. **内存优化**: 使用 OXC 的内存分配器减少内存使用
4. **规则选择**: 根据文件类型和配置智能选择需要运行的规则

## 总结

OXC Linter 是一个高性能的 JavaScript/TypeScript 代码检查工具，它结合了 ESLint 的易用性和 Rust 的性能优势。通过模块化的设计和与 OXC 其他组件的紧密集成，它提供了快速、可靠的代码质量检查功能，适用于各种规模的项目。
