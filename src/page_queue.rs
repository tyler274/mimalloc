use crate::internal::{_mi_align_up, _mi_heap_empty};
use crate::internal::{_mi_wsize_from_size, mi_bsr};
use crate::os::_mi_os_page_size;
use crate::types::{
    mi_page_queue_t, MI_BIN_HUGE, MI_INTPTR_SIZE, MI_MAX_ALIGN_SIZE, MI_MEDIUM_OBJ_SIZE_MAX,
    MI_MEDIUM_OBJ_WSIZE_MAX,
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
    if (wsize <= 1) {
        bin = 1;
    } else if (MI_ALIGNMENT == MI_ALIGN4W && wsize <= 4) {
        bin = ((wsize + 1) & !1) as u8; // round to double word sizes
    } else if (MI_ALIGNMENT == MI_ALIGN2W && wsize <= 8) {
        bin = ((wsize + 1) & !1) as u8; // round to double word sizes
    } else if (wsize <= 8) {
        bin = wsize as u8;
    } else if (wsize > MI_MEDIUM_OBJ_WSIZE_MAX) {
        bin = MI_BIN_HUGE as u8;
    } else {
        if (wsize <= 16 && MI_ALIGNMENT == MI_ALIGN4W) {
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

fn _mi_bin_size(bin: u8) -> usize {
    return _mi_heap_empty.pages[bin as usize].block_size;
}

// Good size for allocation
fn mi_good_size(size: usize) -> usize {
    if (size <= MI_MEDIUM_OBJ_SIZE_MAX) {
        return _mi_bin_size(mi_bin(size));
    } else {
        return _mi_align_up(size, _mi_os_page_size());
    }
}
