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
    bin::MI_BIN_FULL,
    block::mi_block_t,
    constants::MI_INTPTR_SIZE,
    page::{mi_page_t, mi_page_uninit},
    page_queue::{mi_page_queue_t, mi_page_queue_uninit},
    thread::{mi_threadid_t, mi_tld_t},
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
pub struct mi_heap_s {
    tld: *mut mi_tld_t,
    pages_free_direct: [mi_page_t; MI_PAGES_DIRECT], // optimize: array where every entry points a page with possibly free blocks in the corresponding queue for that size.
    pub pages: [mi_page_queue_t; MI_BIN_FULL + 1], // queue of pages for each size class (or "bin")
    thread_delayed_free: AtomicPtr<mi_block_t>,
    thread_id: mi_threadid_t, // thread this heap belongs too
    cookie: usize,            // random cookie to verify pointers (see `_mi_ptr_cookie`)
    keys: [usize; 2],         // two random keys used to encode the `thread_delayed_free` list
    random: mi_random_ctx_t,  // random number context used for secure allocation
    page_count: usize,        // total number of pages in the `pages` queues.
    page_retired_min: usize, // smallest retired index (retired pages are fully free, but still in the page queues)
    page_retired_max: usize, // largest retired index into the `pages` array.
    next: *mut mi_heap_t,    // list of heaps per thread
    no_reclaim: bool,        // `true` if this heap should not reclaim abandoned pages
}

impl mi_heap_s {
    pub const _mi_heap_empty: mi_heap_t = mi_heap_t::new(); // read-only empty heap, initial value of the thread local default heap

    pub const fn new() -> Self {
        Self {
            tld: std::ptr::null_mut(),
            pages_free_direct: [mi_page_uninit; MI_PAGES_DIRECT],
            pages: [mi_page_queue_uninit; MI_BIN_FULL + 1],
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
        heap: *const mi_heap_t,
        pq: *const mi_page_queue_t,
    ) -> bool
    where
        *const mi_page_queue_t: ~const PartialOrd,
    {
        pq >= &(*heap).pages[0] && pq <= &(*heap).pages[MI_BIN_FULL]
        // consider introducing a `where` clause, but there might be an alternative better way to express this requirement: ` where *const mi_page_queue_s: ~const PartialOrd`
    }

    // TODO: implement
    // pub const unsafe fn _mi_heap_main_get() -> *mut Self {
    //     // mi_heap_main_init();
    //     return &_mi_heap_main;
    // }
}

impl Default for mi_heap_s {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for mi_heap_s {}

unsafe impl Sync for mi_heap_s {}

pub type mi_heap_t = mi_heap_s;

pub const _mi_heap_main: UnsafeCell<mi_heap_t> = UnsafeCell::new(mi_heap_t::new());

pub const _mi_process_is_initialized: UnsafeCell<bool> = UnsafeCell::new(false);
// mi_heap_t*  _mi_heap_main_get(void);    // statically allocated main backing heap
