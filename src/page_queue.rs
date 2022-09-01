use crate::internal::{_mi_align_up, _mi_heap_empty, mi_page_heap};
use crate::internal::{_mi_wsize_from_size, mi_bsr};
use crate::os::_mi_os_page_size;
use crate::types::mi_heap_t;
use crate::types::MI_BIN_FULL;
use crate::types::{
    mi_page_queue_t, mi_page_t, MI_BIN_HUGE, MI_INTPTR_SIZE, MI_MAX_ALIGN_SIZE,
    MI_MEDIUM_OBJ_SIZE_MAX, MI_MEDIUM_OBJ_WSIZE_MAX,
};

const MI_ALIGN4W: usize = 4;
const MI_ALIGN2W: usize = 2;
const MI_ALIGN1W: usize = 1;
const MI_ALIGNMENT: usize = if MI_MAX_ALIGN_SIZE > (4 * MI_INTPTR_SIZE) {
    unimplemented!()
} else if (MI_MAX_ALIGN_SIZE > 2 * MI_INTPTR_SIZE) {
    MI_ALIGN4W
} else if (MI_MAX_ALIGN_SIZE > MI_INTPTR_SIZE) {
    MI_ALIGN2W
} else {
    MI_ALIGN1W
};

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

/* -----------------------------------------------------------
  Bins
----------------------------------------------------------- */

// Return the bin for a given field size.
// Returns MI_BIN_HUGE if the size is too large.
// We use `wsize` for the size in "machine word sizes",
// i.e. byte size == `wsize*sizeof(void*)`.
#[inline(always)]
const fn mi_bin(size: usize) -> u8 {
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
        bin = MI_BIN_HUGE as u8;
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
        debug_assert!((bin as usize) < MI_BIN_HUGE);
    }
    debug_assert!(bin > 0 && (bin as usize) <= MI_BIN_HUGE);
    bin
}

/* -----------------------------------------------------------
  Queue of pages with free blocks
----------------------------------------------------------- */

const fn _mi_bin(size: usize) -> u8 {
    return mi_bin(size);
}

const fn _mi_bin_size(bin: u8) -> usize {
    return _mi_heap_empty.pages[bin as usize].block_size;
}

// Good size for allocation
pub const fn mi_good_size(size: usize) -> usize {
    if size <= MI_MEDIUM_OBJ_SIZE_MAX {
        return _mi_bin_size(mi_bin(size));
    } else {
        return _mi_align_up(size, _mi_os_page_size());
    }
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

pub unsafe fn mi_page_queue_of(page: *const mi_page_t) -> *mut mi_page_queue_t {
    let bin: u8 = if mi_page_t::mi_page_is_in_full(page) {
        MI_BIN_FULL as u8
    } else {
        mi_bin((*page).xblock_size as usize)
    };
    let heap: *mut mi_heap_t = mi_page_heap(page);
    // mi_assert_internal(heap != NULL && bin <= MI_BIN_FULL);
    debug_assert!(!heap.is_null() && bin <= MI_BIN_FULL as u8);
    let pq: *mut mi_page_queue_t = (*heap).pages[bin as usize..].as_mut_ptr();
    debug_assert!(bin >= MI_BIN_HUGE as u8 || (*page).xblock_size == (*pq).block_size as u32);
    // mi_assert_expensive(mi_page_queue_contains(pq, page));
    return pq;
}
