use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;
use std::mem;

/// 演示 1: OsStr 跨平台字符串处理
fn demo_os_str() {
    println!("\n=== 演示 1: OsStr 跨平台字符串 ===");
    
    let path = PathBuf::from("src/main.rs");
    
    // 转换为 OsStr
    let os_str: &OsStr = path.as_os_str();
    println!("OsStr: {:?}", os_str);
    
    // 转换为字符串（可能丢失信息）
    let lossy = os_str.to_string_lossy();
    println!("Lossy string: {}", lossy);
    
    // 安全转换为字符串
    if let Some(s) = os_str.to_str() {
        println!("UTF-8 string: {}", s);
    }
    
    // 检查文件扩展名
    if let Some(ext) = path.extension() {
        println!("Extension: {:?}", ext);
        println!("Extension as str: {:?}", ext.to_str());
    }
}

/// 演示 2: PathBuf 路径操作
fn demo_path() {
    println!("\n=== 演示 2: PathBuf 路径操作 ===");
    
    let mut path = PathBuf::from("/usr");
    println!("Original path: {:?}", path);
    
    // 拼接路径
    path.push("local");
    path.push("bin");
    println!("After push: {:?}", path);
    
    // 获取各个组件
    println!("Parent: {:?}", path.parent());
    println!("File name: {:?}", path.file_name());
    println!("Extension: {:?}", path.extension());
    
    // 创建相对路径
    let relative = Path::new("src").join("main.rs");
    println!("Relative path: {:?}", relative);
}

/// 演示 3: Arc 原子引用计数
fn demo_arc() {
    println!("\n=== 演示 3: Arc 原子引用计数 ===");
    
    // 创建数据并用 Arc 包装
    let data = Arc::new(vec![1, 2, 3, 4, 5]);
    println!("Original data: {:?}", data);
    println!("Arc strong count: {}", Arc::strong_count(&data));
    
    // 克隆 Arc（不复制数据）
    let cloned1 = Arc::clone(&data);
    let cloned2 = Arc::clone(&data);
    println!("After cloning twice, strong count: {}", Arc::strong_count(&data));
    
    // 数据仍然只有一份
    println!("All references point to same data:");
    println!("  data: {:?}", data);
    println!("  cloned1: {:?}", cloned1);
    println!("  cloned2: {:?}", cloned2);
    
    // 所有引用都指向同一内存地址（如果可用）
    println!("Are they the same? {}", Arc::ptr_eq(&data, &cloned1));
}

/// 演示 4: mpsc 通道通信
fn demo_mpsc() {
    println!("\n=== 演示 4: mpsc 通道通信 ===");
    
    let (sender, receiver) = mpsc::channel();
    
    // 创建多个生产者线程
    for i in 0..3 {
        let sender_clone = sender.clone();
        thread::spawn(move || {
            let data = format!("Message from thread {}", i);
            sender_clone.send(data).unwrap();
            println!("Thread {} sent message", i);
        });
    }
    
    // 主线程关闭发送端
    drop(sender);
    
    // 接收所有消息
    println!("Receiving messages:");
    for msg in receiver {
        println!("  Received: {}", msg);
    }
}

/// 演示 5: 批量发送优化（类似 walk.rs 的做法）
fn demo_batch_send() {
    println!("\n=== 演示 5: 批量发送优化 ===");
    
    let (sender, receiver) = mpsc::channel::<Vec<i32>>();
    
    // 模拟多个收集器线程
    for thread_id in 0..3 {
        let sender_clone = sender.clone();
        thread::spawn(move || {
            // 模拟收集数据
            let mut batch = Vec::new();
            for i in 0..3 {
                batch.push(thread_id * 10 + i);
            }
            
            // 批量发送
            sender_clone.send(batch).unwrap();
            println!("Thread {} sent batch", thread_id);
        });
    }
    
    drop(sender);
    
    // 接收并展平所有批次
    let all_data: Vec<i32> = receiver.into_iter().flatten().collect();
    println!("All collected data: {:?}", all_data);
}

/// 演示 6: mem::take 所有权转移
fn demo_take() {
    println!("\n=== 演示 6: mem::take 所有权转移 ===");
    
    struct Resource {
        data: Vec<i32>,
    }
    
    impl Resource {
        fn new() -> Self {
            Self { data: vec![1, 2, 3, 4, 5] }
        }
        
        fn extract_data(&mut self) -> Vec<i32> {
            // 使用 take 安全地转移所有权
            mem::take(&mut self.data)
        }
    }
    
    let mut resource = Resource::new();
    println!("Before extract: {:?}", resource.data);
    
    let extracted = resource.extract_data();
    println!("Extracted: {:?}", extracted);
    println!("After extract: {:?}", resource.data); // 现在是空 Vec
}

/// 演示 7: Arc<OsStr> 在实际场景中的应用
fn demo_arc_osstr() {
    println!("\n=== 演示 7: Arc<OsStr> 实际应用 ===");
    
    // 模拟文件路径
    let paths = vec![
        PathBuf::from("src/main.rs"),
        PathBuf::from("src/lib.rs"),
        PathBuf::from("tests/test.rs"),
    ];
    
    // 转换为 Arc<OsStr>（避免克隆字符串）
    let arc_paths: Vec<Arc<OsStr>> = paths
        .iter()
        .map(|p| Arc::from(p.as_os_str()))
        .collect();
    
    println!("Original paths:");
    for path in &paths {
        println!("  {:?}", path);
    }
    
    println!("\nArc<OsStr> paths:");
    for arc_path in &arc_paths {
        println!("  {:?}", arc_path);
    }
    
    // 克隆 Arc（不复制底层数据）
    let cloned = arc_paths.clone();
    println!("\nCloned Arc paths (no data copy):");
    for arc_path in &cloned {
        println!("  {:?}", arc_path);
    }
    
    // 验证 Arc 共享数据
    println!("\nAre they sharing the same data?");
    for (original, cloned) in arc_paths.iter().zip(cloned.iter()) {
        println!("  {:?} == {:?}: {}", original, cloned, Arc::ptr_eq(original, cloned));
    }
}

/// 主函数
fn main() {
    println!("🚀 Rust 标准库学习演示");
    println!("基于 apps/oxlint/src/walk.rs 的分析");
    
    demo_os_str();
    demo_path();
    demo_arc();
    demo_mpsc();
    demo_batch_send();
    demo_take();
    demo_arc_osstr();
    
    println!("\n✅ 所有演示完成！");
    println!("\n💡 提示：查看 learn_demo/docs/std_library_analysis.md 获取详细说明");
}

