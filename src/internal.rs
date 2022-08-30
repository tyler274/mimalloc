use crate::types::{mi_heap_t, MI_INTPTR_BITS};

#[inline(always)]
pub const fn mi_clz(x: usize) -> usize {
    if (x == 0) {
        return MI_INTPTR_BITS;
    }
    x.leading_zeros() as usize
    //   #if (INTPTR_MAX == LONG_MAX)
    //     return __builtin_clzl(x);
    //   #else
    //     return __builtin_clzll(x);
    //   #endif
}

#[inline(always)]
pub const fn mi_ctz(x: usize) -> usize {
    if (x == 0) {
        return MI_INTPTR_BITS;
    }
    x.trailing_zeros() as usize
    //   #if (INTPTR_MAX == LONG_MAX)
    //     return __builtin_ctzl(x);
    //   #else
    //     return __builtin_ctzll(x);
    //   #endif
}

#[inline(always)]
pub const fn mi_bsr(x: usize) -> usize {
    if x == 0 {
        MI_INTPTR_BITS
    } else {
        MI_INTPTR_BITS - 1 - mi_clz(x)
    }
}

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

pub const _mi_heap_empty: mi_heap_t = mi_heap_t::new(); // read-only empty heap, initial value of the thread local default heap
pub static _mi_process_is_initialized: bool = false;
// mi_heap_t*  _mi_heap_main_get(void);    // statically allocated main backing heap

// Align upwards
#[inline(always)]
pub const fn _mi_align_up(sz: usize, alignment: usize) -> usize {
    debug_assert!(alignment != 0);
    let mask: usize = alignment - 1;
    if (alignment & mask) == 0 {
        // power of two?
        return (sz + mask) & !mask;
    } else {
        return ((sz + mask) / alignment) * alignment;
    }
}

// Align downwards
#[inline(always)]
pub const fn _mi_align_down(sz: usize, alignment: usize) -> usize {
    debug_assert!(alignment != 0);
    let mask: usize = alignment - 1;
    if (alignment & mask) == 0 {
        // power of two?
        return sz & !mask;
    } else {
        return (sz / alignment) * alignment;
    }
}

// Divide upwards: `s <= _mi_divide_up(s,d)*d < s+d`.
#[inline(always)]
pub const fn _mi_divide_up(size: usize, divider: usize) -> usize {
    debug_assert!(divider != 0);
    if divider == 0 {
        size
    } else {
        ((size + divider - 1) / divider)
    }
}

// Is memory zero initialized?
#[inline(always)]
pub const fn mi_mem_is_zero(p: *const u8, size: usize) -> bool {
    let mut i: usize = 0;
    while i <= size {
        unsafe {
            if *p.offset(i as isize) != 0 {
                return false;
            }
        }
        i += 1;
    }
    true
}

// Align a byte size to a size in _machine words_,
// i.e. byte size == `wsize*sizeof(void*)`.
#[inline(always)]
pub const fn _mi_wsize_from_size(size: usize) -> usize {
    debug_assert!(size <= usize::MAX - std::mem::size_of::<usize>());
    return (size + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>();
}
