// ------------------------------------------------------
// Thread Local and related data
// ------------------------------------------------------

use crate::{segment::MI_SEGMENT_BIN_MAX, span_queue::mi_span_queue_t, stats::mi_stats_t};

// thread id's
pub type mi_threadid_t = usize;

// The delayed flags are used for efficient multi-threaded free-ing
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum mi_delayed_e {
    MI_USE_DELAYED_FREE = 0,   // push on the owning heap thread delayed list
    MI_DELAYED_FREEING = 1,    // temporary: another thread is accessing the owning heap
    MI_NO_DELAYED_FREE = 2, // optimize: push on page local thread free queue if another block is already in the heap thread delayed free list
    MI_NEVER_DELAYED_FREE = 3, // sticky, only resets on page reclaim
}

impl mi_delayed_e {
    pub const fn new(x: usize) -> Self {
        match x {
            0 => mi_delayed_t::MI_USE_DELAYED_FREE,
            1 => mi_delayed_t::MI_DELAYED_FREEING,
            2 => mi_delayed_t::MI_NO_DELAYED_FREE,
            3 => mi_delayed_t::MI_NEVER_DELAYED_FREE,
            _ => unimplemented!(),
        }
    }
}

pub type mi_delayed_t = mi_delayed_e;

// OS thread local data
#[derive(Debug, Clone)]
struct mi_os_tld_s {
    region_idx: usize,      // start point for next allocation
    stats: *mut mi_stats_t, // points to tld stats
}

impl mi_os_tld_s {
    const fn new() -> Self {
        Self {
            region_idx: 0,
            stats: std::ptr::null_mut(),
        }
    }
}

impl Default for mi_os_tld_s {
    fn default() -> Self {
        Self::new()
    }
}

type mi_os_tld_t = mi_os_tld_s;

// Segments thread local data

struct mi_segments_tld_s {
    spans: [mi_span_queue_t; MI_SEGMENT_BIN_MAX + 1], // free slice spans inside segments
    count: usize,                                     // current number of segments;
    peak_count: usize,                                // peak number of segments
    current_size: usize,                              // current size of all segments
    peak_size: usize,                                 // peak size of all segments
    stats: *mut mi_stats_t,                           // points to tld stats
    os: *mut mi_os_tld_t,                             // points to os stats
}
type mi_segments_tld_t = mi_segments_tld_s;

// Thread local data
pub struct mi_tld_s {
    heartbeat: u64,                            // monotonic heartbeat count
    recurse: bool, // true if deferred was called; used to prevent infinite recursion.
    heap_backing: *mut crate::heap::mi_heap_t, // backing heap of this thread (cannot be deleted)
    heaps: *mut crate::heap::mi_heap_t, // list of heaps in this thread (so we can abandon all when the thread terminates)
    segments: mi_segments_tld_t,        // segment tld
    os: mi_os_tld_t,                    // os tld
    stats: crate::stats::mi_stats_t,    // statistics
}

pub type mi_tld_t = mi_tld_s;
