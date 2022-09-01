// ------------------------------------------------------
// Heaps
// Provide first-class heaps to allocate from.
// A heap just owns a set of pages for allocation and
// can only be allocate/reallocate from the thread that created it.
// Freeing blocks can be done from any thread though.
// Per thread, the segments are shared among its heaps.
// Per thread, there is always a default heap that is
// used for allocation; it is initialized to statically
// point to an empty heap to avoid initialization checks
// in the fast path.
// ------------------------------------------------------

use crate::{
    block::mi_block_t,
    constants::{MI_ALIGN2W, MI_ALIGN4W, MI_ALIGNMENT, MI_INTPTR_SIZE},
    internal::{_mi_align_up, _mi_wsize_from_size, mi_bsr},
    os::_mi_os_page_size,
    page::{mi_page_t, mi_page_uninit, MI_MEDIUM_OBJ_SIZE_MAX, MI_MEDIUM_OBJ_WSIZE_MAX},
    page_queue::{mi_page_queue_t, mi_page_queue_uninit},
    thread::{mi_threadid_t, ThreadLocalData},
};
use std::{borrow::BorrowMut, cell::UnsafeCell, sync::atomic::AtomicPtr};

// Random context
#[derive(Debug, Clone, Copy)]
struct mi_random_cxt_s {
    input: [u32; 16],
    output: [u32; 16],
    output_available: i32,
}
impl mi_random_cxt_s {
    const fn new() -> Self {
        Self {
            input: [0; 16],
            output: [0; 16],
            output_available: 0,
        }
    }
}

impl Default for mi_random_cxt_s {
    fn default() -> Self {
        Self::new()
    }
}

type mi_random_ctx_t = mi_random_cxt_s;

// In debug mode there is a padding structure at the end of the blocks to check for buffer overflows
// #if (MI_PADDING)
struct mi_padding_s {
    canary: u32, // encoded block value to check validity of the padding (in case of overflow)
    delta: u32, // padding bytes before the block. (mi_usable_size(p) - delta == exact allocated bytes)
}

type mi_padding_t = mi_padding_s;

const MI_PADDING_SIZE: usize = std::mem::size_of::<mi_padding_t>();
const MI_PADDING_WSIZE: usize = ((MI_PADDING_SIZE + MI_INTPTR_SIZE - 1) / MI_INTPTR_SIZE);
// #else
// #define MI_PADDING_SIZE 0
// #define MI_PADDING_WSIZE 0
// #endif

// ------------------------------------------------------
// Extended functionality
// ------------------------------------------------------
const MI_SMALL_WSIZE_MAX: usize = (128);
const MI_SMALL_SIZE_MAX: usize = (MI_SMALL_WSIZE_MAX * std::mem::size_of::<*mut usize>());

const MI_PAGES_DIRECT: usize = MI_SMALL_WSIZE_MAX + MI_PADDING_WSIZE + 1;

// A heap owns a set of pages.
// #[derive(Clone)]
pub struct Heap {
    tld: *mut ThreadLocalData,
    pages_free_direct: [mi_page_t; MI_PAGES_DIRECT], // optimize: array where every entry points a page with possibly free blocks in the corresponding queue for that size.
    pages: [mi_page_queue_t; Self::MI_BIN_FULL + 1], // queue of pages for each size class (or "bin")
    thread_delayed_free: AtomicPtr<mi_block_t>,
    thread_id: mi_threadid_t, // thread this heap belongs too
    cookie: usize,            // random cookie to verify pointers (see `_mi_ptr_cookie`)
    keys: [usize; 2],         // two random keys used to encode the `thread_delayed_free` list
    random: mi_random_ctx_t,  // random number context used for secure allocation
    page_count: usize,        // total number of pages in the `pages` queues.
    page_retired_min: usize, // smallest retired index (retired pages are fully free, but still in the page queues)
    page_retired_max: usize, // largest retired index into the `pages` array.
    next: *mut Heap,         // list of heaps per thread
    no_reclaim: bool,        // `true` if this heap should not reclaim abandoned pages
}

impl Heap {
    pub const _mi_heap_empty: Heap = Heap::new(); // read-only empty heap, initial value of the thread local default heap

    pub const fn new() -> Self {
        Self {
            tld: std::ptr::null_mut(),
            pages_free_direct: [mi_page_uninit; MI_PAGES_DIRECT],
            pages: [mi_page_queue_uninit; Self::MI_BIN_FULL + 1],
            thread_delayed_free: AtomicPtr::new(std::ptr::null_mut()),
            thread_id: 0,
            cookie: 0,
            keys: [0, 0],
            random: mi_random_ctx_t::new(),
            page_count: 0,
            page_retired_min: 0,
            page_retired_max: 0,
            next: std::ptr::null_mut(),
            no_reclaim: false,
        }
    }
    #[cfg(debug_assertions)]
    pub const unsafe fn mi_heap_contains_queue(
        heap: *const Heap,
        pq: *const mi_page_queue_t,
    ) -> bool
    where
        *const mi_page_queue_t: ~const PartialOrd,
    {
        pq >= &(*heap).pages[0] && pq <= &(*heap).pages[Self::MI_BIN_FULL]
        // consider introducing a `where` clause, but there might be an alternative better way to express this requirement: ` where *const mi_page_queue_s: ~const PartialOrd`
    }

    pub const fn bin_size(bin: u8) -> usize {
        return Heap::_mi_heap_empty.pages[bin as usize].block_size;
    }

    pub const unsafe fn get_page_queue(heap: *mut Heap, bin: u8) -> *mut mi_page_queue_t {
        (*heap).pages[bin as usize..].as_mut_ptr()
    }

    /* -----------------------------------------------------------
      Bins
    ----------------------------------------------------------- */

    // Maximum number of size classes. (spaced exponentially in 12.5% increments)
    pub const MI_BIN_HUGE: usize = 73;
    pub const MI_BIN_FULL: usize = (Self::MI_BIN_HUGE + 1);

    // Return the bin for a given field size.
    // Returns MI_BIN_HUGE if the size is too large.
    // We use `wsize` for the size in "machine word sizes",
    // i.e. byte size == `wsize*sizeof(void*)`.
    #[inline(always)]
    pub const fn mi_bin(size: usize) -> u8 {
        let mut wsize: usize = _mi_wsize_from_size(size);
        let bin: u8;
        if wsize <= 1 {
            bin = 1;
        } else if MI_ALIGNMENT == MI_ALIGN4W && wsize <= 4 {
            bin = ((wsize + 1) & !1) as u8; // round to double word sizes
        } else if MI_ALIGNMENT == MI_ALIGN2W && wsize <= 8 {
            bin = ((wsize + 1) & !1) as u8; // round to double word sizes
        } else if wsize <= 8 {
            bin = wsize as u8;
        } else if wsize > MI_MEDIUM_OBJ_WSIZE_MAX {
            bin = Self::MI_BIN_HUGE as u8;
        } else {
            if wsize <= 16 && MI_ALIGNMENT == MI_ALIGN4W {
                wsize = (wsize + 3) & !3;
            } // round to 4x word sizes
            wsize -= 1;
            // find the highest bit
            let b = mi_bsr(wsize) as u8; // note: wsize != 0
                                         // and use the top 3 bits to determine the bin (~12.5% worst internal fragmentation).
                                         // - adjust with 3 because we use do not round the first 8 sizes
                                         //   which each get an exact bin
            bin = ((b << 2) + ((wsize >> (b - 2)) & 0x03) as u8) - 3;
            debug_assert!((bin as usize) < Self::MI_BIN_HUGE);
        }
        debug_assert!(bin > 0 && (bin as usize) <= Self::MI_BIN_HUGE);
        bin
    }

    // Good size for allocation
    pub const fn mi_good_size(size: usize) -> usize {
        if size <= MI_MEDIUM_OBJ_SIZE_MAX {
            return Self::bin_size(Self::mi_bin(size));
        } else {
            return _mi_align_up(size, _mi_os_page_size());
        }
    }

    // TODO: implement
    // pub const unsafe fn _mi_heap_main_get() -> *mut Self {
    //     // mi_heap_main_init();
    //     return &_mi_heap_main;
    // }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

// unsafe impl Send for mi_heap_s {}

// unsafe impl Sync for mi_heap_s {}

pub const _mi_heap_main: UnsafeCell<Heap> = UnsafeCell::new(Heap::new());

pub const _mi_process_is_initialized: UnsafeCell<bool> = UnsafeCell::new(false);
// mi_heap_t*  _mi_heap_main_get(void);    // statically allocated main backing heap

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
}
