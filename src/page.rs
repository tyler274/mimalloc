use std::sync::atomic::{AtomicPtr, Ordering};

use crate::{
    block::mi_block_t,
    constants::{MI_ALIGNMENT_MAX, MI_INTPTR_SIZE},
    heap::Heap,
    segment::MI_SEGMENT_SLICE_SIZE,
    thread::mi_delayed_t,
};

// Main tuning parameters for page sizes
// Sizes for 64-bit (usually divide by two for 32-bit)

pub const MI_SMALL_PAGE_SHIFT: usize = crate::segment::MI_SEGMENT_SLICE_SHIFT; // 64KiB
pub const MI_MEDIUM_PAGE_SHIFT: usize = 3 + MI_SMALL_PAGE_SHIFT; // 512KiB

// Derived constants
pub const MI_SMALL_PAGE_SIZE: usize = 1 << MI_SMALL_PAGE_SHIFT;
pub const MI_MEDIUM_PAGE_SIZE: usize = 1 << MI_MEDIUM_PAGE_SHIFT;

pub const MI_SMALL_OBJ_SIZE_MAX: usize = MI_SMALL_PAGE_SIZE / 4; // 8KiB on 64-bit
pub const MI_MEDIUM_OBJ_SIZE_MAX: usize = MI_MEDIUM_PAGE_SIZE / 4; // 128KiB on 64-bit
pub const MI_MEDIUM_OBJ_WSIZE_MAX: usize = MI_MEDIUM_OBJ_SIZE_MAX / MI_INTPTR_SIZE;
pub const MI_LARGE_OBJ_SIZE_MAX: usize = crate::segment::MI_SEGMENT_SIZE / 2; // 32MiB on 64-bit
pub const MI_LARGE_OBJ_WSIZE_MAX: usize = MI_LARGE_OBJ_SIZE_MAX / MI_INTPTR_SIZE;

#[derive(Debug, Copy, Clone)]
pub struct embedded_flag {
    pub in_full: bool,
    pub has_aligned: bool,
}

impl embedded_flag {
    const fn new() -> Self {
        Self {
            in_full: true,
            has_aligned: true,
        }
    }
}
impl Default for embedded_flag {
    fn default() -> Self {
        Self::new()
    }
}

// The `in_full` and `has_aligned` page flags are put in a union to efficiently
// test if both are false (`full_aligned == 0`) in the `mi_free` routine.
#[repr(C)]
#[derive(Copy, Clone)]
pub union mi_page_flags_s {
    pub full_aligned: u8,
    pub x: embedded_flag,
}

impl mi_page_flags_s {
    const fn new() -> Self {
        Self {
            x: embedded_flag::new(),
        }
    }
}

impl Default for mi_page_flags_s {
    fn default() -> Self {
        Self::new()
    }
}

// impl Debug for mi_page_flags_s {}

type mi_page_flags_t = mi_page_flags_s;

// Thread free list.
// We use the bottom 2 bits of the pointer for mi_delayed_t flags
type mi_thread_free_t = usize;

// A page contains blocks of one specific size (`block_size`).
// Each page has three list of free blocks:
// `free` for blocks that can be allocated,
// `local_free` for freed blocks that are not yet available to `mi_malloc`
// `thread_free` for freed blocks by other threads
// The `local_free` and `thread_free` lists are migrated to the `free` list
// when it is exhausted. The separate `local_free` list is necessary to
// implement a monotonic heartbeat. The `thread_free` list is needed for
// avoiding atomic operations in the common case.
//
//
// `used - |thread_free|` == actual blocks that are in use (alive)
// `used - |thread_free| + |free| + |local_free| == capacity`
//
// We don't count `freed` (as |free|) but use `used` to reduce
// the number of memory accesses in the `mi_page_all_free` function(s).
//
// Notes:
// - Access is optimized for `mi_free` and `mi_page_alloc` (in `alloc.c`)
// - Using `uint16_t` does not seem to slow things down
// - The size is 8 words on 64-bit which helps the page index calculations
//   (and 10 words on 32-bit, and encoded free lists add 2 words. Sizes 10
//    and 12 are still good for address calculation)
// - To limit the structure size, the `xblock_size` is 32-bits only; for
//   blocks > MI_HUGE_BLOCK_SIZE the size is determined from the segment page size
// - `thread_free` uses the bottom bits as a delayed-free flags to optimize
//   concurrent frees where only the first concurrent free adds to the owning
//   heap `thread_delayed_free` list (see `alloc.c:mi_free_block_mt`).
//   The invariant is that no-delayed-free is only set if there is
//   at least one block that will be added, or as already been added, to
//   the owning heap `thread_delayed_free` list. This guarantees that pages
//   will be freed correctly even if only other threads free blocks.
#[repr(C)]
pub struct mi_page_s {
    // "owned" by the segment
    slice_count: u32,  // slices in this page (0 if not a page)
    slice_offset: u32, // distance from the actual page data slice (0 if a page)

    is_reset: u8,     // `true` if the page memory was reset
    is_committed: u8, // `true` if the page virtual memory is committed
    is_zero_init: u8, // `true` if the page was zero initialized

    // layout like this to optimize access in `mi_malloc` and `mi_free`
    capacity: u16, // number of blocks committed, must be the first field, see `segment.c:page_clear`
    reserved: u16, // number of blocks reserved in memory
    pub flags: mi_page_flags_t, // `in_full` and `has_aligned` flags (8 bits)

    is_zero: u8,       // `true` if the blocks in the free list are zero initialized
    retire_expire: u8, // expiration count for retired blocks

    pub free: *mut mi_block_t, // list of available free blocks (`malloc` allocates from this list)
    // TODO:
    // #ifdef MI_ENCODE_FREELIST
    //   uintptr_t keys[2]; // two random keys to encode the free lists (see `_mi_block_next`)
    // #endif
    used: u32, // number of blocks in use (including blocks in `local_free` and `thread_free`)
    pub xblock_size: u32, // size available in each block (always `>0`)

    local_free: *mut mi_block_t, // list of deferred free blocks by this thread (migrates to `free`)
    pub xthread_free: AtomicPtr<mi_thread_free_t>, // list of deferred free blocks freed by other threads
    pub xheap: AtomicPtr<Heap>,

    pub next: *mut mi_page_s, // next page owned by this thread with the same `block_size`
    pub prev: *mut mi_page_s, // previous page owned by this thread with the same `block_size`

                              // TODO:
                              // 64-bit 9 words, 32-bit 12 words, (+2 for secure)
                              // #if MI_INTPTR_SIZE == 8
                              // uintptr_t padding[1];
                              // #endif
}

const HAS_DATA: usize = 0x3;
const FLAG_MASK: usize = !HAS_DATA;

impl mi_page_s {
    const fn new() -> Self {
        Self {
            slice_count: 0,
            slice_offset: 0,

            is_reset: 1,
            is_committed: 1,
            is_zero_init: 1,

            capacity: 0,
            reserved: 0,
            flags: mi_page_flags_t::new(),

            is_zero: 1,
            retire_expire: 7,

            free: std::ptr::null_mut(),
            used: 0,
            xblock_size: 0,

            local_free: std::ptr::null_mut(),
            xthread_free: AtomicPtr::new(std::ptr::null_mut()),
            xheap: AtomicPtr::new(std::ptr::null_mut()),

            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
        }
    }
    // are all blocks in a page freed?
    // note: needs up-to-date used count, (as the `xthread_free` list may not be empty). see
    // `_mi_page_collect_free`.
    #[inline(always)]
    const unsafe fn mi_page_all_free(page: *const mi_page_t) -> bool {
        debug_assert!(!page.is_null());
        (*page).used == 0
    }

    #[inline(always)]
    pub const unsafe fn is_in_full(page: &mi_page_t) -> bool {
        page.flags.x.in_full
    }

    #[inline(always)]
    pub const fn set_in_full(page: &mut mi_page_t, in_full: bool) {
        page.flags.x.in_full = in_full
    }

    #[inline(always)]
    pub const unsafe fn has_aligned(page: &mi_page_t) -> bool {
        page.flags.x.has_aligned
    }

    #[inline(always)]
    pub const fn set_has_aligned(page: &mut mi_page_t, has_aligned: bool) {
        page.flags.x.has_aligned = has_aligned
    }

    // Thread free access
    // Untag and read the pointer
    #[inline(always)]
    pub unsafe fn thread_free(page: &mi_page_t) -> *mut mi_block_t {
        page.xthread_free
            .load(Ordering::Relaxed)
            .map_addr(|addr| addr & FLAG_MASK) as *mut mi_block_t
    }

    #[inline(always)]
    pub unsafe fn thread_free_flag(page: &mi_page_t) -> mi_delayed_t {
        let data = *page
            .xthread_free
            .load(Ordering::Relaxed)
            .map_addr(|addr| addr & HAS_DATA);
        match data {
            0 => mi_delayed_t::MI_USE_DELAYED_FREE,
            1 => mi_delayed_t::MI_DELAYED_FREEING,
            2 => mi_delayed_t::MI_NO_DELAYED_FREE,
            3 => mi_delayed_t::MI_NEVER_DELAYED_FREE,
            _ => unimplemented!(),
        }
    }

    // Heap access
    #[inline(always)]
    pub unsafe fn heap(page: &mi_page_t) -> *mut crate::heap::Heap {
        page.xheap.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub unsafe fn set_heap(page: &mut mi_page_t, heap: *mut crate::heap::Heap) {
        debug_assert!(
            Self::thread_free_flag(page) as usize != mi_delayed_t::MI_DELAYED_FREEING as usize
        );
        page.xheap.store(heap, Ordering::Release);
    }
}

impl Default for mi_page_s {
    fn default() -> Self {
        Self::new()
    }
}

pub const mi_page_uninit: mi_page_s = mi_page_s::new();

pub type mi_page_t = mi_page_s;

pub type mi_slice_t = mi_page_t;

// Maximum slice offset (15)
const MI_MAX_SLICE_OFFSET: usize = (MI_ALIGNMENT_MAX / MI_SEGMENT_SLICE_SIZE) - 1;

enum mi_page_kind_e {
    MI_PAGE_SMALL,  // small blocks go into 64KiB pages inside a segment
    MI_PAGE_MEDIUM, // medium blocks go into medium pages inside a segment
    MI_PAGE_LARGE,  // larger blocks go into a page of just one block
    MI_PAGE_HUGE,   // huge blocks (> 16 MiB) are put into a single page in a single segment.
}
type mi_page_kind_t = mi_page_kind_e;
