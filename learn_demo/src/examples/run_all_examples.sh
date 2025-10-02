#!/bin/bash

# Oxc Allocator 学习示例运行脚本
# 按顺序运行所有示例程序

set -e  # 遇到错误时退出

echo "🎯 开始运行 Oxc Allocator 学习示例"
echo "=" | head -c 50; echo

# 检查是否在正确的目录
if [ ! -f "Cargo.toml" ]; then
    echo "❌ 错误: 请在 learn_docs/examples 目录中运行此脚本"
    exit 1
fi

# 示例列表
examples=(
    "01_allocator_basics:基础使用"
    "02_performance_comparison:性能对比"
    "03_arena_data_structures:Arena 数据结构"
    "04_memory_management:内存管理"
    "05_ast_simulation:AST 模拟"
    "06_advanced_features:高级特性"
)

# 运行每个示例
for example in "${examples[@]}"; do
    IFS=':' read -r bin_name description <<< "$example"

    echo "🚀 运行示例: $description ($bin_name)"
    echo "-" | head -c 30; echo

    # 运行示例
    if cargo run --bin "$bin_name"; then
        echo "✅ $description 完成"
    else
        echo "❌ $description 失败"
        exit 1
    fi

    echo
    echo "按 Enter 继续下一个示例，或 Ctrl+C 退出..."
    read -r
    echo
done

echo "🎉 所有示例运行完成！"
echo
echo "📚 学习建议:"
echo "  1. 重新运行感兴趣的示例: cargo run --bin <示例名>"
echo "  2. 使用 --release 模式获得更好的性能数据"
echo "  3. 修改示例代码进行实验"
echo "  4. 阅读源码理解实现细节"
echo
echo "🎯 下一步: 开始学习 oxc_ast 模块！"


