# Oxlint 逻辑流程图

## 1. 总体启动流程

```mermaid
flowchart TD
    Start([用户执行 oxlint 命令]) --> Init[初始化运行环境]
    Init --> ParseArgs[解析命令行参数]
    ParseArgs --> ArgsValid{参数是否<br/>有效?}

    ArgsValid -->|无效| PrintError[打印错误信息]
    PrintError --> ExitError([退出: 错误码])

    ArgsValid -->|有效| CheckSpecial{特殊命令?}
    CheckSpecial -->|--rules| ListRules[列出所有规则]
    ListRules --> ExitSuccess([退出: 成功])

    CheckSpecial -->|--init| InitConfig[创建配置文件]
    InitConfig --> ExitSuccess

    CheckSpecial -->|--print-config| PrintConfig[打印配置]
    PrintConfig --> ExitSuccess

    CheckSpecial -->|正常 lint| InitThreads[初始化线程池]
    InitThreads --> LoadConfig[加载配置文件]
    LoadConfig --> FindFiles[查找待检查文件]
    FindFiles --> CreateLinter[创建 Linter 实例]
    CreateLinter --> ExecuteLint[执行 Linting]
    ExecuteLint --> OutputResults[输出结果]
    OutputResults --> DetermineExit{判断<br/>退出状态}

    DetermineExit -->|有错误| ExitError
    DetermineExit -->|无错误| ExitSuccess
```

---

## 2. 初始化运行环境详细流程

```mermaid
flowchart TD
    Start([开始初始化]) --> CheckEnv{检查 OXC_LOG<br/>环境变量}
    CheckEnv -->|存在| EnableTracing[启用日志追踪]
    CheckEnv -->|不存在| SkipTracing[跳过日志]

    EnableTracing --> InitMiette[初始化错误报告]
    SkipTracing --> InitMiette

    InitMiette --> CheckNode{第一个参数<br/>是 'node'?}
    CheckNode -->|是| SkipTwo[跳过前两个参数]
    CheckNode -->|否| SkipOne[跳过第一个参数]

    SkipTwo --> CollectArgs[收集剩余参数]
    SkipOne --> CollectArgs

    CollectArgs --> End([初始化完成])
```

---

## 3. 配置文件加载流程

```mermaid
flowchart TD
    Start([开始加载配置]) --> CheckFlag{用户指定<br/>--config?}

    CheckFlag -->|是| LoadSpecified[加载指定配置文件]
    CheckFlag -->|否| SearchDefault[在当前目录查找<br/>.oxlintrc.json]

    LoadSpecified --> ParseConfig{解析<br/>成功?}
    SearchDefault --> Found{找到<br/>配置文件?}

    Found -->|是| ParseConfig
    Found -->|否| UseDefault[使用默认配置]

    ParseConfig -->|失败| Error[报告配置错误]
    Error --> Exit([退出])

    ParseConfig -->|成功| CheckNested{允许嵌套<br/>配置?}
    UseDefault --> CheckNested

    CheckNested -->|是| SearchNested[搜索子目录中的<br/>配置文件]
    CheckNested -->|否| MergePlugins[合并插件配置]

    SearchNested --> MergeConfigs[合并所有配置]
    MergeConfigs --> MergePlugins

    MergePlugins --> ApplyFilters[应用命令行过滤器<br/>-A/-W/-D]
    ApplyFilters --> BuildStore[构建配置存储]
    BuildStore --> End([配置加载完成])
```

---

## 4. 文件查找和过滤流程

```mermaid
flowchart TD
    Start([开始查找文件]) --> CheckPaths{用户是否<br/>指定路径?}

    CheckPaths -->|否| UseCwd[使用当前目录]
    CheckPaths -->|是| ValidatePaths{路径包含<br/>父目录 '..'?}

    ValidatePaths -->|是| ErrorPath[报错: 不允许<br/>父目录路径]
    ErrorPath --> Exit([退出])

    ValidatePaths -->|否| AbsolutePaths[转换为绝对路径]
    UseCwd --> AbsolutePaths

    AbsolutePaths --> CheckIgnore{使用<br/>--no-ignore?}

    CheckIgnore -->|是| SkipIgnoreCheck[跳过 ignore 检查]
    CheckIgnore -->|否| LoadGitignore[加载 .gitignore]

    LoadGitignore --> ApplyIgnorePatterns[应用 --ignore-pattern]
    ApplyIgnorePatterns --> PreFilter[预过滤显式指定的文件]

    PreFilter --> CheckEmpty{过滤后<br/>是否为空?}
    SkipIgnoreCheck --> CheckEmpty

    CheckEmpty -->|是且用户指定了路径| NoFiles[报告: 没有找到文件]
    NoFiles --> Exit

    CheckEmpty -->|是且未指定路径| UseCwd
    CheckEmpty -->|否| WalkFS[遍历文件系统]

    WalkFS --> FilterExtensions[过滤文件扩展名<br/>.js/.ts/.jsx/.tsx/etc]
    FilterExtensions --> ApplyIgnoreMatcher[应用嵌套 ignore 模式]
    ApplyIgnoreMatcher --> CollectFiles[收集所有文件路径]
    CollectFiles --> End([文件收集完成])
```

---

## 5. Linter 创建和配置流程

```mermaid
flowchart TD
    Start([开始创建 Linter]) --> CreateStore[创建配置存储<br/>ConfigStore]
    CreateStore --> CheckTypeAware{启用<br/>--type-aware?}

    CheckTypeAware -->|是| RunTsGoLint[运行 tsgolint<br/>类型感知检查]
    CheckTypeAware -->|否| CreateLinter[创建 Linter 实例]

    RunTsGoLint --> TsGoSuccess{tsgolint<br/>成功?}
    TsGoSuccess -->|失败| TsGoError[报告 tsgolint 错误]
    TsGoError --> Exit([退出])

    TsGoSuccess -->|成功| CreateLinter

    CreateLinter --> CheckFix{启用<br/>自动修复?}

    CheckFix -->|是| ConfigureFix[配置修复类型<br/>safe/suggestion/dangerous]
    CheckFix -->|否| CheckDirectives{报告未使用<br/>的指令?}

    ConfigureFix --> CheckDirectives

    CheckDirectives -->|是| EnableDirectives[启用指令报告]
    CheckDirectives -->|否| CheckTsconfig{指定<br/>tsconfig?}

    EnableDirectives --> CheckTsconfig

    CheckTsconfig -->|是| ValidateTsconfig{tsconfig<br/>存在?}
    CheckTsconfig -->|否| CreateService[创建 LintService]

    ValidateTsconfig -->|否| TsconfigError[报告 tsconfig 错误]
    TsconfigError --> Exit

    ValidateTsconfig -->|是| LoadTsconfig[加载 tsconfig<br/>用于 import 插件]
    LoadTsconfig --> CreateService

    CreateService --> End([Linter 创建完成])
```

---

## 6. Linting 执行流程（并发）

```mermaid
flowchart TD
    Start([开始执行 Linting]) --> SpawnWorker[在独立线程中<br/>启动 LintService]
    Start --> StartDiagnostic[在主线程中<br/>启动诊断服务]

    SpawnWorker --> ParallelLoop[并行遍历所有文件]

    ParallelLoop --> ProcessFile[处理单个文件]

    ProcessFile --> ReadFile[读取文件内容<br/>source_text: &str]
    ReadFile --> DetectType[检测文件类型<br/>JS/TS/JSX/TSX]
    DetectType --> CreateAllocator[创建内存分配器<br/>Arena Allocator]
    CreateAllocator --> Parse[🔥 解析成 AST<br/>oxc_parser::Parser.parse]

    Parse --> ParseSuccess{解析<br/>成功?}

    ParseSuccess -->|失败| SendParseError[发送解析错误]
    SendParseError --> NextFile{还有<br/>文件?}

    ParseSuccess -->|成功| GetAST[获得 AST<br/>Program 根节点]
    GetAST --> Semantic[🔥 语义分析<br/>oxc_semantic::SemanticBuilder.build<br/>构建符号表/作用域/CFG]
    Semantic --> SemanticSuccess{语义分析<br/>成功?}

    SemanticSuccess -->|失败| SendParseError
    SemanticSuccess -->|成功| RunLinter[🔥 运行 Linter<br/>基于 AST 执行所有规则]
    RunLinter --> CheckFix2{启用<br/>修复?}

    CheckFix2 -->|是| ApplyFixes[应用修复]
    CheckFix2 -->|否| CollectDiagnostics[收集诊断信息]

    ApplyFixes --> WriteFile[写回文件]
    WriteFile --> CollectDiagnostics

    CollectDiagnostics --> SendDiagnostics[发送诊断到主线程]
    SendDiagnostics --> NextFile

    NextFile -->|是| ProcessFile
    NextFile -->|否| WorkerDone[工作线程完成]

    StartDiagnostic --> WaitMessages[等待诊断消息]
    WaitMessages --> ReceiveMsg{接收到<br/>消息?}

    ReceiveMsg -->|文件诊断| FormatOutput[格式化输出]
    ReceiveMsg -->|工作完成| DiagnosticDone[诊断服务完成]

    FormatOutput --> PrintToStdout[打印到 stdout]
    PrintToStdout --> CountErrors[统计错误/警告]
    CountErrors --> WaitMessages

    WorkerDone --> CloseChannel[关闭通道]
    CloseChannel -.通知.-> ReceiveMsg

    DiagnosticDone --> OutputStats[输出统计信息]
    OutputStats --> End([Linting 完成])
```

---

## 7. 单个文件的规则执行流程

```mermaid
flowchart TD
    Start([开始执行规则]) --> CreateContext[创建上下文宿主<br/>ContextHost]
    CreateContext --> LoopStart{开始循环<br/>处理脚本块}

    LoopStart --> FilterRules[过滤适用的规则]
    FilterRules --> CheckOptimize{启用运行时<br/>优化?}

    CheckOptimize -->|是| CheckNodeTypes[检查文件是否包含<br/>规则需要的节点类型]
    CheckOptimize -->|否| DetermineStrategy{判断<br/>执行策略}

    CheckNodeTypes --> RemoveIrrelevant[移除不相关的规则]
    RemoveIrrelevant --> DetermineStrategy

    DetermineStrategy -->|小文件<br/>≤200K 节点| SmallFileStrategy[策略A: 外层遍历规则<br/>内层遍历节点]
    DetermineStrategy -->|大文件<br/>>200K 节点| LargeFileStrategy[策略B: 外层遍历节点<br/>内层遍历规则]

    SmallFileStrategy --> RunRuleOnce[执行 rule.run_once]
    LargeFileStrategy --> BucketRules[按 AST 类型<br/>分桶规则]

    BucketRules --> RunRuleOnce2[执行 rule.run_once]
    RunRuleOnce2 --> IterateNodes[遍历所有节点]

    RunRuleOnce --> IterateRules[遍历所有规则]
    IterateRules --> IterateNodesPerRule[为每个规则遍历节点]

    IterateNodesPerRule --> CheckNodeType{节点类型<br/>匹配?}
    CheckNodeType -->|是| RunRule[执行 rule.run]
    CheckNodeType -->|否| SkipNode[跳过节点]

    RunRule --> NextNode{还有<br/>节点?}
    SkipNode --> NextNode

    NextNode -->|是| IterateNodesPerRule
    NextNode -->|否| CheckJest{是测试<br/>框架?}

    IterateNodes --> GetBucket[获取该节点类型的<br/>规则桶]
    GetBucket --> RunBucketRules[执行桶中的规则]
    RunBucketRules --> RunAnyRules[执行通用规则]
    RunAnyRules --> NextNode2{还有<br/>节点?}

    NextNode2 -->|是| IterateNodes
    NextNode2 -->|否| CheckJest

    CheckJest -->|是| FindJestNodes[查找 Jest 调用节点]
    CheckJest -->|否| RunExternal[运行外部规则]

    FindJestNodes --> RunJestRules[执行 Jest 规则]
    RunJestRules --> RunExternal

    RunExternal --> CheckUnused{报告未使用<br/>指令?}
    CheckUnused -->|是| ReportUnused[报告未使用的<br/>eslint-disable]
    CheckUnused -->|否| NextBlock{还有下一个<br/>脚本块?}

    ReportUnused --> NextBlock

    NextBlock -->|是 Vue/Svelte| UpdateContext[切换到下一个<br/>脚本块上下文]
    NextBlock -->|否| CollectDiag[收集所有诊断]

    UpdateContext --> LoopStart

    CollectDiag --> ExtractDirectives[提取禁用指令信息]
    ExtractDirectives --> End([规则执行完成])
```

---

## 8. 退出状态判断流程

```mermaid
flowchart TD
    Start([检查诊断结果]) --> HasErrors{有错误<br/>诊断?}

    HasErrors -->|是| ExitError([退出码 1<br/>LintFoundErrors])

    HasErrors -->|否| CheckDenyWarnings{启用<br/>--deny-warnings?}

    CheckDenyWarnings -->|是| HasWarnings{有警告?}
    CheckDenyWarnings -->|否| CheckMaxWarnings{设置<br/>--max-warnings?}

    HasWarnings -->|是| ExitDenyWarn([退出码 1<br/>LintNoWarningsAllowed])
    HasWarnings -->|否| CheckMaxWarnings

    CheckMaxWarnings -->|是| ExceedMax{警告数超过<br/>阈值?}
    CheckMaxWarnings -->|否| ExitSuccess([退出码 0<br/>LintSucceeded])

    ExceedMax -->|是| ExitMaxWarn([退出码 1<br/>LintMaxWarningsExceeded])
    ExceedMax -->|否| ExitSuccess
```

---

## 9. 性能优化决策流程

```mermaid
flowchart TD
    Start([开始性能优化]) --> CheckFileSize{文件 AST 节点数}

    CheckFileSize -->|≤ 200,000 节点| SmallFile[小文件策略]
    CheckFileSize -->|> 200,000 节点| LargeFile[大文件策略]

    SmallFile --> SmallReason[原因: 节点数据可以<br/>保留在 CPU 缓存中]
    SmallReason --> SmallApproach[方法: 外层遍历规则<br/>避免多次迭代规则]
    SmallApproach --> SmallBenefit[优点: 减少规则迭代<br/>开销]

    LargeFile --> LargeReason[原因: 节点数据会<br/>挤出缓存]
    LargeReason --> LargeApproach[方法: 外层遍历节点<br/>规则数据较小]
    LargeApproach --> LargeBenefit[优点: 避免缓存抖动<br/>Cache thrashing]

    SmallBenefit --> CheckTypes{规则指定了<br/>节点类型?}
    LargeBenefit --> CheckTypes

    CheckTypes -->|是| FilterByType[只在相关节点类型<br/>上运行规则]
    CheckTypes -->|否| RunOnAll[在所有节点上<br/>运行规则]

    FilterByType --> End([优化完成])
    RunOnAll --> End
```

---

## 10. 配置合并优先级流程

```mermaid
flowchart TD
    Start([开始配置合并]) --> Level1[级别 1: 默认配置]
    Level1 --> Level2[级别 2: 根目录配置文件]
    Level2 --> Level3[级别 3: 嵌套配置文件]
    Level3 --> Level4[级别 4: 命令行过滤器<br/>-A/-W/-D]
    Level4 --> Level5[级别 5: 命令行插件开关<br/>--*-plugin]

    Level5 --> MergeRules[合并规则配置]
    MergeRules --> CheckConflict{有冲突?}

    CheckConflict -->|是| UseHigher[使用优先级更高的配置]
    CheckConflict -->|否| MergeSettings[合并其他设置]

    UseHigher --> MergeSettings
    MergeSettings --> ValidateConfig{配置<br/>有效?}

    ValidateConfig -->|否| ReportError[报告配置错误]
    ValidateConfig -->|是| BuildFinal[构建最终配置]

    ReportError --> Exit([退出])
    BuildFinal --> End([配置合并完成])
```

---

## ❗必须的处理步骤

**每个文件都必须经过以下步骤，不能跳过**：

```
1. 读取文件内容 (source_text)
   ↓
2. 🔥 解析成 AST (oxc_parser)
   ↓
3. 🔥 语义分析 (oxc_semantic)
   ↓
4. 🔥 执行 Lint 规则 (基于 AST 节点)
   ↓
5. 输出诊断或应用修复
```

**为什么必须要 AST？**

- Lint 规则需要理解代码结构
- 需要区分不同类型的节点（变量声明、函数、表达式等）
- 需要访问语义信息（作用域、符号表、引用关系）
- 字符串匹配无法准确检测代码问题

---

## 关键逻辑决策点总结

| 决策点          | 选项                     | 影响                     |
| --------------- | ------------------------ | ------------------------ |
| **文件大小**    | ≤200K 节点 vs >200K 节点 | 决定迭代策略（缓存优化） |
| **类型感知**    | 启用 vs 禁用             | 是否运行 tsgolint        |
| **自动修复**    | 启用 vs 禁用             | 是否写回文件             |
| **嵌套配置**    | 启用 vs 禁用             | 是否搜索子目录配置       |
| **输出格式**    | default/json/junit/等    | 决定诊断输出格式         |
| **警告处理**    | deny/max-warnings        | 影响退出码               |
| **并发策略**    | 线程数                   | 影响处理速度             |
| **ignore 模式** | 启用 vs --no-ignore      | 决定文件过滤行为         |

---

## 并发模型

```mermaid
graph TB
    subgraph 主线程
        A[解析参数] --> B[加载配置]
        B --> C[查找文件]
        C --> D[创建 Linter]
        D --> E[启动诊断服务]
        E --> F[等待并输出诊断]
        F --> G[输出统计信息]
    end

    subgraph Rayon 线程池
        H[并行处理文件 1]
        I[并行处理文件 2]
        J[并行处理文件 N]
    end

    D -.启动.-> H
    D -.启动.-> I
    D -.启动.-> J

    H -.诊断消息.-> F
    I -.诊断消息.-> F
    J -.诊断消息.-> F

    style 主线程 fill:#e1f5ff
    style Rayon 线程池 fill:#fff4e1
```

这个逻辑流程图展示了 Oxlint 的核心决策点和执行路径，帮助理解其设计思想和优化策略。
