use crate::constants::MI_INTPTR_BITS;

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
        sz & !mask
    } else {
        (sz / alignment) * alignment
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
    (size + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>()
}
