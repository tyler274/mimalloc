use crate::{page::mi_slice_t, segment::MI_SEGMENT_BIN_MAX};

// A "span" is is an available range of slices. The span queues keep
// track of slice spans of at most the given `slice_count` (but more than the previous size class).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpanQueue {
    first: *mut mi_slice_t,
    last: *mut mi_slice_t,
    slice_count: usize,
}

impl SpanQueue {
    const fn new(size: usize) -> Self {
        Self {
            first: std::ptr::null_mut(),
            last: std::ptr::null_mut(),
            slice_count: size,
        }
    }
    const QUEUE_SIZES: [usize; MI_SEGMENT_BIN_MAX + 1] = [
        1, 1, 2, 3, 4, 5, 6, 7, 10, /* 8 */ 12, 14, 16, 20, 24, 28, 32, 40, /* 16 */ 48,
        56, 64, 80, 96, 112, 128, 160, /* 24 */ 192, 224, 256, 320, 384, 448, 512,
        640, /* 32 */
        768, 896, 1024, /* 35 */
    ];

    pub const MI_SEGMENT_SPAN_QUEUES_EMPTY: [SpanQueue; MI_SEGMENT_BIN_MAX + 1] = {
        let mut empty_queues = Some([Self::new(0); MI_SEGMENT_BIN_MAX + 1]);
        let mut i = 0;
        while i < Self::QUEUE_SIZES.len() {
            empty_queues.as_mut().unwrap()[i] = Self::new(Self::QUEUE_SIZES[i]);
            i += 1;
        }
        empty_queues.unwrap()
    };
}

impl Default for SpanQueue {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_queues_empty() {
        assert_eq!(
            SpanQueue::MI_SEGMENT_SPAN_QUEUES_EMPTY[34],
            SpanQueue::new(896)
        );

        assert_eq!(
            SpanQueue::MI_SEGMENT_SPAN_QUEUES_EMPTY[0],
            SpanQueue::new(1)
        );

        assert_eq!(
            SpanQueue::MI_SEGMENT_SPAN_QUEUES_EMPTY[24],
            SpanQueue::new(160)
        )
    }
}
