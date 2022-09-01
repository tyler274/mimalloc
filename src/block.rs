use crate::constants::MI_GiB;

// ------------------------------------------------------
// Mimalloc pages contain allocated blocks
// ------------------------------------------------------

// The free lists use encoded next fields
// (Only actually encodes when MI_ENCODED_FREELIST is defined.)
type mi_encoded_t = usize;

// Used as a special value to encode block sizes in 32 bits.
pub const MI_HUGE_BLOCK_SIZE: u32 = (2 * MI_GiB) as u32;

// free lists contain blocks
#[derive(Debug, Copy, Clone)]
pub struct mi_block_s {
    next: mi_encoded_t,
}

impl mi_block_s {
    const fn new() -> Self {
        Self { next: 0 }
    }
}

pub type mi_block_t = mi_block_s;
