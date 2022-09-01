use crate::heap::mi_heap_t;
use crate::page::mi_page_t;
use crate::constants::MI_INTPTR_BITS;
use crate::{block::mi_block_t, thread::mi_delayed_t};
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::segment::mi_segment_t;

#[inline(always)]
pub const fn mi_clz(x: usize) -> usize {
    if (x == 0) {
        return MI_INTPTR_BITS;
    }
    x.leading_zeros() as usize
    //   #if (INTPTR_MAX == LONG_MAX)
    //     return __builtin_clzl(x);
    //   #else
    //     return __builtin_clzll(x);
    //   #endif
}

#[inline(always)]
pub const fn mi_ctz(x: usize) -> usize {
    if (x == 0) {
        return MI_INTPTR_BITS;
    }
    x.trailing_zeros() as usize
    //   #if (INTPTR_MAX == LONG_MAX)
    //     return __builtin_ctzl(x);
    //   #else
    //     return __builtin_ctzll(x);
    //   #endif
}

#[inline(always)]
pub const fn mi_bsr(x: usize) -> usize {
    if x == 0 {
        MI_INTPTR_BITS
    } else {
        MI_INTPTR_BITS - 1 - mi_clz(x)
    }
}

/* ----------------------------------------------------------------------------------------
The thread local default heap: `_mi_get_default_heap` returns the thread local heap.
On most platforms (Windows, Linux, FreeBSD, NetBSD, etc), this just returns a
__thread local variable (`_mi_heap_default`). With the initial-exec TLS model this ensures
that the storage will always be available (allocated on the thread stacks).
On some platforms though we cannot use that when overriding `malloc` since the underlying
TLS implementation (or the loader) will call itself `malloc` on a first access and recurse.
We try to circumvent this in an efficient way:
- macOSX : we use an unused TLS slot from the OS allocated slots (MI_TLS_SLOT). On OSX, the
           loader itself calls `malloc` even before the modules are initialized.
- OpenBSD: we use an unused slot from the pthread block (MI_TLS_PTHREAD_SLOT_OFS).
- DragonFly: defaults are working but seem slow compared to freeBSD (see PR #323)
------------------------------------------------------------------------------------------- */

// Align upwards
#[inline(always)]
pub const fn _mi_align_up(sz: usize, alignment: usize) -> usize {
    debug_assert!(alignment != 0);
    let mask: usize = alignment - 1;
    if (alignment & mask) == 0 {
        // power of two?
        return (sz + mask) & !mask;
    } else {
        return ((sz + mask) / alignment) * alignment;
    }
}

// Align downwards
#[inline(always)]
pub const fn _mi_align_down(sz: usize, alignment: usize) -> usize {
    debug_assert!(alignment != 0);
    let mask: usize = alignment - 1;
    if (alignment & mask) == 0 {
        // power of two?
        return sz & !mask;
    } else {
        return (sz / alignment) * alignment;
    }
}

// Divide upwards: `s <= _mi_divide_up(s,d)*d < s+d`.
#[inline(always)]
pub const fn _mi_divide_up(size: usize, divider: usize) -> usize {
    debug_assert!(divider != 0);
    if divider == 0 {
        size
    } else {
        ((size + divider - 1) / divider)
    }
}

// Is memory zero initialized?
#[inline(always)]
pub const fn mi_mem_is_zero(p: *const u8, size: usize) -> bool {
    let mut i: usize = 0;
    while i <= size {
        unsafe {
            if *p.offset(i as isize) != 0 {
                return false;
            }
        }
        i += 1;
    }
    true
}

// Align a byte size to a size in _machine words_,
// i.e. byte size == `wsize*sizeof(void*)`.
#[inline(always)]
pub const fn _mi_wsize_from_size(size: usize) -> usize {
    debug_assert!(size <= usize::MAX - std::mem::size_of::<usize>());
    (size + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>()
}

// are all blocks in a page freed?
// note: needs up-to-date used count, (as the `xthread_free` list may not be empty). see
// `_mi_page_collect_free`.
// #[inline(always)]
// pub const unsafe fn mi_page_all_free(page: *const mi_page_t) -> bool {
//     debug_assert!(!page.is_null());
//     ((*page).used == 0)
// }

//   // are there any available blocks?
//   #[inline(always)]
//   pub const unsafe fn mi_page_has_any_available(page: *const mi_page_t) -> bool {
//     debug_assert!(!page.is_null() && (*page).reserved > 0);
//     (*page).used < (*page).reserved || (!mi_page_thread_free(page).is_null())

//   }

//   static inline bool mi_page_has_any_available(const mi_page_t* page) {
//     mi_assert_internal(page != NULL && page->reserved > 0);
//     return (page->used < page->reserved || (mi_page_thread_free(page) != NULL));
//   }

//   // are there immediately available blocks, i.e. blocks available on the free list.
//   static inline bool mi_page_immediate_available(const mi_page_t* page) {
//     mi_assert_internal(page != NULL);
//     return (page->free != NULL);
//   }

//   // is more than 7/8th of a page in use?
//   static inline bool mi_page_mostly_used(const mi_page_t* page) {
//     if (page==NULL) return true;
//     uint16_t frac = page->reserved / 8U;
//     return (page->reserved - page->used <= frac);
//   }

//   static inline mi_page_queue_t* mi_page_queue(const mi_heap_t* heap, size_t size) {
//     return &((mi_heap_t*)heap)->pages[_mi_bin(size)];
//   }

//-----------------------------------------------------------
// Page flags
//-----------------------------------------------------------
#[inline(always)]
pub const unsafe fn mi_page_is_in_full(page: &mi_page_t) -> bool {
    mi_page_t::is_in_full(page)
}

#[inline(always)]
pub const fn mi_page_set_in_full(page: &mut mi_page_t, in_full: bool) {
    mi_page_t::set_in_full(page, in_full)
}

#[inline(always)]
pub const unsafe fn mi_page_has_aligned(page: &mi_page_t) -> bool {
    mi_page_t::has_aligned(page)
}

#[inline(always)]
pub const fn mi_page_set_has_aligned(page: &mut mi_page_t, has_aligned: bool) {
    mi_page_t::set_has_aligned(page, has_aligned)
}

// size of a segment
#[inline(always)]
pub const fn mi_segment_size(segment: &mi_segment_t) -> usize {
    mi_segment_t::mi_segment_size(segment)
}

#[inline(always)]
pub const unsafe fn mi_segment_end(segment: &mi_segment_t) -> *const u8 {
    mi_segment_t::mi_segment_end(segment)
}

// Thread free access
// Untag and read the pointer
#[inline(always)]
pub unsafe fn mi_page_thread_free(page: &mi_page_t) -> *mut mi_block_t {
    mi_page_t::thread_free(page)
}

#[inline(always)]
pub unsafe fn mi_page_thread_free_flag(page: &mi_page_t) -> mi_delayed_t {
    mi_page_t::thread_free_flag(page)
}

// Heap access
#[inline(always)]
pub unsafe fn mi_page_heap(page: &mi_page_t) -> *mut mi_heap_t {
    mi_page_t::heap(page)
}

#[inline(always)]
pub unsafe fn mi_page_set_heap(page: &mut mi_page_t, heap: *mut mi_heap_t) {
    mi_page_t::set_heap(page, heap)
}
