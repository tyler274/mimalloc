use std::sync::atomic::AtomicPtr;

use crate::{
    page::mi_slice_t,
    thread::mi_threadid_t,
    constants::{MI_MiB, MI_INTPTR_SHIFT, MI_SIZE_BITS},
};

// Main tuning parameters for segment sizes
// Sizes for 64-bit (usually divide by two for 32-bit)
pub const MI_SEGMENT_SLICE_SHIFT: usize = 13 + MI_INTPTR_SHIFT; // 64KiB  (32KiB on 32-bit)
pub const MI_SEGMENT_BIN_MAX: usize = 35; // 35 == mi_segment_bin(MI_SLICES_PER_SEGMENT)
pub const MI_SEGMENT_SHIFT: usize = 10 + MI_SEGMENT_SLICE_SHIFT; // 64MiB

// Derived constants
pub const MI_SEGMENT_SIZE: usize = 1 << MI_SEGMENT_SHIFT;
pub const MI_SEGMENT_ALIGN: usize = MI_SEGMENT_SIZE;
pub const MI_SEGMENT_MASK: usize = MI_SEGMENT_SIZE - 1;
pub const MI_SEGMENT_SLICE_SIZE: usize = 1 << MI_SEGMENT_SLICE_SHIFT;
pub const MI_SLICES_PER_SEGMENT: usize = MI_SEGMENT_SIZE / MI_SEGMENT_SLICE_SIZE; // 1024

enum mi_segment_kind_e {
    MI_SEGMENT_NORMAL, // MI_SEGMENT_SIZE size with pages inside.
    MI_SEGMENT_HUGE,   // > MI_LARGE_SIZE_MAX segment with just one huge page inside.
}
type mi_segment_kind_t = mi_segment_kind_e;

// ------------------------------------------------------
// A segment holds a commit mask where a bit is set if
// the corresponding MI_COMMIT_SIZE area is committed.
// The MI_COMMIT_SIZE must be a multiple of the slice
// size. If it is equal we have the most fine grained
// decommit (but setting it higher can be more efficient).
// The MI_MINIMAL_COMMIT_SIZE is the minimal amount that will
// be committed in one go which can be set higher than
// MI_COMMIT_SIZE for efficiency (while the decommit mask
// is still tracked in fine-grained MI_COMMIT_SIZE chunks)
// ------------------------------------------------------

const MI_MINIMAL_COMMIT_SIZE: usize = 2 * MI_MiB;
const MI_COMMIT_SIZE: usize = MI_SEGMENT_SLICE_SIZE; // 64KiB
const MI_COMMIT_MASK_BITS: usize = MI_SEGMENT_SIZE / MI_COMMIT_SIZE;
const MI_COMMIT_MASK_FIELD_BITS: usize = MI_SIZE_BITS;
const MI_COMMIT_MASK_FIELD_COUNT: usize = MI_COMMIT_MASK_BITS / MI_COMMIT_MASK_FIELD_BITS;

// TODO:
// #if (MI_COMMIT_MASK_BITS != (MI_COMMIT_MASK_FIELD_COUNT * MI_COMMIT_MASK_FIELD_BITS))
// #error "the segment size must be exactly divisible by the (commit size * size_t bits)"
// #endif

struct mi_commit_mask_s {
    mask: [usize; MI_COMMIT_MASK_FIELD_COUNT],
}
type mi_commit_mask_t = mi_commit_mask_s;

type mi_msecs_t = i64;

// Segments are large allocated memory blocks (8mb on 64 bit) from
// the OS. Inside segments we allocated fixed size _pages_ that
// contain blocks.
pub struct mi_segment_s {
    memid: usize,           // memory id for arena allocation
    mem_is_pinned: bool, // `true` if we cannot decommit/reset/protect in this memory (i.e. when allocated using large OS pages)
    mem_is_large: bool,  // in large/huge os pages?
    mem_is_committed: bool, // `true` if the whole segment is eagerly committed

    allow_decommit: bool,
    decommit_expire: mi_msecs_t,
    decommit_mask: mi_commit_mask_t,
    commit_mask: mi_commit_mask_t,

    abandoned_next: AtomicPtr<mi_segment_s>,

    // from here is zero initialized
    next: *mut mi_segment_s, // the list of freed segments in the cache (must be first field, see `segment.c:mi_segment_init`)

    abandoned: usize, // abandoned pages (i.e. the original owning thread stopped) (`abandoned <= used`)
    abandoned_visits: usize, // count how often this segment is visited in the abandoned list (to force reclaim it it is too long)
    used: usize,             // count of pages in use
    cookie: usize, // uintptr_t, verify addresses in debug mode: `mi_ptr_cookie(segment) == segment->cookie`

    segment_slices: usize, // for huge segments this may be different from `MI_SLICES_PER_SEGMENT`
    segment_info_slices: usize, // initial slices we are using segment info and possible guard pages.

    // layout like this to optimize access in `mi_free`
    kind: mi_segment_kind_t,
    thread_id: AtomicPtr<mi_threadid_t>, // unique id of the thread owning this segment
    slice_entries: usize, // entries in the `slices` array, at most `MI_SLICES_PER_SEGMENT`
    slices: [mi_slice_t; MI_SLICES_PER_SEGMENT],
}
impl mi_segment_s {
    // size of a segment
    #[inline(always)]
    pub const fn mi_segment_size(segment: &mi_segment_t) -> usize {
        segment.segment_slices * MI_SEGMENT_SLICE_SIZE
    }

    #[inline(always)]
    pub const unsafe fn mi_segment_end(segment: &Self) -> *const u8 {
        (segment as *const Self as *const u8).offset(Self::mi_segment_size(segment) as isize)
    }
}

pub type mi_segment_t = mi_segment_s;

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_segment_size() {
        assert_eq!(true, true);
        todo!()
    }

    #[test]
    fn test_segment_end() {
        assert_eq!(true, true);
        todo!()
    }
}
