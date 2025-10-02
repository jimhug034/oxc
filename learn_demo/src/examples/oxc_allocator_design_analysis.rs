// oxc_allocator 设计分析实践
// 运行方式：cd learn_docs/examples && cargo run --bin oxc_allocator_design_analysis

use oxc_allocator::{Allocator, Box as ArenaBox, Vec as ArenaVec, CloneIn};
use std::marker::PhantomData;
use std::ptr::NonNull;

fn main() {
    println!("🔬 oxc_allocator 设计分析实践");
    println!("{}", "=".repeat(50));

    // 1. 生命周期系统分析
    lifetime_system_analysis();

    // 2. 类型安全设计分析
    type_safety_analysis();

    // 3. PhantomData 使用分析
    phantom_data_analysis();

    // 4. 性能优化技巧分析
    performance_optimization_analysis();

    // 5. Trait 系统设计分析
    trait_system_analysis();

    // 6. 实践：自定义 Arena 类型
    custom_arena_types_practice();

    println!("\n🎉 设计分析实践完成！");
}

fn lifetime_system_analysis() {
    println!("\n📚 1. 生命周期系统分析");

    // 分析点 1: 生命周期绑定策略
    let allocator = Allocator::default();

    // 所有分配的对象都与 allocator 的生命周期绑定
    let data1 = allocator.alloc(42);
    let data2 = allocator.alloc(84);

    println!("   生命周期绑定演示:");
    println!("     data1: {} (生命周期与 allocator 绑定)", data1);
    println!("     data2: {} (生命周期与 allocator 绑定)", data2);

    // 分析点 2: 生命周期省略规则的应用
    demonstrate_lifetime_elision(&allocator);

    // 分析点 3: 多个生命周期参数
    demonstrate_multiple_lifetimes();
}

fn demonstrate_lifetime_elision(allocator: &Allocator) {
    // alloc 方法的签名分析：
    // pub fn alloc<T>(&self, val: T) -> &mut T
    // 等价于：
    // pub fn alloc<'a, T>(&'a self, val: T) -> &'a mut T

    let value = allocator.alloc(100);
    println!("   生命周期省略: {}", value);
}

fn demonstrate_multiple_lifetimes() {
    let alloc1 = Allocator::default();
    let alloc2 = Allocator::default();

    let data1 = alloc1.alloc(10);
    let data2 = alloc2.alloc(20);

    // 比较来自不同 allocator 的数据
    let result = compare_data_from_different_allocators(data1, data2);
    println!("   多生命周期参数: 比较结果 = {}", result);
}

fn compare_data_from_different_allocators<'a, 'b>(
    data1: &'a i32,
    data2: &'b i32
) -> bool {
    data1 > data2
}

fn type_safety_analysis() {
    println!("\n📚 2. 类型安全设计分析");

    let allocator = Allocator::default();

    // 分析点 1: 编译时类型检查
    println!("   编译时类型检查:");

    // 这些类型是安全的（不需要 Drop）
    let _safe_int = allocator.alloc(42);
    let _safe_array = allocator.alloc([1, 2, 3]);
    let _safe_tuple = allocator.alloc((1, "hello"));
    println!("     ✅ 基本类型分配成功");

    // 这些会在编译时失败：
    // let _bad_vec = allocator.alloc(Vec::new()); // 编译错误！
    // let _bad_string = allocator.alloc(String::new()); // 编译错误！

    // 分析点 2: const 断言的使用
    demonstrate_const_assertions();

    // 分析点 3: 类型约束的实现
    demonstrate_type_constraints(&allocator);
}

fn demonstrate_const_assertions() {
    println!("   const 断言分析:");

    // 模拟 oxc_allocator 中的编译时检查
    const SAFE_CHECK: bool = !std::mem::needs_drop::<i32>();
    const UNSAFE_CHECK: bool = std::mem::needs_drop::<Vec<i32>>();

    println!("     i32 需要 Drop: {}", !SAFE_CHECK);
    println!("     Vec<i32> 需要 Drop: {}", UNSAFE_CHECK);

    // 编译时断言（如果条件不满足会编译失败）
    const _: () = assert!(SAFE_CHECK, "i32 should not need drop");
    const _: () = assert!(UNSAFE_CHECK, "Vec<i32> should need drop");
}

fn demonstrate_type_constraints(allocator: &Allocator) {
    println!("   类型约束实现:");

    // 创建一个类型化的分配器
    let typed_alloc = TypedAllocator::<i32>::new(allocator);
    let value = typed_alloc.alloc(42);
    println!("     TypedAllocator<i32>: {}", value);

    // 不同类型需要不同的实例
    let string_alloc = TypedAllocator::<&str>::new(allocator);
    let text = string_alloc.alloc("hello");
    println!("     TypedAllocator<&str>: {}", text);
}

// 自定义类型化分配器
struct TypedAllocator<'alloc, T> {
    allocator: &'alloc Allocator,
    _phantom: PhantomData<T>,
}

impl<'alloc, T> TypedAllocator<'alloc, T> {
    fn new(allocator: &'alloc Allocator) -> Self {
        // 编译时检查
        const { assert!(!std::mem::needs_drop::<T>(), "T must not need Drop") };

        Self {
            allocator,
            _phantom: PhantomData,
        }
    }

    fn alloc(&self, value: T) -> &'alloc mut T {
        self.allocator.alloc(value)
    }
}

fn phantom_data_analysis() {
    println!("\n📚 3. PhantomData 使用分析");

    let allocator = Allocator::default();

    // 分析 ArenaBox 的 PhantomData 使用
    let arena_box = ArenaBox::new_in(42, &allocator);

    println!("   ArenaBox 分析:");
    println!("     值: {}", *arena_box);
    println!("     ArenaBox 大小: {} bytes", std::mem::size_of::<ArenaBox<i32>>());
    println!("     标准 Box 大小: {} bytes", std::mem::size_of::<Box<i32>>());

    // PhantomData 的作用演示
    demonstrate_phantom_data_effects(&allocator);
}

fn demonstrate_phantom_data_effects(allocator: &Allocator) {
    println!("   PhantomData 作用分析:");

    // 创建自定义的智能指针
    let smart_ptr = SmartPtr::new_in(100, allocator);
    println!("     SmartPtr 值: {}", *smart_ptr);
    println!("     SmartPtr 大小: {} bytes", std::mem::size_of::<SmartPtr<i32>>());

    // PhantomData 确保生命周期正确
    drop(smart_ptr);
    println!("     ✅ 生命周期检查通过");
}

// 自定义智能指针演示 PhantomData
struct SmartPtr<'alloc, T> {
    ptr: NonNull<T>,
    _phantom: PhantomData<&'alloc T>, // 标记生命周期
}

impl<'alloc, T> SmartPtr<'alloc, T> {
    fn new_in(value: T, allocator: &'alloc Allocator) -> Self {
        let ptr = NonNull::from(allocator.alloc(value));
        Self {
            ptr,
            _phantom: PhantomData,
        }
    }
}

impl<T> std::ops::Deref for SmartPtr<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

fn performance_optimization_analysis() {
    println!("\n📚 4. 性能优化技巧分析");

    let allocator = Allocator::default();

    // 分析点 1: 内联优化的效果
    demonstrate_inlining_effects(&allocator);

    // 分析点 2: 零成本抽象
    demonstrate_zero_cost_abstractions(&allocator);

    // 分析点 3: 内存布局优化
    demonstrate_memory_layout_optimization(&allocator);
}

#[inline(always)]
fn hot_path_function(allocator: &Allocator, value: i32) -> &mut i32 {
    // 模拟热路径函数，总是内联
    allocator.alloc(value)
}

fn demonstrate_inlining_effects(allocator: &Allocator) {
    println!("   内联优化分析:");

    let start = std::time::Instant::now();
    for i in 0..10000 {
        let _data = hot_path_function(allocator, i);
    }
    let inlined_time = start.elapsed();

    println!("     内联函数调用 10000 次: {:?}", inlined_time);
    println!("     ✅ 内联优化减少函数调用开销");
}

fn demonstrate_zero_cost_abstractions(allocator: &Allocator) {
    println!("   零成本抽象分析:");

    // ArenaBox 的解引用是零成本的
    let arena_box = ArenaBox::new_in(42, allocator);
    let value = *arena_box; // Deref trait 的零成本抽象

    println!("     ArenaBox 解引用: {}", value);
    println!("     ✅ Deref trait 提供零成本抽象");
}

fn demonstrate_memory_layout_optimization(allocator: &Allocator) {
    println!("   内存布局优化分析:");

    // 连续分配展示内存局部性
    let data: Vec<&i32> = (0..1000).map(|i| allocator.alloc(i) as &i32).collect();

    // 计算地址连续性
    let mut continuous_count = 0;
    for i in 1..data.len() {
        let addr1 = data[i-1] as *const i32 as usize;
        let addr2 = data[i] as *const i32 as usize;
        if addr2.abs_diff(addr1) == std::mem::size_of::<i32>() {
            continuous_count += 1;
        }
    }

    let continuity_ratio = continuous_count as f64 / (data.len() - 1) as f64;
    println!("     内存连续性: {:.2}%", continuity_ratio * 100.0);
    println!("     ✅ 高内存局部性提升缓存命中率");
}

fn trait_system_analysis() {
    println!("\n📚 5. Trait 系统设计分析");

    let allocator = Allocator::default();

    // 分析点 1: CloneIn trait 的设计
    demonstrate_clone_in_design(&allocator);

    // 分析点 2: 关联类型 vs 泛型参数
    demonstrate_associated_types(&allocator);

    // 分析点 3: Blanket implementation
    demonstrate_blanket_implementation(&allocator);
}

fn demonstrate_clone_in_design(allocator: &Allocator) {
    println!("   CloneIn trait 设计分析:");

    // 创建一个复杂的嵌套结构
    let mut original_vec = ArenaVec::new_in(allocator);
    original_vec.push(1);
    original_vec.push(2);
    original_vec.push(3);

    // 克隆到新的分配器
    let new_allocator = Allocator::default();
    let cloned_vec = original_vec.clone_in(&new_allocator);

    println!("     原始 vec: {:?} (地址: {:p})", original_vec, original_vec.as_ptr());
    println!("     克隆 vec: {:?} (地址: {:p})", cloned_vec, cloned_vec.as_ptr());
    println!("     ✅ CloneIn 实现跨 Allocator 克隆");
}

fn demonstrate_associated_types(allocator: &Allocator) {
    println!("   关联类型设计分析:");

    // CloneIn 使用关联类型而不是泛型参数
    // trait CloneIn<'new_alloc> {
    //     type Cloned;  // 关联类型
    // }

    let data = CustomStruct::new_in(allocator, 42, "hello");
    let new_allocator = Allocator::default();
    let cloned = data.clone_in(&new_allocator);

    println!("     原始数据: {:?}", data);
    println!("     克隆数据: {:?}", cloned);
    println!("     ✅ 关联类型提供清晰的 API");
}

#[derive(Debug)]
struct CustomStruct<'alloc> {
    number: i32,
    text: &'alloc str,
}

impl<'alloc> CustomStruct<'alloc> {
    fn new_in(allocator: &'alloc Allocator, number: i32, text: &str) -> Self {
        Self {
            number,
            text: allocator.alloc_str(text),
        }
    }
}

impl<'old_alloc, 'new_alloc> CloneIn<'new_alloc> for CustomStruct<'old_alloc> {
    type Cloned = CustomStruct<'new_alloc>;

    fn clone_in(&self, allocator: &'new_alloc Allocator) -> Self::Cloned {
        CustomStruct {
            number: self.number,
            text: allocator.alloc_str(self.text),
        }
    }
}

fn demonstrate_blanket_implementation(_allocator: &Allocator) {
    println!("   Blanket Implementation 分析:");

    // oxc_allocator 中 IntoIn 的 blanket implementation:
    // impl<'a, T, U> IntoIn<'a, U> for T where U: FromIn<'a, T>

    println!("     ✅ Blanket implementation 减少重复代码");
    println!("     ✅ 提供一致的 API 体验");
    println!("     ✅ 自动为满足条件的类型实现 trait");
}

fn custom_arena_types_practice() {
    println!("\n📚 6. 实践：自定义 Arena 类型");

    let allocator = Allocator::default();

    // 实践 1: Arena 链表
    let mut list = ArenaLinkedList::new_in(&allocator);
    list.push(1);
    list.push(2);
    list.push(3);

    println!("   Arena 链表:");
    list.print();

    // 实践 2: Arena 二叉树
    let mut tree = ArenaBinaryTree::new_in(&allocator);
    tree.insert(5);
    tree.insert(3);
    tree.insert(7);
    tree.insert(1);
    tree.insert(9);

    println!("   Arena 二叉树:");
    tree.print_inorder();

    // 实践 3: 内存使用统计
    demonstrate_memory_usage_stats(&allocator);
}

// Arena 链表实现
struct ArenaLinkedList<'alloc, T> {
    head: Option<&'alloc mut ListNode<'alloc, T>>,
    allocator: &'alloc Allocator,
}

struct ListNode<'alloc, T> {
    value: T,
    next: Option<&'alloc mut ListNode<'alloc, T>>,
}

impl<'alloc, T: std::fmt::Display> ArenaLinkedList<'alloc, T> {
    fn new_in(allocator: &'alloc Allocator) -> Self {
        Self { head: None, allocator }
    }

    fn push(&mut self, value: T) {
        let new_node = self.allocator.alloc(ListNode {
            value,
            next: self.head.take(),
        });
        self.head = Some(new_node);
    }

    fn print(&self) {
        print!("     链表: ");
        let mut current = self.head.as_ref();
        while let Some(node) = current {
            print!("{} -> ", node.value);
            current = node.next.as_ref();
        }
        println!("null");
    }
}

// Arena 二叉树实现
struct ArenaBinaryTree<'alloc, T> {
    root: Option<&'alloc mut TreeNode<'alloc, T>>,
    allocator: &'alloc Allocator,
}

struct TreeNode<'alloc, T> {
    value: T,
    left: Option<&'alloc mut TreeNode<'alloc, T>>,
    right: Option<&'alloc mut TreeNode<'alloc, T>>,
}

impl<'alloc, T: Ord + std::fmt::Display + Copy> ArenaBinaryTree<'alloc, T> {
    fn new_in(allocator: &'alloc Allocator) -> Self {
        Self { root: None, allocator }
    }

    fn insert(&mut self, value: T) {
        if self.root.is_none() {
            self.root = Some(self.allocator.alloc(TreeNode {
                value,
                left: None,
                right: None,
            }));
        } else {
            Self::insert_recursive(self.root.as_mut().unwrap(), value, self.allocator);
        }
    }

    fn insert_recursive(
        node: &mut TreeNode<'alloc, T>,
        value: T,
        allocator: &'alloc Allocator,
    ) {
        if value < node.value {
            if node.left.is_none() {
                node.left = Some(allocator.alloc(TreeNode {
                    value,
                    left: None,
                    right: None,
                }));
            } else {
                Self::insert_recursive(node.left.as_mut().unwrap(), value, allocator);
            }
        } else {
            if node.right.is_none() {
                node.right = Some(allocator.alloc(TreeNode {
                    value,
                    left: None,
                    right: None,
                }));
            } else {
                Self::insert_recursive(node.right.as_mut().unwrap(), value, allocator);
            }
        }
    }

    fn print_inorder(&self) {
        print!("     二叉树 (中序): ");
        if let Some(root) = &self.root {
            Self::print_inorder_recursive(root);
        }
        println!();
    }

    fn print_inorder_recursive(node: &TreeNode<'alloc, T>) {
        if let Some(left) = &node.left {
            Self::print_inorder_recursive(left);
        }
        print!("{} ", node.value);
        if let Some(right) = &node.right {
            Self::print_inorder_recursive(right);
        }
    }
}

fn demonstrate_memory_usage_stats(allocator: &Allocator) {
    println!("   内存使用统计:");

    // 分配不同大小的对象
    let small_objects: Vec<_> = (0..1000).map(|i| allocator.alloc(i as u8)).collect();
    let medium_objects: Vec<_> = (0..100).map(|i| allocator.alloc([i as u32; 16])).collect();
    let large_objects: Vec<_> = (0..10).map(|i| allocator.alloc([i as u64; 128])).collect();

    println!("     小对象 (1 byte): {} 个", small_objects.len());
    println!("     中对象 (64 bytes): {} 个", medium_objects.len());
    println!("     大对象 (1KB): {} 个", large_objects.len());

    // 估算内存使用
    let total_small = small_objects.len() * std::mem::size_of::<u8>();
    let total_medium = medium_objects.len() * std::mem::size_of::<[u32; 16]>();
    let total_large = large_objects.len() * std::mem::size_of::<[u64; 128]>();
    let total = total_small + total_medium + total_large;

    println!("     估算总内存: {} KB", total / 1024);
    println!("     ✅ Arena 高效管理不同大小的对象");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_allocator() {
        let allocator = Allocator::default();
        let typed_alloc = TypedAllocator::<i32>::new(&allocator);
        let value = typed_alloc.alloc(42);
        assert_eq!(*value, 42);
    }

    #[test]
    fn test_custom_structures() {
        let allocator = Allocator::default();

        // 测试链表
        let mut list = ArenaLinkedList::new_in(&allocator);
        list.push(1);
        list.push(2);
        assert!(list.head.is_some());

        // 测试二叉树
        let mut tree = ArenaBinaryTree::new_in(&allocator);
        tree.insert(5);
        tree.insert(3);
        assert!(tree.root.is_some());
    }

    #[test]
    fn test_clone_in_trait() {
        let allocator1 = Allocator::default();
        let allocator2 = Allocator::default();

        let original = CustomStruct::new_in(&allocator1, 42, "hello");
        let cloned = original.clone_in(&allocator2);

        assert_eq!(original.number, cloned.number);
        assert_eq!(original.text, cloned.text);

        // 验证它们在不同的分配器中
        assert_ne!(original.text.as_ptr(), cloned.text.as_ptr());
    }
}
