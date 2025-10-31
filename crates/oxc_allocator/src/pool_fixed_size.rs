use std::{
    alloc::{self, GlobalAlloc, Layout, System},
    mem::ManuallyDrop,
    ops::Deref,
    ptr::NonNull,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

use oxc_ast_macros::ast;

use crate::{
    Allocator,
    fixed_size_constants::{BLOCK_ALIGN, BLOCK_SIZE, RAW_METADATA_SIZE},
};

const TWO_GIB: usize = 1 << 31;
const FOUR_GIB: usize = 1 << 32;

/// 线程安全的 [`Allocator`] 池，通过复用实例降低分配开销。
///
/// 内部使用 `Mutex` 保护的 `Vec` 存储可用的分配器。
///
/// # 设计目标
///
/// - 避免频繁创建/销毁大块内存（每个分配器占用 2 GiB）
/// - 支持多线程并发获取与归还分配器
/// - 按需创建分配器，而非预先分配全部
pub struct AllocatorPool {
    /// 池中可复用的分配器列表
    allocators: Mutex<Vec<FixedSizeAllocator>>,
    /// 下一个新建分配器的唯一 ID
    next_id: AtomicU32,
}

impl AllocatorPool {
    /// 创建一个新的 [`AllocatorPool`]，用于指定数量的线程。
    ///
    /// 预留容量但不预先分配分配器，避免浪费内存（例如 language server 未启用 `import` 插件时）。
    pub fn new(thread_count: usize) -> AllocatorPool {
        // 每个分配器占用大量内存，因此按需创建而非预先分配，
        // 以防部分线程未被使用（例如 language server 未启用 `import` 插件）
        let allocators = Vec::with_capacity(thread_count);
        AllocatorPool { allocators: Mutex::new(allocators), next_id: AtomicU32::new(0) }
    }

    /// 从池中获取一个 [`Allocator`]，若池为空则创建新实例。
    ///
    /// 返回 [`AllocatorGuard`] 以提供对分配器的访问。
    ///
    /// # Panics
    ///
    /// 若底层 mutex 被污染则 panic。
    pub fn get(&self) -> AllocatorGuard<'_> {
        let allocator = {
            let mut allocators = self.allocators.lock().unwrap();
            allocators.pop()
        };

        let allocator = allocator.unwrap_or_else(|| {
            // 每个分配器需要唯一 ID，但分配顺序无关紧要，因此使用 `Ordering::Relaxed`
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            // 防止 ID 溢出
            // TODO: 这个检查是否有效？是否真的需要？
            assert!(id < u32::MAX, "Created too many allocators");
            FixedSizeAllocator::new(id)
        });

        AllocatorGuard { allocator: ManuallyDrop::new(allocator), pool: self }
    }

    /// 将一个 [`FixedSizeAllocator`] 归还到池中。
    ///
    /// 该分配器应已清空，准备好被复用。
    ///
    /// # Panics
    ///
    /// 若底层 mutex 被污染则 panic。
    fn add(&self, allocator: FixedSizeAllocator) {
        let mut allocators = self.allocators.lock().unwrap();
        allocators.push(allocator);
    }
}

/// 守卫对象，代表对池中 [`Allocator`] 的独占访问。
///
/// 当 drop 时，`Allocator` 会被重置并归还到池中。
pub struct AllocatorGuard<'alloc_pool> {
    /// 从池中借出的分配器（使用 `ManuallyDrop` 防止自动释放）
    allocator: ManuallyDrop<FixedSizeAllocator>,
    /// 所属的池引用
    pool: &'alloc_pool AllocatorPool,
}

impl Deref for AllocatorGuard<'_> {
    type Target = Allocator;

    fn deref(&self) -> &Self::Target {
        &self.allocator.allocator
    }
}

impl Drop for AllocatorGuard<'_> {
    /// 将 [`Allocator`] 归还到池中。
    fn drop(&mut self) {
        // SAFETY: 取得 `FixedSizeAllocator` 的所有权后，不再访问 `ManuallyDrop`
        let mut allocator = unsafe { ManuallyDrop::take(&mut self.allocator) };
        allocator.reset();
        self.pool.add(allocator);
    }
}

/// [`FixedSizeAllocator`] 的元数据。
///
/// 存储在 [`FixedSizeAllocator`] 的内存块中，位于 `RawTransferMetadata` 之后，
/// 即 [`Allocator`] chunk 使用区域之后。
#[ast]
pub struct FixedSizeAllocatorMetadata {
    /// 分配器的唯一 ID
    pub id: u32,
    /// 指向 `FixedSizeAllocator` 原始分配起始位置的指针
    pub alloc_ptr: NonNull<u8>,
    /// 若 Rust 和 JS 同时持有对此 `FixedSizeAllocator` 的引用，则为 `true`。
    ///
    /// * 初始为 `false`。
    /// * 当 buffer 与 JS 共享时设为 `true`。
    /// * 当 JS 垃圾回收器回收 buffer 时重新设为 `false`。
    ///   内存将在 Rust 侧 drop `FixedSizeAllocator` 时释放。
    /// * 若 Rust 侧 drop `FixedSizeAllocator` 时也设为 `false`。
    ///   内存将在 JS 垃圾回收器回收 buffer 时的 finalizer 中释放。
    pub is_double_owned: AtomicBool,
}

// What we ideally want is an allocation 2 GiB in size, aligned on 4 GiB.
// But system allocator on Mac OS refuses allocations with 4 GiB alignment.
// https://github.com/rust-lang/rust/blob/556d20a834126d2d0ac20743b9792b8474d6d03c/library/std/src/sys/alloc/unix.rs#L16-L27
// https://github.com/rust-lang/rust/issues/30170
//
// So we instead allocate 4 GiB with 2 GiB alignment, and then use either the 1st or 2nd half
// of the allocation, one of which is guaranteed to be on a 4 GiB boundary.
//
// TODO: We could use this workaround only on Mac OS, and just allocate what we actually want on Linux.
// Windows OS allocator also doesn't support high alignment allocations, so Rust contains a workaround
// which over-allocates (6 GiB in this case).
// https://github.com/rust-lang/rust/blob/556d20a834126d2d0ac20743b9792b8474d6d03c/library/std/src/sys/alloc/windows.rs#L120-L137
// Could just use that built-in workaround, rather than implementing our own, or allocate a 6 GiB chunk
// with alignment 16, to skip Rust's built-in workaround.
// Note: Rust's workaround will likely commit a whole page of memory, just to store the real pointer.
const ALLOC_SIZE: usize = BLOCK_SIZE + TWO_GIB;
const ALLOC_ALIGN: usize = TWO_GIB;

/// 固定大小分配器的底层分配布局。
pub const ALLOC_LAYOUT: Layout = match Layout::from_size_align(ALLOC_SIZE, ALLOC_ALIGN) {
    Ok(layout) => layout,
    Err(_) => unreachable!(),
};

/// 封装一个固定大小为 2 GiB - 16 字节、对齐到 4 GiB 的 [`Allocator`] 的结构体。
///
/// # 分配策略
///
/// 为实现此目标，我们手动分配内存以支持 `Allocator` 的单个 chunk，
/// 并存储其他元数据。
///
/// 我们过度分配 4 GiB，然后仅使用其中一半 - 第 1 半或第 2 半，
/// 取决于从 `alloc.alloc()` 收到的分配的对齐方式。
/// 其中一半必定对齐到 4 GiB，我们使用那一半。
///
/// 内部 `Allocator` 被包装在 `ManuallyDrop` 中以防止其自行释放内存，
/// `FixedSizeAllocator` 有自定义的 `Drop` 实现来释放整个原始分配。
///
/// 我们通过 `System` 分配器分配，绕过任何已注册的替代全局分配器
/// （例如 linter 中的 Mimalloc）。Mimalloc 抱怨无法提供高对齐分配，
/// 并且从线程本地堆获取如此大的分配可能毫无意义，
/// 因此最好直接使用系统分配器。
///
/// # 已分配内存的区域
///
/// 已分配内存中有 2 GiB 完全未使用（见上文）。
///
/// 剩余的 2 GiB - 16 字节（实际使用的部分）划分如下：
///
/// ```txt
///                                                         WHOLE BLOCK - aligned on 4 GiB
/// <-----------------------------------------------------> Allocated block (`BLOCK_SIZE` bytes)
///
///                                                         ALLOCATOR
/// <----------------------------------------->             `Allocator` chunk (`CHUNK_SIZE` bytes)
///                                      <---->             Bumpalo's `ChunkFooter` (aligned on 16)
/// <----------------------------------->                   `Allocator` chunk data storage (for AST)
///
///                                                         METADATA
///                                            <---->       `RawTransferMetadata`
///                                                  <----> `FixedSizeAllocatorMetadata`
///
///                                                         BUFFER SENT TO JS
/// <----------------------------------------------->       Buffer sent to JS (`BUFFER_SIZE` bytes)
/// ```
///
/// 注意：发送到 JS 的 buffer 包含 `Allocator` chunk 和 `RawTransferMetadata`，
/// 但不包含 `FixedSizeAllocatorMetadata`。
///
/// `Allocator` chunk 使用区域的末尾必须对齐到 `Allocator::RAW_MIN_ALIGN` (16)，
/// 这是 Bumpalo 的要求。我们通过以下方式实现：
/// * `BLOCK_SIZE` 是 16 的倍数。
/// * `RawTransferMetadata` 是 16 字节。
/// * `FixedSizeAllocatorMetadata` 的大小向上舍入为 16 的倍数。
pub struct FixedSizeAllocator {
    /// 利用原始分配的一部分的 `Allocator`
    allocator: ManuallyDrop<Allocator>,
}

impl FixedSizeAllocator {
    /// 创建一个新的 [`FixedSizeAllocator`]。
    #[expect(clippy::items_after_statements)]
    pub fn new(id: u32) -> Self {
        // Only support little-endian systems. `Allocator::from_raw_parts` includes this same assertion.
        // This module is only compiled on 64-bit little-endian systems, so it should be impossible for
        // this panic to occur. But we want to make absolutely sure that if there's a mistake elsewhere,
        // `Allocator::from_raw_parts` cannot panic, as that'd result in a large memory leak.
        // Compiler will optimize this out.
        #[expect(clippy::manual_assert)]
        if cfg!(target_endian = "big") {
            panic!("`FixedSizeAllocator` is not supported on big-endian systems.");
        }

        // Allocate block of memory.
        // SAFETY: `ALLOC_LAYOUT` does not have zero size.
        let alloc_ptr = unsafe { System.alloc(ALLOC_LAYOUT) };
        let Some(alloc_ptr) = NonNull::new(alloc_ptr) else {
            alloc::handle_alloc_error(ALLOC_LAYOUT);
        };

        // All code in the rest of this function is infallible, so the allocation will always end up
        // owned by a `FixedSizeAllocator`, which takes care of freeing the memory correctly on drop

        // Get pointer to use for allocator chunk, aligned to 4 GiB.
        // `alloc_ptr` is aligned on 2 GiB, so `alloc_ptr % FOUR_GIB` is either 0 or `TWO_GIB`.
        //
        // * If allocation is already aligned on 4 GiB, `offset == 0`.
        //   Chunk occupies 1st half of the allocation.
        // * If allocation is not aligned on 4 GiB, `offset == TWO_GIB`.
        //   Adding `offset` to `alloc_ptr` brings it up to 4 GiB alignment.
        //   Chunk occupies 2nd half of the allocation.
        //
        // Either way, `chunk_ptr` is aligned on 4 GiB.
        let offset = alloc_ptr.as_ptr() as usize % FOUR_GIB;
        // SAFETY: We allocated 4 GiB of memory, so adding `offset` to `alloc_ptr` is in bounds
        let chunk_ptr = unsafe { alloc_ptr.add(offset) };

        debug_assert!(chunk_ptr.as_ptr() as usize % BLOCK_ALIGN == 0);

        const FIXED_METADATA_SIZE_ROUNDED: usize =
            size_of::<FixedSizeAllocatorMetadata>().next_multiple_of(Allocator::RAW_MIN_ALIGN);
        const FIXED_METADATA_OFFSET: usize = BLOCK_SIZE - FIXED_METADATA_SIZE_ROUNDED;
        const _: () =
            assert!(FIXED_METADATA_OFFSET % align_of::<FixedSizeAllocatorMetadata>() == 0);

        const CHUNK_SIZE: usize = FIXED_METADATA_OFFSET - RAW_METADATA_SIZE;
        const _: () = assert!(CHUNK_SIZE % Allocator::RAW_MIN_ALIGN == 0);

        // SAFETY: Memory region starting at `chunk_ptr` with `CHUNK_SIZE` bytes is within
        // the allocation we just made.
        // `chunk_ptr` has high alignment (4 GiB). `CHUNK_SIZE` is large and a multiple of 16.
        let allocator = unsafe { Allocator::from_raw_parts(chunk_ptr, CHUNK_SIZE) };
        let allocator = ManuallyDrop::new(allocator);

        // Write `FixedSizeAllocatorMetadata` to after space reserved for `RawTransferMetadata`,
        // which is after the end of the allocator chunk
        let metadata =
            FixedSizeAllocatorMetadata { alloc_ptr, id, is_double_owned: AtomicBool::new(false) };
        // SAFETY: `FIXED_METADATA_OFFSET` is `FIXED_METADATA_SIZE_ROUNDED` bytes before end of
        // the allocation, so there's space for `FixedSizeAllocatorMetadata`.
        // It's sufficiently aligned for `FixedSizeAllocatorMetadata`.
        unsafe {
            let metadata_ptr =
                chunk_ptr.add(FIXED_METADATA_OFFSET).cast::<FixedSizeAllocatorMetadata>();
            metadata_ptr.write(metadata);
        }

        Self { allocator }
    }

    /// 重置此 [`FixedSizeAllocator`]。
    fn reset(&mut self) {
        // Set cursor back to end
        self.allocator.reset();

        // Set data pointer back to start.
        // SAFETY: Fixed-size allocators have data pointer originally aligned on `BLOCK_ALIGN`,
        // and size less than `BLOCK_ALIGN`. So we can restore original data pointer by rounding down
        // to next multiple of `BLOCK_ALIGN`.
        // We're restoring the original data pointer, so it cannot break invariants about alignment,
        // being within the chunk's allocation, or being before cursor pointer.
        unsafe {
            let data_ptr = self.allocator.data_ptr();
            let offset = data_ptr.as_ptr() as usize % BLOCK_ALIGN;
            let data_ptr = data_ptr.sub(offset);
            self.allocator.set_data_ptr(data_ptr);
        }
    }
}

impl Drop for FixedSizeAllocator {
    fn drop(&mut self) {
        // SAFETY: This `Allocator` was created by this `FixedSizeAllocator`
        unsafe {
            let metadata_ptr = self.allocator.fixed_size_metadata_ptr();
            free_fixed_size_allocator(metadata_ptr);
        }
    }
}

/// 若 `FixedSizeAllocator` 未被双重拥有，则释放其底层内存
/// （双重拥有是指 Rust 侧的 `FixedSizeAllocator` 和 JS 侧的 buffer 同时持有）。
///
/// 若是双重拥有，则不释放内存，但设置标志表示不再双重拥有，
/// 以便下次调用此函数时释放。
///
/// # SAFETY
///
/// 此函数只能在以下情况下调用：
/// 1. Rust 侧对应的 `FixedSizeAllocator` 被 drop。或
/// 2. JS 侧对应此 `FixedSizeAllocatorMetadata` 的 buffer 被垃圾回收。
///
/// 在任何其他情况下调用此函数会导致双重释放。
///
/// `metadata_ptr` 必须指向有效的 `FixedSizeAllocatorMetadata`。
pub unsafe fn free_fixed_size_allocator(metadata_ptr: NonNull<FixedSizeAllocatorMetadata>) {
    // Get pointer to start of original allocation from `FixedSizeAllocatorMetadata`
    let alloc_ptr = {
        // SAFETY: This `Allocator` was created by the `FixedSizeAllocator`.
        // `&FixedSizeAllocatorMetadata` ref only lives until end of this block.
        let metadata = unsafe { metadata_ptr.as_ref() };

        // * If `is_double_owned` is already `false`, then one of:
        //   1. The `Allocator` was never sent to JS side, or
        //   2. The `FixedSizeAllocator` was already dropped on Rust side, or
        //   3. Garbage collector already collected it on JS side.
        //   We can deallocate the memory.
        //
        // * If `is_double_owned` is `true`, set it to `false` and exit.
        //   Memory will be freed when `FixedSizeAllocator` is dropped on Rust side
        //   or JS garbage collector collects the buffer.
        //
        // Maybe a more relaxed `Ordering` would be OK, but I (@overlookmotel) am not sure,
        // so going with `Ordering::SeqCst` to be on safe side.
        // Deallocation only happens at the end of the whole process, so it shouldn't matter much.
        // TODO: Figure out if can use `Ordering::Relaxed`.
        let is_double_owned = metadata.is_double_owned.swap(false, Ordering::SeqCst);
        if is_double_owned {
            return;
        }

        metadata.alloc_ptr
    };

    // Deallocate the memory backing the `FixedSizeAllocator`.
    // SAFETY: Originally allocated from `System` allocator at `alloc_ptr`, with layout `ALLOC_LAYOUT`.
    unsafe { System.dealloc(alloc_ptr.as_ptr(), ALLOC_LAYOUT) }
}

impl Allocator {
    /// 获取此 [`Allocator`] 的 `FixedSizeAllocatorMetadata` 指针。
    ///
    /// # SAFETY
    /// * 此 `Allocator` 必须由 `FixedSizeAllocator` 创建。
    /// * 此指针不得用于创建对 `FixedSizeAllocatorMetadata` 的可变引用，
    ///   只能创建不可变引用。
    pub unsafe fn fixed_size_metadata_ptr(&self) -> NonNull<FixedSizeAllocatorMetadata> {
        // SAFETY: Caller guarantees this `Allocator` was created by a `FixedSizeAllocator`.
        //
        // `FixedSizeAllocator::new` writes `FixedSizeAllocatorMetadata` after the end of
        // the chunk owned by the `Allocator`, and `RawTransferMetadata` (see above).
        // `end_ptr` is end of the allocator chunk (after the chunk header).
        // So `end_ptr + RAW_METADATA_SIZE` points to a valid, initialized `FixedSizeAllocatorMetadata`.
        //
        // We never create `&mut` references to `FixedSizeAllocatorMetadata`,
        // and it's not part of the buffer sent to JS, so no danger of aliasing violations.
        unsafe { self.end_ptr().add(RAW_METADATA_SIZE).cast::<FixedSizeAllocatorMetadata>() }
    }
}
