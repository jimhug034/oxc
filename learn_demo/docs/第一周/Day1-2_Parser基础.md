# 第一周 Day 1-2: Parser 基础

> 理解 Oxc 如何将 JavaScript/TypeScript 代码解析成 AST

## 📖 学习目标

- [ ] 理解什么是 AST（抽象语法树）
- [ ] 能够运行 Parser 查看代码的 AST 结构
- [ ] 认识常见的 AST 节点类型
- [ ] 理解 Parser 的基本工作流程

## 🚀 快速开始

### 1. 运行官方 Parser 示例

```bash
# 进入 Oxc 项目根目录
cd /Users/makeblock/Developer/my-git/oxc

# 创建一个测试文件
echo "const greeting = 'Hello, Oxc!';" > test.js

# 运行 parser 示例
cargo run -p oxc_parser --example parser -- test.js
```

### 2. 运行我们的学习示例

```bash
# 运行基础 parser 示例
cargo run --bin 01_parser_basics
```

## 📚 核心概念

### 什么是 AST？

**抽象语法树 (Abstract Syntax Tree)** 是代码的树状表示形式，它：

- 移除了语法细节（如括号、分号）
- 保留了代码的结构和语义
- 是编译器/工具分析代码的基础

### 示例：从代码到 AST

#### 代码
```javascript
const x = 1 + 2;
```

#### AST 结构（简化）
```
Program
└── VariableDeclaration (const)
    └── VariableDeclarator
        ├── Identifier: "x"
        └── BinaryExpression (+)
            ├── NumericLiteral: 1
            └── NumericLiteral: 2
```

### 常见的 AST 节点类型

#### 1. Program
- **作用**: AST 的根节点
- **包含**: 所有顶层语句

#### 2. Statement (语句)
- `VariableDeclaration` - 变量声明: `const x = 1`
- `ExpressionStatement` - 表达式语句: `console.log('hi')`
- `IfStatement` - if 语句
- `ForStatement` - for 循环
- `FunctionDeclaration` - 函数声明

#### 3. Expression (表达式)
- `Identifier` - 标识符: `x`, `myVar`
- `Literal` - 字面量: `42`, `"hello"`, `true`
- `BinaryExpression` - 二元表达式: `a + b`
- `CallExpression` - 函数调用: `foo()`
- `MemberExpression` - 成员访问: `obj.prop`

#### 4. Declaration (声明)
- `FunctionDeclaration` - 函数声明
- `ClassDeclaration` - 类声明
- `VariableDeclaration` - 变量声明

## 🔬 实践练习

### 练习 1: 观察简单代码的 AST

创建以下文件并解析：

#### test1.js - 变量声明
```javascript
let name = "Oxc";
const version = 1;
var count = 0;
```

运行解析：
```bash
cargo run -p oxc_parser --example parser -- test1.js
```

**观察要点**:
- 三种变量声明的区别在哪里？
- 字符串和数字字面量的表示有何不同？

---

#### test2.js - 函数
```javascript
function greet(name) {
    return "Hello, " + name;
}

const result = greet("World");
```

**观察要点**:
- 函数声明包含哪些部分？
- 函数调用如何表示？
- 字符串拼接是什么节点类型？

---

#### test3.js - 条件语句
```javascript
if (x > 0) {
    console.log("positive");
} else {
    console.log("non-positive");
}
```

**观察要点**:
- if 语句包含哪些子节点？
- 比较操作如何表示？
- console.log 调用的结构是什么？

---

### 练习 2: 识别节点类型

对于以下代码片段，尝试在心里构建 AST 结构：

```javascript
const numbers = [1, 2, 3];
const doubled = numbers.map(x => x * 2);
```

**思考**:
1. 顶层有几个语句？
2. 数组字面量 `[1, 2, 3]` 是什么节点类型？
3. 箭头函数 `x => x * 2` 包含哪些部分？
4. 方法调用 `numbers.map()` 的结构是什么？

<details>
<summary>点击查看答案</summary>

```
Program
├── VariableDeclaration (const)
│   └── VariableDeclarator
│       ├── Identifier: "numbers"
│       └── ArrayExpression
│           ├── NumericLiteral: 1
│           ├── NumericLiteral: 2
│           └── NumericLiteral: 3
└── VariableDeclaration (const)
    └── VariableDeclarator
        ├── Identifier: "doubled"
        └── CallExpression
            ├── MemberExpression
            │   ├── Object: Identifier "numbers"
            │   └── Property: Identifier "map"
            └── Arguments
                └── ArrowFunctionExpression
                    ├── Params: [Identifier "x"]
                    └── Body: BinaryExpression (*)
                        ├── Left: Identifier "x"
                        └── Right: NumericLiteral 2
```
</details>

---

### 练习 3: TypeScript 特性

创建 test4.ts：

```typescript
interface User {
    name: string;
    age: number;
}

const user: User = {
    name: "Alice",
    age: 30
};
```

运行解析：
```bash
cargo run -p oxc_parser --example parser -- test4.ts
```

**观察要点**:
- TypeScript 的类型注解如何表示？
- interface 声明的结构是什么？
- 类型和值的 AST 节点有何区别？

## 🔍 深入理解

### Parser 的工作流程

```
源代码 (Source Code)
    ↓
词法分析 (Lexer/Tokenizer)
    ↓
Token 流
    ↓
语法分析 (Parser)
    ↓
AST (抽象语法树)
```

### Oxc Parser 的特点

1. **高性能**:
   - 使用 Rust 编写，速度极快
   - 零拷贝设计
   - 并行处理能力

2. **完整支持**:
   - 最新的 JavaScript/TypeScript 语法
   - JSX/TSX 支持
   - 装饰器等实验性特性

3. **错误恢复**:
   - 遇到语法错误能继续解析
   - 提供详细的错误信息
   - 适合编辑器实时解析

### 在 Oxc 中查看 AST 定义

所有 AST 节点定义在：
```
crates/oxc_ast/src/ast/
├── js.rs           # JavaScript 节点
├── ts.rs           # TypeScript 节点
├── jsx.rs          # JSX 节点
└── literal.rs      # 字面量节点
```

示例：查看 `VariableDeclaration` 的定义：
```bash
# 搜索定义
grep -r "struct VariableDeclaration" crates/oxc_ast/src/
```

## 📝 学习笔记模板

记录你的学习心得：

### 今天我学到了：
-
-
-

### 印象最深的概念：


### 遇到的问题：


### 解决方案：


### 明天的计划：
- [ ]
- [ ]

## 🎯 检查点

完成以下任务，检验学习效果：

- [ ] 能够独立运行 Parser 解析 JS/TS 文件
- [ ] 能够识别 5 种以上常见的 AST 节点类型
- [ ] 理解语句 (Statement) 和表达式 (Expression) 的区别
- [ ] 能够看懂简单代码的 AST 结构
- [ ] 知道 Oxc AST 定义文件的位置

## 🔗 相关资源

### 在线工具
- [AST Explorer](https://astexplorer.net/) - 在线查看 AST（选择 @typescript-eslint/parser）

### 代码位置
- Parser 实现: `crates/oxc_parser/src/`
- AST 定义: `crates/oxc_ast/src/ast/`
- Parser 示例: `crates/oxc_parser/examples/parser.rs`

### 延伸阅读
- [The Super Tiny Compiler](https://github.com/jamiebuilds/the-super-tiny-compiler) - 编译器入门
- [Crafting Interpreters](https://craftinginterpreters.com/) - 解释器原理

---

## ➡️ 下一步

完成 Day 1-2 的学习后，继续：
- [Day 3-4: Linter 基础](./第一周_Day3-4_Linter基础.md)

---

**学习日期**: ___________
**完成情况**: ⬜ 未开始 / ⬜ 进行中 / ⬜ 已完成

