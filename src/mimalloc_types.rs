use std::sync::atomic::AtomicPtr;

// TODO: Tons of platform specific values need to be set for these constants
// Minimal alignment necessary. On most platforms 16 bytes are needed
// due to SSE registers for example. This must be at least `sizeof(void*)`
const MI_MAX_ALIGN_SIZE: usize = 16; // sizeof(max_align_t)

const MI_INTPTR_SHIFT: usize = 3;
const MI_SIZE_SHIFT: usize = 3;

pub type mi_ssize_t = i64;

const MI_INTPTR_SIZE: usize = 1 << MI_INTPTR_SHIFT;
const MI_INTPTR_BITS: usize = MI_INTPTR_SIZE * 8;

const MI_SIZE_SIZE: usize = 1 << MI_SIZE_SHIFT;
const MI_SIZE_BITS: usize = MI_SIZE_SIZE * 8;

const MI_KiB: usize = 1024;
const MI_MiB: usize = MI_KiB * MI_KiB;
const MI_GiB: usize = MI_MiB * MI_KiB;

// ------------------------------------------------------
// Main internal data-structures
// ------------------------------------------------------

// Main tuning parameters for segment and page sizes
// Sizes for 64-bit (usually divide by two for 32-bit)
const MI_SEGMENT_SLICE_SHIFT: usize = 13 + MI_INTPTR_SHIFT; // 64KiB  (32KiB on 32-bit)

const MI_SEGMENT_SHIFT: usize = 10 + MI_SEGMENT_SLICE_SHIFT; // 64MiB

const MI_SMALL_PAGE_SHIFT: usize = MI_SEGMENT_SLICE_SHIFT; // 64KiB
const MI_MEDIUM_PAGE_SHIFT: usize = 3 + MI_SMALL_PAGE_SHIFT; // 512KiB

// Derived constants
const MI_SEGMENT_SIZE: usize = 1 << MI_SEGMENT_SHIFT;
const MI_SEGMENT_ALIGN: usize = MI_SEGMENT_SIZE;
const MI_SEGMENT_MASK: usize = MI_SEGMENT_SIZE - 1;
const MI_SEGMENT_SLICE_SIZE: usize = 1 << MI_SEGMENT_SLICE_SHIFT;
pub const MI_SLICES_PER_SEGMENT: usize = MI_SEGMENT_SIZE / MI_SEGMENT_SLICE_SIZE; // 1024

const MI_SMALL_PAGE_SIZE: usize = 1 << MI_SMALL_PAGE_SHIFT;
const MI_MEDIUM_PAGE_SIZE: usize = 1 << MI_MEDIUM_PAGE_SHIFT;

const MI_SMALL_OBJ_SIZE_MAX: usize = MI_SMALL_PAGE_SIZE / 4; // 8KiB on 64-bit
const MI_MEDIUM_OBJ_SIZE_MAX: usize = MI_MEDIUM_PAGE_SIZE / 4; // 128KiB on 64-bit
const MI_MEDIUM_OBJ_WSIZE_MAX: usize = MI_MEDIUM_OBJ_SIZE_MAX / MI_INTPTR_SIZE;
const MI_LARGE_OBJ_SIZE_MAX: usize = MI_SEGMENT_SIZE / 2; // 32MiB on 64-bit
const MI_LARGE_OBJ_WSIZE_MAX: usize = MI_LARGE_OBJ_SIZE_MAX / MI_INTPTR_SIZE;

// Maximum number of size classes. (spaced exponentially in 12.5% increments)
const MI_BIN_HUGE: usize = 73;

// TODO: Implement the following constant checks.
// #if (MI_MEDIUM_OBJ_WSIZE_MAX >= 655360)
// #error "mimalloc internal: define more bins"
// #endif
// #if (MI_ALIGNMENT_MAX > MI_SEGMENT_SIZE/2)
// #error "mimalloc internal: the max aligned boundary is too large for the segment size"
// #endif
// #if (MI_ALIGNED_MAX % MI_SEGMENT_SLICE_SIZE != 0)
// #error "mimalloc internal: the max aligned boundary must be an integral multiple of the segment slice size"
// #endif
const MI_ALIGNMENT_MAX: usize = 1024 * 1024; // maximum supported alignment is 1MiB

// Maximum slice offset (15)
const MI_MAX_SLICE_OFFSET: usize = (MI_ALIGNMENT_MAX / MI_SEGMENT_SLICE_SIZE) - 1;

// Used as a special value to encode block sizes in 32 bits.
const MI_HUGE_BLOCK_SIZE: u32 = (2 * MI_GiB) as u32;

// blocks up to this size are always allocated aligned
const MI_MAX_ALIGN_GUARANTEE: usize = 8 * MI_MAX_ALIGN_SIZE;

// ------------------------------------------------------
// Mimalloc pages contain allocated blocks
// ------------------------------------------------------

// The free lists use encoded next fields
// (Only actually encodes when MI_ENCODED_FREELIST is defined.)
type mi_encoded_t = usize;

// thread id's
type mi_threadid_t = usize;

// free lists contain blocks
#[derive(Debug, Copy, Clone)]
struct mi_block_s {
    next: mi_encoded_t,
}

type mi_block_t = mi_block_s;

#[derive(Debug, Copy, Clone)]
enum mi_delayed_e {
    MI_USE_DELAYED_FREE = 0,   // push on the owning heap thread delayed list
    MI_DELAYED_FREEING = 1,    // temporary: another thread is accessing the owning heap
    MI_NO_DELAYED_FREE = 2, // optimize: push on page local thread free queue if another block is already in the heap thread delayed free list
    MI_NEVER_DELAYED_FREE = 3, // sticky, only resets on page reclaim
}

type mi_delayed_t = mi_delayed_e;

#[derive(Debug, Copy, Clone)]
struct embedded_flag {
    in_full: u8,
    has_aligned: u8,
}
impl Default for embedded_flag {
    fn default() -> Self {
        Self {
            in_full: 1,
            has_aligned: 1,
        }
    }
}

#[derive(Copy, Clone)]
union mi_page_flags_s {
    full_aligned: u8,
    x: embedded_flag,
}

type mi_page_flags_t = mi_page_flags_s;

// Thread free list.
// We use the bottom 2 bits of the pointer for mi_delayed_t flags
type mi_thread_free_t = usize;

// A page contains blocks of one specific size (`block_size`).
// Each page has three list of free blocks:
// `free` for blocks that can be allocated,
// `local_free` for freed blocks that are not yet available to `mi_malloc`
// `thread_free` for freed blocks by other threads
// The `local_free` and `thread_free` lists are migrated to the `free` list
// when it is exhausted. The separate `local_free` list is necessary to
// implement a monotonic heartbeat. The `thread_free` list is needed for
// avoiding atomic operations in the common case.
//
//
// `used - |thread_free|` == actual blocks that are in use (alive)
// `used - |thread_free| + |free| + |local_free| == capacity`
//
// We don't count `freed` (as |free|) but use `used` to reduce
// the number of memory accesses in the `mi_page_all_free` function(s).
//
// Notes:
// - Access is optimized for `mi_free` and `mi_page_alloc` (in `alloc.c`)
// - Using `uint16_t` does not seem to slow things down
// - The size is 8 words on 64-bit which helps the page index calculations
//   (and 10 words on 32-bit, and encoded free lists add 2 words. Sizes 10
//    and 12 are still good for address calculation)
// - To limit the structure size, the `xblock_size` is 32-bits only; for
//   blocks > MI_HUGE_BLOCK_SIZE the size is determined from the segment page size
// - `thread_free` uses the bottom bits as a delayed-free flags to optimize
//   concurrent frees where only the first concurrent free adds to the owning
//   heap `thread_delayed_free` list (see `alloc.c:mi_free_block_mt`).
//   The invariant is that no-delayed-free is only set if there is
//   at least one block that will be added, or as already been added, to
//   the owning heap `thread_delayed_free` list. This guarantees that pages
//   will be freed correctly even if only other threads free blocks.
struct mi_page_s {
    // "owned" by the segment
    slice_count: u32,  // slices in this page (0 if not a page)
    slice_offset: u32, // distance from the actual page data slice (0 if a page)

    is_reset: u8,     // `true` if the page memory was reset
    is_committed: u8, // `true` if the page virtual memory is committed
    is_zero_init: u8, // `true` if the page was zero initialized

    // layout like this to optimize access in `mi_malloc` and `mi_free`
    capacity: u16, // number of blocks committed, must be the first field, see `segment.c:page_clear`
    reserved: u16, // number of blocks reserved in memory
    flags: mi_page_flags_t, // `in_full` and `has_aligned` flags (8 bits)

    is_zero: u8,       // `true` if the blocks in the free list are zero initialized
    retire_expire: u8, // expiration count for retired blocks

    free: *mut mi_block_t, // list of available free blocks (`malloc` allocates from this list)
    // TODO:
    // #ifdef MI_ENCODE_FREELIST
    //   uintptr_t keys[2]; // two random keys to encode the free lists (see `_mi_block_next`)
    // #endif
    used: u32, // number of blocks in use (including blocks in `local_free` and `thread_free`)
    xblock_size: u32, // size available in each block (always `>0`)

    local_free: *mut mi_block_t, // list of deferred free blocks by this thread (migrates to `free`)
    xthread_free: AtomicPtr<mi_thread_free_t>, // list of deferred free blocks freed by other threads
    xheap: AtomicPtr<usize>,

    next: *mut mi_page_s, // next page owned by this thread with the same `block_size`
    prev: *mut mi_page_s, // previous page owned by this thread with the same `block_size`

                          // TODO:
                          // 64-bit 9 words, 32-bit 12 words, (+2 for secure)
                          // #if MI_INTPTR_SIZE == 8
                          // uintptr_t padding[1];
                          // #endif
}

impl Default for mi_page_s {
    fn default() -> Self {
        Self {
            slice_count: 0,
            slice_offset: 0,

            is_reset: 1,
            is_committed: 1,
            is_zero_init: 1,

            capacity: todo!(),
            reserved: todo!(),
            flags: todo!(),

            is_zero: 1,
            retire_expire: 7,

            free: todo!(),
            used: todo!(),
            xblock_size: todo!(),

            local_free: todo!(),
            xthread_free: todo!(),
            xheap: todo!(),

            next: todo!(),
            prev: todo!(),
        }
    }
}
type mi_page_t = mi_page_s;

enum mi_page_kind_e {
    MI_PAGE_SMALL,  // small blocks go into 64KiB pages inside a segment
    MI_PAGE_MEDIUM, // medium blocks go into medium pages inside a segment
    MI_PAGE_LARGE,  // larger blocks go into a page of just one block
    MI_PAGE_HUGE,   // huge blocks (> 16 MiB) are put into a single page in a single segment.
}
type mi_page_kind_t = mi_page_kind_e;

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

type mi_slice_t = mi_page_t;
type mi_msecs_t = i64;

// Segments are large allocated memory blocks (8mb on 64 bit) from
// the OS. Inside segments we allocated fixed size _pages_ that
// contain blocks.
struct mi_segment_s {
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
type mi_segment_t = mi_segment_s;

// ------------------------------------------------------
// Heaps
// Provide first-class heaps to allocate from.
// A heap just owns a set of pages for allocation and
// can only be allocate/reallocate from the thread that created it.
// Freeing blocks can be done from any thread though.
// Per thread, the segments are shared among its heaps.
// Per thread, there is always a default heap that is
// used for allocation; it is initialized to statically
// point to an empty heap to avoid initialization checks
// in the fast path.
// ------------------------------------------------------

// Pages of a certain block size are held in a queue.
struct mi_page_queue_s {
    first: *mut mi_page_t,
    last: *mut mi_page_t,
    block_size: usize,
}
type mi_page_queue_t = mi_page_queue_s;

const MI_BIN_FULL: usize = (MI_BIN_HUGE + 1);

// Random context
struct mi_random_cxt_s {
    input: [u32; 16],
    output: [u32; 16],
    output_available: i32,
}

type mi_random_ctx_t = mi_random_cxt_s;

// In debug mode there is a padding structure at the end of the blocks to check for buffer overflows
// #if (MI_PADDING)
struct mi_padding_s {
    canary: u32, // encoded block value to check validity of the padding (in case of overflow)
    delta: u32, // padding bytes before the block. (mi_usable_size(p) - delta == exact allocated bytes)
}

type mi_padding_t = mi_padding_s;

const MI_PADDING_SIZE: usize = std::mem::size_of::<mi_padding_t>();
const MI_PADDING_WSIZE: usize = ((MI_PADDING_SIZE + MI_INTPTR_SIZE - 1) / MI_INTPTR_SIZE);
// #else
// #define MI_PADDING_SIZE 0
// #define MI_PADDING_WSIZE 0
// #endif

// ------------------------------------------------------
// Extended functionality
// ------------------------------------------------------
const MI_SMALL_WSIZE_MAX: usize = (128);
const MI_SMALL_SIZE_MAX: usize = (MI_SMALL_WSIZE_MAX * std::mem::size_of::<*mut usize>());

const MI_PAGES_DIRECT: usize = MI_SMALL_WSIZE_MAX + MI_PADDING_WSIZE + 1;

// A heap owns a set of pages.
struct mi_heap_s {
    tld: *mut mi_tld_t,
    pages_free_direct: [mi_page_t; MI_PAGES_DIRECT], // optimize: array where every entry points a page with possibly free blocks in the corresponding queue for that size.
    pages: [mi_page_queue_t; MI_BIN_FULL + 1], // queue of pages for each size class (or "bin")
    thread_delayed_free: AtomicPtr<mi_block_t>,
    thread_id: mi_threadid_t, // thread this heap belongs too
    cookie: usize,            // random cookie to verify pointers (see `_mi_ptr_cookie`)
    keys: [usize; 2],         // two random keys used to encode the `thread_delayed_free` list
    random: mi_random_ctx_t,  // random number context used for secure allocation
    page_count: usize,        // total number of pages in the `pages` queues.
    page_retired_min: usize, // smallest retired index (retired pages are fully free, but still in the page queues)
    page_retired_max: usize, // largest retired index into the `pages` array.
    next: *mut mi_heap_t,    // list of heaps per thread
    no_reclaim: bool,        // `true` if this heap should not reclaim abandoned pages
}

type mi_heap_t = mi_heap_s;

// ------------------------------------------------------
// Debug
// ------------------------------------------------------

// #if !defined(MI_DEBUG_UNINIT)
const MI_DEBUG_UNINIT: usize = (0xD0);
// #endif
// #if !defined(MI_DEBUG_FREED)
const MI_DEBUG_FREED: usize = (0xDF);
// #endif
// #if !defined(MI_DEBUG_PADDING)
const MI_DEBUG_PADDING: usize = (0xDE);
// #endif

// #if (MI_DEBUG)
// use our own assertion to print without memory allocation
// void _mi_assert_fail(const char *assertion, const char *fname, unsigned int line, const char *func);
// #define mi_assert(expr) ((expr) ? (void)0 : _mi_assert_fail(#expr, __FILE__, __LINE__, __func__))
// #else
// #define mi_assert(x)
// #endif

// #if (MI_DEBUG > 1)
// #define mi_assert_internal mi_assert
// #else
// #define mi_assert_internal(x)
// #endif

// #if (MI_DEBUG > 2)
// #define mi_assert_expensive mi_assert
// #else
// #define mi_assert_expensive(x)
// #endif

// ------------------------------------------------------
// Statistics
// ------------------------------------------------------

// #ifndef MI_STAT
// #if (MI_DEBUG > 0)
// #define MI_STAT 2
// #else
// #define MI_STAT 0
// #endif
// #endif

struct mi_stat_count_s {
    allocated: i64,
    freed: i64,
    peak: i64,
    current: i64,
}
type mi_stat_count_t = mi_stat_count_s;

struct mi_stat_counter_s {
    total: i64,
    count: i64,
}
type mi_stat_counter_t = mi_stat_count_s;

struct mi_stats_s {
    segments: mi_stat_count_t,
    pages: mi_stat_count_t,
    reserved: mi_stat_count_t,
    committed: mi_stat_counter_t,
    reset: mi_stat_counter_t,
    page_committed: mi_stat_counter_t,
    segments_abandoned: mi_stat_counter_t,
    pages_abandoned: mi_stat_counter_t,
    threads: mi_stat_counter_t,
    normal: mi_stat_counter_t,
    huge: mi_stat_counter_t,
    large: mi_stat_counter_t,
    malloc: mi_stat_counter_t,
    segments_cache: mi_stat_counter_t,
    pages_extended: mi_stat_counter_t,
    mmap_calls: mi_stat_counter_t,
    commit_calls: mi_stat_counter_t,
    page_no_retire: mi_stat_counter_t,
    searches: mi_stat_counter_t,
    normal_count: mi_stat_counter_t,
    huge_count: mi_stat_counter_t,
    large_count: mi_stat_counter_t,
    // #if MI_STAT > 1
    normal_bins: [mi_stat_counter_t; MI_BIN_HUGE + 1],
    // #endif
}
type mi_stats_t = mi_stats_s;

// TODO:
// void _mi_stat_increase(mi_stat_count_t *stat, size_t amount);
// void _mi_stat_decrease(mi_stat_count_t *stat, size_t amount);
// void _mi_stat_counter_increase(mi_stat_counter_t *stat, size_t amount);

// #if (MI_STAT)
// #define mi_stat_increase(stat, amount) _mi_stat_increase(&(stat), amount)
// #define mi_stat_decrease(stat, amount) _mi_stat_decrease(&(stat), amount)
// #define mi_stat_counter_increase(stat, amount) _mi_stat_counter_increase(&(stat), amount)
// #else
// #define mi_stat_increase(stat, amount) (void)0
// #define mi_stat_decrease(stat, amount) (void)0
// #define mi_stat_counter_increase(stat, amount) (void)0
// #endif

// #define mi_heap_stat_counter_increase(heap, stat, amount) mi_stat_counter_increase((heap)->tld->stats.stat, amount)
// #define mi_heap_stat_increase(heap, stat, amount) mi_stat_increase((heap)->tld->stats.stat, amount)
// #define mi_heap_stat_decrease(heap, stat, amount) mi_stat_decrease((heap)->tld->stats.stat, amount)

// ------------------------------------------------------
// Thread Local data
// ------------------------------------------------------

// A "span" is is an available range of slices. The span queues keep
// track of slice spans of at most the given `slice_count` (but more than the previous size class).
struct mi_span_queue_s {
    first: *mut mi_slice_t,
    last: *mut mi_slice_t,
    slice_count: usize,
}
type mi_span_queue_t = mi_span_queue_s;

const MI_SEGMENT_BIN_MAX: usize = 35; // 35 == mi_segment_bin(MI_SLICES_PER_SEGMENT)

// OS thread local data
struct mi_os_tld_s {
    region_idx: usize,      // start point for next allocation
    stats: *mut mi_stats_t, // points to tld stats
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
struct mi_tld_s {
    heartbeat: u64,               // monotonic heartbeat count
    recurse: bool, // true if deferred was called; used to prevent infinite recursion.
    heap_backing: *mut mi_heap_t, // backing heap of this thread (cannot be deleted)
    heaps: *mut mi_heap_t, // list of heaps in this thread (so we can abandon all when the thread terminates)
    segments: mi_segments_tld_t, // segment tld
    os: mi_os_tld_t,       // os tld
    stats: mi_stats_t,     // statistics
}

type mi_tld_t = mi_tld_s;
