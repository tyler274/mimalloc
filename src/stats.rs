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

use crate::heap::Heap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
struct StatCount {
    allocated: i64,
    freed: i64,
    peak: i64,
    current: i64,
}

impl StatCount {
    pub const fn new() -> Self {
        Self {
            allocated: 0,
            freed: 0,
            peak: 0,
            current: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
struct StatCounter {
    total: i64,
    count: i64,
}

impl StatCounter {
    pub const fn new() -> Self {
        Self { total: 0, count: 0 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Stats {
    segments: StatCount,
    pages: StatCount,
    reserved: StatCount,
    committed: StatCounter,
    reset: StatCounter,
    page_committed: StatCounter,
    segments_abandoned: StatCounter,
    pages_abandoned: StatCounter,
    threads: StatCounter,
    normal: StatCounter,
    huge: StatCounter,
    large: StatCounter,
    malloc: StatCounter,
    segments_cache: StatCounter,
    pages_extended: StatCounter,
    mmap_calls: StatCounter,
    commit_calls: StatCounter,
    page_no_retire: StatCounter,
    searches: StatCounter,
    normal_count: StatCounter,
    huge_count: StatCounter,
    large_count: StatCounter,
    // #if MI_STAT > 1
    normal_bins: [StatCounter; Heap::MI_BIN_HUGE as usize + 1],
    // #endif
}

impl Stats {
    pub const fn new() -> Self {
        Self {
            segments: StatCount::new(),
            pages: StatCount::new(),
            reserved: StatCount::new(),
            committed: StatCounter::new(),
            reset: StatCounter::new(),
            page_committed: StatCounter::new(),
            segments_abandoned: StatCounter::new(),
            pages_abandoned: StatCounter::new(),
            threads: StatCounter::new(),
            normal: StatCounter::new(),
            huge: StatCounter::new(),
            large: StatCounter::new(),
            malloc: StatCounter::new(),
            segments_cache: StatCounter::new(),
            pages_extended: StatCounter::new(),
            mmap_calls: StatCounter::new(),
            commit_calls: StatCounter::new(),
            page_no_retire: StatCounter::new(),
            searches: StatCounter::new(),
            normal_count: StatCounter::new(),
            huge_count: StatCounter::new(),
            large_count: StatCounter::new(),
            normal_bins: [StatCounter::new(); Heap::MI_BIN_HUGE as usize + 1],
        }
    }
}

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

pub const MI_STAT_COUNT_NULL: [i32; 4] = [0, 0, 0, 0];

// Empty statistics
// pub const MI_STAT_COUNT_END_NULL = if MI_STAT > 1 {

// } else {

// }
// #if MI_STAT>1
// #define MI_STAT_COUNT_END_NULL()  , { MI_STAT_COUNT_NULL(), MI_INIT32(MI_STAT_COUNT_NULL) }
// #else
// #define MI_STAT_COUNT_END_NULL()
// #endif
