use core::ptr::NonNull;

use crate::{AllocError, AllocResult, BaseAllocator, ByteAllocator, PageAllocator};

pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: usize,
    end: usize,
    orig_start: usize,
    orig_end: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 1,
            end: 1,
            orig_start: 1,
            orig_end: 1,
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.orig_end = self.end;
        self.orig_start = self.start;

        if self.start == 0 {
            self.start += 1;
        }
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        panic!("Can't")
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: core::alloc::Layout) -> AllocResult<NonNull<u8>> {
        use AllocError::*;

        let res = self
            .start
            .checked_next_multiple_of(layout.align())
            .ok_or(NoMemory)?;
        let new_start = res.checked_add(layout.size()).ok_or(NoMemory)?;
        if new_start > self.end {
            return Err(NoMemory);
        }

        self.start = new_start;
        let ptr = res as *mut u8;
        Ok(NonNull::new(ptr).expect("should have skipped zero address"))
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: core::alloc::Layout) {
        // Can't
    }

    fn total_bytes(&self) -> usize {
        self.orig_end - self.orig_start
    }

    fn used_bytes(&self) -> usize {
        self.start - self.orig_start
    }

    fn available_bytes(&self) -> usize {
        self.end - self.start
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        use AllocError::*;
        assert!(align_pow2 % PAGE_SIZE == 0);
        let num_bytes = (num_pages * PAGE_SIZE)
            .checked_next_multiple_of(align_pow2)
            .ok_or(NoMemory)?;
        let region_end = self.end - (self.end % align_pow2);
        let res = region_end - num_bytes;
        if self.start > res {
            return Err(NoMemory);
        }

        self.end = res;
        Ok(res)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        // Can't
    }

    fn total_pages(&self) -> usize {
        (self.orig_end - self.start) / PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.orig_end - self.end) / PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        (self.end - self.start) / PAGE_SIZE
    }
}
