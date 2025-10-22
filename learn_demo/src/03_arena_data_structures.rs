// 第三个示例：Arena 数据结构使用
// 运行方式：cd learn_docs/examples && cargo run --bin 03_arena_data_structures

use oxc_allocator::{Allocator, Box as ArenaBox, HashMap as ArenaHashMap, Vec as ArenaVec};

fn main() {
    println!("🏗️ Arena 数据结构使用示例");
    println!("{}", "=".repeat(50));

    let allocator = Allocator::default();

    // Arena Box 使用
    arena_box_demo(&allocator);

    // Arena Vec 使用
    arena_vec_demo(&allocator);

    // Arena HashMap 使用
    arena_hashmap_demo(&allocator);

    // 嵌套结构演示
    nested_structures_demo(&allocator);

    // 复杂数据结构演示
    complex_structures_demo(&allocator);

    println!("\n🎉 Arena 数据结构示例完成！");
}

fn arena_box_demo(allocator: &Allocator) {
    println!("\n📦 Arena Box 使用:");

    // 基本类型的 Box
    let int_box = ArenaBox::new_in(42, allocator);
    let string_box = ArenaBox::new_in("Hello Arena".to_string(), allocator);

    println!("   基本 ArenaBox:");
    println!("     int_box: {}", int_box);
    println!("     string_box: {}", string_box);
    println!("     int_box 地址: {:p}", &*int_box);
    println!("     string_box 地址: {:p}", &*string_box);

    // 结构体的 Box
    #[derive(Debug)]
    struct Person {
        name: String,
        age: u32,
        email: String,
    }

    let person_box = ArenaBox::new_in(
        Person { name: "Alice".to_string(), age: 30, email: "alice@example.com".to_string() },
        allocator,
    );

    println!("   结构体 ArenaBox:");
    println!("     person: {:?}", person_box);

    // 修改 Box 中的数据
    let mut mutable_box = ArenaBox::new_in(vec![1, 2, 3], allocator);
    mutable_box.push(4);
    mutable_box.push(5);

    println!("   可变 ArenaBox:");
    println!("     修改后的 vec: {:?}", mutable_box);
}

fn arena_vec_demo(allocator: &Allocator) {
    println!("\n📋 Arena Vec 使用:");

    // 创建空的 ArenaVec
    let mut numbers = ArenaVec::new_in(allocator);

    // 添加元素
    for i in 1..=10 {
        numbers.push(i * i); // 平方数
    }

    println!("   基本 ArenaVec:");
    println!("     平方数: {:?}", numbers);
    println!("     长度: {}", numbers.len());
    println!("     容量: {}", numbers.capacity());

    // 字符串 Vec
    let mut words = ArenaVec::new_in(allocator);
    words.push("Oxc");
    words.push("is");
    words.push("fast");
    words.push("and");
    words.push("efficient");

    println!("   字符串 ArenaVec:");
    println!("     words: {:?}", words);

    // 从迭代器创建
    let squares: ArenaVec<i32> = (1..=5).map(|x| x * x).collect_in(allocator);
    println!("   从迭代器创建: {:?}", squares);

    // Vec 操作
    let mut operations = ArenaVec::new_in(allocator);
    operations.extend([1, 2, 3, 4, 5]);
    operations.retain(|&x| x % 2 == 0); // 保留偶数

    println!("   Vec 操作:");
    println!("     过滤后的偶数: {:?}", operations);

    // 嵌套 Vec
    let mut matrix = ArenaVec::new_in(allocator);
    for i in 0..3 {
        let mut row = ArenaVec::new_in(allocator);
        for j in 0..3 {
            row.push(i * 3 + j);
        }
        matrix.push(row);
    }

    println!("   嵌套 Vec (3x3 矩阵):");
    for (i, row) in matrix.iter().enumerate() {
        println!("     行 {}: {:?}", i, row);
    }
}

fn arena_hashmap_demo(allocator: &Allocator) {
    println!("\n🗂️ Arena HashMap 使用:");

    // 基本 HashMap
    let mut config = ArenaHashMap::new_in(allocator);
    config.insert("host", "localhost");
    config.insert("port", "8080");
    config.insert("debug", "true");
    config.insert("max_connections", "100");

    println!("   配置 HashMap:");
    for (key, value) in &config {
        println!("     {}: {}", key, value);
    }

    // 数字键的 HashMap
    let mut scores = ArenaHashMap::new_in(allocator);
    scores.insert("Alice", 95);
    scores.insert("Bob", 87);
    scores.insert("Charlie", 92);
    scores.insert("Diana", 98);

    println!("   分数 HashMap:");
    for (name, score) in &scores {
        println!("     {}: {}", name, score);
    }

    // HashMap 操作
    println!("   HashMap 操作:");
    println!("     Alice 的分数: {:?}", scores.get("Alice"));
    println!("     包含 Eve: {}", scores.contains_key("Eve"));

    // 更新值
    scores.insert("Alice", 97); // 更新 Alice 的分数
    println!("     更新后 Alice 的分数: {:?}", scores.get("Alice"));

    // 复杂值类型的 HashMap
    let mut user_data = ArenaHashMap::new_in(allocator);

    let mut alice_hobbies = ArenaVec::new_in(allocator);
    alice_hobbies.push("reading");
    alice_hobbies.push("coding");
    alice_hobbies.push("hiking");

    let mut bob_hobbies = ArenaVec::new_in(allocator);
    bob_hobbies.push("gaming");
    bob_hobbies.push("cooking");

    user_data.insert("Alice", alice_hobbies);
    user_data.insert("Bob", bob_hobbies);

    println!("   用户爱好 HashMap:");
    for (user, hobbies) in &user_data {
        println!("     {} 的爱好: {:?}", user, hobbies);
    }
}

fn nested_structures_demo(allocator: &Allocator) {
    println!("\n🏗️ 嵌套结构演示:");

    // 创建一个复杂的嵌套结构：公司 -> 部门 -> 员工
    let mut company = ArenaHashMap::new_in(allocator);

    // 工程部
    let mut engineering = ArenaVec::new_in(allocator);
    engineering.push("Alice (Senior Developer)");
    engineering.push("Bob (DevOps Engineer)");
    engineering.push("Charlie (Frontend Developer)");

    // 市场部
    let mut marketing = ArenaVec::new_in(allocator);
    marketing.push("Diana (Marketing Manager)");
    marketing.push("Eve (Content Creator)");

    // 人事部
    let mut hr = ArenaVec::new_in(allocator);
    hr.push("Frank (HR Manager)");
    hr.push("Grace (Recruiter)");

    company.insert("Engineering", engineering);
    company.insert("Marketing", marketing);
    company.insert("HR", hr);

    println!("   公司组织结构:");
    for (department, employees) in &company {
        println!("     {} 部门:", department);
        for employee in employees {
            println!("       - {}", employee);
        }
    }

    // 统计信息
    let total_employees: usize = company.values().map(|dept| dept.len()).sum();
    println!("   总员工数: {}", total_employees);
    println!("   部门数: {}", company.len());
}

fn complex_structures_demo(allocator: &Allocator) {
    println!("\n🧩 复杂数据结构演示:");

    // 模拟一个简单的图结构
    #[derive(Debug)]
    struct Node<'a> {
        id: u32,
        name: &'a str,
        connections: ArenaVec<'a, u32>, // 连接到其他节点的 ID
    }

    let mut graph = ArenaHashMap::new_in(allocator);

    // 创建节点
    let node1 = Node {
        id: 1,
        name: allocator.alloc_str("Node A"),
        connections: {
            let mut conn = ArenaVec::new_in(allocator);
            conn.push(2);
            conn.push(3);
            conn
        },
    };

    let node2 = Node {
        id: 2,
        name: allocator.alloc_str("Node B"),
        connections: {
            let mut conn = ArenaVec::new_in(allocator);
            conn.push(1);
            conn.push(4);
            conn
        },
    };

    let node3 = Node {
        id: 3,
        name: allocator.alloc_str("Node C"),
        connections: {
            let mut conn = ArenaVec::new_in(allocator);
            conn.push(1);
            conn.push(4);
            conn
        },
    };

    let node4 = Node {
        id: 4,
        name: allocator.alloc_str("Node D"),
        connections: {
            let mut conn = ArenaVec::new_in(allocator);
            conn.push(2);
            conn.push(3);
            conn
        },
    };

    // 将节点添加到图中
    graph.insert(1, node1);
    graph.insert(2, node2);
    graph.insert(3, node3);
    graph.insert(4, node4);

    println!("   图结构:");
    for (id, node) in &graph {
        println!("     节点 {} ({}): 连接到 {:?}", id, node.name, node.connections);
    }

    // 图遍历示例
    fn find_path<'a>(
        graph: &ArenaHashMap<'a, u32, Node<'a>>,
        start: u32,
        end: u32,
        visited: &mut ArenaVec<'a, u32>,
        allocator: &'a Allocator,
    ) -> bool {
        if start == end {
            return true;
        }

        visited.push(start);

        if let Some(node) = graph.get(&start) {
            for &neighbor in &node.connections {
                if !visited.contains(&neighbor) {
                    if find_path(graph, neighbor, end, visited, allocator) {
                        return true;
                    }
                }
            }
        }

        visited.pop();
        false
    }

    let mut visited = ArenaVec::new_in(allocator);
    let path_exists = find_path(&graph, 1, 4, &mut visited, allocator);
    println!("   从节点 1 到节点 4 是否有路径: {}", path_exists);

    // 展示所有数据都在同一个 Arena 中
    println!("   🎯 重要：所有这些复杂的嵌套数据结构都在同一个 Arena 中！");
    println!("      - 所有节点、连接列表、字符串都是连续存储的");
    println!("      - 当 allocator 被 drop 时，所有数据一起释放");
    println!("      - 没有内存泄漏的风险");
}
