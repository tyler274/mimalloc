use crate::page::mi_slice_t;

// A "span" is is an available range of slices. The span queues keep
// track of slice spans of at most the given `slice_count` (but more than the previous size class).
pub struct mi_span_queue_s {
    first: *mut mi_slice_t,
    last: *mut mi_slice_t,
    slice_count: usize,
}

impl mi_span_queue_s {
    const fn new() -> Self {
        Self {
            first: std::ptr::null_mut(),
            last: std::ptr::null_mut(),
            slice_count: 0,
        }
    }
}

impl Default for mi_span_queue_s {
    fn default() -> Self {
        Self::new()
    }
}
pub type mi_span_queue_t = mi_span_queue_s;
