// TODO: Tons of platform specific values need to be set for these constants
// Minimal alignment necessary. On most platforms 16 bytes are needed
// due to SSE registers for example. This must be at least `sizeof(void*)`
pub const MI_MAX_ALIGN_SIZE: usize = 16; // sizeof(max_align_t)

pub const MI_INTPTR_SHIFT: usize = 3;
const MI_SIZE_SHIFT: usize = 3;

pub type mi_ssize_t = i64;

pub const MI_INTPTR_SIZE: usize = 1 << MI_INTPTR_SHIFT;
pub const MI_INTPTR_BITS: usize = MI_INTPTR_SIZE * 8;

pub const MI_SIZE_SIZE: usize = 1 << MI_SIZE_SHIFT;
pub const MI_SIZE_BITS: usize = MI_SIZE_SIZE * 8;

pub const MI_KiB: usize = 1024;
pub const MI_MiB: usize = MI_KiB * MI_KiB;
pub const MI_GiB: usize = MI_MiB * MI_KiB;

// TODO: Implement the following constant checks.
// const bin_check = if (MI_MEDIUM_OBJ_WSIZE_MAX >= 655360) {
//     debug_assert!()
// }
// #if (MI_MEDIUM_OBJ_WSIZE_MAX >= 655360)
// #error "mimalloc internal: define more bins"
// #endif
// #if (MI_ALIGNMENT_MAX > MI_SEGMENT_SIZE/2)
// #error "mimalloc internal: the max aligned boundary is too large for the segment size"
// #endif
// #if (MI_ALIGNED_MAX % MI_SEGMENT_SLICE_SIZE != 0)
// #error "mimalloc internal: the max aligned boundary must be an integral multiple of the segment slice size"
// #endif
pub const MI_ALIGNMENT_MAX: usize = 1024 * 1024; // maximum supported alignment is 1MiB

// blocks up to this size are always allocated aligned
pub const MI_MAX_ALIGN_GUARANTEE: usize = 8 * MI_MAX_ALIGN_SIZE;

pub const MI_ALIGN4W: usize = 4;
pub const MI_ALIGN2W: usize = 2;
pub const MI_ALIGN1W: usize = 1;
pub const MI_ALIGNMENT: usize = if MI_MAX_ALIGN_SIZE > (4 * MI_INTPTR_SIZE) {
    unimplemented!()
} else if (MI_MAX_ALIGN_SIZE > 2 * MI_INTPTR_SIZE) {
    MI_ALIGN4W
} else if (MI_MAX_ALIGN_SIZE > MI_INTPTR_SIZE) {
    MI_ALIGN2W
} else {
    MI_ALIGN1W
};
