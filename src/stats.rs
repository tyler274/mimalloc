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

use crate::heap::mi_heap_t;

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

pub struct mi_stats_s {
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
    normal_bins: [mi_stat_counter_t; mi_heap_t::MI_BIN_HUGE as usize + 1],
    // #endif
}
pub type mi_stats_t = mi_stats_s;

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
