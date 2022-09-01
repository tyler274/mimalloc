use crate::{page::mi_slice_t, segment::MI_SEGMENT_BIN_MAX};

// A "span" is is an available range of slices. The span queues keep
// track of slice spans of at most the given `slice_count` (but more than the previous size class).
#[derive(Clone, Copy)]
pub struct mi_span_queue_s {
    first: *mut mi_slice_t,
    last: *mut mi_slice_t,
    slice_count: usize,
}

impl mi_span_queue_s {
    const fn new(size: usize) -> Self {
        Self {
            first: std::ptr::null_mut(),
            last: std::ptr::null_mut(),
            slice_count: size,
        }
    }
    const queue_sizes: [usize; MI_SEGMENT_BIN_MAX + 1] = [
        1, 1, 2, 3, 4, 5, 6, 7, 10, /* 8 */ 12, 14, 16, 20, 24, 28, 32, 40, /* 16 */ 48,
        56, 64, 80, 96, 112, 128, 160, /* 24 */ 192, 224, 256, 320, 384, 448, 512,
        640, /* 32 */
        768, 896, 1024, /* 35 */
    ];

    pub const MI_SEGMENT_SPAN_QUEUES_EMPTY: [mi_span_queue_s; MI_SEGMENT_BIN_MAX + 1] = {
        let mut empty_queues = Some([Self::new(0); MI_SEGMENT_BIN_MAX + 1]);
        let mut i = 0;
        while i < Self::queue_sizes.len() {
            empty_queues.unwrap()[i] = Self::new(Self::queue_sizes[i]);
            i += 1;
        }
        empty_queues.unwrap()
    };
}

impl Default for mi_span_queue_s {
    fn default() -> Self {
        Self::new(0)
    }
}

pub type mi_span_queue_t = mi_span_queue_s;
