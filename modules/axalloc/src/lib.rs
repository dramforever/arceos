#![no_std]
#[macro_use]
extern crate log;
extern crate alloc;
mod page;
use allocator::{AllocResult, BaseAllocator, ByteAllocator, EarlyAllocator, PageAllocator};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use spinlock::SpinNoIrq;
const PAGE_SIZE: usize = 0x1000;
pub use page::GlobalPage;
/// The global allocator used by ArceOS.
pub struct GlobalAllocator {
    inner: SpinNoIrq<EarlyAllocator<PAGE_SIZE>>,
}
impl GlobalAllocator {
    /// Creates an empty [`GlobalAllocator`].
    pub const fn new() -> Self {
        Self {
            inner: SpinNoIrq::new(EarlyAllocator::new()),
        }
    }
    /// Returns the name of the allocator.
    pub const fn name(&self) -> &'static str {
        "early"
    }
    /// Initializes the allocator with the given region.
    pub fn init(&self, start_vaddr: usize, size: usize) {
        self.inner.lock().init(start_vaddr, size);
    }
    /// Add the given region to the allocator.
    pub fn add_memory(&self, _start_vaddr: usize, _size: usize) -> AllocResult {
        unimplemented!()
    }
    /// Allocate arbitrary number of bytes. Returns the left bound of the
    /// allocated region.
    pub fn alloc(&self, layout: Layout) -> AllocResult<NonNull<u8>> {
        self.inner.lock().alloc(layout)
    }
    /// Gives back the allocated region to the byte allocator.
    pub fn dealloc(&self, pos: NonNull<u8>, layout: Layout) {
        self.inner.lock().dealloc(pos, layout)
    }
    /// Allocates contiguous pages.
    pub fn alloc_pages(&self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        self.inner.lock().alloc_pages(num_pages, align_pow2)
    }
    /// Gives back the allocated pages starts from `pos` to the page allocator.
    /// [`alloc_pages`]: GlobalAllocator::alloc_pages
    pub fn dealloc_pages(&self, pos: usize, num_pages: usize) {
        self.inner.lock().dealloc_pages(pos, num_pages)
    }
    /// Returns the number of allocated bytes in the byte allocator.
    pub fn used_bytes(&self) -> usize {
        self.inner.lock().used_bytes()
    }
    /// Returns the number of available bytes in the byte allocator.
    pub fn available_bytes(&self) -> usize {
        self.inner.lock().available_bytes()
    }
    /// Returns the number of allocated pages in the page allocator.
    pub fn used_pages(&self) -> usize {
        self.inner.lock().used_pages()
    }
    /// Returns the number of available pages in the page allocator.
    pub fn available_pages(&self) -> usize {
        self.inner.lock().available_pages()
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Ok(ptr) = GlobalAllocator::alloc(self, layout) {
            ptr.as_ptr()
        } else {
            alloc::alloc::handle_alloc_error(layout)
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        GlobalAllocator::dealloc(
            self,
            NonNull::new(ptr).expect(
                "dealloc null
ptr",
            ),
            layout,
        )
    }
}
#[cfg_attr(all(target_os = "none", not(test)), global_allocator)]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator::new();
/// Returns the reference to the global allocator.
pub fn global_allocator() -> &'static GlobalAllocator {
    &GLOBAL_ALLOCATOR
}
/// Initializes the global allocator with the given memory region.
pub fn global_init(start_vaddr: usize, size: usize) {
    debug!(
        "initialize global allocator at: [{:#x}, {:#x})",
        start_vaddr,
        start_vaddr + size
    );
    GLOBAL_ALLOCATOR.init(start_vaddr, size);
}
/// Add the given memory region to the global allocator.
pub fn global_add_memory(_start_vaddr: usize, _size: usize) -> AllocResult {
    unimplemented!()
}
