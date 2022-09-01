use crate::constants::{MI_ALIGN2W, MI_ALIGN4W, MI_ALIGNMENT, MI_INTPTR_SIZE, MI_MAX_ALIGN_SIZE};
use crate::heap::mi_heap_t;
use crate::internal::_mi_align_up;
use crate::internal::{_mi_wsize_from_size, mi_bsr};
use crate::os::_mi_os_page_size;
use crate::page::{mi_page_t, MI_MEDIUM_OBJ_SIZE_MAX, MI_MEDIUM_OBJ_WSIZE_MAX};

/* -----------------------------------------------------------
  Queue of pages with free blocks
----------------------------------------------------------- */

// Pages of a certain block size are held in a queue.
#[derive(PartialEq, PartialOrd)]
pub struct mi_page_queue_s {
    pub first: *mut mi_page_t,
    pub last: *mut mi_page_t,
    pub block_size: usize,
}
impl mi_page_queue_s {
    const fn new() -> Self {
        Self {
            first: std::ptr::null_mut(),
            last: std::ptr::null_mut(),
            block_size: 0,
        }
    }

    /* -----------------------------------------------------------
      Queue query
    ----------------------------------------------------------- */
    #[inline(always)]
    const fn mi_page_queue_is_huge(pq: mi_page_queue_t) -> bool {
        pq.block_size == MI_MEDIUM_OBJ_SIZE_MAX + std::mem::size_of::<usize>()
    }

    #[inline(always)]
    const fn mi_page_queue_is_full(pq: mi_page_queue_t) -> bool {
        pq.block_size == MI_MEDIUM_OBJ_SIZE_MAX + (2 * std::mem::size_of::<usize>())
    }

    #[inline(always)]
    const fn mi_page_queue_is_special(pq: mi_page_queue_t) -> bool {
        pq.block_size > MI_MEDIUM_OBJ_SIZE_MAX
    }

    pub unsafe fn mi_page_queue_of(page: &mi_page_t) -> *mut mi_page_queue_t {
        let bin: u8 = if mi_page_t::is_in_full(page) {
            mi_heap_t::MI_BIN_FULL as u8
        } else {
            mi_heap_t::mi_bin((*page).xblock_size as usize)
        };
        let heap: *mut mi_heap_t = mi_page_t::heap(page);
        debug_assert!(!heap.is_null() && bin <= mi_heap_t::MI_BIN_FULL as u8);
        let pq: *mut mi_page_queue_t = mi_heap_t::get_page_queue(heap, bin);
        debug_assert!(
            bin >= mi_heap_t::MI_BIN_HUGE as u8 || page.xblock_size == (*pq).block_size as u32
        );
        // TODO: mi_assert_expensive(mi_page_queue_contains(pq, page));
        pq
    }

    #[cfg(debug_assertions)]
    pub const unsafe fn mi_page_queue_contains(
        queue: *const mi_page_queue_t,
        page: *const mi_page_t,
    ) -> bool {
        debug_assert!(!page.is_null());

        let mut list: *mut mi_page_t = (*queue).first;
        while (!list.is_null()) {
            debug_assert!((*list).next.is_null() || (*(*list).next).prev.guaranteed_eq(list));
            debug_assert!((*list).prev.is_null() || (*(*list).prev).next.guaranteed_eq(list));
            if (page.guaranteed_eq(list)) {
                break;
            }
            list = (*list).next;
        }
        page.guaranteed_eq(list)
    }
}

impl Default for mi_page_queue_s {
    fn default() -> Self {
        Self::new()
    }
}

pub const mi_page_queue_uninit: mi_page_queue_s = mi_page_queue_s::new();

pub type mi_page_queue_t = mi_page_queue_s;
