// ------------------------------------------------------
// Thread Local and related data
// ------------------------------------------------------

use crate::{
    heap::{Heap, _mi_heap_main},
    segment::MI_SEGMENT_BIN_MAX,
    span_queue::SpanQueue,
    stats::Stats,
};

use std::cell::UnsafeCell;

// thread id's
pub type mi_threadid_t = usize;

// The delayed flags are used for efficient multi-threaded free-ing
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Debug)]
pub struct OsThreadLocalData {
    region_idx: usize,                // start point for next allocation
    stats: Option<UnsafeCell<Stats>>, // points to tld stats
}

impl OsThreadLocalData {
    const fn new() -> Self {
        Self {
            region_idx: 0,
            stats: None,
        }
    }
}

impl Default for OsThreadLocalData {
    fn default() -> Self {
        Self::new()
    }
}

// Segments thread local data
#[derive(Debug)]
struct SegmentsThreadLocalData {
    spans: [SpanQueue; MI_SEGMENT_BIN_MAX + 1], // free slice spans inside segments
    count: usize,                               // current number of segments;
    peak_count: usize,                          // peak number of segments
    current_size: usize,                        // current size of all segments
    peak_size: usize,                           // peak size of all segments
    stats: Option<UnsafeCell<Stats>>,           // points to tld stats
    os: Option<UnsafeCell<OsThreadLocalData>>,  // points to os stats
}

impl SegmentsThreadLocalData {
    pub const fn new() -> Self {
        // todo!();
        Self {
            spans: SpanQueue::MI_SEGMENT_SPAN_QUEUES_EMPTY,
            count: 0,
            peak_count: 0,
            current_size: 0,
            peak_size: 0,
            stats: Some(UnsafeCell::new(Stats::new())),
            os: Some(UnsafeCell::new(OsThreadLocalData::new())),
        }
    }
}

// Thread local data
#[derive(Debug)]
pub struct ThreadLocalData {
    heartbeat: u64,                    // monotonic heartbeat count
    recurse: bool, // true if deferred was called; used to prevent infinite recursion.
    heap_backing: Option<*mut Heap>, // backing heap of this thread (cannot be deleted)
    heaps: Option<*mut Heap>, // list of heaps in this thread (so we can abandon all when the thread terminates)
    segments: SegmentsThreadLocalData, // segment tld
    os: OsThreadLocalData,    // os tld
    stats: Stats,             // statistics
}

impl ThreadLocalData {
    pub const fn new() -> Self {
        // todo!();
        Self {
            heartbeat: 0,
            recurse: false,
            heap_backing: None,
            heaps: None,
            segments: SegmentsThreadLocalData::new(),
            os: OsThreadLocalData::new(),
            stats: Stats::new(),
            // { MI_SEGMENT_SPAN_QUEUES_EMPTY, 0, 0, 0, 0, &tld_main.stats, &tld_main.os }, // segments
            // { 0, &tld_main.stats },  // os
            // { MI_STATS_NULL }       // stats
        }
    }
    pub const unsafe fn init_main(&mut self) {
        self.segments = SegmentsThreadLocalData::new();
    }
}

unsafe impl Send for ThreadLocalData {}

unsafe impl Sync for ThreadLocalData {}

pub const tld_empty_stats: Stats = Stats::new();

pub const tld_empty_os: OsThreadLocalData = OsThreadLocalData::new();

pub const tld_empty: UnsafeCell<ThreadLocalData> = UnsafeCell::new(ThreadLocalData::new());

pub const tld_main: UnsafeCell<ThreadLocalData> = unsafe {
    UnsafeCell::new({
        let mut x = ThreadLocalData::new();
        // x.init();
        x
    })
};

// TODO:
/* ----------------------------------------------------------------------------------------
The thread local default heap: `_mi_get_default_heap` returns the thread local heap.
On most platforms (Windows, Linux, FreeBSD, NetBSD, etc), this just returns a
__thread local variable (`_mi_heap_default`). With the initial-exec TLS model this ensures
that the storage will always be available (allocated on the thread stacks).
On some platforms though we cannot use that when overriding `malloc` since the underlying
TLS implementation (or the loader) will call itself `malloc` on a first access and recurse.
We try to circumvent this in an efficient way:
- macOSX : we use an unused TLS slot from the OS allocated slots (MI_TLS_SLOT). On OSX, the
           loader itself calls `malloc` even before the modules are initialized.
- OpenBSD: we use an unused slot from the pthread block (MI_TLS_PTHREAD_SLOT_OFS).
- DragonFly: defaults are working but seem slow compared to freeBSD (see PR #323)
------------------------------------------------------------------------------------------- */
